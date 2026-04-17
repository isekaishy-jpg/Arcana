use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use windows_metadata::reader::{
    Field, Item, ItemIndex, MethodDef, TypeCategory, TypeDef, TypeIndex,
};
use windows_metadata::{FieldAttributes, Signature, Type, Value};

type GenResult<T> = Result<T, String>;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> GenResult<()> {
    let mode = parse_mode(env::args().skip(1))?;
    let repo_root = repo_root();
    let config_root = repo_root
        .join("grimoires")
        .join("arcana")
        .join("winapi")
        .join("generation");

    let metadata: MetadataConfig = load_toml(&config_root.join("metadata.toml"))?;
    let projection: ProjectionConfig = load_toml(&config_root.join("projection.toml"))?;
    let imports: BTreeMap<String, ImportProjection> = load_toml(&config_root.join("imports.toml"))?;
    let constants: ConstantsManifest = load_toml(&config_root.join("constants.toml"))?;
    let callbacks: CallbacksManifest = load_toml(&config_root.join("callbacks.toml"))?;
    let types: TypesManifest = load_toml(&config_root.join("types.toml"))?;
    let skiplist: SkiplistManifest = load_toml(&config_root.join("skiplist.toml"))?;
    let exceptions: ExceptionManifest = load_toml(&config_root.join("exceptions.toml"))?;
    let windows_sys_parity: WindowsSysParityManifest =
        load_toml(&config_root.join("windows-sys-parity.toml"))?;

    let metadata_path = resolve_metadata_path(&metadata)?;
    let windows_sys_root = resolve_windows_sys_root(&metadata)?;
    let metadata_hash = sha256_hex(&metadata_path)?;
    if !metadata_hash.eq_ignore_ascii_case(&metadata.sha256) {
        return Err(format!(
            "pinned Windows metadata hash mismatch for {}: expected {}, found {}",
            metadata_path.display(),
            metadata.sha256,
            metadata_hash
        ));
    }

    let type_index = TypeIndex::read(&metadata_path).ok_or_else(|| {
        format!(
            "failed to read {} as ECMA-335 metadata",
            metadata_path.display()
        )
    })?;
    let item_index = ItemIndex::new(&type_index);
    let manifests = Manifests {
        imports: &imports,
        constants: &constants,
        callbacks: &callbacks,
        types: &types,
        skiplist: &skiplist,
        exceptions: &exceptions,
        windows_sys_parity: &windows_sys_parity,
    };

    validate_projection(&repo_root, &projection, manifests)?;

    let mut changed = Vec::new();
    let mut reports = Vec::new();
    for leaf in &projection.leaves {
        let output_path = repo_root
            .join("grimoires")
            .join("arcana")
            .join("winapi")
            .join("src")
            .join("raw")
            .join(format!("{}.arc", leaf.name));

        let rendered_leaf =
            render_leaf_body(leaf, &type_index, &item_index, manifests, &windows_sys_root)?;
        reports.push(rendered_leaf.report);
        let rendered = render_generated_file(
            &metadata,
            rendered_leaf.source_of_truth,
            &rendered_leaf.body,
        );
        let existing = fs::read_to_string(&output_path).unwrap_or_default();
        if existing != rendered {
            changed.push(repo_relative(&repo_root, &output_path));
            if matches!(mode, Mode::Write) {
                fs::write(&output_path, rendered)
                    .map_err(|err| format!("failed to write {}: {err}", output_path.display()))?;
            }
        }
    }

    let report_path = config_root.join("parity-report.md");
    let rendered_report = render_parity_report(&metadata, &projection, &reports);
    let existing_report = fs::read_to_string(&report_path).unwrap_or_default();
    if existing_report != rendered_report {
        changed.push(repo_relative(&repo_root, &report_path));
        if matches!(mode, Mode::Write) {
            fs::write(&report_path, rendered_report)
                .map_err(|err| format!("failed to write {}: {err}", report_path.display()))?;
        }
    }

    if matches!(mode, Mode::Check) && !changed.is_empty() {
        return Err(format!(
            "arcana_winapi raw bindings are out of date:\n{}",
            changed.join("\n")
        ));
    }

    Ok(())
}

fn parse_mode(args: impl Iterator<Item = String>) -> GenResult<Mode> {
    let args = args.collect::<Vec<_>>();
    match args.as_slice() {
        [flag] if flag == "--write" => Ok(Mode::Write),
        [flag] if flag == "--check" => Ok(Mode::Check),
        [] => Err("usage: cargo run -p arcana-winapi-gen -- --write|--check".to_string()),
        _ => Err(format!(
            "unsupported arguments `{}`; expected --write or --check",
            args.join(" ")
        )),
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir should have workspace parent")
        .parent()
        .expect("workspace crates dir should have repo root")
        .to_path_buf()
}

fn load_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> GenResult<T> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    toml::from_str(&text).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn resolve_metadata_path(metadata: &MetadataConfig) -> GenResult<PathBuf> {
    if let Ok(value) = env::var(&metadata.env_var) {
        let path = PathBuf::from(value);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "{} points to {}, but that file does not exist",
            metadata.env_var,
            path.display()
        ));
    }

    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .map(|path| path.join(".cargo"))
        })
        .ok_or_else(|| {
            format!(
                "failed to locate {}; set {} to the pinned {} snapshot",
                metadata.file_name, metadata.env_var, metadata.cargo_registry_package
            )
        })?;
    let registry_src = cargo_home.join("registry").join("src");
    let relative = PathBuf::from(&metadata.cargo_registry_relative_path);
    let mut candidates = Vec::new();
    if registry_src.is_dir() {
        let roots = fs::read_dir(&registry_src)
            .map_err(|err| format!("failed to read {}: {err}", registry_src.display()))?;
        for root in roots.flatten() {
            let path = root
                .path()
                .join(&metadata.cargo_registry_package)
                .join(&relative);
            if path.is_file() {
                candidates.push(path);
            }
        }
    }
    candidates.into_iter().next().ok_or_else(|| {
        format!(
            "failed to locate pinned Windows metadata {}; set {} or install {} under the cargo registry",
            metadata.file_name, metadata.env_var, metadata.cargo_registry_package
        )
    })
}

fn resolve_windows_sys_root(metadata: &MetadataConfig) -> GenResult<PathBuf> {
    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .map(|path| path.join(".cargo"))
        })
        .ok_or_else(|| {
            format!(
                "failed to locate {}; install {} under the cargo registry",
                metadata.windows_sys_package, metadata.windows_sys_package
            )
        })?;
    let registry_src = cargo_home.join("registry").join("src");
    if !registry_src.is_dir() {
        return Err(format!(
            "failed to locate cargo registry source root {}; install {} first",
            registry_src.display(),
            metadata.windows_sys_package
        ));
    }

    let roots = fs::read_dir(&registry_src)
        .map_err(|err| format!("failed to read {}: {err}", registry_src.display()))?;
    for root in roots.flatten() {
        let candidate = root
            .path()
            .join(&metadata.windows_sys_package)
            .join(&metadata.windows_sys_relative_root);
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    Err(format!(
        "failed to locate {} under the cargo registry source roots",
        metadata.windows_sys_package
    ))
}

