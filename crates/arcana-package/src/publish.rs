use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::{
    DEFAULT_REGISTRY_NAME, DependencySource, DependencySourceSpec, ForewordAdapterProductSpec,
    LOCAL_REGISTRY_METADATA_FILE, LOCAL_REGISTRY_SNAPSHOT_DIR, NativeProductSpec, PackageResult,
    SemverVersion, WorkspaceGraph, WorkspaceMember, load_workspace_graph,
    local_registry_package_dir,
};

pub fn publish_workspace_member(
    workspace_dir: &Path,
    member_name: &str,
) -> PackageResult<Vec<String>> {
    let graph = load_workspace_graph(workspace_dir)?;
    let member = graph
        .local_member(member_name)
        .ok_or_else(|| format!("workspace has no local member `{member_name}`"))?;
    if member.kind != crate::GrimoireKind::Lib {
        return Err(format!(
            "`arcana publish` currently supports only `kind = \"lib\"`; `{member_name}` is `{}`",
            member.kind.as_str()
        ));
    }

    let publish_ids = collect_publishable_local_closure(&graph, &member.package_id)?;
    let publish_order = crate::plan_workspace(&graph)?
        .into_iter()
        .filter(|package_id| publish_ids.contains(package_id))
        .collect::<Vec<_>>();
    let mut published = Vec::new();
    for package_id in publish_order {
        let member = graph
            .member_by_id(&package_id)
            .ok_or_else(|| format!("workspace is missing package `{package_id}`"))?;
        publish_single_member(&graph, &publish_ids, member)?;
        published.push(member.display_label());
    }
    Ok(published)
}

fn collect_publishable_local_closure(
    graph: &WorkspaceGraph,
    root_package_id: &str,
) -> PackageResult<BTreeSet<String>> {
    let mut pending = vec![root_package_id.to_string()];
    let mut visited = BTreeSet::new();
    while let Some(package_id) = pending.pop() {
        if !visited.insert(package_id.clone()) {
            continue;
        }
        let member = graph
            .member_by_id(&package_id)
            .ok_or_else(|| format!("missing workspace package `{package_id}`"))?;
        if member.source_kind != DependencySource::Path {
            continue;
        }
        if member.kind != crate::GrimoireKind::Lib {
            return Err(format!(
                "published local dependency closure may only contain libs; `{}` is `{}`",
                member.display_label(),
                member.kind.as_str()
            ));
        }
        for dep_id in &member.deps {
            let dep = graph
                .member_by_id(dep_id)
                .ok_or_else(|| format!("missing dependency `{dep_id}`"))?;
            if dep.source_kind == DependencySource::Path {
                pending.push(dep.package_id.clone());
            }
        }
    }
    Ok(visited)
}

fn publish_single_member(
    graph: &WorkspaceGraph,
    publish_ids: &BTreeSet<String>,
    member: &WorkspaceMember,
) -> PackageResult<()> {
    let version = member.version.as_ref().ok_or_else(|| {
        format!(
            "published package `{}` must declare `version = \"x.y.z\"`",
            member.name
        )
    })?;
    let native_product_sidecar_hashes = compute_native_product_sidecar_hashes(member)?;
    let metadata_text = render_published_manifest(
        graph,
        publish_ids,
        member,
        version,
        &native_product_sidecar_hashes,
    )?;
    let snapshot_paths = collect_snapshot_paths(member)?;
    let checksum = compute_publish_checksum(member, &metadata_text, &snapshot_paths)?;
    let package_dir = local_registry_package_dir(DEFAULT_REGISTRY_NAME, &member.name, version)?;
    let metadata_path = package_dir.join(LOCAL_REGISTRY_METADATA_FILE);
    if metadata_path.is_file() {
        let existing = fs::read_to_string(&metadata_path).map_err(|e| {
            format!(
                "failed to read existing registry metadata `{}`: {e}",
                metadata_path.display()
            )
        })?;
        let existing_checksum = read_published_checksum(&existing).unwrap_or_default();
        if existing_checksum == checksum {
            return Ok(());
        }
        return Err(format!(
            "registry package `{}` version `{version}` already exists with different content",
            member.name
        ));
    }

    let temp_dir = package_dir.with_extension(format!("tmp-{}", std::process::id()));
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).map_err(|e| {
            format!(
                "failed to clean temp publish directory `{}`: {e}",
                temp_dir.display()
            )
        })?;
    }
    fs::create_dir_all(temp_dir.join(LOCAL_REGISTRY_SNAPSHOT_DIR)).map_err(|e| {
        format!(
            "failed to create publish directory `{}`: {e}",
            temp_dir.display()
        )
    })?;
    let metadata_with_checksum = inject_published_checksum(&metadata_text, &checksum);
    fs::write(
        temp_dir.join(LOCAL_REGISTRY_METADATA_FILE),
        &metadata_with_checksum,
    )
    .map_err(|e| {
        format!(
            "failed to write registry metadata for `{}`: {e}",
            member.display_label()
        )
    })?;
    fs::write(
        temp_dir.join(LOCAL_REGISTRY_SNAPSHOT_DIR).join("book.toml"),
        &metadata_with_checksum,
    )
    .map_err(|e| {
        format!(
            "failed to write snapshot manifest for `{}`: {e}",
            member.name
        )
    })?;
    for relative_path in snapshot_paths {
        let source_path = member.abs_dir.join(&relative_path);
        let target_path = temp_dir
            .join(LOCAL_REGISTRY_SNAPSHOT_DIR)
            .join(&relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "failed to create snapshot directory `{}`: {e}",
                    parent.display()
                )
            })?;
        }
        fs::copy(&source_path, &target_path).map_err(|e| {
            format!(
                "failed to copy published file `{}`: {e}",
                source_path.display()
            )
        })?;
    }
    if let Some(parent) = package_dir.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create registry package parent `{}`: {e}",
                parent.display()
            )
        })?;
    }
    fs::rename(&temp_dir, &package_dir).map_err(|e| {
        format!(
            "failed to finalize published package `{}`: {e}",
            package_dir.display()
        )
    })?;
    Ok(())
}

