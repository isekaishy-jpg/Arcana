use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[cfg(windows)]
use std::ffi::{CStr, c_char};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(windows)]
use std::time::{SystemTime, UNIX_EPOCH};

use arcana_aot::{
    ARCANA_NATIVE_PRODUCT_TEMP_PROBES_ENV, AotInstanceProductSpec, compile_instance_product,
};
use arcana_cabi::ArcanaCabiProductRole;
#[cfg(windows)]
use arcana_cabi::{
    ARCANA_CABI_CONTRACT_VERSION_V1, ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL, ArcanaCabiChildOpsV1,
    ArcanaCabiExportOpsV1, ArcanaCabiInstanceOpsV1, ArcanaCabiPluginOpsV1, ArcanaCabiProductApiV1,
};
#[cfg(windows)]
use libloading::Library;
use sha2::{Digest, Sha256};

use crate::build::{BuildStatus, selected_native_product_for_build};
use crate::build_identity::read_cached_output_metadata;
use crate::{
    BuildOutputKey, BuildTarget, NativeProductProducer, PackageResult, WorkspaceGraph,
    WorkspaceMember, collect_validated_support_file_paths, validate_support_file_relative_path,
};

pub const DISTRIBUTION_BUNDLE_FORMAT: &str = "arcana-distribution-bundle-v1";
const DISTRIBUTION_MANIFEST_FILE: &str = "arcana.bundle.toml";
const EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC: &[u8] = b"ARCANA_DIST_MANIFEST_V1\0";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DistributionBundle {
    pub member: String,
    pub target: BuildTarget,
    pub product: Option<String>,
    pub target_format: String,
    pub root_artifact: String,
    pub support_files: Vec<String>,
    pub artifact_hash: String,
    pub toolchain: String,
    pub bundle_dir: PathBuf,
    pub manifest_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NativeBundleFile {
    pub(crate) relative_path: String,
    pub(crate) source_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DistributionNativeProduct {
    package_name: String,
    product_name: String,
    role: ArcanaCabiProductRole,
    contract_id: String,
    contract_version: u32,
    producer: String,
    sidecars: Vec<String>,
    file: String,
    file_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DistributionChildBinding {
    consumer_member: String,
    dependency_alias: String,
    package_name: String,
    product_name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct StagedNativeDependencyProducts {
    files: Vec<NativeBundleFile>,
    products: Vec<DistributionNativeProduct>,
    child_bindings: Vec<DistributionChildBinding>,
    runtime_child_binding: Option<DistributionChildBinding>,
    root_native_product: Option<DistributionNativeProduct>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct NativeSelectionPlan {
    products: Vec<DistributionNativeProduct>,
    child_bindings: Vec<DistributionChildBinding>,
    runtime_child_binding: Option<DistributionChildBinding>,
}

pub fn default_distribution_dir(
    graph: &WorkspaceGraph,
    member: &str,
    target: &BuildTarget,
) -> PathBuf {
    default_distribution_dir_for_build(graph, member, &BuildOutputKey::target(target.clone()))
}

pub fn default_distribution_dir_for_build(
    graph: &WorkspaceGraph,
    member: &str,
    build_key: &BuildOutputKey,
) -> PathBuf {
    let mut dir = graph
        .root_dir
        .join("dist")
        .join(member)
        .join(build_key.target.key());
    if let Some(product) = build_key.product() {
        dir = dir.join(product);
    }
    dir
}

pub fn stage_distribution_bundle(
    graph: &WorkspaceGraph,
    statuses: &[BuildStatus],
    member: &str,
    target: &BuildTarget,
    bundle_dir: &Path,
) -> PackageResult<DistributionBundle> {
    stage_distribution_bundle_for_build(
        graph,
        statuses,
        member,
        &BuildOutputKey::target(target.clone()),
        bundle_dir,
    )
}

pub fn stage_distribution_bundle_for_build(
    graph: &WorkspaceGraph,
    statuses: &[BuildStatus],
    member: &str,
    build_key: &BuildOutputKey,
    bundle_dir: &Path,
) -> PackageResult<DistributionBundle> {
    let status = statuses
        .iter()
        .find(|status| status.member() == member && status.build_key() == build_key)
        .ok_or_else(|| {
            format!(
                "missing build status for member `{member}` build `{}`",
                build_key.storage_key()
            )
        })?;
    let source_root = graph.root_dir.join(status.artifact_rel_path());
    let metadata = read_cached_output_metadata(&source_root, build_key.target_ref())?;
    if metadata.member != member {
        return Err(format!(
            "cached build metadata for `{}` reports member `{}`",
            source_root.display(),
            metadata.member
        ));
    }
    if &metadata.target != build_key.target_ref() {
        return Err(format!(
            "cached build metadata for `{}` reports target `{}` not `{}`",
            source_root.display(),
            metadata.target,
            build_key.target_ref()
        ));
    }

    let root_file_name = source_root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("invalid built artifact path `{}`", source_root.display()))?
        .to_string();
    reset_distribution_dir(bundle_dir, &root_file_name)?;
    let root_native_product =
        distribution_root_native_product(graph, member, build_key, &metadata.artifact_hash)?;
    let staged_native_products =
        stage_native_dependency_products(graph, member, build_key, bundle_dir)?;
    let mut support_files = metadata
        .support_files
        .iter()
        .filter(|path| !distribution_hides_support_file(path))
        .cloned()
        .collect::<Vec<_>>();
    support_files.extend(
        staged_native_products
            .files
            .iter()
            .map(|file| file.relative_path.clone()),
    );
    if support_files.iter().any(|path| path == &root_file_name) {
        return Err(format!(
            "bundle support files for `{member}` build `{}` include root artifact path `{root_file_name}`",
            build_key.storage_key()
        ));
    }
    let support_files =
        collect_validated_support_file_paths(support_files.iter().map(String::as_str))?;

    copy_distribution_file(&source_root, &bundle_dir.join(&root_file_name))?;
    for relative_path in metadata
        .support_files
        .iter()
        .filter(|path| !distribution_hides_support_file(path))
    {
        let source_path = source_root
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(relative_path);
        copy_distribution_file(&source_path, &bundle_dir.join(relative_path))?;
    }
    for file in &staged_native_products.files {
        copy_distribution_file(&file.source_path, &bundle_dir.join(&file.relative_path))?;
    }

    let manifest_text = render_distribution_manifest(
        member,
        build_key,
        &metadata.target_format,
        &root_file_name,
        &support_files,
        &metadata.artifact_hash,
        &metadata.toolchain,
        metadata.native_product_closure.as_deref(),
        root_native_product.as_ref(),
        &staged_native_products.products,
        staged_native_products.runtime_child_binding.as_ref(),
        &staged_native_products.child_bindings,
    );
    let root_artifact_path = bundle_dir.join(&root_file_name);
    embed_distribution_manifest(&root_artifact_path, &manifest_text)?;

    Ok(DistributionBundle {
        member: member.to_string(),
        target: build_key.target.clone(),
        product: build_key.product.clone(),
        target_format: metadata.target_format,
        root_artifact: root_file_name,
        support_files,
        artifact_hash: metadata.artifact_hash,
        toolchain: metadata.toolchain,
        bundle_dir: bundle_dir.to_path_buf(),
        manifest_text,
    })
}

fn stage_native_dependency_products(
    graph: &WorkspaceGraph,
    root_member: &str,
    build_key: &BuildOutputKey,
    bundle_dir: &Path,
) -> PackageResult<StagedNativeDependencyProducts> {
    if !matches!(
        build_key.target_ref(),
        BuildTarget::WindowsExe | BuildTarget::WindowsDll
    ) {
        return Ok(StagedNativeDependencyProducts::default());
    }

    let selections = collect_native_dependency_product_selections(graph, root_member)?;
    native_product_probe(
        "selection_summary",
        format!(
            "root_member={root_member} target={} selected_products={}",
            build_key.target_ref(),
            selections.products.len()
        ),
    );
    let mut staged = Vec::new();
    let mut staged_products = Vec::new();
    let mut seen_paths = BTreeMap::<String, String>::new();
    for selected in &selections.products {
        let member_name = &selected.package_name;
        let product_name = &selected.product_name;
        let role = selected.role;
        let member = graph
            .member(member_name)
            .ok_or_else(|| format!("missing workspace member `{member_name}`"))?;
        let product = member
            .native_products
            .get(product_name.as_str())
            .ok_or_else(|| {
                format!(
                    "workspace member `{}` is missing selected native product `{}`",
                    member.name, product_name
                )
            })?;
        if product.role != role {
            native_product_probe(
                "selection_role_mismatch",
                format!(
                    "member={} product={} declared_role={} selected_role={}",
                    member.name,
                    product.name,
                    product.role.as_str(),
                    role.as_str()
                ),
            );
            return Err(format!(
                "native product `{}` on `{}` uses role `{}`, but the bundle selected it as `{}`",
                product.name,
                member.name,
                product.role.as_str(),
                role.as_str()
            ));
        }
        let resolved_files = resolve_native_product_files(graph, member, product)?;
        let file_hash = hash_native_bundle_file(
            resolved_files
                .iter()
                .find(|file| file.relative_path == product.file)
                .ok_or_else(|| {
                    format!(
                        "native product `{}` on `{}` did not resolve its primary file `{}`",
                        product.name, member.name, product.file
                    )
                })?,
        )?;
        staged_products.push(DistributionNativeProduct {
            package_name: selected.package_name.clone(),
            product_name: selected.product_name.clone(),
            role: selected.role,
            contract_id: selected.contract_id.clone(),
            contract_version: selected.contract_version,
            producer: selected.producer.clone(),
            sidecars: selected.sidecars.clone(),
            file: selected.file.clone(),
            file_hash,
        });
        for file in resolved_files {
            if let Some(existing) = seen_paths.insert(
                file.relative_path.clone(),
                format!("{}:{}", member.name, product.name),
            ) {
                native_product_probe(
                    "duplicate_stage_path",
                    format!(
                        "path={} first={} second={}:{}",
                        file.relative_path, existing, member.name, product.name
                    ),
                );
                return Err(format!(
                    "native bundle staging would write duplicate file `{}` from `{existing}` and `{}:{}`",
                    file.relative_path, member.name, product.name
                ));
            }
            validate_support_file_relative_path(&file.relative_path)?;
            if bundle_dir.join(&file.relative_path) == bundle_dir.join(".") {
                native_product_probe(
                    "invalid_stage_path",
                    format!(
                        "member={} product={} path={}",
                        member.name, product.name, file.relative_path
                    ),
                );
                return Err(format!(
                    "native bundle staging produced invalid output path `{}`",
                    file.relative_path
                ));
            }
            native_product_probe(
                "stage_file",
                format!(
                    "member={} product={} role={} file={} source={}",
                    member.name,
                    product.name,
                    product.role.as_str(),
                    file.relative_path,
                    file.source_path.display()
                ),
            );
            staged.push(file);
        }
    }
    Ok(StagedNativeDependencyProducts {
        files: staged,
        products: staged_products,
        child_bindings: selections.child_bindings,
        runtime_child_binding: selections.runtime_child_binding,
        root_native_product: None,
    })
}

pub(crate) fn native_product_closure_digest(
    graph: &WorkspaceGraph,
    root_member: &str,
    build_key: &BuildOutputKey,
) -> PackageResult<Option<String>> {
    if !matches!(build_key.target_ref(), BuildTarget::WindowsExe) {
        return Ok(None);
    }
    let selections = collect_native_dependency_product_selections(graph, root_member)?;
    if selections.products.is_empty() && selections.child_bindings.is_empty() {
        return Ok(None);
    }
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_native_product_closure_v1\n");
    hasher.update(root_member.as_bytes());
    hasher.update(b"\n");
    hasher.update(build_key.storage_key().as_bytes());
    for product in &selections.products {
        hasher.update(b"\nproduct\n");
        hasher.update(product.package_name.as_bytes());
        hasher.update(b"\n");
        hasher.update(product.product_name.as_bytes());
        hasher.update(b"\n");
        hasher.update(product.role.as_str().as_bytes());
        hasher.update(b"\n");
        hasher.update(product.contract_id.as_bytes());
        hasher.update(b"\n");
        hasher.update(product.contract_version.to_string().as_bytes());
        hasher.update(b"\n");
        hasher.update(product.producer.as_bytes());
        hasher.update(b"\n");
        hasher.update(product.file.as_bytes());
        for sidecar in &product.sidecars {
            hasher.update(b"\nsidecar\n");
            hasher.update(sidecar.as_bytes());
        }
    }
    for binding in &selections.child_bindings {
        hasher.update(b"\nbinding\n");
        hasher.update(binding.consumer_member.as_bytes());
        hasher.update(b"\n");
        hasher.update(binding.dependency_alias.as_bytes());
        hasher.update(b"\n");
        hasher.update(binding.package_name.as_bytes());
        hasher.update(b"\n");
        hasher.update(binding.product_name.as_bytes());
    }
    if let Some(binding) = &selections.runtime_child_binding {
        hasher.update(b"\nruntime_child_binding\n");
        hasher.update(binding.consumer_member.as_bytes());
        hasher.update(b"\n");
        hasher.update(binding.dependency_alias.as_bytes());
        hasher.update(b"\n");
        hasher.update(binding.package_name.as_bytes());
        hasher.update(b"\n");
        hasher.update(binding.product_name.as_bytes());
    }
    Ok(Some(format!("sha256:{:x}", hasher.finalize())))
}

fn collect_native_dependency_product_selections(
    graph: &WorkspaceGraph,
    root_member: &str,
) -> PackageResult<NativeSelectionPlan> {
    let mut pending = VecDeque::from([root_member.to_string()]);
    let mut visited = BTreeSet::new();
    let mut selected_product_keys = BTreeSet::new();
    let mut products = Vec::new();
    let mut child_bindings = Vec::new();

    while let Some(member_name) = pending.pop_front() {
        if !visited.insert(member_name.clone()) {
            continue;
        }
        let member = graph
            .member(&member_name)
            .ok_or_else(|| format!("missing workspace member `{member_name}`"))?;
        for dep in &member.deps {
            pending.push_back(dep.clone());
        }
        for (alias, spec) in &member.direct_dep_specs {
            let package_name = member.direct_dep_packages.get(alias).ok_or_else(|| {
                format!(
                    "workspace member `{}` is missing direct dependency package metadata for alias `{alias}`",
                    member.name
                )
            })?;
            let dependency_member = graph.member(package_name).ok_or_else(|| {
                format!(
                    "dependency `{}` in `{}` resolved package `{package_name}`, but that package is missing from the workspace graph",
                    alias,
                    member.name
                )
            })?;
            if let Some(child) = spec.selected_native_child() {
                require_selected_native_product(
                    dependency_member,
                    child,
                    ArcanaCabiProductRole::Child,
                    alias,
                    &member.name,
                )?;
                let child_product = dependency_member.native_products.get(child).ok_or_else(|| {
                    format!(
                        "dependency `{alias}` in `{}` selects native product `{child}`, but `{}` does not define it",
                        member.name, dependency_member.name
                    )
                })?;
                if selected_product_keys.insert((dependency_member.name.clone(), child.to_string()))
                {
                    products.push(DistributionNativeProduct {
                        package_name: dependency_member.name.clone(),
                        product_name: child.to_string(),
                        role: ArcanaCabiProductRole::Child,
                        contract_id: child_product.contract.clone(),
                        contract_version: 1,
                        producer: child_product.producer.as_str().to_string(),
                        sidecars: child_product.sidecars.clone(),
                        file: child_product.file.clone(),
                        file_hash: String::new(),
                    });
                }
                child_bindings.push(DistributionChildBinding {
                    consumer_member: member.name.clone(),
                    dependency_alias: alias.clone(),
                    package_name: dependency_member.name.clone(),
                    product_name: child.to_string(),
                });
            }
            for plugin in &spec.native_plugins {
                require_selected_native_product(
                    dependency_member,
                    plugin,
                    ArcanaCabiProductRole::Plugin,
                    alias,
                    &member.name,
                )?;
                let plugin_product =
                    dependency_member.native_products.get(plugin).ok_or_else(|| {
                        format!(
                            "dependency `{alias}` in `{}` selects native product `{plugin}`, but `{}` does not define it",
                            member.name, dependency_member.name
                        )
                    })?;
                if selected_product_keys.insert((dependency_member.name.clone(), plugin.clone())) {
                    products.push(DistributionNativeProduct {
                        package_name: dependency_member.name.clone(),
                        product_name: plugin.clone(),
                        role: ArcanaCabiProductRole::Plugin,
                        contract_id: plugin_product.contract.clone(),
                        contract_version: 1,
                        producer: plugin_product.producer.as_str().to_string(),
                        sidecars: plugin_product.sidecars.clone(),
                        file: plugin_product.file.clone(),
                        file_hash: String::new(),
                    });
                }
            }
        }
    }

    products.sort_by(|left, right| {
        left.package_name
            .cmp(&right.package_name)
            .then_with(|| left.product_name.cmp(&right.product_name))
    });
    child_bindings.sort_by(|left, right| {
        left.consumer_member
            .cmp(&right.consumer_member)
            .then_with(|| left.dependency_alias.cmp(&right.dependency_alias))
            .then_with(|| left.package_name.cmp(&right.package_name))
            .then_with(|| left.product_name.cmp(&right.product_name))
    });
    let runtime_child_bindings = child_bindings
        .iter()
        .filter(|binding| binding.consumer_member == root_member)
        .cloned()
        .collect::<Vec<_>>();
    let runtime_child_binding = match runtime_child_bindings.as_slice() {
        [] => None,
        [binding] => {
            native_product_probe(
                "runtime_child_binding_selected",
                format!(
                    "root_member={root_member} consumer={} alias={} package={} product={}",
                    binding.consumer_member,
                    binding.dependency_alias,
                    binding.package_name,
                    binding.product_name
                ),
            );
            Some(binding.clone())
        }
        bindings => {
            let binding_list = bindings
                .iter()
                .map(|binding| {
                    format!(
                        "{}:{}=>{}:{}",
                        binding.consumer_member,
                        binding.dependency_alias,
                        binding.package_name,
                        binding.product_name
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            native_product_probe(
                "runtime_child_binding_ambiguous",
                format!("root_member={root_member} bindings={binding_list}"),
            );
            return Err(format!(
                "bundle root `{root_member}` selects multiple native child runtime providers and current cabi runtime selection would be ambiguous: {binding_list}"
            ));
        }
    };

    Ok(NativeSelectionPlan {
        products,
        child_bindings,
        runtime_child_binding,
    })
}

fn require_selected_native_product(
    member: &WorkspaceMember,
    product_name: &str,
    expected_role: ArcanaCabiProductRole,
    alias: &str,
    consumer_member: &str,
) -> PackageResult<()> {
    let product = member.native_products.get(product_name).ok_or_else(|| {
        format!(
            "dependency `{alias}` in `{consumer_member}` selects native product `{product_name}`, but `{}` does not define it",
            member.name
        )
    })?;
    if product.role != expected_role {
        return Err(format!(
            "dependency `{alias}` in `{consumer_member}` selects native product `{product_name}` on `{}`, but it uses role `{}` instead of `{}`",
            member.name,
            product.role.as_str(),
            expected_role.as_str()
        ));
    }
    Ok(())
}

pub(crate) fn resolve_native_product_files(
    graph: &WorkspaceGraph,
    member: &WorkspaceMember,
    product: &crate::NativeProductSpec,
) -> PackageResult<Vec<NativeBundleFile>> {
    native_product_probe(
        "resolve_product",
        format!(
            "member={} product={} role={} producer={}",
            member.name,
            product.name,
            product.role.as_str(),
            product.producer.as_str()
        ),
    );
    match product.producer {
        NativeProductProducer::RustCdylib => build_rust_cdylib_product(graph, member, product),
        NativeProductProducer::ArcanaSource => build_generated_cabi_product(graph, member, product),
    }
}

fn build_rust_cdylib_product(
    graph: &WorkspaceGraph,
    member: &WorkspaceMember,
    product: &crate::NativeProductSpec,
) -> PackageResult<Vec<NativeBundleFile>> {
    let crate_rel = product.rust_cdylib_crate.as_deref().ok_or_else(|| {
        format!(
            "native product `{}` on `{}` is missing `rust_cdylib_crate`",
            product.name, member.name
        )
    })?;
    let crate_dir = member.abs_dir.join(crate_rel);
    let manifest_path = crate_dir.join("Cargo.toml");
    if !manifest_path.is_file() {
        return Err(format!(
            "native product `{}` on `{}` points at missing crate manifest `{}`",
            product.name,
            member.name,
            manifest_path.display()
        ));
    }

    let target_dir = repo_root()
        .join("target")
        .join("native-products")
        .join(short_path_fingerprint(&graph.root_dir))
        .join(short_path_fingerprint(&member.abs_dir))
        .join(&product.name);
    fs::create_dir_all(&target_dir).map_err(|e| {
        format!(
            "failed to create native product target directory `{}`: {e}",
            target_dir.display()
        )
    })?;
    native_product_probe(
        "rust_cdylib_build_start",
        format!(
            "member={} product={} manifest={} target_dir={}",
            member.name,
            product.name,
            manifest_path.display(),
            target_dir.display()
        ),
    );
    run_cargo_build(&manifest_path, &target_dir, &product.name)?;

    let output_path = target_dir.join("debug").join(&product.file);
    if !output_path.is_file() {
        native_product_probe(
            "rust_cdylib_missing_output",
            format!(
                "member={} product={} expected_output={}",
                member.name,
                product.name,
                output_path.display()
            ),
        );
        return Err(format!(
            "native product `{}` on `{}` did not produce `{}` under `{}`",
            product.name,
            member.name,
            product.file,
            target_dir.join("debug").display()
        ));
    }

    let mut files = vec![NativeBundleFile {
        relative_path: product.file.clone(),
        source_path: output_path,
    }];
    for sidecar in &product.sidecars {
        validate_support_file_relative_path(sidecar)?;
        let sidecar_path = member.abs_dir.join(sidecar);
        if !sidecar_path.is_file() {
            return Err(format!(
                "native product `{}` on `{}` is missing declared sidecar `{}`",
                product.name,
                member.name,
                sidecar_path.display()
            ));
        }
        files.push(NativeBundleFile {
            relative_path: sidecar.clone(),
            source_path: sidecar_path,
        });
    }
    validate_native_product_dependency_closure(member, product, &files)?;
    validate_native_product_cabi_contract(member, product, &files)?;
    Ok(files)
}

fn build_generated_cabi_product(
    graph: &WorkspaceGraph,
    member: &WorkspaceMember,
    product: &crate::NativeProductSpec,
) -> PackageResult<Vec<NativeBundleFile>> {
    if !matches!(
        product.role,
        ArcanaCabiProductRole::Child | ArcanaCabiProductRole::Plugin
    ) {
        native_product_probe(
            "generated_product_rejected_role",
            format!(
                "member={} product={} role={} producer={}",
                member.name,
                product.name,
                product.role.as_str(),
                product.producer.as_str()
            ),
        );
        return Err(format!(
            "native product `{}` on `{}` uses producer `{}`, but generated cabi products currently support only `child` and `plugin` roles",
            product.name,
            member.name,
            product.producer.as_str()
        ));
    }

    let project_dir = repo_root()
        .join("target")
        .join("native-product-projects")
        .join(short_path_fingerprint(&graph.root_dir))
        .join(short_path_fingerprint(&member.abs_dir))
        .join(&product.name);

    let target_dir = repo_root()
        .join("target")
        .join("native-products")
        .join(short_path_fingerprint(&graph.root_dir))
        .join(short_path_fingerprint(&member.abs_dir))
        .join(&product.name);
    fs::create_dir_all(&target_dir).map_err(|e| {
        format!(
            "failed to create generated native product target directory `{}`: {e}",
            target_dir.display()
        )
    })?;
    let compiled = compile_instance_product(
        &AotInstanceProductSpec {
            package_name: member.name.clone(),
            product_name: product.name.clone(),
            role: product.role,
            contract_id: product.contract.clone(),
            output_file_name: product.file.clone(),
        },
        &project_dir,
        &target_dir,
    )?;

    let mut files = vec![NativeBundleFile {
        relative_path: product.file.clone(),
        source_path: compiled.output_path,
    }];
    for sidecar in &product.sidecars {
        validate_support_file_relative_path(sidecar)?;
        let sidecar_path = member.abs_dir.join(sidecar);
        if !sidecar_path.is_file() {
            return Err(format!(
                "native product `{}` on `{}` is missing declared sidecar `{}`",
                product.name,
                member.name,
                sidecar_path.display()
            ));
        }
        files.push(NativeBundleFile {
            relative_path: sidecar.clone(),
            source_path: sidecar_path,
        });
    }
    validate_native_product_dependency_closure(member, product, &files)?;
    validate_native_product_cabi_contract(member, product, &files)?;
    Ok(files)
}

fn distribution_root_native_product(
    graph: &WorkspaceGraph,
    member_name: &str,
    build_key: &BuildOutputKey,
    artifact_hash: &str,
) -> PackageResult<Option<DistributionNativeProduct>> {
    let Some(member) = graph.member(member_name) else {
        return Err(format!("missing workspace member `{member_name}`"));
    };
    let Some(product) =
        selected_native_product_for_build(member, build_key.target_ref(), build_key.product())?
    else {
        return Ok(None);
    };
    Ok(Some(DistributionNativeProduct {
        package_name: member.name.clone(),
        product_name: product.name.clone(),
        role: product.role,
        contract_id: product.contract.clone(),
        contract_version: ARCANA_CABI_CONTRACT_VERSION_V1,
        producer: product.producer.as_str().to_string(),
        sidecars: product.sidecars.clone(),
        file: product.file.clone(),
        file_hash: artifact_hash.to_string(),
    }))
}

fn validate_native_product_cabi_contract(
    member: &WorkspaceMember,
    product: &crate::NativeProductSpec,
    files: &[NativeBundleFile],
) -> PackageResult<()> {
    #[cfg(windows)]
    {
        let primary = files
            .iter()
            .find(|file| file.relative_path == product.file)
            .ok_or_else(|| {
                format!(
                    "native product `{}` on `{}` did not resolve its primary file `{}`",
                    product.name, member.name, product.file
                )
            })?;
        let validation_dir = repo_root()
            .join("target")
            .join("native-product-validation")
            .join(short_path_fingerprint(&member.abs_dir))
            .join(format!(
                "{}_{}_{}",
                member.name,
                product.name,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0)
            ));
        fs::create_dir_all(&validation_dir).map_err(|e| {
            format!(
                "failed to create native product validation directory `{}`: {e}",
                validation_dir.display()
            )
        })?;
        let validate_result = (|| {
            for file in files {
                copy_distribution_file(
                    &file.source_path,
                    &validation_dir.join(&file.relative_path),
                )?;
            }
            validate_native_product_descriptor_from_staged_root(
                member,
                product,
                &validation_dir.join(&primary.relative_path),
            )
        })();
        let cleanup_result = fs::remove_dir_all(&validation_dir);
        if let Err(err) = validate_result {
            if let Err(cleanup_err) = cleanup_result {
                native_product_probe(
                    "validation_cleanup_error",
                    format!(
                        "member={} product={} path={} error={cleanup_err}",
                        member.name,
                        product.name,
                        validation_dir.display()
                    ),
                );
            }
            return Err(err);
        }
        if let Err(cleanup_err) = cleanup_result {
            native_product_probe(
                "validation_cleanup_error",
                format!(
                    "member={} product={} path={} error={cleanup_err}",
                    member.name,
                    product.name,
                    validation_dir.display()
                ),
            );
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (member, product, files);
    }

    Ok(())
}

fn run_cargo_build(
    manifest_path: &Path,
    target_dir: &Path,
    product_name: &str,
) -> PackageResult<()> {
    let status = Command::new("cargo")
        .arg("build")
        .arg("-q")
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--target-dir")
        .arg(target_dir)
        .status()
        .map_err(|e| {
            format!(
                "failed to build native product `{product_name}` from `{}`: {e}",
                manifest_path.display()
            )
        })?;
    if !status.success() {
        return Err(format!(
            "native product build failed for `{product_name}` from `{}` with status {status}",
            manifest_path.display()
        ));
    }
    Ok(())
}

fn validate_native_product_dependency_closure(
    member: &WorkspaceMember,
    product: &crate::NativeProductSpec,
    files: &[NativeBundleFile],
) -> PackageResult<()> {
    #[cfg(windows)]
    {
        files
            .iter()
            .find(|file| file.relative_path == product.file)
            .ok_or_else(|| {
                format!(
                    "native product `{}` on `{}` is missing primary staged file `{}`",
                    product.name, member.name, product.file
                )
            })?;
        let declared = files
            .iter()
            .filter_map(|file| Path::new(&file.relative_path).file_name())
            .filter_map(|name| name.to_str())
            .map(|name| name.to_ascii_lowercase())
            .collect::<BTreeSet<_>>();
        for staged_dll in files.iter().filter(|file| is_windows_dll_bundle_file(file)) {
            let imported = scan_windows_pe_imports(&staged_dll.source_path)?;
            native_product_probe(
                "dependency_scan",
                format!(
                    "member={} product={} file={} imports={}",
                    member.name,
                    product.name,
                    staged_dll.source_path.display(),
                    imported.join(",")
                ),
            );
            validate_windows_imported_dependencies(
                &member.name,
                &product.name,
                &staged_dll.relative_path,
                &imported,
                &declared,
            )
            .map_err(|err| err.to_string())?;
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (member, product, files);
    }

    Ok(())
}

#[cfg(windows)]
fn is_windows_dll_bundle_file(file: &NativeBundleFile) -> bool {
    Path::new(&file.relative_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("dll"))
        .unwrap_or(false)
}

#[cfg(windows)]
fn validate_windows_imported_dependencies(
    member_name: &str,
    product_name: &str,
    staged_relative_path: &str,
    imported: &[String],
    declared: &BTreeSet<String>,
) -> Result<(), String> {
    for dependency in imported {
        let dependency_name = dependency.to_ascii_lowercase();
        if dependency_name.starts_with("std-") && dependency_name.ends_with(".dll") {
            return Err(format!(
                "native product `{}` on `{}` staged DLL `{}` depends on Rust runtime DLL `{dependency}`; Phase 2 requires child/plugin products to avoid staged Rust `std-*.dll` closures",
                product_name, member_name, staged_relative_path
            ));
        }
        if windows_system_dll_allowed(&dependency_name) || declared.contains(&dependency_name) {
            continue;
        }
        native_product_probe(
            "undeclared_dependency",
            format!(
                "member={} product={} file={} dependency={}",
                member_name, product_name, staged_relative_path, dependency
            ),
        );
        return Err(format!(
            "native product `{}` on `{}` staged DLL `{}` depends on undeclared non-system DLL `{dependency}`; declare it in `sidecars` or remove the dependency",
            product_name, member_name, staged_relative_path
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn scan_windows_pe_imports(path: &Path) -> PackageResult<Vec<String>> {
    let bytes = fs::read(path)
        .map_err(|e| format!("failed to read native product `{}`: {e}", path.display()))?;
    let pe = goblin::pe::PE::parse(&bytes).map_err(|e| {
        format!(
            "failed to parse native product `{}` as PE for dependency scanning: {e}",
            path.display()
        )
    })?;
    let mut imports = pe
        .imports
        .into_iter()
        .map(|import| import.dll.to_ascii_lowercase())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    imports.sort();
    Ok(imports)
}

#[cfg(windows)]
fn windows_system_dll_allowed(name: &str) -> bool {
    name.starts_with("api-ms-win-")
        || name.starts_with("ext-ms-")
        || matches!(
            name,
            "advapi32.dll"
                | "avrt.dll"
                | "bcrypt.dll"
                | "cfgmgr32.dll"
                | "combase.dll"
                | "comctl32.dll"
                | "comdlg32.dll"
                | "crypt32.dll"
                | "dwmapi.dll"
                | "gdi32.dll"
                | "hid.dll"
                | "imm32.dll"
                | "kernel32.dll"
                | "msvcp140.dll"
                | "ntdll.dll"
                | "ole32.dll"
                | "oleaut32.dll"
                | "powrprof.dll"
                | "rpcrt4.dll"
                | "secur32.dll"
                | "setupapi.dll"
                | "shell32.dll"
                | "shcore.dll"
                | "shlwapi.dll"
                | "ucrtbase.dll"
                | "user32.dll"
                | "userenv.dll"
                | "vcruntime140.dll"
                | "vcruntime140_1.dll"
                | "version.dll"
                | "winmm.dll"
                | "ws2_32.dll"
        )
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should exist")
        .to_path_buf()
}

fn short_path_fingerprint(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_os_str().to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(16);
    for byte in &digest[..8] {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn copy_distribution_file(source: &Path, destination: &Path) -> PackageResult<()> {
    let bytes = fs::read(source)
        .map_err(|e| format!("failed to read staged file `{}`: {e}", source.display()))?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create distribution subdirectory `{}`: {e}",
                parent.display()
            )
        })?;
    }
    fs::write(destination, bytes).map_err(|e| {
        format!(
            "failed to write distribution file `{}`: {e}",
            destination.display()
        )
    })
}

fn reset_distribution_dir(bundle_dir: &Path, expected_root_artifact: &str) -> PackageResult<()> {
    if !bundle_dir.exists() {
        return fs::create_dir_all(bundle_dir).map_err(|e| {
            format!(
                "failed to create distribution directory `{}`: {e}",
                bundle_dir.display()
            )
        });
    }
    if !bundle_dir.is_dir() {
        return Err(format!(
            "distribution path `{}` exists and is not a directory",
            bundle_dir.display()
        ));
    }
    if directory_is_empty(bundle_dir)? {
        return Ok(());
    }
    validate_managed_distribution_dir(bundle_dir, expected_root_artifact)?;
    clear_distribution_dir_contents(bundle_dir)
}

fn directory_is_empty(dir: &Path) -> PackageResult<bool> {
    let mut entries = fs::read_dir(dir).map_err(|e| {
        format!(
            "failed to read distribution directory `{}`: {e}",
            dir.display()
        )
    })?;
    Ok(entries.next().is_none())
}

fn validate_managed_distribution_dir(
    bundle_dir: &Path,
    expected_root_artifact: &str,
) -> PackageResult<()> {
    let manifest_path = bundle_dir.join(DISTRIBUTION_MANIFEST_FILE);
    if manifest_path.is_file() {
        let manifest_text = fs::read_to_string(&manifest_path).map_err(|e| {
            format!(
                "failed to read distribution manifest `{}`: {e}",
                manifest_path.display()
            )
        })?;
        validate_distribution_manifest_text(&manifest_text).map_err(|e| {
            format!(
                "refusing to overwrite unmanaged distribution directory `{}` because `{}` is not an `{DISTRIBUTION_BUNDLE_FORMAT}` manifest: {e}",
                bundle_dir.display(),
                manifest_path.display()
            )
        })?;
        return Ok(());
    }
    let root_artifact_path = bundle_dir.join(expected_root_artifact);
    if root_artifact_path.is_file() {
        if let Some(manifest_text) = read_embedded_distribution_manifest(&root_artifact_path)? {
            validate_distribution_manifest_text(&manifest_text).map_err(|e| {
                format!(
                    "refusing to overwrite unmanaged distribution directory `{}` because embedded distribution metadata in `{}` is invalid: {e}",
                    bundle_dir.display(),
                    root_artifact_path.display()
                )
            })?;
            return Ok(());
        }
    }
    Err(format!(
        "refusing to overwrite non-empty unmanaged distribution directory `{}`",
        bundle_dir.display()
    ))
}

fn clear_distribution_dir_contents(bundle_dir: &Path) -> PackageResult<()> {
    let entries = fs::read_dir(bundle_dir).map_err(|e| {
        format!(
            "failed to read distribution directory `{}`: {e}",
            bundle_dir.display()
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| {
            format!(
                "failed to enumerate distribution directory `{}`: {e}",
                bundle_dir.display()
            )
        })?;
        let path = entry.path();
        let remove_result = if path.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        remove_result.map_err(|e| {
            format!(
                "failed to clear staged distribution entry `{}`: {e}",
                path.display()
            )
        })?;
    }
    Ok(())
}

fn render_distribution_manifest(
    member: &str,
    build_key: &BuildOutputKey,
    target_format: &str,
    root_artifact: &str,
    support_files: &[String],
    artifact_hash: &str,
    toolchain: &str,
    native_product_closure: Option<&str>,
    root_native_product: Option<&DistributionNativeProduct>,
    native_products: &[DistributionNativeProduct],
    runtime_child_binding: Option<&DistributionChildBinding>,
    child_bindings: &[DistributionChildBinding],
) -> String {
    let support_files = support_files
        .iter()
        .map(|file| format!("\"{}\"", escape_toml(file)))
        .collect::<Vec<_>>()
        .join(", ");
    let mut rendered = format!(
        concat!(
            "format = \"{}\"\n",
            "member = \"{}\"\n",
            "target = \"{}\"\n",
            "target_format = \"{}\"\n",
            "root_artifact = \"{}\"\n",
            "artifact_hash = \"{}\"\n",
            "toolchain = \"{}\"\n",
            "support_files = [{}]\n"
        ),
        DISTRIBUTION_BUNDLE_FORMAT,
        escape_toml(member),
        build_key.target_ref(),
        escape_toml(target_format),
        escape_toml(root_artifact),
        escape_toml(artifact_hash),
        escape_toml(toolchain),
        support_files,
    );
    if let Some(product) = build_key.product() {
        rendered.push_str(&format!("product = \"{}\"\n", escape_toml(product)));
    }
    if let Some(closure) = native_product_closure {
        rendered.push_str(&format!(
            "native_product_closure = \"{}\"\n",
            escape_toml(closure)
        ));
    }
    if let Some(product) = root_native_product {
        rendered.push_str("\n[root_native_product]\n");
        rendered.push_str(&format!(
            "package_name = \"{}\"\n",
            escape_toml(&product.package_name)
        ));
        rendered.push_str(&format!(
            "product_name = \"{}\"\n",
            escape_toml(&product.product_name)
        ));
        rendered.push_str(&format!("role = \"{}\"\n", product.role.as_str()));
        rendered.push_str(&format!(
            "contract_id = \"{}\"\n",
            escape_toml(&product.contract_id)
        ));
        rendered.push_str(&format!(
            "contract_version = {}\n",
            product.contract_version
        ));
        rendered.push_str(&format!(
            "producer = \"{}\"\n",
            escape_toml(&product.producer)
        ));
        rendered.push_str(&format!("file = \"{}\"\n", escape_toml(&product.file)));
        rendered.push_str(&format!(
            "file_hash = \"{}\"\n",
            escape_toml(&product.file_hash)
        ));
        let sidecars = product
            .sidecars
            .iter()
            .map(|sidecar| format!("\"{}\"", escape_toml(sidecar)))
            .collect::<Vec<_>>()
            .join(", ");
        rendered.push_str(&format!("sidecars = [{}]\n", sidecars));
    }
    if let Some(binding) = runtime_child_binding {
        rendered.push_str("\n[runtime_child_binding]\n");
        rendered.push_str(&format!(
            "consumer_member = \"{}\"\n",
            escape_toml(&binding.consumer_member)
        ));
        rendered.push_str(&format!(
            "dependency_alias = \"{}\"\n",
            escape_toml(&binding.dependency_alias)
        ));
        rendered.push_str(&format!(
            "package_name = \"{}\"\n",
            escape_toml(&binding.package_name)
        ));
        rendered.push_str(&format!(
            "product_name = \"{}\"\n",
            escape_toml(&binding.product_name)
        ));
    }
    for product in native_products {
        rendered.push_str("\n[[native_products]]\n");
        rendered.push_str(&format!(
            "package_name = \"{}\"\n",
            escape_toml(&product.package_name)
        ));
        rendered.push_str(&format!(
            "product_name = \"{}\"\n",
            escape_toml(&product.product_name)
        ));
        rendered.push_str(&format!("role = \"{}\"\n", product.role.as_str()));
        rendered.push_str(&format!(
            "contract_id = \"{}\"\n",
            escape_toml(&product.contract_id)
        ));
        rendered.push_str(&format!(
            "contract_version = {}\n",
            product.contract_version
        ));
        rendered.push_str(&format!(
            "producer = \"{}\"\n",
            escape_toml(&product.producer)
        ));
        rendered.push_str(&format!("file = \"{}\"\n", escape_toml(&product.file)));
        rendered.push_str(&format!(
            "file_hash = \"{}\"\n",
            escape_toml(&product.file_hash)
        ));
        let sidecars = product
            .sidecars
            .iter()
            .map(|sidecar| format!("\"{}\"", escape_toml(sidecar)))
            .collect::<Vec<_>>()
            .join(", ");
        rendered.push_str(&format!("sidecars = [{}]\n", sidecars));
    }
    for binding in child_bindings {
        rendered.push_str("\n[[child_bindings]]\n");
        rendered.push_str(&format!(
            "consumer_member = \"{}\"\n",
            escape_toml(&binding.consumer_member)
        ));
        rendered.push_str(&format!(
            "dependency_alias = \"{}\"\n",
            escape_toml(&binding.dependency_alias)
        ));
        rendered.push_str(&format!(
            "package_name = \"{}\"\n",
            escape_toml(&binding.package_name)
        ));
        rendered.push_str(&format!(
            "product_name = \"{}\"\n",
            escape_toml(&binding.product_name)
        ));
    }
    rendered
}

fn distribution_hides_support_file(relative_path: &str) -> bool {
    relative_path.ends_with(".arcana-bundle.toml")
}

fn embed_distribution_manifest(
    root_artifact_path: &Path,
    manifest_text: &str,
) -> PackageResult<()> {
    let payload = manifest_text.as_bytes();
    let payload_len = u64::try_from(payload.len()).map_err(|_| {
        format!(
            "distribution manifest too large to embed in `{}`",
            root_artifact_path.display()
        )
    })?;
    let mut file = OpenOptions::new()
        .append(true)
        .open(root_artifact_path)
        .map_err(|e| {
            format!(
                "failed to open root artifact `{}` for manifest embedding: {e}",
                root_artifact_path.display()
            )
        })?;
    file.write_all(payload).map_err(|e| {
        format!(
            "failed to append distribution manifest payload to `{}`: {e}",
            root_artifact_path.display()
        )
    })?;
    file.write_all(&payload_len.to_le_bytes()).map_err(|e| {
        format!(
            "failed to append distribution manifest length to `{}`: {e}",
            root_artifact_path.display()
        )
    })?;
    file.write_all(EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC)
        .map_err(|e| {
            format!(
                "failed to append distribution manifest marker to `{}`: {e}",
                root_artifact_path.display()
            )
        })?;
    Ok(())
}

fn read_embedded_distribution_manifest(root_artifact_path: &Path) -> PackageResult<Option<String>> {
    let bytes = fs::read(root_artifact_path).map_err(|e| {
        format!(
            "failed to read embedded distribution manifest from `{}`: {e}",
            root_artifact_path.display()
        )
    })?;
    let trailer_len = EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC.len() + std::mem::size_of::<u64>();
    if bytes.len() < trailer_len {
        return Ok(None);
    }
    let magic_start = bytes.len() - EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC.len();
    if bytes[magic_start..] != *EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC {
        return Ok(None);
    }
    let len_start = magic_start - std::mem::size_of::<u64>();
    let payload_len = u64::from_le_bytes(
        bytes[len_start..magic_start]
            .try_into()
            .expect("trailer length bytes should match u64 size"),
    ) as usize;
    if len_start < payload_len {
        return Err(format!(
            "embedded distribution manifest trailer in `{}` is truncated",
            root_artifact_path.display()
        ));
    }
    let payload_start = len_start - payload_len;
    let payload = std::str::from_utf8(&bytes[payload_start..len_start]).map_err(|e| {
        format!(
            "embedded distribution manifest in `{}` is not valid utf8: {e}",
            root_artifact_path.display()
        )
    })?;
    Ok(Some(payload.to_string()))
}

fn validate_distribution_manifest_text(text: &str) -> Result<(), String> {
    let value = text
        .parse::<toml::Value>()
        .map_err(|e| format!("failed to parse distribution manifest: {e}"))?;
    let format = value
        .as_table()
        .and_then(|table| table.get("format"))
        .and_then(toml::Value::as_str);
    if format != Some(DISTRIBUTION_BUNDLE_FORMAT) {
        return Err(format!(
            "expected format `{DISTRIBUTION_BUNDLE_FORMAT}`, found `{}`",
            format.unwrap_or("<missing>")
        ));
    }
    Ok(())
}

fn hash_native_bundle_file(file: &NativeBundleFile) -> PackageResult<String> {
    let bytes = fs::read(&file.source_path).map_err(|e| {
        format!(
            "failed to read native bundle file `{}` for hashing: {e}",
            file.source_path.display()
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn escape_toml(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(windows)]
fn validate_native_product_descriptor_from_staged_root(
    member: &WorkspaceMember,
    product: &crate::NativeProductSpec,
    dll_path: &Path,
) -> PackageResult<()> {
    native_product_probe(
        "validate_descriptor_start",
        format!(
            "member={} product={} role={} producer={} path={}",
            member.name,
            product.name,
            product.role.as_str(),
            product.producer.as_str(),
            dll_path.display()
        ),
    );
    let library = unsafe { Library::new(dll_path) }.map_err(|e| {
        format!(
            "failed to load native product `{}` on `{}` for cabi validation from `{}`: {e}",
            product.name,
            member.name,
            dll_path.display()
        )
    })?;
    let symbol_name = format!("{ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL}\0");
    let get_api = unsafe {
        library
            .get::<unsafe extern "system" fn() -> *const ArcanaCabiProductApiV1>(
                symbol_name.as_bytes(),
            )
            .map_err(|e| {
                format!(
                    "native product `{}` on `{}` is missing `{ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL}` during cabi validation: {e}",
                    product.name, member.name
                )
            })?
    };
    let api = unsafe { get_api().as_ref() }.ok_or_else(|| {
        format!(
            "native product `{}` on `{}` returned a null cabi product descriptor",
            product.name, member.name
        )
    })?;
    if api.descriptor_size < std::mem::size_of::<ArcanaCabiProductApiV1>() {
        return Err(format!(
            "native product `{}` on `{}` reported descriptor_size={} smaller than expected {}",
            product.name,
            member.name,
            api.descriptor_size,
            std::mem::size_of::<ArcanaCabiProductApiV1>()
        ));
    }
    let package_name = unsafe { read_cabi_utf8_field(api.package_name, "package_name") }?;
    let product_name = unsafe { read_cabi_utf8_field(api.product_name, "product_name") }?;
    let role_text = unsafe { read_cabi_utf8_field(api.role, "role") }?;
    let role = ArcanaCabiProductRole::parse(&role_text)?;
    let contract_id = unsafe { read_cabi_utf8_field(api.contract_id, "contract_id") }?;
    if package_name != member.name
        || product_name != product.name
        || role != product.role
        || contract_id != product.contract
    {
        native_product_probe(
            "validate_descriptor_mismatch",
            format!(
                "member={} product={} descriptor={}:{}:{}:{} expected={}:{}:{}:{}",
                member.name,
                product.name,
                package_name,
                product_name,
                role.as_str(),
                contract_id,
                member.name,
                product.name,
                product.role.as_str(),
                product.contract
            ),
        );
        return Err(format!(
            "native product `{}` on `{}` cabi descriptor mismatch: expected `{}:{}` role `{}` contract `{}`, found `{}:{}` role `{}` contract `{}`",
            product.name,
            member.name,
            member.name,
            product.name,
            product.role.as_str(),
            product.contract,
            package_name,
            product_name,
            role.as_str(),
            contract_id
        ));
    }
    if api.contract_version != ARCANA_CABI_CONTRACT_VERSION_V1 {
        return Err(format!(
            "native product `{}` on `{}` reported contract version `{}` but package validation expects `{}`",
            product.name, member.name, api.contract_version, ARCANA_CABI_CONTRACT_VERSION_V1
        ));
    }
    if api.role_ops.is_null() {
        return Err(format!(
            "native product `{}` on `{}` returned null role_ops for role `{}`",
            product.name,
            member.name,
            role.as_str()
        ));
    }
    match role {
        ArcanaCabiProductRole::Export => {
            let export_ops = unsafe { &*(api.role_ops as *const ArcanaCabiExportOpsV1) };
            if export_ops.ops_size < std::mem::size_of::<ArcanaCabiExportOpsV1>() {
                return Err(format!(
                    "native export product `{}` on `{}` reported export ops size {} smaller than expected {}",
                    product.name,
                    member.name,
                    export_ops.ops_size,
                    std::mem::size_of::<ArcanaCabiExportOpsV1>()
                ));
            }
        }
        ArcanaCabiProductRole::Child => {
            let child_ops = unsafe { &*(api.role_ops as *const ArcanaCabiChildOpsV1) };
            if child_ops.base.ops_size < std::mem::size_of::<ArcanaCabiInstanceOpsV1>() {
                return Err(format!(
                    "native child product `{}` on `{}` reported instance ops size {} smaller than expected {}",
                    product.name,
                    member.name,
                    child_ops.base.ops_size,
                    std::mem::size_of::<ArcanaCabiInstanceOpsV1>()
                ));
            }
        }
        ArcanaCabiProductRole::Plugin => {
            let plugin_ops = unsafe { &*(api.role_ops as *const ArcanaCabiPluginOpsV1) };
            if plugin_ops.base.ops_size < std::mem::size_of::<ArcanaCabiInstanceOpsV1>() {
                return Err(format!(
                    "native plugin product `{}` on `{}` reported instance ops size {} smaller than expected {}",
                    product.name,
                    member.name,
                    plugin_ops.base.ops_size,
                    std::mem::size_of::<ArcanaCabiInstanceOpsV1>()
                ));
            }
        }
    }
    native_product_probe(
        "validate_descriptor_ok",
        format!(
            "member={} product={} role={} contract={} version={}",
            member.name,
            product.name,
            role.as_str(),
            product.contract,
            api.contract_version
        ),
    );
    Ok(())
}

#[cfg(windows)]
unsafe fn read_cabi_utf8_field(ptr: *const c_char, field: &str) -> PackageResult<String> {
    if ptr.is_null() {
        return Err(format!(
            "native product cabi descriptor field `{field}` must not be null"
        ));
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(ToOwned::to_owned)
        .map_err(|e| format!("native product cabi descriptor field `{field}` is not utf8: {e}"))
}

fn native_product_probe(event: &str, message: impl AsRef<str>) {
    if std::env::var_os(ARCANA_NATIVE_PRODUCT_TEMP_PROBES_ENV).is_some() {
        eprintln!(
            "[arcana-native-product-probe] {event}: {}",
            message.as_ref()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn windows_dll_bundle_file_detection_is_case_insensitive() {
        assert!(is_windows_dll_bundle_file(&NativeBundleFile {
            relative_path: "bin\\HELPER.DLL".to_string(),
            source_path: PathBuf::from("C:\\repo\\HELPER.DLL"),
        }));
        assert!(!is_windows_dll_bundle_file(&NativeBundleFile {
            relative_path: "data\\config.json".to_string(),
            source_path: PathBuf::from("C:\\repo\\config.json"),
        }));
    }

    #[cfg(windows)]
    #[test]
    fn validate_windows_imported_dependencies_reports_sidecar_context() {
        let declared = ["primary.dll", "helper.dll"]
            .into_iter()
            .map(|name| name.to_string())
            .collect::<BTreeSet<_>>();
        let err = validate_windows_imported_dependencies(
            "app",
            "desktop",
            "helper.dll",
            &["mystery.dll".to_string()],
            &declared,
        )
        .expect_err("undeclared sidecar dependency should fail");
        assert!(err.contains("helper.dll"), "{err}");
        assert!(err.contains("mystery.dll"), "{err}");
    }
}
