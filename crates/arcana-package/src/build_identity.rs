#![allow(clippy::too_many_arguments)]

use std::fs;
use std::path::{Path, PathBuf};

use arcana_aot::{
    AotEmissionFile, AotPackageArtifact, AotPackageEmission, parse_package_artifact,
    render_package_artifact,
};
use sha2::{Digest, Sha256};

use crate::{
    BuildTarget, GrimoireKind, PackageResult, build::BuildExecutionContext,
    collect_validated_support_file_paths,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedOutputMetadata {
    pub member: String,
    pub kind: GrimoireKind,
    pub fingerprint: String,
    pub api_fingerprint: String,
    pub target: BuildTarget,
    pub target_format: String,
    pub toolchain: String,
    pub artifact_hash: String,
    pub native_product_closure: Option<String>,
    pub support_files: Vec<String>,
}

#[cfg(test)]
pub fn current_build_toolchain() -> PackageResult<String> {
    current_build_toolchain_for_target_with_context(
        &BuildTarget::internal_aot(),
        &BuildExecutionContext::default(),
    )
}

pub(crate) fn current_build_toolchain_for_target_with_context(
    target: &BuildTarget,
    context: &BuildExecutionContext,
) -> PackageResult<String> {
    compute_current_build_toolchain_with_context(target, context)
}

fn compute_current_build_toolchain_with_context(
    target: &BuildTarget,
    _context: &BuildExecutionContext,
) -> PackageResult<String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("failed to resolve current toolchain binary: {e}"))?;
    let bytes = fs::read(&exe).map_err(|e| {
        format!(
            "failed to read current toolchain binary `{}`: {e}",
            exe.display()
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_driver_binary_v1\n");
    hasher.update(b"arcana_build_target_v2\n");
    hasher.update(target.key().as_bytes());
    hasher.update(&bytes);
    match target {
        BuildTarget::InternalAot => {
            hasher.update(b"arcana_internal_artifact_backend_v1\n");
        }
        BuildTarget::WindowsExe | BuildTarget::WindowsDll => {
            hasher.update(b"arcana_generated_rust_native_backend_v1\n");
            hasher.update(command_fingerprint_output("cargo", &["-V"])?.as_bytes());
            hasher.update(command_fingerprint_output("rustc", &["-vV"])?.as_bytes());
        }
        BuildTarget::Other(other) => {
            hasher.update(b"arcana_unknown_backend_v1\n");
            hasher.update(other.as_bytes());
        }
    }
    Ok(format!("binary-sha256:{:x}", hasher.finalize()))
}

fn command_fingerprint_output(program: &str, args: &[&str]) -> PackageResult<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("failed to execute `{program}` for toolchain fingerprinting: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "`{program} {}` failed during toolchain fingerprinting with status {}",
            args.join(" "),
            output.status
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|e| format!("`{program}` emitted non-utf8 fingerprint data: {e}"))
}

pub fn render_cached_artifact(
    member: &str,
    kind: &GrimoireKind,
    fingerprint: &str,
    api_fingerprint: &str,
    target: &BuildTarget,
    target_format: &str,
    toolchain: &str,
    emission: &AotPackageEmission,
    artifact_hash: &str,
    native_product_closure: Option<&str>,
) -> String {
    let support_files = emission
        .support_files
        .iter()
        .map(|file| format!("\"{}\"", escape_toml(&file.relative_path)))
        .collect::<Vec<_>>()
        .join(", ");
    let root_artifact_length = emission
        .root_artifact_bytes
        .as_ref()
        .map(|bytes| format!("root_artifact_length = {}\n", bytes.len()))
        .unwrap_or_default();
    let support_file_meta = emission
        .support_files
        .iter()
        .map(|file| {
            format!(
                "[[support_file_meta]]\npath = \"{}\"\nlength = {}\n",
                escape_toml(&file.relative_path),
                file.bytes.len()
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let mut rendered = format!(
        concat!(
            "member = \"{}\"\n",
            "kind = \"{}\"\n",
            "fingerprint = \"{}\"\n",
            "api_fingerprint = \"{}\"\n",
            "target = \"{}\"\n",
            "target_format = \"{}\"\n",
            "toolchain = \"{}\"\n",
            "artifact_hash = \"{}\"\n",
            "support_files = [{}]\n",
            "{}"
        ),
        escape_toml(member),
        kind.as_str(),
        escape_toml(fingerprint),
        escape_toml(api_fingerprint),
        target,
        escape_toml(target_format),
        escape_toml(toolchain),
        escape_toml(artifact_hash),
        support_files,
        emission.primary_artifact_body
    );
    if let Some(closure) = native_product_closure {
        rendered = rendered.replacen(
            &format!("support_files = [{}]\n", support_files),
            &format!(
                "support_files = [{}]\nnative_product_closure = \"{}\"\n",
                support_files,
                escape_toml(closure)
            ),
            1,
        );
    }
    rendered.push_str(&root_artifact_length);
    rendered.push_str(&support_file_meta);
    rendered
}

pub fn render_cached_artifact_index(
    member: &str,
    kind: &GrimoireKind,
    fingerprint: &str,
    api_fingerprint: &str,
    target: &BuildTarget,
    target_format: &str,
    toolchain: &str,
    emission: &AotPackageEmission,
    artifact_hash: &str,
    native_product_closure: Option<&str>,
) -> String {
    let required_support_files = emission
        .support_files
        .iter()
        .filter(|file| !file.relative_path.starts_with("package-assets/"))
        .map(|file| format!("\"{}\"", escape_toml(&file.relative_path)))
        .collect::<Vec<_>>()
        .join(", ");
    let required_support_lengths = emission
        .support_files
        .iter()
        .filter(|file| !file.relative_path.starts_with("package-assets/"))
        .map(|file| {
            format!(
                "{} = {}\n",
                quote_toml_key(&file.relative_path),
                file.bytes.len()
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let package_asset_roots = emission
        .support_files
        .iter()
        .filter_map(|file| package_asset_root_prefix(&file.relative_path))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|root| format!("\"{}\"", escape_toml(root)))
        .collect::<Vec<_>>()
        .join(", ");
    let mut rendered = format!(
        concat!(
            "member = \"{}\"\n",
            "kind = \"{}\"\n",
            "fingerprint = \"{}\"\n",
            "api_fingerprint = \"{}\"\n",
            "target = \"{}\"\n",
            "target_format = \"{}\"\n",
            "toolchain = \"{}\"\n",
            "artifact_hash = \"{}\"\n",
            "required_support_files = [{}]\n",
            "package_asset_roots = [{}]\n",
        ),
        escape_toml(member),
        kind.as_str(),
        escape_toml(fingerprint),
        escape_toml(api_fingerprint),
        target,
        escape_toml(target_format),
        escape_toml(toolchain),
        escape_toml(artifact_hash),
        required_support_files,
        package_asset_roots,
    );
    if let Some(root_artifact_bytes) = &emission.root_artifact_bytes {
        rendered.push_str(&format!(
            "root_artifact_length = {}\n",
            root_artifact_bytes.len()
        ));
    }
    if let Some(closure) = native_product_closure {
        rendered.push_str(&format!(
            "native_product_closure = \"{}\"\n",
            escape_toml(closure)
        ));
    }
    if !required_support_lengths.is_empty() {
        rendered.push_str("[required_support_lengths]\n");
        rendered.push_str(&required_support_lengths);
    }
    rendered
}

pub fn cache_metadata_path_for_output(output_path: &Path, target: &BuildTarget) -> PathBuf {
    match target {
        BuildTarget::InternalAot => output_path.to_path_buf(),
        _ => {
            let file_name = output_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("artifact");
            output_path.with_file_name(format!("{file_name}.arcana-cache.toml"))
        }
    }
}

pub fn cache_index_path_for_output(output_path: &Path) -> PathBuf {
    let file_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    output_path.with_file_name(format!("{file_name}.arcana-index.toml"))
}

pub fn cached_artifact_matches_status(
    output_path: &Path,
    expected_member: &str,
    expected_kind: &GrimoireKind,
    expected_fingerprint: &str,
    expected_api_fingerprint: &str,
    expected_target: &BuildTarget,
    expected_format: &str,
    expected_toolchain: &str,
    expected_artifact_hash: &str,
    expected_native_product_closure: Option<&str>,
) -> bool {
    if expected_artifact_hash.is_empty() {
        return false;
    }
    if cached_index_matches_status(
        output_path,
        expected_member,
        expected_kind,
        expected_fingerprint,
        expected_api_fingerprint,
        expected_target,
        expected_format,
        expected_toolchain,
        expected_artifact_hash,
        expected_native_product_closure,
    ) {
        if matches!(expected_target, BuildTarget::InternalAot) {
            return cached_emission_hash_for_path(output_path, expected_target)
                .ok()
                .as_deref()
                == Some(expected_artifact_hash);
        }
        return true;
    }
    let metadata_path = cache_metadata_path_for_output(output_path, expected_target);
    let Ok(text) = fs::read_to_string(&metadata_path) else {
        return false;
    };
    let Ok(value) = text.parse::<toml::Value>() else {
        return false;
    };
    let Some(table) = value.as_table() else {
        return false;
    };
    let matches_header = table.get("member").and_then(toml::Value::as_str) == Some(expected_member)
        && table.get("kind").and_then(toml::Value::as_str) == Some(expected_kind.as_str())
        && table.get("fingerprint").and_then(toml::Value::as_str) == Some(expected_fingerprint)
        && table.get("api_fingerprint").and_then(toml::Value::as_str)
            == Some(expected_api_fingerprint)
        && table.get("target").and_then(toml::Value::as_str) == Some(expected_target.key())
        && table.get("target_format").and_then(toml::Value::as_str) == Some(expected_format)
        && table.get("toolchain").and_then(toml::Value::as_str) == Some(expected_toolchain)
        && table.get("artifact_hash").and_then(toml::Value::as_str) == Some(expected_artifact_hash)
        && table
            .get("native_product_closure")
            .and_then(toml::Value::as_str)
            == expected_native_product_closure;
    if !matches_header {
        return false;
    }
    if cached_output_files_match(output_path, expected_target, table) {
        return true;
    }
    let Ok(artifact) = parse_package_artifact(&text) else {
        return false;
    };
    if artifact.package_id != expected_member {
        return false;
    }
    cached_emission_hash_for_output_path(output_path, expected_target, &text)
        .ok()
        .as_deref()
        == Some(expected_artifact_hash)
}

fn cached_index_matches_status(
    output_path: &Path,
    expected_member: &str,
    expected_kind: &GrimoireKind,
    expected_fingerprint: &str,
    expected_api_fingerprint: &str,
    expected_target: &BuildTarget,
    expected_format: &str,
    expected_toolchain: &str,
    expected_artifact_hash: &str,
    expected_native_product_closure: Option<&str>,
) -> bool {
    let index_path = cache_index_path_for_output(output_path);
    let Ok(text) = fs::read_to_string(&index_path) else {
        return false;
    };
    let Ok(value) = text.parse::<toml::Value>() else {
        return false;
    };
    let Some(table) = value.as_table() else {
        return false;
    };
    let matches_header = table.get("member").and_then(toml::Value::as_str) == Some(expected_member)
        && table.get("kind").and_then(toml::Value::as_str) == Some(expected_kind.as_str())
        && table.get("fingerprint").and_then(toml::Value::as_str) == Some(expected_fingerprint)
        && table.get("api_fingerprint").and_then(toml::Value::as_str)
            == Some(expected_api_fingerprint)
        && table.get("target").and_then(toml::Value::as_str) == Some(expected_target.key())
        && table.get("target_format").and_then(toml::Value::as_str) == Some(expected_format)
        && table.get("toolchain").and_then(toml::Value::as_str) == Some(expected_toolchain)
        && table.get("artifact_hash").and_then(toml::Value::as_str) == Some(expected_artifact_hash)
        && table
            .get("native_product_closure")
            .and_then(toml::Value::as_str)
            == expected_native_product_closure;
    if !matches_header {
        return false;
    }
    cached_output_index_matches(output_path, expected_target, table)
}

pub fn read_cached_output_metadata(
    output_path: &Path,
    target: &BuildTarget,
) -> PackageResult<CachedOutputMetadata> {
    let metadata_path = cache_metadata_path_for_output(output_path, target);
    let text = fs::read_to_string(&metadata_path)
        .map_err(|e| format!("failed to read artifact `{}`: {e}", metadata_path.display()))?;
    let value = text.parse::<toml::Value>().map_err(|e| {
        format!(
            "failed to parse artifact `{}` as TOML: {e}",
            metadata_path.display()
        )
    })?;
    let table = value.as_table().ok_or_else(|| {
        format!(
            "artifact `{}` root must be a table",
            metadata_path.display()
        )
    })?;
    let kind = match table.get("kind").and_then(toml::Value::as_str) {
        Some("app") => GrimoireKind::App,
        Some("lib") => GrimoireKind::Lib,
        Some(other) => {
            return Err(format!(
                "artifact `{}` has unsupported kind `{other}`",
                metadata_path.display()
            ));
        }
        None => {
            return Err(format!(
                "artifact `{}` is missing `kind` metadata",
                metadata_path.display()
            ));
        }
    };
    let target = table
        .get("target")
        .and_then(toml::Value::as_str)
        .map(BuildTarget::from_storage_key)
        .ok_or_else(|| {
            format!(
                "artifact `{}` is missing `target` metadata",
                metadata_path.display()
            )
        })?;
    Ok(CachedOutputMetadata {
        member: required_header_field(table, &metadata_path, "member")?,
        kind,
        fingerprint: required_header_field(table, &metadata_path, "fingerprint")?,
        api_fingerprint: required_header_field(table, &metadata_path, "api_fingerprint")?,
        target,
        target_format: table
            .get("target_format")
            .and_then(toml::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        toolchain: required_header_field(table, &metadata_path, "toolchain")?,
        artifact_hash: required_header_field(table, &metadata_path, "artifact_hash")?,
        native_product_closure: table
            .get("native_product_closure")
            .and_then(toml::Value::as_str)
            .map(ToString::to_string),
        support_files: support_files_from_table(table).map_err(|e| {
            format!(
                "artifact `{}` has invalid support file metadata: {e}",
                metadata_path.display()
            )
        })?,
    })
}

pub fn cached_emission_hash_for_path(path: &Path, target: &BuildTarget) -> PackageResult<String> {
    let metadata_path = cache_metadata_path_for_output(path, target);
    let text = fs::read_to_string(&metadata_path)
        .map_err(|e| format!("failed to read artifact `{}`: {e}", metadata_path.display()))?;
    cached_emission_hash_for_output_path(path, target, &text)
}

pub fn cached_emission_hash(
    target_key: &str,
    target_format: &str,
    artifact: &AotPackageArtifact,
    root_artifact_bytes: Option<&[u8]>,
    support_files: &[AotEmissionFile],
) -> String {
    let rendered = render_package_artifact(artifact);
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_emission_bundle_v1\n");
    hasher.update(target_key.as_bytes());
    hasher.update(b"\n");
    hasher.update(target_format.as_bytes());
    hasher.update(b"\n");
    hasher.update(rendered.as_bytes());
    hasher.update(b"\nroot\n");
    if let Some(bytes) = root_artifact_bytes {
        hasher.update(bytes);
    }
    for file in sorted_support_files(support_files) {
        hasher.update(b"\nfile\n");
        hasher.update(file.relative_path.as_bytes());
        hasher.update(b"\n");
        hasher.update(&file.bytes);
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn cached_emission_hash_for_output_path(
    output_path: &Path,
    target: &BuildTarget,
    text: &str,
) -> PackageResult<String> {
    let metadata_path = cache_metadata_path_for_output(output_path, target);
    let value = text.parse::<toml::Value>().map_err(|e| {
        format!(
            "failed to parse artifact `{}` as TOML: {e}",
            metadata_path.display()
        )
    })?;
    let table = value.as_table().ok_or_else(|| {
        format!(
            "artifact `{}` root must be a table",
            metadata_path.display()
        )
    })?;
    let artifact = parse_package_artifact(text).map_err(|e| {
        format!(
            "failed to parse artifact `{}`: {e}",
            metadata_path.display()
        )
    })?;
    let target_key = table
        .get("target")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| {
            format!(
                "artifact `{}` is missing `target` metadata",
                metadata_path.display()
            )
        })?;
    let target_format = table
        .get("target_format")
        .and_then(toml::Value::as_str)
        .unwrap_or(artifact.format.as_str());
    let support_files = support_files_from_table(table).map_err(|e| {
        format!(
            "artifact `{}` has invalid support file metadata: {e}",
            metadata_path.display()
        )
    })?;
    let artifact_dir = output_path.parent().unwrap_or_else(|| Path::new("."));
    let root_artifact_bytes = match target {
        BuildTarget::InternalAot => None,
        _ => Some(fs::read(output_path).map_err(|e| {
            format!(
                "failed to read emitted artifact `{}`: {e}",
                output_path.display()
            )
        })?),
    };
    let support_file_payloads = support_files
        .into_iter()
        .map(|relative_path| {
            let bytes = fs::read(artifact_dir.join(&relative_path)).map_err(|e| {
                format!(
                    "failed to read support file `{}` beside `{}`: {e}",
                    relative_path,
                    output_path.display()
                )
            })?;
            Ok(AotEmissionFile {
                relative_path,
                bytes,
            })
        })
        .collect::<PackageResult<Vec<_>>>()?;
    Ok(cached_emission_hash(
        target_key,
        target_format,
        &artifact,
        root_artifact_bytes.as_deref(),
        &support_file_payloads,
    ))
}

fn cached_output_files_match(
    output_path: &Path,
    target: &BuildTarget,
    table: &toml::Table,
) -> bool {
    let Ok(support_files) = support_files_from_table(table) else {
        return false;
    };
    if !matches!(target, BuildTarget::InternalAot) {
        let Some(expected_root_length) = table
            .get("root_artifact_length")
            .and_then(toml::Value::as_integer)
        else {
            return false;
        };
        let Ok(metadata) = fs::metadata(output_path) else {
            return false;
        };
        if metadata.len() != expected_root_length as u64 {
            return false;
        }
    }
    if support_files.is_empty() {
        return true;
    }
    let artifact_dir = output_path.parent().unwrap_or_else(|| Path::new("."));
    if support_files.len() > 64 {
        return cached_large_support_set_matches(artifact_dir, &support_files);
    }
    let Some(support_file_lengths) = support_file_lengths_from_table(table) else {
        return false;
    };
    if support_files
        .iter()
        .any(|path| !support_file_lengths.contains_key(path))
    {
        return false;
    }
    support_files.into_iter().all(|relative_path| {
        let Some(expected_length) = support_file_lengths.get(&relative_path) else {
            return false;
        };
        fs::metadata(artifact_dir.join(relative_path))
            .map(|metadata| metadata.len() == *expected_length)
            .unwrap_or(false)
    })
}

fn cached_large_support_set_matches(artifact_dir: &Path, support_files: &[String]) -> bool {
    let mut package_asset_roots = std::collections::BTreeSet::new();
    for relative_path in support_files {
        if let Some(root) = package_asset_root_prefix(relative_path) {
            package_asset_roots.insert(root);
            continue;
        }
        if !artifact_dir.join(relative_path).exists() {
            return false;
        }
    }
    package_asset_roots
        .into_iter()
        .all(|root| artifact_dir.join(root).is_dir())
}

fn package_asset_root_prefix(relative_path: &str) -> Option<&str> {
    let prefix = "package-assets/";
    if !relative_path.starts_with(prefix) {
        return None;
    }
    let tail = &relative_path[prefix.len()..];
    let slash = tail.find('/')?;
    Some(&relative_path[..prefix.len() + slash])
}

fn cached_output_index_matches(
    output_path: &Path,
    target: &BuildTarget,
    table: &toml::Table,
) -> bool {
    if !matches!(target, BuildTarget::InternalAot) {
        let Some(expected_root_length) = table
            .get("root_artifact_length")
            .and_then(toml::Value::as_integer)
        else {
            return false;
        };
        let Ok(metadata) = fs::metadata(output_path) else {
            return false;
        };
        if metadata.len() != expected_root_length as u64 {
            return false;
        }
    }
    let artifact_dir = output_path.parent().unwrap_or_else(|| Path::new("."));
    let Ok(required_support_files) = required_support_files_from_table(table) else {
        return false;
    };
    let Some(required_support_lengths) = required_support_lengths_from_table(table) else {
        return false;
    };
    for relative_path in &required_support_files {
        let Some(expected_length) = required_support_lengths.get(relative_path) else {
            return false;
        };
        let Ok(metadata) = fs::metadata(artifact_dir.join(relative_path)) else {
            return false;
        };
        if metadata.len() != *expected_length {
            return false;
        }
    }
    let package_asset_roots = package_asset_roots_from_table(table).unwrap_or_default();
    package_asset_roots
        .into_iter()
        .all(|root| artifact_dir.join(root).is_dir())
}

fn support_files_from_table(table: &toml::Table) -> Result<Vec<String>, String> {
    let Some(value) = table.get("support_files") else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| "`support_files` must be an array".to_string())?;
    let paths = items
        .iter()
        .map(|item| {
            item.as_str()
                .map(ToString::to_string)
                .ok_or_else(|| "support file entries must be strings".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    collect_validated_support_file_paths(paths.iter().map(String::as_str))
        .map_err(|e| e.to_string())
}

fn support_file_lengths_from_table(
    table: &toml::Table,
) -> Option<std::collections::HashMap<String, u64>> {
    let items = table.get("support_file_meta")?.as_array()?;
    let mut lengths = std::collections::HashMap::new();
    for item in items {
        let item = item.as_table()?;
        let path = item.get("path")?.as_str()?.to_string();
        collect_validated_support_file_paths([path.as_str()]).ok()?;
        let length = u64::try_from(item.get("length")?.as_integer()?).ok()?;
        lengths.insert(path, length);
    }
    Some(lengths)
}

fn required_support_files_from_table(table: &toml::Table) -> Result<Vec<String>, String> {
    let Some(value) = table.get("required_support_files") else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| "`required_support_files` must be an array".to_string())?;
    let paths = items
        .iter()
        .map(|item| {
            item.as_str()
                .map(ToString::to_string)
                .ok_or_else(|| "required support file entries must be strings".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    collect_validated_support_file_paths(paths.iter().map(String::as_str))
        .map_err(|e| e.to_string())
}

fn required_support_lengths_from_table(
    table: &toml::Table,
) -> Option<std::collections::HashMap<String, u64>> {
    let Some(value) = table.get("required_support_lengths") else {
        return Some(std::collections::HashMap::new());
    };
    let table = value.as_table()?;
    let mut lengths = std::collections::HashMap::new();
    for (path, value) in table {
        collect_validated_support_file_paths([path.as_str()]).ok()?;
        let length = u64::try_from(value.as_integer()?).ok()?;
        lengths.insert(path.clone(), length);
    }
    Some(lengths)
}

fn package_asset_roots_from_table(table: &toml::Table) -> Option<Vec<String>> {
    let Some(value) = table.get("package_asset_roots") else {
        return Some(Vec::new());
    };
    let items = value.as_array()?;
    let mut roots = Vec::new();
    for item in items {
        let root = item.as_str()?.to_string();
        collect_validated_support_file_paths([root.as_str()]).ok()?;
        roots.push(root);
    }
    Some(roots)
}

fn required_header_field(
    table: &toml::Table,
    metadata_path: &Path,
    name: &str,
) -> PackageResult<String> {
    table
        .get(name)
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            format!(
                "artifact `{}` is missing `{name}` metadata",
                metadata_path.display()
            )
        })
}

fn sorted_support_files(files: &[AotEmissionFile]) -> Vec<&AotEmissionFile> {
    let mut files = files.iter().collect::<Vec<_>>();
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    files
}

fn escape_toml(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn quote_toml_key(text: &str) -> String {
    format!("\"{}\"", escape_toml(text))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    use arcana_aot::{AOT_INTERNAL_FORMAT, AotPackageModuleArtifact};

    use super::*;

    fn repo_root() -> std::path::PathBuf {
        let crate_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        crate_dir
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root should exist")
            .to_path_buf()
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let dir = repo_root()
            .join("target")
            .join("arcana-build-identity-tests")
            .join(format!("{label}_{unique}"));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn test_package_id_for_module(module_id: &str) -> String {
        module_id.split('.').next().unwrap_or(module_id).to_string()
    }

    fn test_package_display_names_with_deps(
        package_id: impl Into<String>,
        package_name: impl Into<String>,
        direct_deps: Vec<String>,
        direct_dep_ids: Vec<String>,
    ) -> BTreeMap<String, String> {
        let mut names = BTreeMap::from([(package_id.into(), package_name.into())]);
        for (dep_name, dep_id) in direct_deps.into_iter().zip(direct_dep_ids) {
            names.entry(dep_id).or_insert(dep_name);
        }
        names
    }

    fn test_package_direct_dep_ids(
        package_id: impl Into<String>,
        direct_deps: Vec<String>,
        direct_dep_ids: Vec<String>,
    ) -> BTreeMap<String, BTreeMap<String, String>> {
        BTreeMap::from([(
            package_id.into(),
            direct_deps.into_iter().zip(direct_dep_ids).collect(),
        )])
    }

    fn dummy_artifact() -> AotPackageArtifact {
        AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT.to_string(),
            package_id: "tool".to_string(),
            package_name: "tool".to_string(),
            root_module_id: "tool".to_string(),
            direct_deps: Vec::new(),
            direct_dep_ids: Vec::new(),
            package_display_names: test_package_display_names_with_deps(
                "tool".to_string(),
                "tool".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            package_direct_dep_ids: test_package_direct_dep_ids(
                test_package_id_for_module("tool"),
                Vec::new(),
                Vec::new(),
            ),
            module_count: 1,
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: Vec::new(),
            runtime_requirements: Vec::new(),
            foreword_index: Vec::new(),
            foreword_registrations: Vec::new(),
            entrypoints: Vec::new(),
            routines: Vec::new(),
            native_callbacks: Vec::new(),
            shackle_decls: Vec::new(),
            binding_layouts: Vec::new(),
            owners: Vec::new(),
            modules: vec![AotPackageModuleArtifact {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                symbol_count: 0,
                item_count: 0,
                line_count: 0,
                non_empty_line_count: 0,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
        }
    }

    fn dummy_emission(files: Vec<AotEmissionFile>) -> AotPackageEmission {
        let artifact = dummy_artifact();
        AotPackageEmission {
            target: arcana_aot::AotEmitTarget::InternalArtifact,
            primary_artifact_body: render_package_artifact(&artifact),
            artifact,
            root_artifact_bytes: None,
            support_files: files,
        }
    }

    #[test]
    fn cached_emission_hash_roundtrips_rendered_support_files() {
        let dir = temp_dir("hash_roundtrip");
        let artifact_path = dir.join("app.artifact.toml");
        let emission = dummy_emission(vec![AotEmissionFile {
            relative_path: "bin/app.exe".to_string(),
            bytes: b"exe-bytes".to_vec(),
        }]);
        fs::create_dir_all(dir.join("bin")).expect("bin dir should exist");
        fs::write(
            dir.join("bin").join("app.exe"),
            &emission.support_files[0].bytes,
        )
        .expect("support file should write");
        let hash = cached_emission_hash(
            BuildTarget::internal_aot().key(),
            AOT_INTERNAL_FORMAT,
            &emission.artifact,
            None,
            &emission.support_files,
        );
        fs::write(
            &artifact_path,
            render_cached_artifact(
                "tool",
                &GrimoireKind::App,
                "fp",
                "api",
                &BuildTarget::internal_aot(),
                AOT_INTERNAL_FORMAT,
                "toolchain",
                &emission,
                &hash,
                None,
            ),
        )
        .expect("artifact should write");

        assert_eq!(
            cached_emission_hash_for_path(&artifact_path, &BuildTarget::internal_aot())
                .expect("hash should roundtrip"),
            hash
        );
        assert!(cached_artifact_matches_status(
            &artifact_path,
            "tool",
            &GrimoireKind::App,
            "fp",
            "api",
            &BuildTarget::internal_aot(),
            AOT_INTERNAL_FORMAT,
            "toolchain",
            &hash,
            None,
        ));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cached_artifact_match_rejects_changed_support_file_bytes() {
        let dir = temp_dir("hash_support_change");
        let artifact_path = dir.join("app.artifact.toml");
        let emission = dummy_emission(vec![AotEmissionFile {
            relative_path: "bin/app.exe".to_string(),
            bytes: b"exe-bytes".to_vec(),
        }]);
        fs::create_dir_all(dir.join("bin")).expect("bin dir should exist");
        fs::write(
            dir.join("bin").join("app.exe"),
            &emission.support_files[0].bytes,
        )
        .expect("support file should write");
        let hash = cached_emission_hash(
            BuildTarget::internal_aot().key(),
            AOT_INTERNAL_FORMAT,
            &emission.artifact,
            None,
            &emission.support_files,
        );
        fs::write(
            &artifact_path,
            render_cached_artifact(
                "tool",
                &GrimoireKind::App,
                "fp",
                "api",
                &BuildTarget::internal_aot(),
                AOT_INTERNAL_FORMAT,
                "toolchain",
                &emission,
                &hash,
                None,
            ),
        )
        .expect("artifact should write");

        fs::write(dir.join("bin").join("app.exe"), b"changed")
            .expect("support file should rewrite");
        assert!(!cached_artifact_matches_status(
            &artifact_path,
            "tool",
            &GrimoireKind::App,
            "fp",
            "api",
            &BuildTarget::internal_aot(),
            AOT_INTERNAL_FORMAT,
            "toolchain",
            &hash,
            None,
        ));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_cached_output_metadata_rejects_invalid_support_file_path() {
        let dir = temp_dir("invalid_support_metadata");
        let artifact_path = dir.join("app.exe");
        let metadata_path =
            cache_metadata_path_for_output(&artifact_path, &BuildTarget::windows_exe());
        fs::write(
            &metadata_path,
            concat!(
                "member = \"tool\"\n",
                "kind = \"app\"\n",
                "fingerprint = \"fp\"\n",
                "api_fingerprint = \"api\"\n",
                "target = \"windows-exe\"\n",
                "target_format = \"arcana-aot-windows-exe-v1\"\n",
                "toolchain = \"toolchain\"\n",
                "artifact_hash = \"sha256:abc\"\n",
                "support_files = [\"..\\\\escape.exe\"]\n",
            ),
        )
        .expect("metadata should write");

        let err = read_cached_output_metadata(&artifact_path, &BuildTarget::windows_exe())
            .expect_err("invalid support file metadata should fail");
        assert!(err.contains("invalid support file metadata"), "{err}");
        assert!(err.contains("invalid support file path"), "{err}");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cached_emission_hash_rejects_invalid_support_file_path_metadata() {
        let dir = temp_dir("invalid_support_hash");
        let artifact_path = dir.join("app.artifact.toml");
        let emission = dummy_emission(vec![AotEmissionFile {
            relative_path: "..\\escape.exe".to_string(),
            bytes: b"unused".to_vec(),
        }]);
        let hash = cached_emission_hash(
            BuildTarget::internal_aot().key(),
            AOT_INTERNAL_FORMAT,
            &emission.artifact,
            None,
            &[],
        );
        fs::write(
            &artifact_path,
            render_cached_artifact(
                "tool",
                &GrimoireKind::App,
                "fp",
                "api",
                &BuildTarget::internal_aot(),
                AOT_INTERNAL_FORMAT,
                "toolchain",
                &emission,
                &hash,
                None,
            ),
        )
        .expect("metadata should write");

        let err = cached_emission_hash_for_path(&artifact_path, &BuildTarget::internal_aot())
            .expect_err("invalid support file metadata should fail hashing");
        assert!(err.contains("invalid support file metadata"), "{err}");
        assert!(err.contains("invalid support file path"), "{err}");

        let _ = fs::remove_dir_all(&dir);
    }
}