fn render_published_manifest(
    graph: &WorkspaceGraph,
    publish_ids: &BTreeSet<String>,
    member: &WorkspaceMember,
    version: &SemverVersion,
    native_product_sidecar_hashes: &BTreeMap<String, BTreeMap<String, String>>,
) -> PackageResult<String> {
    let mut out = String::new();
    out.push_str(&format!("name = \"{}\"\n", escape_toml(&member.name)));
    out.push_str(&format!("kind = \"{}\"\n", member.kind.as_str()));
    out.push_str(&format!("version = \"{}\"\n", version));
    if !member.direct_dep_specs.is_empty() {
        out.push_str("\n[deps]\n");
        for (alias, spec) in &member.direct_dep_specs {
            out.push_str(&format!(
                "{} = {}\n",
                quote_toml_key(alias),
                render_published_dependency(graph, publish_ids, member, alias, spec)?
            ));
        }
    }
    render_native_products(
        &mut out,
        &member.native_products,
        native_product_sidecar_hashes,
    );
    render_foreword_products(&mut out, &member.foreword_products);
    Ok(out)
}

fn render_published_dependency(
    graph: &WorkspaceGraph,
    publish_ids: &BTreeSet<String>,
    member: &WorkspaceMember,
    alias: &str,
    spec: &crate::DependencySpec,
) -> PackageResult<String> {
    let mut fields = Vec::new();
    if spec.package != alias {
        fields.push(format!("package = \"{}\"", escape_toml(&spec.package)));
    }
    match &spec.source {
        DependencySourceSpec::Path { .. } => {
            let dep_id = member
                .direct_dep_ids
                .get(alias)
                .ok_or_else(|| format!("missing resolved dependency id for `{alias}`"))?;
            let dep = graph
                .member_by_id(dep_id)
                .ok_or_else(|| format!("missing dependency `{dep_id}`"))?;
            if dep.source_kind != DependencySource::Path || !publish_ids.contains(dep_id) {
                return Err(format!(
                    "published package `{}` has path dependency `{alias}` outside the published slice",
                    member.name
                ));
            }
            let dep_version = dep.version.as_ref().ok_or_else(|| {
                format!(
                    "published dependency `{}` must declare `version = \"x.y.z\"`",
                    dep.name
                )
            })?;
            fields.push(format!("version = \"{}\"", dep_version));
            fields.push(format!("registry = \"{}\"", DEFAULT_REGISTRY_NAME));
        }
        DependencySourceSpec::Registry {
            registry_name,
            version,
            checksum,
        } => {
            fields.push(format!("version = \"{}\"", version));
            if let Some(registry_name) = registry_name {
                fields.push(format!("registry = \"{}\"", escape_toml(registry_name)));
            }
            if let Some(checksum) = checksum {
                fields.push(format!("checksum = \"{}\"", escape_toml(checksum)));
            }
        }
        DependencySourceSpec::Git { url, selector } => {
            fields.push(format!("git = \"{}\"", escape_toml(url)));
            if let Some(selector) = selector {
                match selector {
                    crate::GitSelector::Rev(value) => {
                        fields.push(format!("rev = \"{}\"", escape_toml(value)));
                    }
                    crate::GitSelector::Tag(value) => {
                        fields.push(format!("tag = \"{}\"", escape_toml(value)));
                    }
                    crate::GitSelector::Branch(value) => {
                        fields.push(format!("branch = \"{}\"", escape_toml(value)));
                    }
                }
            }
        }
    }
    fields.push(format!(
        "native_delivery = \"{}\"",
        spec.native_delivery.as_str()
    ));
    if let Some(native_child) = &spec.native_child {
        fields.push(format!("native_child = \"{}\"", escape_toml(native_child)));
    }
    if let Some(native_provider) = &spec.native_provider {
        fields.push(format!(
            "native_provider = \"{}\"",
            escape_toml(native_provider)
        ));
    }
    if !spec.native_plugins.is_empty() {
        fields.push(format!(
            "native_plugins = {}",
            format_string_array(&spec.native_plugins)
        ));
    }
    if spec.executable_forewords {
        fields.push("executable_forewords = true".to_string());
    }
    Ok(format!("{{ {} }}", fields.join(", ")))
}