fn sha256_hex(path: &Path) -> GenResult<String> {
    let bytes =
        fs::read(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let digest = Sha256::digest(bytes);
    Ok(format!("{digest:X}"))
}

fn validate_projection(
    repo_root: &Path,
    projection: &ProjectionConfig,
    manifests: Manifests<'_>,
) -> GenResult<()> {
    let supported_kinds = ["callbacks", "constants", "imports", "raw-shim", "types"];
    let configured = projection
        .leaves
        .iter()
        .map(|leaf| {
            if !supported_kinds.contains(&leaf.kind.as_str()) {
                return Err(format!(
                    "projection leaf `{}` uses unsupported kind `{}`",
                    leaf.name, leaf.kind
                ));
            }
            Ok((leaf.name.clone(), leaf.kind.clone()))
        })
        .collect::<GenResult<Vec<_>>>()?;

    let configured_names = configured
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    if configured_names.len() != configured.len() {
        return Err("projection.toml contains duplicate raw leaf names".to_string());
    }

    let output_root = repo_root
        .join("grimoires")
        .join("arcana")
        .join("winapi")
        .join("src")
        .join("raw");
    let output_names = arc_names_in_dir(&output_root)?;
    if output_names != configured_names {
        return Err(format!(
            "projection/output mismatch: projection={:?}, outputs={:?}",
            configured_names, output_names
        ));
    }

    let legacy_input_root = repo_root
        .join("grimoires")
        .join("arcana")
        .join("winapi")
        .join("generation")
        .join("inputs")
        .join("raw");
    if legacy_input_root.exists() {
        return Err(format!(
            "legacy handwritten raw input subtree must be removed once generation is metadata-driven: {}",
            legacy_input_root.display()
        ));
    }

    validate_raw_routing(repo_root, projection)?;

    let imports_leaves = manifests.imports.keys().cloned().collect::<BTreeSet<_>>();
    let expected_import_leaves = configured
        .iter()
        .filter(|(_, kind)| kind == "imports")
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    if imports_leaves != expected_import_leaves {
        return Err(format!(
            "imports.toml must cover exactly the import leaves in projection.toml\nexpected: {:?}\nfound: {:?}",
            expected_import_leaves, imports_leaves
        ));
    }
    let supported_parity_leaves = manifests
        .windows_sys_parity
        .supported_leaves
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let skipped_parity_leaves = manifests
        .windows_sys_parity
        .skipped_leaves
        .iter()
        .map(|entry| entry.leaf.clone())
        .collect::<BTreeSet<_>>();
    if !supported_parity_leaves.is_disjoint(&skipped_parity_leaves) {
        return Err(
            "windows-sys parity manifest must not classify the same raw import leaf as both supported and skipped"
                .to_string(),
        );
    }
    let scoped_parity_leaves = supported_parity_leaves
        .union(&skipped_parity_leaves)
        .cloned()
        .collect::<BTreeSet<_>>();
    if scoped_parity_leaves != expected_import_leaves {
        return Err(format!(
            "windows-sys parity manifest must classify every import leaf exactly once\nexpected: {:?}\nfound: {:?}",
            expected_import_leaves, scoped_parity_leaves
        ));
    }
    for leaf in &expected_import_leaves {
        let projection = manifests.imports.get(leaf).unwrap_or_else(|| {
            panic!("validated import leaf `{leaf}` should exist in imports manifest")
        });
        if projection.symbols.is_empty() && projection.namespaces.is_empty() {
            return Err(format!(
                "import leaf `{leaf}` must use explicit symbols, namespace discovery, or both"
            ));
        }
        if projection
            .libraries
            .iter()
            .any(|library| normalize_library_name(library).is_empty())
        {
            return Err(format!(
                "import leaf `{leaf}` contains an empty library match entry"
            ));
        }
    }

    let constant_leaves = manifests
        .constants
        .entries
        .iter()
        .map(|entry| entry.leaf.clone())
        .collect::<BTreeSet<_>>();
    let expected_constant_leaves = configured
        .iter()
        .filter(|(_, kind)| kind == "constants")
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    if !constant_leaves.is_subset(&expected_constant_leaves) {
        return Err(format!(
            "constants.toml must target only the constant leaves in projection.toml\nexpected subset of: {:?}\nfound: {:?}",
            expected_constant_leaves, constant_leaves
        ));
    }
    let skipped_constant_ids = manifests
        .skiplist
        .entries
        .iter()
        .filter(|entry| entry.kind == "constant")
        .map(|entry| entry.id.as_str())
        .collect::<BTreeSet<_>>();
    for leaf in expected_constant_leaves.difference(&constant_leaves) {
        if !skipped_constant_ids
            .iter()
            .any(|id| id.starts_with(leaf.as_str()))
            && !skipped_constant_ids.contains(leaf.as_str())
        {
            return Err(format!(
                "constant leaf `{leaf}` has no generated entries; add metadata-backed entries or an explicit skiplist rationale"
            ));
        }
    }

    let callback_leaves = configured
        .iter()
        .filter(|(_, kind)| kind == "callbacks")
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    if callback_leaves != ["callbacks".to_string()] {
        return Err(format!(
            "projection.toml should contain exactly one callbacks leaf named `callbacks`, found {:?}",
            callback_leaves
        ));
    }
    if manifests.callbacks.entries.is_empty() {
        return Err("callbacks.toml must contain at least one projected callback".to_string());
    }

    let type_leaves = configured
        .iter()
        .filter(|(_, kind)| kind == "types")
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    if type_leaves != ["types".to_string()] {
        return Err(format!(
            "projection.toml should contain exactly one types leaf named `types`, found {:?}",
            type_leaves
        ));
    }
    if manifests.types.projected.is_empty() {
        return Err("types.toml must contain projected metadata-backed types".to_string());
    }

    let raw_shim_leaves = configured
        .iter()
        .filter(|(_, kind)| kind == "raw-shim")
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    let exception_leaves = manifests
        .exceptions
        .exceptions
        .iter()
        .map(|entry| entry.leaf.clone())
        .collect::<BTreeSet<_>>();
    if raw_shim_leaves != exception_leaves {
        return Err(format!(
            "raw-shim leaves must match the checked-in exception manifest\nexpected: {:?}\nfound: {:?}",
            raw_shim_leaves, exception_leaves
        ));
    }

    if manifests.skiplist.entries.is_empty() {
        return Err("skiplist.toml must exist and carry at least one explicit entry".to_string());
    }

    let required_legacy = [
        "session_policy_bootstrap",
        "avrt_registration_helper",
        "xaudio2_bootstrap_helper",
        "x3daudio_bootstrap_helper",
    ];
    for legacy in required_legacy {
        if !manifests
            .exceptions
            .legacy_items
            .iter()
            .any(|item| item.name == legacy)
        {
            return Err(format!(
                "exceptions.toml must classify approved legacy bootstrap-ish item `{legacy}`"
            ));
        }
    }

    for entry in &manifests.skiplist.entries {
        if entry.kind.trim().is_empty()
            || entry.id.trim().is_empty()
            || entry.reason.trim().is_empty()
        {
            return Err("skiplist.toml entries must include kind, id, and reason".to_string());
        }
    }

    for item in &manifests.exceptions.legacy_items {
        if !configured_names.contains(item.leaf.as_str()) {
            return Err(format!(
                "legacy item `{}` points at unknown raw leaf `{}`",
                item.name, item.leaf
            ));
        }
        match item.resolution.as_str() {
            "ordinary-generated" => {
                if item.symbols.is_empty() || item.exception_id.is_some() {
                    return Err(format!(
                        "legacy item `{}` must name projected symbols and must not name an exception id when resolution is ordinary-generated",
                        item.name
                    ));
                }
            }
            "exception" => {
                if item.symbols.is_empty() {
                    return Err(format!(
                        "legacy item `{}` must name at least one exceptional symbol",
                        item.name
                    ));
                }
                let Some(exception_id) = &item.exception_id else {
                    return Err(format!(
                        "legacy item `{}` is marked as an exception but has no exception_id",
                        item.name
                    ));
                };
                if !manifests
                    .exceptions
                    .exceptions
                    .iter()
                    .any(|exception| &exception.id == exception_id)
                {
                    return Err(format!(
                        "legacy item `{}` references missing exception `{}`",
                        item.name, exception_id
                    ));
                }
            }
            other => {
                return Err(format!(
                    "legacy item `{}` uses unsupported resolution `{other}`",
                    item.name
                ));
            }
        }
    }

    for exception in &manifests.exceptions.exceptions {
        if !configured_names.contains(exception.leaf.as_str()) {
            return Err(format!(
                "exception `{}` points at unknown raw leaf `{}`",
                exception.id, exception.leaf
            ));
        }
        if exception.symbol.trim().is_empty()
            || exception.reason.trim().is_empty()
            || exception.implementation.trim().is_empty()
        {
            return Err(format!(
                "exception `{}` must include symbol, reason, and implementation text",
                exception.id
            ));
        }
    }

    for divergence in &manifests.windows_sys_parity.divergences {
        if !supported_parity_leaves.contains(divergence.leaf.as_str()) {
            return Err(format!(
                "windows-sys parity divergence for `{}` must target a supported import leaf, found `{}`",
                divergence.symbol, divergence.leaf
            ));
        }
        match divergence.kind.as_str() {
            "missing" | "unexpected" | "library-mismatch" => {}
            other => {
                return Err(format!(
                    "windows-sys parity divergence for `{}` uses unsupported kind `{other}`",
                    divergence.symbol
                ));
            }
        }
        if divergence.reason.trim().is_empty() {
            return Err(format!(
                "windows-sys parity divergence for `{}` must include a reason",
                divergence.symbol
            ));
        }
    }
    for entry in &manifests.windows_sys_parity.skipped_leaves {
        if !expected_import_leaves.contains(entry.leaf.as_str()) {
            return Err(format!(
                "windows-sys parity skip for `{}` must target an import leaf",
                entry.leaf
            ));
        }
        if entry.reason.trim().is_empty() {
            return Err(format!(
                "windows-sys parity skip for `{}` must include a reason",
                entry.leaf
            ));
        }
    }

    Ok(())
}

fn validate_raw_routing(repo_root: &Path, projection: &ProjectionConfig) -> GenResult<()> {
    let raw_arc = repo_root
        .join("grimoires")
        .join("arcana")
        .join("winapi")
        .join("src")
        .join("raw.arc");
    let text = fs::read_to_string(&raw_arc)
        .map_err(|err| format!("failed to read {}: {err}", raw_arc.display()))?;
    let routed = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .filter_map(|line| line.strip_prefix("reexport arcana_winapi.raw."))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let expected = projection
        .leaves
        .iter()
        .map(|leaf| leaf.name.clone())
        .collect::<Vec<_>>();
    if routed != expected {
        return Err(format!(
            "grimoires/arcana/winapi/src/raw.arc must reexport only configured raw leaves in projection order\nexpected: {:?}\nfound: {:?}",
            expected, routed
        ));
    }
    Ok(())
}

fn arc_names_in_dir(dir: &Path) -> GenResult<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for entry in
        fs::read_dir(dir).map_err(|err| format!("failed to read {}: {err}", dir.display()))?
    {
        let entry =
            entry.map_err(|err| format!("failed to read {} entry: {err}", dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("arc") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("non-utf8 raw leaf path {}", path.display()))?
            .to_string();
        names.insert(name);
    }
    Ok(names)
}

fn render_leaf_body<'a>(
    leaf: &ProjectionLeaf,
    type_index: &'a TypeIndex,
    item_index: &'a ItemIndex<'a>,
    manifests: Manifests<'a>,
    windows_sys_root: &Path,
) -> GenResult<RenderedLeaf> {
    match leaf.kind.as_str() {
        "imports" => render_imports_leaf(
            leaf,
            type_index,
            item_index,
            RenderImportsContext {
                imports: manifests.imports,
                types: manifests.types,
                skiplist: manifests.skiplist,
                windows_sys_parity: manifests.windows_sys_parity,
                windows_sys_root,
            },
        ),
        "constants" => Ok(RenderedLeaf {
            source_of_truth: "grimoires/arcana/winapi/generation/constants.toml",
            body: render_constants_leaf(leaf, type_index, item_index, manifests.constants)?,
            report: LeafReport {
                name: leaf.name.clone(),
                kind: leaf.kind.clone(),
                lines: vec![format!(
                    "metadata-backed constants: {}",
                    manifests
                        .constants
                        .entries
                        .iter()
                        .filter(|entry| entry.leaf == leaf.name)
                        .count()
                )],
            },
        }),
        "callbacks" => Ok(RenderedLeaf {
            source_of_truth: "grimoires/arcana/winapi/generation/callbacks.toml",
            body: render_callbacks_leaf(
                type_index,
                item_index,
                manifests.callbacks,
                manifests.types,
            )?,
            report: LeafReport {
                name: leaf.name.clone(),
                kind: leaf.kind.clone(),
                lines: vec![format!(
                    "projected callbacks: {}",
                    manifests.callbacks.entries.len()
                )],
            },
        }),
        "types" => Ok(RenderedLeaf {
            source_of_truth: "grimoires/arcana/winapi/generation/types.toml",
            body: render_types_leaf(type_index, item_index, manifests.types)?,
            report: LeafReport {
                name: leaf.name.clone(),
                kind: leaf.kind.clone(),
                lines: vec![
                    format!("aliases: {}", manifests.types.aliases.len()),
                    format!("array aliases: {}", manifests.types.array_aliases.len()),
                    format!("manual records: {}", manifests.types.manual_records.len()),
                    format!(
                        "projected metadata types: {}",
                        manifests.types.projected.len()
                    ),
                    format!(
                        "nested projected records: {}",
                        manifests.types.nested_records.len()
                    ),
                    format!(
                        "interface projections: {}",
                        manifests.types.interfaces.len()
                    ),
                ],
            },
        }),
        "raw-shim" => Ok(RenderedLeaf {
            source_of_truth: "grimoires/arcana/winapi/generation/exceptions.toml",
            body: render_raw_shim_leaf(leaf, manifests.exceptions)?,
            report: LeafReport {
                name: leaf.name.clone(),
                kind: leaf.kind.clone(),
                lines: manifests
                    .exceptions
                    .exceptions
                    .iter()
                    .filter(|entry| entry.leaf == leaf.name)
                    .map(|entry| {
                        format!(
                            "exception `{}` for `{}` via {}",
                            entry.id, entry.symbol, entry.implementation
                        )
                    })
                    .collect(),
            },
        }),
        other => Err(format!(
            "unsupported projection kind `{other}` for raw leaf `{}`",
            leaf.name
        )),
    }
}