fn render_native_products(
    out: &mut String,
    products: &BTreeMap<String, NativeProductSpec>,
    sidecar_hashes: &BTreeMap<String, BTreeMap<String, String>>,
) {
    if products.is_empty() {
        return;
    }
    out.push_str("\n[native.products]\n");
    for (name, product) in products {
        out.push_str(&format!(
            "{} = {{ kind = \"{}\", role = \"{}\", producer = \"{}\", file = \"{}\", contract = \"{}\"",
            quote_toml_key(name),
            escape_toml(&product.kind),
            product.role.as_str(),
            product.producer.as_str(),
            escape_toml(&product.file),
            escape_toml(&product.contract),
        ));
        if let Some(crate_name) = &product.rust_cdylib_crate {
            out.push_str(&format!(
                ", rust_cdylib_crate = \"{}\"",
                escape_toml(crate_name)
            ));
        }
        if let Some(provider_dir) = &product.provider_dir {
            out.push_str(&format!(
                ", provider_dir = \"{}\"",
                escape_toml(provider_dir)
            ));
        }
        if !product.sidecars.is_empty() {
            out.push_str(&format!(
                ", sidecars = {}",
                format_string_array(&product.sidecars)
            ));
        }
        if let Some(product_hashes) = sidecar_hashes.get(name)
            && !product_hashes.is_empty()
        {
            out.push_str(&format!(
                ", sidecar_hashes = {}",
                format_string_map(product_hashes)
            ));
        }
        out.push_str(" }\n");
    }
}

fn render_foreword_products(
    out: &mut String,
    products: &BTreeMap<String, ForewordAdapterProductSpec>,
) {
    if products.is_empty() {
        return;
    }
    out.push_str("\n[toolchain.foreword_products]\n");
    for (name, product) in products {
        out.push_str(&format!(
            "{} = {{ path = \"{}\"",
            quote_toml_key(name),
            escape_toml(&product.path),
        ));
        if let Some(runner) = &product.runner {
            out.push_str(&format!(", runner = \"{}\"", escape_toml(runner)));
        }
        if !product.args.is_empty() {
            out.push_str(&format!(", args = {}", format_string_array(&product.args)));
        }
        out.push_str(" }\n");
    }
}