fn render_imports_leaf<'a>(
    leaf: &ProjectionLeaf,
    type_index: &'a TypeIndex,
    item_index: &'a ItemIndex<'a>,
    ctx: RenderImportsContext<'a>,
) -> GenResult<RenderedLeaf> {
    let Some(projection) = ctx.imports.get(&leaf.name) else {
        return Err(format!(
            "missing import projection group for raw leaf `{}`",
            leaf.name
        ));
    };
    let binding_library = leaf.binding_library.as_deref().unwrap_or(&leaf.name);
    let type_names = projected_type_names(ctx.types);
    let renderer = TypeRenderer::new(type_index, ctx.types, &type_names, true);
    let mut rendered = Vec::new();
    let selection = collect_import_selection(leaf, item_index, projection, ctx.skiplist)?;
    let windows_sys_parity = compare_import_selection_with_windows_sys(
        leaf,
        projection,
        &selection,
        ctx.skiplist,
        ctx.windows_sys_parity,
        ctx.windows_sys_root,
    )?;
    for candidate in &selection.emitted {
        let method = candidate.method;
        let params = render_params(method, &renderer)
            .map_err(|err| format!("failed to render {}: {err}", candidate.metadata_id))?;
        let return_type = render_return_type(method.signature(&[]).return_type.clone(), &renderer)
            .map_err(|err| format!("failed to render {}: {err}", candidate.metadata_id))?;
        let mut line = format!(
            "export shackle import fn {}({})",
            candidate.symbol,
            params.join(", ")
        );
        if let Some(return_type) = return_type {
            line.push_str(&format!(" -> {return_type}"));
        }
        line.push_str(&format!(" = {}.{}", binding_library, candidate.symbol));
        rendered.push(line);
    }
    let strategy = if !projection.namespaces.is_empty() && !projection.symbols.is_empty() {
        "hybrid namespace discovery + explicit symbols"
    } else if !projection.namespaces.is_empty() {
        "namespace-driven broad discovery"
    } else {
        "explicit symbol list"
    };
    let mut report_lines = vec![format!("strategy: {strategy}")];
    report_lines.push(format!("binding library: {binding_library}"));
    if !projection.libraries.is_empty() {
        report_lines.push(format!(
            "matched import libraries: {}",
            projection.libraries.join(", ")
        ));
    }
    if !projection.namespaces.is_empty() {
        report_lines.push(format!(
            "namespace prefixes: {}",
            projection.namespaces.join(", ")
        ));
    }
    if !projection.symbol_prefixes.is_empty() {
        report_lines.push(format!(
            "symbol prefixes: {}",
            projection.symbol_prefixes.join(", ")
        ));
    }
    report_lines.push(format!(
        "metadata candidates: {}",
        selection.metadata_candidates
    ));
    report_lines.push(format!("emitted declarations: {}", selection.emitted.len()));
    report_lines.push(format!(
        "windows-sys parity scope: {}",
        if windows_sys_parity.skipped_reason.is_some() {
            "skipped"
        } else {
            "compared"
        }
    ));
    if let Some(reason) = &windows_sys_parity.skipped_reason {
        report_lines.push(format!("windows-sys parity skip reason: {reason}"));
    } else {
        report_lines.push(format!(
            "windows-sys parity: exact symbol parity across {} matched declarations",
            windows_sys_parity.matched
        ));
    }
    if windows_sys_parity.classified_divergences > 0 {
        report_lines.push(format!(
            "windows-sys classified divergences: {}",
            windows_sys_parity.classified_divergences
        ));
    }
    report_lines.push(format!("excluded by config: {}", selection.excluded.len()));
    report_lines.push(format!("skipped by skiplist: {}", selection.skipped.len()));
    if !selection.excluded.is_empty() {
        let mut excluded_symbols = selection
            .excluded
            .iter()
            .map(|candidate| candidate.symbol.as_str())
            .collect::<Vec<_>>();
        excluded_symbols.sort_unstable();
        report_lines.push(format!("excluded symbols: {}", excluded_symbols.join(", ")));
    }
    if !selection.skipped.is_empty() {
        let mut skipped_ids = selection
            .skipped
            .iter()
            .map(|candidate| candidate.metadata_id.as_str())
            .collect::<Vec<_>>();
        skipped_ids.sort_unstable();
        report_lines.push(format!("skipped metadata ids: {}", skipped_ids.join(", ")));
    }
    Ok(RenderedLeaf {
        source_of_truth: "grimoires/arcana/winapi/generation/imports.toml",
        body: rendered.join("\n"),
        report: LeafReport {
            name: leaf.name.clone(),
            kind: leaf.kind.clone(),
            lines: report_lines,
        },
    })
}

fn collect_import_selection<'a>(
    leaf: &ProjectionLeaf,
    item_index: &'a ItemIndex<'a>,
    projection: &'a ImportProjection,
    skiplist: &'a SkiplistManifest,
) -> GenResult<ImportSelection<'a>> {
    let allowed_libraries = if projection.libraries.is_empty() {
        vec![
            leaf.binding_library
                .as_deref()
                .unwrap_or(&leaf.name)
                .to_string(),
        ]
    } else {
        projection.libraries.clone()
    };
    let allowed_libraries = allowed_libraries
        .iter()
        .map(|library| normalize_library_name(library))
        .collect::<BTreeSet<_>>();
    let excluded_symbols = projection
        .exclude_symbols
        .iter()
        .map(|symbol| symbol.as_str())
        .collect::<BTreeSet<_>>();
    let skipped_imports = skiplist
        .entries
        .iter()
        .filter(|entry| entry.kind == "import")
        .map(|entry| entry.id.as_str())
        .collect::<BTreeSet<_>>();

    let mut metadata_candidates = 0usize;
    let mut emitted = BTreeMap::<String, ImportCandidate<'a>>::new();
    let mut excluded = Vec::new();
    let mut skipped = Vec::new();

    if !projection.namespaces.is_empty() {
        for (namespace, item_name, item) in item_index.iter() {
            let Item::Fn(method) = item else {
                continue;
            };
            if !namespace_matches(namespace, &projection.namespaces) {
                continue;
            }
            let Some(candidate) = import_candidate(namespace, item_name, *method) else {
                continue;
            };
            if !allowed_libraries.is_empty()
                && !allowed_libraries.contains(&normalize_library_name(&candidate.import_library))
            {
                continue;
            }
            metadata_candidates += 1;
            if !projection.symbol_prefixes.is_empty()
                && !projection
                    .symbol_prefixes
                    .iter()
                    .any(|prefix| candidate.symbol.starts_with(prefix))
            {
                excluded.push(candidate);
                continue;
            }
            if excluded_symbols.contains(candidate.symbol.as_str()) {
                excluded.push(candidate);
                continue;
            }
            if skipped_imports.contains(candidate.metadata_id.as_str()) {
                skipped.push(candidate);
                continue;
            }
            insert_import_candidate(&mut emitted, candidate)?;
        }
    }

    for symbol in &projection.symbols {
        metadata_candidates += 1;
        let candidate = find_import_function(item_index, symbol, &allowed_libraries)?;
        if excluded_symbols.contains(candidate.symbol.as_str()) {
            excluded.push(candidate);
            continue;
        }
        if skipped_imports.contains(candidate.metadata_id.as_str()) {
            skipped.push(candidate);
            continue;
        }
        insert_import_candidate(&mut emitted, candidate)?;
    }

    Ok(ImportSelection {
        metadata_candidates,
        emitted: emitted.into_values().collect(),
        excluded,
        skipped,
    })
}

fn insert_import_candidate<'a>(
    emitted: &mut BTreeMap<String, ImportCandidate<'a>>,
    candidate: ImportCandidate<'a>,
) -> GenResult<()> {
    if let Some(existing) = emitted.get(&candidate.symbol) {
        if existing.metadata_id != candidate.metadata_id {
            return Err(format!(
                "raw import symbol `{}` is ambiguous between {} and {}; add projection disambiguation",
                candidate.symbol, existing.metadata_id, candidate.metadata_id
            ));
        }
        return Ok(());
    }
    emitted.insert(candidate.symbol.clone(), candidate);
    Ok(())
}

fn import_candidate<'a>(
    namespace: &'a str,
    item_name: &'a str,
    method: MethodDef<'a>,
) -> Option<ImportCandidate<'a>> {
    let import_library = method.impl_map()?.import_scope().name().to_string();
    Some(ImportCandidate {
        symbol: item_name.to_string(),
        metadata_id: format!("{namespace}.{item_name}"),
        import_library,
        method,
    })
}

fn find_import_function<'a>(
    item_index: &'a ItemIndex<'a>,
    name: &str,
    allowed_libraries: &BTreeSet<String>,
) -> GenResult<ImportCandidate<'a>> {
    let matches = item_index
        .iter()
        .filter_map(|(namespace, item_name, item)| match item {
            Item::Fn(method) if item_name == name => {
                import_candidate(namespace, item_name, *method)
            }
            _ => None,
        })
        .filter(|candidate| {
            allowed_libraries.is_empty()
                || allowed_libraries.contains(&normalize_library_name(&candidate.import_library))
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [method] => Ok(method.clone()),
        [] => Err(format!(
            "failed to resolve metadata function `{name}` in the configured import-library set"
        )),
        _ => Err(format!(
            "metadata function `{name}` is ambiguous across the configured import-library set; add projection disambiguation"
        )),
    }
}

fn namespace_matches(namespace: &str, configured: &[String]) -> bool {
    configured.iter().any(|prefix| {
        namespace == prefix
            || namespace
                .strip_prefix(prefix.as_str())
                .is_some_and(|suffix| suffix.starts_with('.'))
    })
}

fn normalize_library_name(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    lower.strip_suffix(".dll").unwrap_or(&lower).to_string()
}