fn collect_snapshot_paths(member: &WorkspaceMember) -> PackageResult<Vec<String>> {
    let mut paths = Vec::new();
    collect_relative_files(&member.abs_dir.join("src"), &member.abs_dir, &mut paths)?;
    let assets_dir = member.abs_dir.join("assets");
    if assets_dir.is_dir() {
        collect_relative_files(&assets_dir, &member.abs_dir, &mut paths)?;
    }
    for product in member.native_products.values() {
        paths.push(product.file.clone());
        if let Some(provider_dir) = &product.provider_dir {
            let provider_root = member.abs_dir.join(provider_dir);
            if provider_root.is_dir() {
                collect_relative_files(&provider_root, &member.abs_dir, &mut paths)?;
            } else {
                paths.push(provider_dir.clone());
            }
        }
        for sidecar in &product.sidecars {
            paths.push(sidecar.clone());
        }
    }
    for product in member.foreword_products.values() {
        paths.push(product.path.clone());
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn collect_relative_files(dir: &Path, root: &Path, out: &mut Vec<String>) -> PackageResult<()> {
    for entry in
        fs::read_dir(dir).map_err(|e| format!("failed to read `{}`: {e}", dir.display()))?
    {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_relative_files(&path, root, out)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|e| format!("failed to relativize `{}`: {e}", path.display()))?;
        out.push(relative.to_string_lossy().replace('\\', "/"));
    }
    Ok(())
}

fn compute_native_product_sidecar_hashes(
    member: &WorkspaceMember,
) -> PackageResult<BTreeMap<String, BTreeMap<String, String>>> {
    let mut products = BTreeMap::new();
    for (product_name, product) in &member.native_products {
        let mut sidecars = BTreeMap::new();
        for sidecar in &product.sidecars {
            sidecars.insert(
                sidecar.clone(),
                compute_file_sha256(&member.abs_dir.join(sidecar))?,
            );
        }
        if !sidecars.is_empty() {
            products.insert(product_name.clone(), sidecars);
        }
    }
    Ok(products)
}

fn compute_file_sha256(path: &Path) -> PackageResult<String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("failed to read published file `{}`: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn compute_publish_checksum(
    member: &WorkspaceMember,
    metadata_text: &str,
    snapshot_paths: &[String],
) -> PackageResult<String> {
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_publish_v1\n");
    hasher.update(metadata_text.as_bytes());
    for relative_path in snapshot_paths {
        hasher.update(b"\nfile\n");
        hasher.update(relative_path.as_bytes());
        let bytes = fs::read(member.abs_dir.join(relative_path)).map_err(|e| {
            format!(
                "failed to read published file `{}`: {e}",
                member.abs_dir.join(relative_path).display()
            )
        })?;
        hasher.update(b"\n");
        hasher.update(&bytes);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn inject_published_checksum(metadata_text: &str, checksum: &str) -> String {
    format!("published_checksum = \"{checksum}\"\n{metadata_text}")
}

fn read_published_checksum(metadata_text: &str) -> Option<String> {
    let value = metadata_text.parse::<toml::Value>().ok()?;
    value
        .get("published_checksum")
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
}

fn escape_toml(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\"', "\\\"")
}

fn quote_toml_key(text: &str) -> String {
    format!("\"{}\"", escape_toml(text))
}

fn format_string_array(items: &[String]) -> String {
    let rendered = items
        .iter()
        .map(|item| format!("\"{}\"", escape_toml(item)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{rendered}]")
}

fn format_string_map(items: &BTreeMap<String, String>) -> String {
    let rendered = items
        .iter()
        .map(|(key, value)| format!("\"{}\" = \"{}\"", escape_toml(key), escape_toml(value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{ {rendered} }}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load_workspace_graph;
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn repo_root() -> PathBuf {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        crate_dir
            .parent()
            .and_then(Path::parent)
            .expect("workspace root should exist")
            .to_path_buf()
    }

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let dir = repo_root()
            .join("target")
            .join("arcana-package-publish-tests")
            .join(format!("{label}_{unique}"));
        fs::create_dir_all(&dir).expect("temp dir should exist");
        dir
    }

    fn write_file(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent should exist");
        }
        fs::write(path, text).expect("file should write");
    }

    #[test]
    fn publish_and_resolve_local_registry_dependency() {
        let _guard = env_lock().lock().expect("env lock should acquire");
        let home = temp_dir("home");
        let core = temp_dir("core");
        write_file(
            &core.join("book.toml"),
            "name = \"core\"\nkind = \"lib\"\nversion = \"1.2.3\"\n",
        );
        write_file(
            &core.join("src").join("book.arc"),
            "fn util() -> Int:\n    return 1\n",
        );
        write_file(&core.join("src").join("types.arc"), "// types\n");

        // SAFETY: tests serialize access to process env through env_lock().
        unsafe {
            std::env::set_var("ARCANA_HOME", &home);
        }
        publish_workspace_member(&core, "core").expect("publish should succeed");

        let app = temp_dir("app");
        write_file(
            &app.join("book.toml"),
            "name = \"app\"\nkind = \"app\"\n[deps]\ncore = { version = \"^1.2.0\" }\n",
        );
        write_file(
            &app.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&app.join("src").join("types.arc"), "// types\n");

        let graph = load_workspace_graph(&app).expect("graph should load");
        let app_member = graph.local_member("app").expect("app member should exist");
        let core_dep_id = app_member
            .direct_dep_ids
            .get("core")
            .expect("core dependency id should resolve");
        assert_eq!(core_dep_id, "registry:local:core@1.2.3");
        assert!(graph.member_by_id(core_dep_id).is_some());
    }

    #[test]
    fn resolver_accepts_matching_local_registry_checksum() {
        let _guard = env_lock().lock().expect("env lock should acquire");
        let home = temp_dir("home_checksum_ok");
        let core = temp_dir("core_checksum_ok");
        write_file(
            &core.join("book.toml"),
            "name = \"core\"\nkind = \"lib\"\nversion = \"1.2.3\"\n",
        );
        write_file(
            &core.join("src").join("book.arc"),
            "fn util() -> Int:\n    return 1\n",
        );
        write_file(&core.join("src").join("types.arc"), "// types\n");

        // SAFETY: tests serialize access to process env through env_lock().
        unsafe {
            std::env::set_var("ARCANA_HOME", &home);
        }
        publish_workspace_member(&core, "core").expect("publish should succeed");

        let metadata_path = local_registry_package_dir(
            DEFAULT_REGISTRY_NAME,
            "core",
            &SemverVersion::parse("1.2.3").expect("version should parse"),
        )
        .expect("package dir should resolve")
        .join(LOCAL_REGISTRY_METADATA_FILE);
        let checksum = read_published_checksum(
            &fs::read_to_string(&metadata_path).expect("metadata should read"),
        )
        .expect("published checksum should exist");

        let app = temp_dir("app_checksum_ok");
        write_file(
            &app.join("book.toml"),
            &format!(
                "name = \"app\"\nkind = \"app\"\n[deps]\ncore = {{ version = \"1.2.3\", checksum = \"{checksum}\" }}\n"
            ),
        );
        write_file(
            &app.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&app.join("src").join("types.arc"), "// types\n");

        let graph = load_workspace_graph(&app).expect("graph should load");
        let app_member = graph.local_member("app").expect("app member should exist");
        assert_eq!(
            app_member.direct_dep_ids.get("core").map(String::as_str),
            Some("registry:local:core@1.2.3")
        );
    }

    #[test]
    fn resolver_rejects_local_registry_checksum_mismatch() {
        let _guard = env_lock().lock().expect("env lock should acquire");
        let home = temp_dir("home_checksum_bad");
        let core = temp_dir("core_checksum_bad");
        write_file(
            &core.join("book.toml"),
            "name = \"core\"\nkind = \"lib\"\nversion = \"1.2.3\"\n",
        );
        write_file(
            &core.join("src").join("book.arc"),
            "fn util() -> Int:\n    return 1\n",
        );
        write_file(&core.join("src").join("types.arc"), "// types\n");

        // SAFETY: tests serialize access to process env through env_lock().
        unsafe {
            std::env::set_var("ARCANA_HOME", &home);
        }
        publish_workspace_member(&core, "core").expect("publish should succeed");

        let app = temp_dir("app_checksum_bad");
        write_file(
            &app.join("book.toml"),
            "name = \"app\"\nkind = \"app\"\n[deps]\ncore = { version = \"1.2.3\", checksum = \"sha256:deadbeef\" }\n",
        );
        write_file(
            &app.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&app.join("src").join("types.arc"), "// types\n");

        let err = load_workspace_graph(&app).expect_err("graph load should fail");
        assert!(err.contains("requested checksum"));
    }

    #[test]
    fn publish_rejects_republish_with_different_content() {
        let _guard = env_lock().lock().expect("env lock should acquire");
        let home = temp_dir("home_conflict");
        let core = temp_dir("core_conflict");
        write_file(
            &core.join("book.toml"),
            "name = \"core\"\nkind = \"lib\"\nversion = \"2.0.0\"\n",
        );
        write_file(
            &core.join("src").join("book.arc"),
            "fn util() -> Int:\n    return 1\n",
        );
        write_file(&core.join("src").join("types.arc"), "// types\n");

        // SAFETY: tests serialize access to process env through env_lock().
        unsafe {
            std::env::set_var("ARCANA_HOME", &home);
        }
        publish_workspace_member(&core, "core").expect("first publish should succeed");
        write_file(
            &core.join("src").join("book.arc"),
            "fn util() -> Int:\n    return 2\n",
        );
        let err = publish_workspace_member(&core, "core").expect_err("publish should fail");
        assert!(err.contains("already exists with different content"));
    }

    #[test]
    fn publish_records_native_product_sidecar_hashes() {
        let _guard = env_lock().lock().expect("env lock should acquire");
        let home = temp_dir("home_sidecar_hashes");
        let core = temp_dir("core_sidecar_hashes");
        write_file(
            &core.join("book.toml"),
            concat!(
                "name = \"core\"\n",
                "kind = \"lib\"\n",
                "version = \"1.0.0\"\n",
                "[native.products.default]\n",
                "kind = \"dll\"\n",
                "role = \"export\"\n",
                "producer = \"arcana-source\"\n",
                "file = \"bin/core.dll\"\n",
                "contract = \"core-v1\"\n",
                "sidecars = [\"assets/runtime.txt\"]\n",
            ),
        );
        write_file(
            &core.join("src").join("book.arc"),
            "fn util() -> Int:\n    return 1\n",
        );
        write_file(&core.join("src").join("types.arc"), "// types\n");
        write_file(&core.join("bin").join("core.dll"), "dll bytes\n");
        write_file(
            &core.join("assets").join("runtime.txt"),
            "runtime sidecar\n",
        );

        // SAFETY: tests serialize access to process env through env_lock().
        unsafe {
            std::env::set_var("ARCANA_HOME", &home);
        }
        publish_workspace_member(&core, "core").expect("publish should succeed");

        let metadata_path = local_registry_package_dir(
            DEFAULT_REGISTRY_NAME,
            "core",
            &SemverVersion::parse("1.0.0").expect("version should parse"),
        )
        .expect("package dir should resolve")
        .join(LOCAL_REGISTRY_METADATA_FILE);
        let metadata = fs::read_to_string(&metadata_path).expect("metadata should read");
        let expected = compute_file_sha256(&core.join("assets").join("runtime.txt"))
            .expect("hash should compute");
        assert!(
            metadata.contains("sidecar_hashes"),
            "published metadata should record sidecar hashes: {metadata}"
        );
        assert!(
            metadata.contains(&expected),
            "published metadata should record the sidecar checksum `{expected}`: {metadata}"
        );
    }

    #[test]
    fn publish_includes_package_assets_in_snapshot() {
        let _guard = env_lock().lock().expect("env lock should acquire");
        let home = temp_dir("home_assets_snapshot");
        let core = temp_dir("core_assets_snapshot");
        write_file(
            &core.join("book.toml"),
            concat!(
                "name = \"core\"\n",
                "kind = \"lib\"\n",
                "version = \"1.0.0\"\n",
            ),
        );
        write_file(
            &core.join("src").join("book.arc"),
            "export fn util() -> Int:\n    return 1\n",
        );
        write_file(&core.join("src").join("types.arc"), "// types\n");
        write_file(&core.join("assets").join("runtime.txt"), "runtime asset\n");

        unsafe {
            std::env::set_var("ARCANA_HOME", &home);
        }
        publish_workspace_member(&core, "core").expect("publish should succeed");

        let snapshot_dir = local_registry_package_dir(
            DEFAULT_REGISTRY_NAME,
            "core",
            &SemverVersion::parse("1.0.0").expect("version should parse"),
        )
        .expect("package dir should resolve")
        .join(LOCAL_REGISTRY_SNAPSHOT_DIR);
        let asset_path = snapshot_dir.join("assets").join("runtime.txt");
        assert!(
            asset_path.is_file(),
            "published snapshot should include package assets at {}",
            asset_path.display()
        );
        assert_eq!(
            fs::read_to_string(&asset_path).expect("published asset should read"),
            "runtime asset\n"
        );
    }

    #[test]
    fn publish_preserves_native_provider_dependency_setting() {
        let _guard = env_lock().lock().expect("env lock should acquire");
        let home = temp_dir("home_native_provider_publish");
        let workspace = temp_dir("workspace_native_provider_publish");
        write_file(
            &workspace.join("book.toml"),
            "name = \"workspace\"\nkind = \"app\"\n[workspace]\nmembers = [\"core\", \"text\"]\n",
        );

        write_file(
            &workspace.join("core").join("book.toml"),
            concat!(
                "name = \"core\"\n",
                "kind = \"lib\"\n",
                "version = \"1.0.0\"\n",
                "[deps]\n",
                "text = { path = \"../text\", native_provider = \"default\" }\n",
            ),
        );
        write_file(
            &workspace.join("core").join("src").join("book.arc"),
            "export fn util() -> Int:\n    return 1\n",
        );
        write_file(
            &workspace.join("core").join("src").join("types.arc"),
            "// types\n",
        );

        write_file(
            &workspace.join("text").join("book.toml"),
            concat!(
                "name = \"text\"\n",
                "kind = \"lib\"\n",
                "version = \"1.0.0\"\n",
                "\n[native.products.default]\n",
                "kind = \"dll\"\n",
                "role = \"provider\"\n",
                "producer = \"arcana-source\"\n",
                "file = \"text_provider.dll\"\n",
                "contract = \"arcana.cabi.provider.v1\"\n",
                "sidecars = []\n",
            ),
        );
        write_file(
            &workspace.join("text").join("src").join("book.arc"),
            "export fn ready() -> Int:\n    return 1\n",
        );
        write_file(
            &workspace.join("text").join("src").join("types.arc"),
            "// types\n",
        );
        write_file(
            &workspace.join("text").join("text_provider.dll"),
            "provider-dll\n",
        );

        // SAFETY: tests serialize access to process env through env_lock().
        unsafe {
            std::env::set_var("ARCANA_HOME", &home);
        }
        publish_workspace_member(&workspace, "core").expect("publish should succeed");

        let metadata_path = local_registry_package_dir(
            DEFAULT_REGISTRY_NAME,
            "core",
            &SemverVersion::parse("1.0.0").expect("version should parse"),
        )
        .expect("package dir should resolve")
        .join(LOCAL_REGISTRY_METADATA_FILE);
        let metadata = fs::read_to_string(&metadata_path).expect("metadata should read");
        assert!(
            metadata.contains("native_provider = \"default\""),
            "published metadata should preserve native_provider dependency settings: {metadata}"
        );
    }

    #[test]
    fn publish_preserves_foreword_adapter_products() {
        let _guard = env_lock().lock().expect("env lock should acquire");
        let home = temp_dir("home_foreword_products");
        let core = temp_dir("core_foreword_products");
        write_file(
            &core.join("book.toml"),
            concat!(
                "name = \"core\"\n",
                "kind = \"lib\"\n",
                "version = \"1.0.0\"\n",
                "[toolchain.foreword_products.adapter]\n",
                "path = \"forewords/adapter.cmd\"\n",
                "runner = \"cmd\"\n",
                "args = [\"/c\"]\n",
            ),
        );
        write_file(
            &core.join("src").join("book.arc"),
            "fn util() -> Int:\n    return 1\n",
        );
        write_file(&core.join("src").join("types.arc"), "// types\n");
        write_file(
            &core.join("forewords").join("adapter.cmd"),
            "@echo off\r\necho {}\r\n",
        );

        unsafe {
            std::env::set_var("ARCANA_HOME", &home);
        }
        publish_workspace_member(&core, "core").expect("publish should succeed");

        let metadata_path = local_registry_package_dir(
            DEFAULT_REGISTRY_NAME,
            "core",
            &SemverVersion::parse("1.0.0").expect("version should parse"),
        )
        .expect("package dir should resolve")
        .join(LOCAL_REGISTRY_METADATA_FILE);
        let metadata = fs::read_to_string(&metadata_path).expect("metadata should read");
        assert!(
            metadata.contains("[toolchain.foreword_products]"),
            "published metadata should preserve foreword products: {metadata}"
        );
        assert!(
            metadata.contains("path = \"forewords/adapter.cmd\""),
            "published metadata should keep foreword product path: {metadata}"
        );
    }

    #[test]
    fn resolver_picks_highest_compatible_local_registry_version() {
        let _guard = env_lock().lock().expect("env lock should acquire");
        let home = temp_dir("home_versions");
        // SAFETY: tests serialize access to process env through env_lock().
        unsafe {
            std::env::set_var("ARCANA_HOME", &home);
        }

        for version in ["1.2.0", "1.4.2"] {
            let core = temp_dir(&format!("core_{version}"));
            write_file(
                &core.join("book.toml"),
                &format!("name = \"core\"\nkind = \"lib\"\nversion = \"{version}\"\n"),
            );
            write_file(
                &core.join("src").join("book.arc"),
                "fn util() -> Int:\n    return 1\n",
            );
            write_file(&core.join("src").join("types.arc"), "// types\n");
            publish_workspace_member(&core, "core").expect("publish should succeed");
        }

        let app = temp_dir("app_versions");
        write_file(
            &app.join("book.toml"),
            "name = \"app\"\nkind = \"app\"\n[deps]\ncore = { version = \"^1.2.0\" }\n",
        );
        write_file(
            &app.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&app.join("src").join("types.arc"), "// types\n");

        let graph = load_workspace_graph(&app).expect("graph should load");
        let app_member = graph.local_member("app").expect("app member should exist");
        assert_eq!(
            app_member.direct_dep_ids.get("core").map(String::as_str),
            Some("registry:local:core@1.4.2")
        );
    }

    #[test]
    fn local_path_and_registry_packages_with_same_display_name_coexist_across_members() {
        let _guard = env_lock().lock().expect("env lock should acquire");
        let home = temp_dir("home_same_name_coexist");
        // SAFETY: tests serialize access to process env through env_lock().
        unsafe {
            std::env::set_var("ARCANA_HOME", &home);
        }

        let registry_core = temp_dir("registry_core_same_name_coexist");
        write_file(
            &registry_core.join("book.toml"),
            "name = \"core\"\nkind = \"lib\"\nversion = \"1.2.3\"\n",
        );
        write_file(
            &registry_core.join("src").join("book.arc"),
            "export fn value() -> Int:\n    return 7\n",
        );
        write_file(&registry_core.join("src").join("types.arc"), "// types\n");
        publish_workspace_member(&registry_core, "core").expect("publish should succeed");

        let workspace = temp_dir("workspace_same_name_coexist");
        write_file(
            &workspace.join("book.toml"),
            "name = \"workspace\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\", \"helper\", \"core\"]\n",
        );

        write_file(
            &workspace.join("core").join("book.toml"),
            "name = \"core\"\nkind = \"lib\"\n",
        );
        write_file(
            &workspace.join("core").join("src").join("book.arc"),
            "export fn value() -> Int:\n    return 1\n",
        );
        write_file(
            &workspace.join("core").join("src").join("types.arc"),
            "// core types\n",
        );

        write_file(
            &workspace.join("helper").join("book.toml"),
            "name = \"helper\"\nkind = \"lib\"\n[deps]\ncore = { version = \"1.2.3\" }\n",
        );
        write_file(
            &workspace.join("helper").join("src").join("book.arc"),
            "import core\nexport fn registry_value() -> Int:\n    return core.value :: :: call\n",
        );
        write_file(
            &workspace.join("helper").join("src").join("types.arc"),
            "// helper types\n",
        );

        write_file(
            &workspace.join("app").join("book.toml"),
            concat!(
                "name = \"app\"\n",
                "kind = \"app\"\n",
                "[deps]\n",
                "core = { path = \"../core\" }\n",
                "helper = { path = \"../helper\" }\n",
            ),
        );
        write_file(
            &workspace.join("app").join("src").join("shelf.arc"),
            "import core\nimport helper\nfn main() -> Int:\n    return core.value :: :: call\n",
        );
        write_file(
            &workspace.join("app").join("src").join("types.arc"),
            "// app types\n",
        );

        let graph = load_workspace_graph(&workspace).expect("graph should load");
        let app_member = graph.local_member("app").expect("app member should exist");
        let helper_member = graph
            .local_member("helper")
            .expect("helper member should exist");
        let local_core_id = graph
            .local_member("core")
            .expect("local core should exist")
            .package_id
            .clone();
        let registry_core_id = graph
            .members
            .iter()
            .find(|member| {
                member.name == "core" && member.source_kind == DependencySource::Registry
            })
            .expect("registry core should exist")
            .package_id
            .clone();

        assert_eq!(
            app_member.direct_dep_ids.get("core").map(String::as_str),
            Some(local_core_id.as_str())
        );
        assert_eq!(
            helper_member.direct_dep_ids.get("core").map(String::as_str),
            Some(registry_core_id.as_str())
        );

        let hir = crate::load_workspace_hir_from_graph(&graph.root_dir, &graph)
            .expect("workspace hir should load");
        let resolved = arcana_hir::resolve_workspace(&hir).expect("workspace should resolve");
        assert!(resolved.package_by_id(&local_core_id).is_some());
        assert!(resolved.package_by_id(&registry_core_id).is_some());

        let prepared = crate::prepare_build(&graph).expect("build should prepare");
        let order = crate::plan_workspace(&graph).expect("workspace should plan");
        let statuses =
            crate::plan_build(&graph, &order, &prepared, None).expect("build should plan");
        crate::execute_build(&graph, &prepared, &statuses).expect("build should execute");

        let local_core_status = statuses
            .iter()
            .find(|status| status.member() == local_core_id.as_str())
            .expect("local core status should exist");
        let registry_core_status = statuses
            .iter()
            .find(|status| status.member() == registry_core_id.as_str())
            .expect("registry core status should exist");
        assert_ne!(
            local_core_status.artifact_rel_path(),
            registry_core_status.artifact_rel_path(),
            "local and registry packages with the same display name must not collide in artifact paths"
        );

        let lock_text =
            crate::render_lockfile(&graph, &order, &statuses, None).expect("lock should render");
        let lock_path = workspace.join("Arcana.lock");
        write_file(&lock_path, &lock_text);
        let lock = crate::read_lockfile(&lock_path)
            .expect("lockfile should read")
            .expect("lockfile should exist");
        let local_core_lock = lock
            .members
            .get(&local_core_id)
            .expect("local core lock entry should exist");
        let registry_core_lock = lock
            .members
            .get(&registry_core_id)
            .expect("registry core lock entry should exist");
        assert_eq!(local_core_lock.name, "core");
        assert_eq!(registry_core_lock.name, "core");
        assert_eq!(local_core_lock.source_kind, DependencySource::Path);
        assert_eq!(registry_core_lock.source_kind, DependencySource::Registry);
        assert_ne!(
            local_core_lock
                .target(&crate::BuildTarget::internal_aot())
                .expect("local core build entry should exist")
                .artifact,
            registry_core_lock
                .target(&crate::BuildTarget::internal_aot())
                .expect("registry core build entry should exist")
                .artifact,
            "lockfile build entries must keep same-name local and registry artifacts distinct"
        );
    }

    #[test]
    fn same_member_direct_multi_version_is_rejected() {
        let _guard = env_lock().lock().expect("env lock should acquire");
        let home = temp_dir("home_multi_version");
        // SAFETY: tests serialize access to process env through env_lock().
        unsafe {
            std::env::set_var("ARCANA_HOME", &home);
        }

        for version in ["1.2.0", "2.0.0"] {
            let core = temp_dir(&format!("core_mv_{version}"));
            write_file(
                &core.join("book.toml"),
                &format!("name = \"core\"\nkind = \"lib\"\nversion = \"{version}\"\n"),
            );
            write_file(
                &core.join("src").join("book.arc"),
                "fn util() -> Int:\n    return 1\n",
            );
            write_file(&core.join("src").join("types.arc"), "// types\n");
            publish_workspace_member(&core, "core").expect("publish should succeed");
        }

        let app = temp_dir("app_multi_version");
        write_file(
            &app.join("book.toml"),
            concat!(
                "name = \"app\"\n",
                "kind = \"app\"\n",
                "[deps]\n",
                "core_v1 = { package = \"core\", version = \"^1.2.0\" }\n",
                "core_v2 = { package = \"core\", version = \"2.0.0\" }\n",
            ),
        );
        write_file(
            &app.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&app.join("src").join("types.arc"), "// types\n");

        let err = load_workspace_graph(&app).expect_err("graph load should fail");
        assert!(err.contains("multiple direct versions of `core`"));
    }
}