fn compare_import_selection_with_windows_sys(
    leaf: &ProjectionLeaf,
    projection: &ImportProjection,
    selection: &ImportSelection<'_>,
    skiplist: &SkiplistManifest,
    windows_sys_parity: &WindowsSysParityManifest,
    windows_sys_root: &Path,
) -> GenResult<WindowsSysParity> {
    if let Some(skip) = windows_sys_parity
        .skipped_leaves
        .iter()
        .find(|entry| entry.leaf == leaf.name)
    {
        return Ok(WindowsSysParity {
            matched: 0,
            classified_divergences: 0,
            skipped_reason: Some(skip.reason.clone()),
        });
    }
    if !windows_sys_parity
        .supported_leaves
        .iter()
        .any(|supported| supported == &leaf.name)
    {
        return Err(format!(
            "windows-sys parity manifest does not classify raw import leaf `{}`",
            leaf.name
        ));
    }

    let upstream =
        collect_windows_sys_import_selection(leaf, projection, skiplist, windows_sys_root)?;
    let emitted = selection
        .emitted
        .iter()
        .map(|candidate| {
            (
                candidate.symbol.clone(),
                normalize_library_name(&candidate.import_library),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut missing = Vec::new();
    let mut unexpected = Vec::new();
    let mut mismatched_libraries = Vec::new();

    for (symbol, import_library) in &emitted {
        match upstream.filtered.get(symbol) {
            Some(upstream_library) if upstream_library == import_library => {}
            Some(upstream_library) => mismatched_libraries.push(format!(
                "{symbol} (arcana={import_library}, windows-sys={upstream_library})"
            )),
            None => match upstream.all.get(symbol) {
                Some(upstream_candidate) => mismatched_libraries.push(format!(
                    "{symbol} (arcana={import_library}, windows-sys={})",
                    upstream_candidate.import_library
                )),
                None => unexpected.push(format!("{symbol} ({import_library})")),
            },
        }
    }
    for (symbol, import_library) in &upstream.filtered {
        if !emitted.contains_key(symbol) {
            missing.push(format!("{symbol} ({import_library})"));
        }
    }

    let divergence_filter = windows_sys_parity
        .divergences
        .iter()
        .filter(|entry| entry.leaf == leaf.name)
        .fold(BTreeMap::<&str, BTreeSet<&str>>::new(), |mut acc, entry| {
            acc.entry(entry.kind.as_str())
                .or_default()
                .insert(entry.symbol.as_str());
            acc
        });
    let classified = filter_divergences(&mut missing, divergence_filter.get("missing"))
        + filter_divergences(&mut unexpected, divergence_filter.get("unexpected"))
        + filter_divergences(
            &mut mismatched_libraries,
            divergence_filter.get("library-mismatch"),
        );

    if !missing.is_empty() || !unexpected.is_empty() || !mismatched_libraries.is_empty() {
        let mut message = vec![format!(
            "windows-sys parity mismatch for raw import leaf `{}`",
            leaf.name
        )];
        if !missing.is_empty() {
            message.push(format!(
                "missing from arcana raw surface: {}",
                summarize_items(&missing)
            ));
        }
        if !unexpected.is_empty() {
            message.push(format!(
                "unexpected in arcana raw surface: {}",
                summarize_items(&unexpected)
            ));
        }
        if !mismatched_libraries.is_empty() {
            message.push(format!(
                "library mismatches: {}",
                summarize_items(&mismatched_libraries)
            ));
        }
        return Err(message.join("\n"));
    }

    Ok(WindowsSysParity {
        matched: emitted.len(),
        classified_divergences: classified,
        skipped_reason: None,
    })
}

fn collect_windows_sys_import_selection(
    leaf: &ProjectionLeaf,
    projection: &ImportProjection,
    skiplist: &SkiplistManifest,
    windows_sys_root: &Path,
) -> GenResult<WindowsSysImportSelection> {
    let allowed_libraries = if projection.libraries.is_empty() {
        vec![
            leaf.binding_library
                .as_deref()
                .unwrap_or(&leaf.name)
                .to_string(),
        ]
    } else {
        projection.libraries.clone()
    };
    let allowed_libraries = allowed_libraries
        .iter()
        .map(|library| normalize_library_name(library))
        .collect::<BTreeSet<_>>();
    let excluded_symbols = projection
        .exclude_symbols
        .iter()
        .map(|symbol| symbol.as_str())
        .collect::<BTreeSet<_>>();
    let skipped_imports = skiplist
        .entries
        .iter()
        .filter(|entry| entry.kind == "import")
        .map(|entry| entry.id.as_str())
        .collect::<BTreeSet<_>>();

    let mut all = BTreeMap::<String, WindowsSysImport>::new();
    let mut seen_files = BTreeSet::new();
    for namespace_prefix in &projection.namespaces {
        let namespace_dir = windows_sys_namespace_dir(windows_sys_root, namespace_prefix)?;
        for path in walk_rs_files_recursive(&namespace_dir)? {
            if !seen_files.insert(path.clone()) {
                continue;
            }
            let namespace = windows_sys_namespace_from_path(windows_sys_root, &path)?;
            if !namespace_matches(&namespace, &projection.namespaces) {
                continue;
            }
            let source = fs::read_to_string(&path)
                .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
            for line in source.lines() {
                let Some((import_library, symbol)) = parse_windows_sys_link_line(line) else {
                    continue;
                };
                if !projection.symbol_prefixes.is_empty()
                    && !projection
                        .symbol_prefixes
                        .iter()
                        .any(|prefix| symbol.starts_with(prefix))
                {
                    continue;
                }
                if excluded_symbols.contains(symbol.as_str()) {
                    continue;
                }
                let metadata_id = format!("{namespace}.{symbol}");
                if skipped_imports.contains(metadata_id.as_str()) {
                    continue;
                }
                let candidate = WindowsSysImport {
                    symbol: symbol.clone(),
                    metadata_id,
                    import_library,
                };
                insert_windows_sys_import(&mut all, candidate)?;
            }
        }
    }

    for symbol in &projection.symbols {
        if all.contains_key(symbol) {
            continue;
        }
        return Err(format!(
            "windows-sys parity check could not find explicit import symbol `{symbol}` for raw leaf `{}`",
            leaf.name
        ));
    }

    let filtered = all
        .iter()
        .filter(|(_, candidate)| {
            allowed_libraries.is_empty() || allowed_libraries.contains(&candidate.import_library)
        })
        .map(|(symbol, candidate)| (symbol.clone(), candidate.import_library.clone()))
        .collect::<BTreeMap<_, _>>();

    Ok(WindowsSysImportSelection { filtered, all })
}

fn filter_divergences(items: &mut Vec<String>, allowed_symbols: Option<&BTreeSet<&str>>) -> usize {
    let Some(allowed_symbols) = allowed_symbols else {
        return 0;
    };
    let mut classified = 0usize;
    items.retain(|item| {
        let symbol = item
            .split_once(' ')
            .map(|(symbol, _)| symbol)
            .unwrap_or(item.as_str());
        let keep = !allowed_symbols.contains(symbol);
        if !keep {
            classified += 1;
        }
        keep
    });
    classified
}

fn windows_sys_namespace_dir(windows_sys_root: &Path, namespace: &str) -> GenResult<PathBuf> {
    let relative = namespace
        .strip_prefix("Windows.Win32.")
        .ok_or_else(|| format!("unsupported windows-sys namespace `{namespace}`"))?
        .split('.')
        .collect::<PathBuf>();
    let path = windows_sys_root.join(relative);
    if path.is_dir() {
        Ok(path)
    } else {
        Err(format!(
            "windows-sys source is missing namespace directory `{}` at {}",
            namespace,
            path.display()
        ))
    }
}

fn walk_rs_files_recursive(root: &Path) -> GenResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let mut entries = fs::read_dir(&dir)
            .map_err(|err| format!("failed to read {}: {err}", dir.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| {
                format!(
                    "failed to read directory entry under {}: {err}",
                    dir.display()
                )
            })?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn windows_sys_namespace_from_path(windows_sys_root: &Path, path: &Path) -> GenResult<String> {
    let relative = path.strip_prefix(windows_sys_root).map_err(|_| {
        format!(
            "windows-sys source path {} is not under {}",
            path.display(),
            windows_sys_root.display()
        )
    })?;
    let mut parts = vec!["Windows".to_string(), "Win32".to_string()];
    for component in relative.components() {
        let component = component.as_os_str().to_string_lossy();
        if component == "mod.rs" {
            continue;
        }
        if let Some(stem) = component.strip_suffix(".rs") {
            parts.push(stem.to_string());
        } else {
            parts.push(component.to_string());
        }
    }
    Ok(parts.join("."))
}

fn parse_windows_sys_link_line(line: &str) -> Option<(String, String)> {
    let marker = if line.contains("windows_link::link!(") {
        "windows_link::link!(\""
    } else if line.contains("windows_targets::link!(") {
        "windows_targets::link!(\""
    } else {
        return None;
    };
    let rest = line.split_once(marker)?.1;
    let library = rest.split_once('"')?.0;
    let after_library = rest.split_once(" fn ")?.1;
    let symbol = after_library.split_once('(')?.0.trim();
    Some((normalize_library_name(library), symbol.to_string()))
}

fn insert_windows_sys_import(
    emitted: &mut BTreeMap<String, WindowsSysImport>,
    candidate: WindowsSysImport,
) -> GenResult<()> {
    if let Some(existing) = emitted.get(&candidate.symbol) {
        if existing.metadata_id != candidate.metadata_id
            || existing.import_library != candidate.import_library
        {
            return Err(format!(
                "windows-sys import symbol `{}` is ambiguous between {} ({}) and {} ({})",
                candidate.symbol,
                existing.metadata_id,
                existing.import_library,
                candidate.metadata_id,
                candidate.import_library
            ));
        }
        return Ok(());
    }
    emitted.insert(candidate.symbol.clone(), candidate);
    Ok(())
}

fn summarize_items(items: &[String]) -> String {
    const LIMIT: usize = 12;

    let mut summary = items.iter().take(LIMIT).cloned().collect::<Vec<_>>();
    if items.len() > LIMIT {
        summary.push(format!("... and {} more", items.len() - LIMIT));
    }
    summary.join(", ")
}

fn render_constants_leaf<'a>(
    leaf: &ProjectionLeaf,
    type_index: &'a TypeIndex,
    item_index: &'a ItemIndex<'a>,
    constants: &'a ConstantsManifest,
) -> GenResult<String> {
    let mut rendered = Vec::new();
    for entry in constants
        .entries
        .iter()
        .filter(|entry| entry.leaf == leaf.name)
    {
        let value = if let (Some(source_type), Some(source_field)) =
            (&entry.source_type, &entry.source_field)
        {
            let ty = find_type(type_index, item_index, source_type)?;
            let field = ty
                .fields()
                .find(|field| field.name() == source_field)
                .ok_or_else(|| {
                    format!(
                        "failed to resolve constant {}.{} for export `{}`",
                        source_type, source_field, entry.name
                    )
                })?;
            field.constant().ok_or_else(|| {
                format!(
                    "metadata field {}.{} has no literal constant value",
                    source_type, source_field
                )
            })?
        } else {
            let field = find_constant(item_index, &entry.name)?;
            field.constant().ok_or_else(|| {
                format!(
                    "metadata field `{}` has no literal constant value",
                    entry.name
                )
            })?
        };
        let literal = render_value(value.value())?;
        rendered.push(format!(
            "export shackle const {}: {} = {}",
            entry.name,
            qualify_type_name(&entry.r#type),
            literal
        ));
    }
    Ok(rendered.join("\n"))
}

fn render_callbacks_leaf<'a>(
    type_index: &'a TypeIndex,
    item_index: &'a ItemIndex<'a>,
    callbacks: &'a CallbacksManifest,
    types: &'a TypesManifest,
) -> GenResult<String> {
    let type_names = projected_type_names(types);
    let renderer = TypeRenderer::new(type_index, types, &type_names, true);
    let mut rendered = Vec::new();
    for entry in &callbacks.entries {
        let signature = match entry.kind.as_str() {
            "delegate" => {
                let ty = find_type(type_index, item_index, &entry.source_type)?;
                let invoke = ty
                    .methods()
                    .find(|method| method.name() == "Invoke")
                    .ok_or_else(|| format!("delegate `{}` is missing Invoke", entry.source_type))?;
                invoke.signature(&[])
            }
            "interface-method" => {
                let ty = find_type(type_index, item_index, &entry.source_type)?;
                let method = collect_interface_methods(type_index, ty)?
                    .into_iter()
                    .find(|method| method.name() == entry.source_method)
                    .ok_or_else(|| {
                        format!(
                            "interface `{}` is missing method `{}` for callback `{}`",
                            entry.source_type, entry.source_method, entry.name
                        )
                    })?;
                method.signature(&[])
            }
            other => {
                return Err(format!(
                    "unsupported callback kind `{other}` for `{}`",
                    entry.name
                ));
            }
        };

        let mut params = Vec::new();
        let fallback_names = default_param_names(signature.types.len());
        let chosen_names = entry.param_names.clone().unwrap_or(fallback_names);
        if chosen_names.len() != signature.types.len() {
            return Err(format!(
                "callback `{}` expected {} parameter names, found {}",
                entry.name,
                signature.types.len(),
                chosen_names.len()
            ));
        }
        let param_modes = entry
            .param_modes
            .clone()
            .unwrap_or_else(|| vec![String::new(); signature.types.len()]);
        if param_modes.len() != signature.types.len() {
            return Err(format!(
                "callback `{}` expected {} parameter modes, found {}",
                entry.name,
                signature.types.len(),
                param_modes.len()
            ));
        }

        for ((name, mode), ty) in chosen_names
            .iter()
            .zip(param_modes.iter())
            .zip(signature.types.iter())
        {
            let ty = renderer.render(ty)?;
            if mode.is_empty() {
                params.push(format!("{}: {}", sanitize_identifier(name), ty));
            } else {
                params.push(format!("{} {}: {}", mode, sanitize_identifier(name), ty));
            }
        }

        let return_type = render_callback_return(&signature, &renderer)?;
        let mut line = format!(
            "export shackle callback {}({})",
            entry.name,
            params.join(", ")
        );
        if let Some(return_type) = return_type {
            line.push_str(&format!(" -> {return_type}"));
        }
        rendered.push(line);
    }
    Ok(rendered.join("\n"))
}

fn render_types_leaf<'a>(
    type_index: &'a TypeIndex,
    item_index: &'a ItemIndex<'a>,
    types: &'a TypesManifest,
) -> GenResult<String> {
    let type_names = projected_type_names(types);
    let renderer = TypeRenderer::new(type_index, types, &type_names, false);
    let mut sections = Vec::new();

    for alias in &types.aliases {
        sections.push(format!(
            "export shackle type {} = {}",
            alias.name, alias.expr
        ));
    }
    for alias in derived_pointer_aliases(types) {
        sections.push(format!(
            "export shackle type {} = {}",
            alias.name, alias.expr
        ));
    }
    for alias in &types.array_aliases {
        sections.push(format!(
            "export shackle type {} = [{}; {}]",
            alias.name, alias.element, alias.len
        ));
    }
    for record in &types.manual_records {
        sections.push(render_manual_record(record));
    }
    for projected in &types.projected {
        let metadata_name = projected.source_name.as_deref().unwrap_or(&projected.name);
        let ty = find_type(type_index, item_index, metadata_name)?;
        sections.push(render_projected_type(&renderer, projected, ty)?);
    }
    for record in &types.nested_records {
        let parent = find_type(type_index, item_index, &record.parent)?;
        let nested = find_nested_type(type_index, parent, &record.source_name)?;
        sections.push(render_nested_record(&renderer, record, nested)?);
    }
    for interface in &types.interfaces {
        let ty = find_type(type_index, item_index, &interface.name)?;
        if interface.emit_vtable {
            sections.push(render_interface_vtable(&renderer, ty)?);
        }
        if interface.emit_pointer_alias {
            sections.push(format!(
                "export shackle type {} = *mut c_void",
                interface.name
            ));
        }
    }

    Ok(sections.join("\n\n"))
}

fn render_raw_shim_leaf(
    leaf: &ProjectionLeaf,
    exceptions: &ExceptionManifest,
) -> GenResult<String> {
    match leaf.name.as_str() {
        "x3daudio" => {
            let exception = exceptions
                .exceptions
                .iter()
                .find(|entry| entry.id == "x3daudio_initialize_dynload")
                .ok_or_else(|| {
                    "exceptions.toml must retain x3daudio_initialize_dynload".to_string()
                })?;
            if exception.leaf != "x3daudio" || exception.symbol != "X3DAudioInitialize" {
                return Err(
                    "x3daudio raw-shim leaf must stay aligned with the checked-in exception entry"
                        .to_string(),
                );
            }
            Ok(
                [
                    "export shackle import fn X3DAudioInitialize(channel_mask: arcana_winapi.raw.types.DWORD, speed_of_sound: arcana_winapi.raw.types.FLOAT, handle: arcana_winapi.raw.types.LPVOID) -> arcana_winapi.raw.types.HRESULT = x3daudio.X3DAudioInitialize:",
                    "    pub(crate) type X3DAudioInitializeFn = unsafe extern \"system\" fn(",
                    "        crate::raw::types::DWORD,",
                    "        crate::raw::types::FLOAT,",
                    "        crate::raw::types::LPVOID,",
                    "    ) -> crate::raw::types::HRESULT;",
                    "    pub(crate) unsafe fn X3DAudioInitialize(",
                    "        channel_mask: crate::raw::types::DWORD,",
                    "        speed_of_sound: crate::raw::types::FLOAT,",
                    "        handle: crate::raw::types::LPVOID,",
                    "    ) -> crate::raw::types::HRESULT {",
                    "        let library_name = \"X3DAudio1_7.dll\\0\".encode_utf16().collect::<Vec<u16>>();",
                    "        let module = unsafe { crate::raw::kernel32::LoadLibraryW(library_name.as_ptr().cast_mut()) };",
                    "        if module.is_null() {",
                    "            return -1i32;",
                    "        }",
                    "        let symbol = b\"X3DAudioInitialize\\0\";",
                    "        let proc = unsafe { crate::raw::kernel32::GetProcAddress(module, symbol.as_ptr().cast::<i8>().cast_mut()) };",
                    "        if proc.is_null() {",
                    "            unsafe {",
                    "                crate::raw::kernel32::FreeLibrary(module);",
                    "            }",
                    "            return -1i32;",
                    "        }",
                    "        let init: X3DAudioInitializeFn = unsafe { std::mem::transmute(proc) };",
                    "        let hr = unsafe { init(channel_mask, speed_of_sound, handle) };",
                    "        unsafe {",
                    "            crate::raw::kernel32::FreeLibrary(module);",
                    "        }",
                    "        hr",
                    "    }",
                ]
                .join("\n"),
            )
        }
        other => Err(format!("unsupported raw-shim leaf `{other}`")),
    }
}

fn render_generated_file(metadata: &MetadataConfig, source_of_truth: &str, body: &str) -> String {
    let metadata_display = format!(
        "{} {} sha256:{}",
        metadata.file_name, metadata.metadata_version, metadata.sha256
    );
    let body = body.trim_end();
    format!(
        concat!(
            "// GENERATED FILE. DO NOT EDIT BY HAND.\n",
            "// Source of truth: {source_of_truth}\n",
            "// Projection config: grimoires/arcana/winapi/generation/projection.toml\n",
            "// Source authority: {authoritative_source}\n",
            "// Metadata authority: {metadata_display}\n",
            "// Parity target: {parity_target}; pinned metadata wins on disagreement.\n\n",
            "{body}\n"
        ),
        source_of_truth = source_of_truth,
        authoritative_source = metadata.authoritative_source,
        metadata_display = metadata_display,
        parity_target = metadata.parity_target,
        body = body
    )
}

fn render_parity_report(
    metadata: &MetadataConfig,
    projection: &ProjectionConfig,
    reports: &[LeafReport],
) -> String {
    let mut lines = vec![
        "# arcana_winapi raw parity report".to_string(),
        String::new(),
        format!("Source authority: {}", metadata.authoritative_source),
        format!(
            "Metadata authority: {} {} sha256:{}",
            metadata.file_name, metadata.metadata_version, metadata.sha256
        ),
        format!(
            "Parity target: {}; pinned metadata wins on disagreement.",
            metadata.parity_target
        ),
        format!(
            "Upstream parity source: {} under cargo registry {}",
            metadata.parity_target, metadata.windows_sys_package
        ),
        "Projection config: grimoires/arcana/winapi/generation/projection.toml".to_string(),
        String::new(),
    ];

    for leaf in &projection.leaves {
        let report = reports
            .iter()
            .find(|report| report.name == leaf.name)
            .unwrap_or_else(|| panic!("missing parity report entry for raw leaf `{}`", leaf.name));
        lines.push(format!("## {}", report.name));
        lines.push(format!("kind: {}", report.kind));
        for line in &report.lines {
            lines.push(format!("- {line}"));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

fn find_constant<'a>(item_index: &'a ItemIndex<'a>, name: &str) -> GenResult<Field<'a>> {
    let matches = item_index
        .iter()
        .filter_map(|(_, item_name, item)| match item {
            Item::Const(field) if item_name == name => Some(*field),
            _ => None,
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [field] => Ok(*field),
        [] => Err(format!("failed to resolve metadata constant `{name}`")),
        _ => Err(format!(
            "metadata constant `{name}` is ambiguous; add projection disambiguation"
        )),
    }
}

fn find_type<'a>(
    type_index: &'a TypeIndex,
    item_index: &'a ItemIndex<'a>,
    name: &str,
) -> GenResult<TypeDef<'a>> {
    let matches = item_index
        .iter()
        .filter_map(|(_, item_name, item)| match item {
            Item::Type(ty) if item_name == name => Some(*ty),
            _ => None,
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [ty] => Ok(*ty),
        [] => {
            let direct = type_index
                .types()
                .filter(|ty| ty.name() == name)
                .collect::<Vec<_>>();
            match direct.as_slice() {
                [ty] => Ok(*ty),
                [] => Err(format!("failed to resolve metadata type `{name}`")),
                _ => Err(format!(
                    "metadata type `{name}` is ambiguous; add projection disambiguation"
                )),
            }
        }
        _ => Err(format!(
            "metadata type `{name}` is ambiguous; add projection disambiguation"
        )),
    }
}

fn find_nested_type<'a>(
    type_index: &'a TypeIndex,
    parent: TypeDef<'a>,
    source_name: &str,
) -> GenResult<TypeDef<'a>> {
    let matches = type_index
        .nested(parent)
        .filter(|nested| nested.name() == source_name)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [ty] => Ok(*ty),
        [] => Err(format!(
            "failed to resolve nested metadata type {}.{}",
            parent.name(),
            source_name
        )),
        _ => Err(format!(
            "nested metadata type {}.{} is ambiguous",
            parent.name(),
            source_name
        )),
    }
}

fn render_projected_type<'a>(
    renderer: &TypeRenderer<'a>,
    projected: &ProjectedType,
    ty: TypeDef<'a>,
) -> GenResult<String> {
    match projected.kind.as_str() {
        "enum" => render_enum(renderer, &projected.name, ty),
        "struct" => render_record(renderer, "struct", &projected.name, ty),
        other => Err(format!(
            "unsupported projected type kind `{other}` for `{}`",
            projected.name
        )),
    }
}

fn render_enum<'a>(
    renderer: &TypeRenderer<'a>,
    export_name: &str,
    ty: TypeDef<'a>,
) -> GenResult<String> {
    let underlying = enum_underlying_type(ty)?;
    let mut lines = vec![format!(
        "export shackle type {} = {}:",
        export_name,
        renderer.render(&underlying)?
    )];
    for field in ty
        .fields()
        .filter(|field| field.flags().contains(FieldAttributes::Literal))
    {
        let value = field.constant().ok_or_else(|| {
            format!(
                "enum field {}.{} has no literal value",
                ty.name(),
                field.name()
            )
        })?;
        lines.push(format!(
            "    {} = {}",
            field.name(),
            render_value(value.value())?
        ));
    }
    Ok(lines.join("\n"))
}

fn enum_underlying_type(ty: TypeDef<'_>) -> GenResult<Type> {
    ty.fields()
        .find(|field| !field.flags().contains(FieldAttributes::Literal))
        .map(|field| field.ty())
        .ok_or_else(|| {
            format!(
                "enum `{}` is missing an underlying value__ field",
                ty.name()
            )
        })
}

fn render_record<'a>(
    renderer: &TypeRenderer<'a>,
    keyword: &str,
    export_name: &str,
    ty: TypeDef<'a>,
) -> GenResult<String> {
    let mut lines = vec![format!("export shackle {} {}:", keyword, export_name)];
    for field in ty
        .fields()
        .filter(|field| !field.flags().contains(FieldAttributes::Literal))
    {
        lines.push(format!(
            "    {}: {}",
            field.name(),
            renderer.render(&field.ty())?
        ));
    }
    Ok(lines.join("\n"))
}

fn render_nested_record<'a>(
    renderer: &TypeRenderer<'a>,
    record: &NestedRecord,
    ty: TypeDef<'a>,
) -> GenResult<String> {
    let mut lines = vec![format!("export shackle {} {}:", record.kind, record.name)];
    for field in ty
        .fields()
        .filter(|field| !field.flags().contains(FieldAttributes::Literal))
    {
        lines.push(format!(
            "    {}: {}",
            field.name(),
            renderer.render(&field.ty())?
        ));
    }
    Ok(lines.join("\n"))
}

fn render_manual_record(record: &ManualRecord) -> String {
    let mut lines = vec![format!("export shackle {} {}:", record.kind, record.name)];
    lines.extend(record.fields.iter().map(|field| format!("    {field}")));
    lines.join("\n")
}

fn render_interface_vtable<'a>(renderer: &TypeRenderer<'a>, ty: TypeDef<'a>) -> GenResult<String> {
    let methods = collect_interface_methods(renderer.type_index, ty)?;
    let mut lines = vec![format!("export shackle struct {}VTable:", ty.name())];
    let mut seen_names = BTreeMap::<String, usize>::new();
    for method in methods {
        let signature = method.signature(&[]);
        let mut params = vec!["*mut c_void".to_string()];
        for ty in &signature.types {
            params.push(renderer.render(ty)?);
        }
        let return_type = render_vtable_return(&signature, renderer)?;
        let field_name = unique_vtable_field_name(method.name(), &mut seen_names);
        let mut field = format!(
            "    {}: unsafe extern \"system\" fn({})",
            field_name,
            params.join(", ")
        );
        if let Some(return_type) = return_type {
            field.push_str(&format!(" -> {return_type}"));
        }
        lines.push(field);
    }
    Ok(lines.join("\n"))
}

fn unique_vtable_field_name(name: &str, seen_names: &mut BTreeMap<String, usize>) -> String {
    let seen = seen_names.entry(name.to_string()).or_default();
    *seen += 1;
    if *seen == 1 {
        name.to_string()
    } else {
        format!("{name}_{}", *seen)
    }
}

fn collect_interface_methods<'a>(
    type_index: &'a TypeIndex,
    ty: TypeDef<'a>,
) -> GenResult<Vec<MethodDef<'a>>> {
    let mut methods = Vec::new();
    for base in ty.interface_impls() {
        let base_type = base.interface(&[]);
        let Type::Name(base_name) = base_type else {
            return Err(format!(
                "interface `{}` extends an unsupported metadata type {:?}",
                ty.name(),
                base_type
            ));
        };
        let base_ty = type_index.expect(&base_name.namespace, &base_name.name);
        methods.extend(collect_interface_methods(type_index, base_ty)?);
    }
    methods.extend(ty.methods());
    Ok(methods)
}

fn render_params<'a>(method: MethodDef<'a>, renderer: &TypeRenderer<'a>) -> GenResult<Vec<String>> {
    let signature = method.signature(&[]);
    let names = method_param_names(method, signature.types.len());
    signature
        .types
        .iter()
        .zip(names.iter())
        .map(|(ty, name)| Ok(format!("{}: {}", name, renderer.render(ty)?)))
        .collect()
}

fn method_param_names(method: MethodDef<'_>, expected: usize) -> Vec<String> {
    let mut names = vec![String::new(); expected];
    for param in method.params() {
        let sequence = usize::from(param.sequence());
        if sequence == 0 || sequence > expected {
            continue;
        }
        names[sequence - 1] = sanitize_identifier(param.name());
    }
    names
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            if name.is_empty() {
                format!("arg_{}", index)
            } else {
                name
            }
        })
        .collect()
}

fn default_param_names(count: usize) -> Vec<String> {
    (0..count).map(|index| format!("arg_{}", index)).collect()
}

fn render_return_type<'a>(ty: Type, renderer: &TypeRenderer<'a>) -> GenResult<Option<String>> {
    if matches!(ty, Type::Void) {
        Ok(None)
    } else {
        Ok(Some(renderer.render(&ty)?))
    }
}

fn render_callback_return<'a>(
    signature: &Signature,
    renderer: &TypeRenderer<'a>,
) -> GenResult<Option<String>> {
    if matches!(signature.return_type, Type::Void) {
        Ok(None)
    } else {
        Ok(Some(renderer.render(&signature.return_type)?))
    }
}

fn render_vtable_return<'a>(
    signature: &Signature,
    renderer: &TypeRenderer<'a>,
) -> GenResult<Option<String>> {
    if matches!(signature.return_type, Type::Void) {
        Ok(None)
    } else {
        Ok(Some(renderer.render(&signature.return_type)?))
    }
}

fn render_value(value: Value) -> GenResult<String> {
    match value {
        Value::Bool(value) => Ok(if value {
            "true".to_string()
        } else {
            "false".to_string()
        }),
        Value::U8(value) => Ok(value.to_string()),
        Value::I8(value) => Ok(value.to_string()),
        Value::U16(value) => Ok(value.to_string()),
        Value::I16(value) => Ok(value.to_string()),
        Value::U32(value) => Ok(value.to_string()),
        Value::I32(value) => Ok(value.to_string()),
        Value::U64(value) => Ok(value.to_string()),
        Value::I64(value) => Ok(value.to_string()),
        Value::F32(value) => Ok(format!("{value:?}")),
        Value::F64(value) => Ok(format!("{value:?}")),
        Value::Utf8(value) | Value::Utf16(value) => Err(format!(
            "string literal constants are not supported yet: {value:?}"
        )),
        Value::AttributeEnum(name, value) => Ok(format!("/* {name} */ {value}")),
    }
}

fn projected_type_names(types: &TypesManifest) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for alias in &types.aliases {
        names.insert(alias.name.clone());
    }
    for alias in derived_pointer_aliases(types) {
        names.insert(alias.name);
    }
    for alias in &types.array_aliases {
        names.insert(alias.name.clone());
    }
    for record in &types.manual_records {
        names.insert(record.name.clone());
    }
    for projected in &types.projected {
        names.insert(projected.name.clone());
    }
    for record in &types.nested_records {
        names.insert(record.name.clone());
    }
    for interface in &types.interfaces {
        if interface.emit_pointer_alias {
            names.insert(interface.name.clone());
        }
    }
    names
}

fn derived_pointer_aliases(types: &TypesManifest) -> Vec<ManualAlias> {
    let mut aliases = Vec::new();
    for base in pointer_alias_roots(types) {
        for (mutable, prefix) in [(true, "P"), (false, "PC")] {
            let name = format!("{prefix}{base}");
            if has_type_name(types, &name) {
                continue;
            }
            let expr = if mutable {
                format!("*mut {base}")
            } else {
                format!("*const {base}")
            };
            aliases.push(ManualAlias { name, expr });
        }
    }
    aliases
}

fn pointer_alias_roots(types: &TypesManifest) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for alias in &types.aliases {
        if !alias.expr.trim_start().starts_with("Option<") {
            names.insert(alias.name.clone());
        }
    }
    for record in &types.manual_records {
        names.insert(record.name.clone());
    }
    for projected in &types.projected {
        names.insert(projected.name.clone());
    }
    for record in &types.nested_records {
        names.insert(record.name.clone());
    }
    for interface in &types.interfaces {
        if interface.emit_pointer_alias {
            names.insert(interface.name.clone());
        }
    }
    names
}

fn has_type_name(types: &TypesManifest, name: &str) -> bool {
    types.aliases.iter().any(|alias| alias.name == name)
        || types.array_aliases.iter().any(|alias| alias.name == name)
        || types
            .manual_records
            .iter()
            .any(|record| record.name == name)
        || types
            .projected
            .iter()
            .any(|projected| projected.name == name)
        || types
            .nested_records
            .iter()
            .any(|record| record.name == name)
        || types
            .interfaces
            .iter()
            .filter(|interface| interface.emit_pointer_alias)
            .any(|interface| interface.name == name)
}

fn qualify_type_name(name: &str) -> String {
    format!("arcana_winapi.raw.types.{name}")
}

fn sanitize_identifier(name: &str) -> String {
    if name.is_empty() {
        return "arg".to_string();
    }
    let mut out = String::new();
    let mut previous_lowercase = false;
    for ch in name.chars() {
        if !ch.is_ascii_alphanumeric() {
            if !out.ends_with('_') {
                out.push('_');
            }
            previous_lowercase = false;
            continue;
        }
        if ch.is_ascii_uppercase() {
            if previous_lowercase && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            previous_lowercase = false;
        } else {
            out.push(ch);
            previous_lowercase = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        }
    }
    if out.is_empty() {
        out.push_str("arg");
    }
    if out
        .chars()
        .next()
        .map(|ch| ch.is_ascii_digit())
        .unwrap_or(false)
    {
        out.insert_str(0, "arg_");
    }
    match out.as_str() {
        "Self" | "as" | "async" | "await" | "before" | "break" | "const" | "continue" | "copy"
        | "crate" | "dyn" | "edit" | "else" | "enum" | "export" | "extern" | "fn" | "for"
        | "if" | "impl" | "in" | "let" | "loop" | "macro" | "match" | "mod" | "move" | "mut"
        | "native" | "pub" | "read" | "ref" | "return" | "self" | "static" | "struct" | "super"
        | "take" | "trait" | "type" | "union" | "unsafe" | "use" | "where" | "while" => {
            format!("{out}_arg")
        }
        _ => out,
    }
}

struct TypeRenderer<'a> {
    type_index: &'a TypeIndex,
    types: &'a TypesManifest,
    projected_names: &'a BTreeSet<String>,
    qualify: bool,
}

impl<'a> TypeRenderer<'a> {
    fn new(
        type_index: &'a TypeIndex,
        types: &'a TypesManifest,
        projected_names: &'a BTreeSet<String>,
        qualify: bool,
    ) -> Self {
        Self {
            type_index,
            types,
            projected_names,
            qualify,
        }
    }

    fn render(&self, ty: &Type) -> GenResult<String> {
        match ty {
            Type::Void => Ok("c_void".to_string()),
            Type::Bool => Ok(self.named("BOOL")),
            Type::Char => Ok(self.named("WCHAR")),
            Type::I8 => Ok(self.scalar("i8")),
            Type::U8 => Ok(self.named("BYTE")),
            Type::I16 => Ok(self.named("I16")),
            Type::U16 => Ok(self.named("WORD")),
            Type::I32 => Ok(self.named("I32")),
            Type::U32 => Ok(self.named("U32")),
            Type::I64 => Ok(self.named("I64")),
            Type::U64 => Ok(self.named("U64")),
            Type::F32 => Ok(self.named("FLOAT")),
            Type::F64 => Ok(self.named("DOUBLE")),
            Type::ISize => Ok(self.named("LONG_PTR")),
            Type::USize => Ok(self.named("ULONG_PTR")),
            Type::String => Ok(self.named("PCWSTR")),
            Type::Object => Ok(self.named("LPVOID")),
            Type::Name(name) => self.render_named(&name.namespace, &name.name),
            Type::Array(inner) => self.render_pointer(true, inner),
            Type::ArrayRef(inner) => self.render_pointer(true, inner),
            Type::Generic(index) => Err(format!(
                "generic metadata type parameters are unsupported: {index}"
            )),
            Type::RefMut(inner) => self.render_pointer(true, inner),
            Type::RefConst(inner) => self.render_pointer(false, inner),
            Type::PtrMut(inner, _) => self.render_pointer(true, inner),
            Type::PtrConst(inner, _) => self.render_pointer(false, inner),
            Type::ArrayFixed(inner, len) => self.render_array(inner, *len),
            Type::AttributeEnum => {
                Err("attribute enum metadata is unsupported in raw projection".to_string())
            }
        }
    }

    fn render_named(&self, namespace: &str, name: &str) -> GenResult<String> {
        if namespace == "System" && name == "Guid" {
            return Ok(self.named("GUID"));
        }
        if namespace == "Windows.Win32.UI.WindowsAndMessaging" && name == "WNDPROC" {
            return Ok(self.named("RAW_WNDPROC"));
        }
        if let Some(projected) = self
            .types
            .projected
            .iter()
            .find(|projected| projected.source_name.as_deref() == Some(name))
        {
            return Ok(self.named(&projected.name));
        }
        if let Some(record) = self
            .types
            .nested_records
            .iter()
            .find(|record| record.source_name == name)
        {
            return Ok(self.named(&record.name));
        }
        if self.projected_names.contains(name) {
            return Ok(self.named(name));
        }
        let matches = self.type_index.get(namespace, name).collect::<Vec<_>>();
        match matches.as_slice() {
            [ty] => match ty.category() {
                TypeCategory::Enum => self.render(&enum_underlying_type(*ty)?),
                TypeCategory::Interface
                | TypeCategory::Class
                | TypeCategory::Struct
                | TypeCategory::Delegate => Ok(self.named(fallback_opaque_alias(name))),
                _ => Err(format!(
                    "unprojected metadata type {}.{} leaked into the raw surface; add it to types.toml or the skiplist",
                    namespace, name
                )),
            },
            [] if namespace.starts_with("Windows.Win32") => {
                Ok(self.named(fallback_opaque_alias(name)))
            }
            [] => Err(format!(
                "failed to resolve metadata type {}.{} while rendering raw output",
                namespace, name
            )),
            _ => {
                let first = matches[0];
                if matches
                    .iter()
                    .all(|candidate| candidate.category() == first.category())
                {
                    match first.category() {
                        TypeCategory::Enum => self.render(&enum_underlying_type(first)?),
                        TypeCategory::Interface
                        | TypeCategory::Class
                        | TypeCategory::Struct
                        | TypeCategory::Delegate => Ok(self.named(fallback_opaque_alias(name))),
                        _ => Err(format!(
                            "metadata type {}.{} is ambiguous while rendering raw output",
                            namespace, name
                        )),
                    }
                } else {
                    Err(format!(
                        "metadata type {}.{} is ambiguous while rendering raw output",
                        namespace, name
                    ))
                }
            }
        }
    }

    fn render_array(&self, inner: &Type, len: usize) -> GenResult<String> {
        let inner_rendered = self.render(inner)?;
        let bare_inner = unqualify_type_name(&inner_rendered);
        if let Some(alias) = self
            .types
            .array_aliases
            .iter()
            .find(|alias| alias.element == bare_inner && alias.len == len)
        {
            return Ok(self.named(&alias.name));
        }
        Ok(format!("[{}; {}]", inner_rendered, len))
    }

    fn render_pointer(&self, mutable: bool, inner: &Type) -> GenResult<String> {
        let inner_rendered = self.render(inner)?;
        if let Some(alias) = self.pointer_alias(mutable, &inner_rendered) {
            return Ok(self.named(&alias));
        }
        let pointer = if mutable { "*mut" } else { "*const" };
        Ok(format!("{pointer} {inner_rendered}"))
    }

    fn pointer_alias(&self, mutable: bool, inner: &str) -> Option<String> {
        for candidate in pointer_alias_candidates(unqualify_type_name(inner)) {
            let expr = if mutable {
                format!("*mut {candidate}")
            } else {
                format!("*const {candidate}")
            };
            if let Some(alias) = self.types.aliases.iter().find(|alias| alias.expr == expr) {
                return Some(alias.name.clone());
            }
        }
        let base = unqualify_type_name(inner);
        let generated = if mutable {
            format!("P{base}")
        } else {
            format!("PC{base}")
        };
        pointer_alias_roots(self.types)
            .contains(base)
            .then_some(generated)
    }

    fn named(&self, name: &str) -> String {
        if self.qualify {
            qualify_type_name(name)
        } else {
            name.to_string()
        }
    }

    fn scalar(&self, text: &str) -> String {
        text.to_string()
    }
}

fn unqualify_type_name(name: &str) -> &str {
    name.strip_prefix("arcana_winapi.raw.types.")
        .unwrap_or(name)
}

fn pointer_alias_candidates(name: &str) -> Vec<&str> {
    match name {
        "BOOL" => vec!["BOOL"],
        "I32" => vec!["LONG", "I32"],
        "U8" | "BYTE" => vec!["BYTE", "U8"],
        "U16" | "WORD" => vec!["WORD", "U16"],
        "U32" => vec!["UINT", "DWORD", "ULONG", "UINT32", "U32"],
        "I64" => vec!["I64"],
        "U64" => vec!["UINT64", "U64"],
        "WCHAR" => vec!["WCHAR"],
        "c_void" => vec!["c_void"],
        other => vec![other],
    }
}

fn fallback_opaque_alias(name: &str) -> &str {
    let is_handle_name = name.starts_with('H')
        && name
            .chars()
            .skip(1)
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_');
    if is_handle_name { "HANDLE" } else { "LPVOID" }
}

fn repo_relative(repo_root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(repo_root).unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}

#[derive(Clone, Copy)]
enum Mode {
    Write,
    Check,
}

#[derive(Deserialize)]
struct MetadataConfig {
    authoritative_source: String,
    parity_target: String,
    env_var: String,
    file_name: String,
    metadata_version: String,
    sha256: String,
    cargo_registry_package: String,
    cargo_registry_relative_path: String,
    windows_sys_package: String,
    windows_sys_relative_root: String,
}

#[derive(Deserialize)]
struct ProjectionConfig {
    leaves: Vec<ProjectionLeaf>,
}

#[derive(Deserialize)]
struct ProjectionLeaf {
    name: String,
    kind: String,
    #[serde(default)]
    binding_library: Option<String>,
}

#[derive(Deserialize)]
struct ImportProjection {
    #[serde(default)]
    symbols: Vec<String>,
    #[serde(default)]
    namespaces: Vec<String>,
    #[serde(default)]
    libraries: Vec<String>,
    #[serde(default)]
    symbol_prefixes: Vec<String>,
    #[serde(default)]
    exclude_symbols: Vec<String>,
}

#[derive(Default, Deserialize)]
struct ConstantsManifest {
    #[serde(default)]
    entries: Vec<ConstantEntry>,
}

#[derive(Deserialize)]
struct ConstantEntry {
    leaf: String,
    name: String,
    #[serde(default)]
    source_type: Option<String>,
    #[serde(default)]
    source_field: Option<String>,
    #[serde(rename = "type")]
    r#type: String,
}

#[derive(Default, Deserialize)]
struct CallbacksManifest {
    #[serde(default)]
    entries: Vec<CallbackEntry>,
}

#[derive(Deserialize)]
struct CallbackEntry {
    name: String,
    kind: String,
    source_type: String,
    #[serde(default)]
    source_method: String,
    #[serde(default)]
    param_names: Option<Vec<String>>,
    #[serde(default)]
    param_modes: Option<Vec<String>>,
}

#[derive(Default, Deserialize)]
struct TypesManifest {
    #[serde(default)]
    aliases: Vec<ManualAlias>,
    #[serde(default)]
    array_aliases: Vec<ArrayAlias>,
    #[serde(default)]
    manual_records: Vec<ManualRecord>,
    #[serde(default)]
    projected: Vec<ProjectedType>,
    #[serde(default)]
    nested_records: Vec<NestedRecord>,
    #[serde(default)]
    interfaces: Vec<InterfaceProjection>,
}

#[derive(Deserialize)]
struct ManualAlias {
    name: String,
    expr: String,
}

#[derive(Deserialize)]
struct ArrayAlias {
    name: String,
    element: String,
    len: usize,
}

#[derive(Deserialize)]
struct ManualRecord {
    kind: String,
    name: String,
    fields: Vec<String>,
}

#[derive(Deserialize)]
struct ProjectedType {
    kind: String,
    name: String,
    #[serde(default)]
    source_name: Option<String>,
}

#[derive(Deserialize)]
struct NestedRecord {
    kind: String,
    name: String,
    parent: String,
    source_name: String,
}

#[derive(Deserialize)]
struct InterfaceProjection {
    name: String,
    emit_vtable: bool,
    emit_pointer_alias: bool,
}

#[derive(Default, Deserialize)]
struct SkiplistManifest {
    #[serde(default)]
    entries: Vec<SkipEntry>,
}

#[derive(Deserialize)]
struct SkipEntry {
    kind: String,
    id: String,
    reason: String,
}

#[derive(Default, Deserialize)]
struct ExceptionManifest {
    #[serde(default)]
    legacy_items: Vec<LegacyItem>,
    #[serde(default)]
    exceptions: Vec<ExceptionEntry>,
}

#[derive(Default, Deserialize)]
struct WindowsSysParityManifest {
    #[serde(default)]
    supported_leaves: Vec<String>,
    #[serde(default)]
    skipped_leaves: Vec<WindowsSysParitySkip>,
    #[serde(default)]
    divergences: Vec<WindowsSysParityDivergence>,
}

#[derive(Deserialize)]
struct LegacyItem {
    name: String,
    resolution: String,
    leaf: String,
    symbols: Vec<String>,
    exception_id: Option<String>,
}

#[derive(Deserialize)]
struct ExceptionEntry {
    id: String,
    leaf: String,
    symbol: String,
    reason: String,
    implementation: String,
}

#[derive(Deserialize)]
struct WindowsSysParityDivergence {
    leaf: String,
    symbol: String,
    kind: String,
    reason: String,
}

#[derive(Deserialize)]
struct WindowsSysParitySkip {
    leaf: String,
    reason: String,
}

struct RenderedLeaf {
    source_of_truth: &'static str,
    body: String,
    report: LeafReport,
}

#[derive(Clone, Copy)]
struct RenderImportsContext<'a> {
    imports: &'a BTreeMap<String, ImportProjection>,
    types: &'a TypesManifest,
    skiplist: &'a SkiplistManifest,
    windows_sys_parity: &'a WindowsSysParityManifest,
    windows_sys_root: &'a Path,
}

struct LeafReport {
    name: String,
    kind: String,
    lines: Vec<String>,
}

#[derive(Clone)]
struct ImportCandidate<'a> {
    symbol: String,
    metadata_id: String,
    import_library: String,
    method: MethodDef<'a>,
}

struct ImportSelection<'a> {
    metadata_candidates: usize,
    emitted: Vec<ImportCandidate<'a>>,
    excluded: Vec<ImportCandidate<'a>>,
    skipped: Vec<ImportCandidate<'a>>,
}

struct WindowsSysParity {
    matched: usize,
    classified_divergences: usize,
    skipped_reason: Option<String>,
}

#[derive(Clone)]
struct WindowsSysImport {
    symbol: String,
    metadata_id: String,
    import_library: String,
}

struct WindowsSysImportSelection {
    filtered: BTreeMap<String, String>,
    all: BTreeMap<String, WindowsSysImport>,
}

#[derive(Clone, Copy)]
struct Manifests<'a> {
    imports: &'a BTreeMap<String, ImportProjection>,
    constants: &'a ConstantsManifest,
    callbacks: &'a CallbacksManifest,
    types: &'a TypesManifest,
    skiplist: &'a SkiplistManifest,
    exceptions: &'a ExceptionManifest,
    windows_sys_parity: &'a WindowsSysParityManifest,
}
