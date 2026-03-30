mod artifact;
mod codec;
mod compile;
mod emit;
mod instance_product;
mod native_abi;
mod native_backend;
mod native_layout;
mod native_lowering;
mod native_manifest;
mod native_plan;
mod rust_codegen;
mod rust_toolchain;
mod validate;
mod windows_bundle;
mod windows_dll;

pub use artifact::{
    AOT_INTERNAL_FORMAT, AotArtifact, AotEntrypointArtifact, AotOwnerArtifact,
    AotOwnerExitArtifact, AotOwnerObjectArtifact, AotPackageArtifact, AotPackageModuleArtifact,
    AotRoutineArtifact, AotRoutineParamArtifact,
};
pub use codec::{parse_package_artifact, render_package_artifact};
pub use compile::{compile_module, compile_package};
pub use emit::{
    AOT_WINDOWS_DLL_FORMAT, AOT_WINDOWS_EXE_FORMAT, AotEmissionFile, AotEmitContext, AotEmitTarget,
    AotNativeProduct, AotPackageEmission, AotRuntimeBinding, emit_package,
    emit_package_with_context,
};
pub use instance_product::{
    ARCANA_NATIVE_PRODUCT_TEMP_PROBES_ENV, AotCompiledInstanceProduct, AotInstanceProductSpec,
    compile_instance_product,
};
pub use native_manifest::{
    NATIVE_BUNDLE_MANIFEST_FORMAT, NativeBundleLaunchManifest, NativeBundleManifest,
    parse_native_bundle_manifest, render_native_bundle_manifest,
};
pub use native_plan::{NativeLaunchPlan, NativePackagePlan, build_native_package_plan};
pub use validate::validate_package_artifact;

#[cfg(test)]
mod tests {
    use super::{
        AOT_INTERNAL_FORMAT, AOT_WINDOWS_DLL_FORMAT, AOT_WINDOWS_EXE_FORMAT, AotEmitContext,
        AotEmitTarget, AotEntrypointArtifact, AotPackageArtifact, AotPackageModuleArtifact,
        AotRoutineArtifact, AotRoutineParamArtifact, AotRuntimeBinding,
        NATIVE_BUNDLE_MANIFEST_FORMAT, NativeLaunchPlan, build_native_package_plan, compile_module,
        compile_package, emit_package, emit_package_with_context, parse_native_bundle_manifest,
        parse_package_artifact, render_native_bundle_manifest, render_package_artifact,
        validate_package_artifact,
    };
    use arcana_ir::{
        ExecExpr, ExecPageRollup, ExecPhraseQualifierKind, ExecStmt, IrEntrypoint, IrModule,
        IrPackage, IrPackageModule, IrRoutine, IrRoutineParam, IrRoutineType,
        parse_routine_type_text, render_routine_signature_text,
    };
    use std::collections::BTreeMap;

    trait TestParamRow: Sized {
        fn from_test_row(row: &str) -> Self;
    }

    impl TestParamRow for IrRoutineParam {
        fn from_test_row(row: &str) -> Self {
            let parts = row.splitn(3, ':').collect::<Vec<_>>();
            let mode = parts[0].strip_prefix("mode=").unwrap_or_default();
            let name = parts[1].strip_prefix("name=").unwrap_or_default();
            let ty = parts[2].strip_prefix("ty=").unwrap_or_default();
            Self {
                mode: (!mode.is_empty()).then(|| mode.to_string()),
                name: name.to_string(),
                ty: parse_routine_type_text(ty).expect("type should parse"),
            }
        }
    }

    fn test_return_type(signature: &str) -> Option<IrRoutineType> {
        let (_, tail) = signature.rsplit_once("->")?;
        let trimmed = tail.trim().trim_end_matches(':').trim();
        (!trimmed.is_empty())
            .then(|| parse_routine_type_text(trimmed).expect("return type should parse"))
    }

    fn test_params<T, S>(rows: &[S]) -> Vec<T>
    where
        T: TestParamRow,
        S: AsRef<str>,
    {
        rows.iter()
            .map(|row| T::from_test_row(row.as_ref()))
            .collect()
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

    fn test_emit_context(file_name: &str) -> AotEmitContext {
        AotEmitContext {
            root_artifact_file_name: Some(file_name.to_string()),
            runtime_binding: AotRuntimeBinding::Baked,
            native_product: None,
        }
    }

    fn sync_exported_function_surface_rows(package: &mut IrPackage) {
        let exported_routines = package
            .routines
            .iter()
            .filter(|routine| routine.exported && routine.impl_target_type.is_none())
            .collect::<Vec<_>>();
        package.exported_surface_rows = exported_routines
            .iter()
            .map(|routine| {
                format!(
                    "module={}:export:{}:{}",
                    routine.module_id,
                    routine.symbol_kind,
                    render_routine_signature_text(
                        &routine.symbol_kind,
                        &routine.symbol_name,
                        routine.is_async,
                        &routine.type_params,
                        &routine.params,
                        routine.return_type.as_ref(),
                    )
                )
            })
            .collect();
        for module in &mut package.modules {
            module.exported_surface_rows = exported_routines
                .iter()
                .filter(|routine| routine.module_id == module.module_id)
                .map(|routine| {
                    format!(
                        "export:{}:{}",
                        routine.symbol_kind,
                        render_routine_signature_text(
                            &routine.symbol_kind,
                            &routine.symbol_name,
                            routine.is_async,
                            &routine.type_params,
                            &routine.params,
                            routine.return_type.as_ref(),
                        )
                    )
                })
                .collect();
        }
    }

    fn base_surface_validation_artifact() -> AotPackageArtifact {
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
            exported_surface_rows: vec!["module=tool:export:fn:fn main() -> Int:".to_string()],
            runtime_requirements: Vec::new(),
            entrypoints: Vec::new(),
            routines: vec![AotRoutineArtifact {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: Vec::new(),
            }],
            owners: Vec::new(),
            modules: vec![AotPackageModuleArtifact {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                symbol_count: 1,
                item_count: 1,
                line_count: 1,
                non_empty_line_count: 1,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: vec!["export:fn:fn main() -> Int:".to_string()],
            }],
        }
    }

    #[test]
    fn compile_module_emits_internal_artifact() {
        let artifact = compile_module(&IrModule {
            symbol_count: 1,
            item_count: 3,
        });
        assert_eq!(artifact.format, AOT_INTERNAL_FORMAT);
    }

    #[test]
    fn compile_package_emits_backend_contract_artifact() {
        let artifact = compile_package(&IrPackage {
            package_id: "winspell".to_string(),
            package_name: "winspell".to_string(),
            root_module_id: "winspell".to_string(),
            direct_deps: vec!["std".to_string()],
            direct_dep_ids: vec!["std".to_string()],
            package_display_names: test_package_display_names_with_deps(
                "winspell".to_string(),
                "winspell".to_string(),
                vec!["std".to_string()],
                vec!["std".to_string()],
            ),
            package_direct_dep_ids: test_package_direct_dep_ids(
                "winspell".to_string(),
                vec!["std".to_string()],
                vec!["std".to_string()],
            ),
            modules: vec![
                IrPackageModule {
                    package_id: test_package_id_for_module("winspell"),
                    module_id: "winspell".to_string(),
                    symbol_count: 1,
                    item_count: 3,
                    line_count: 4,
                    non_empty_line_count: 3,
                    directive_rows: vec!["module=winspell:reexport:winspell.window:".to_string()],
                    lang_item_rows: Vec::new(),
                    exported_surface_rows: vec!["export:fn:fn open() -> Int:".to_string()],
                },
                IrPackageModule {
                    package_id: test_package_id_for_module("winspell.window"),
                    module_id: "winspell.window".to_string(),
                    symbol_count: 2,
                    item_count: 5,
                    line_count: 6,
                    non_empty_line_count: 5,
                    directive_rows: vec!["module=winspell.window:import:std.canvas:".to_string()],
                    lang_item_rows: Vec::new(),
                    exported_surface_rows: Vec::new(),
                },
            ],
            dependency_edge_count: 2,
            dependency_rows: vec![
                "source=winspell:reexport:winspell.window:".to_string(),
                "source=winspell.window:import:std.canvas:".to_string(),
            ],
            exported_surface_rows: vec!["module=winspell:export:fn:fn open() -> Int:".to_string()],
            runtime_requirements: vec!["std.canvas".to_string()],
            entrypoints: vec![IrEntrypoint {
                package_id: test_package_id_for_module("winspell"),
                module_id: "winspell".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![IrRoutine {
                package_id: test_package_id_for_module("winspell"),
                module_id: "winspell".to_string(),
                routine_key: "winspell#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(0),
                }],
            }],
            owners: Vec::new(),
        });
        assert_eq!(artifact.format, AOT_INTERNAL_FORMAT);
        assert_eq!(artifact.module_count, 2);
        assert_eq!(artifact.modules[0].module_id, "winspell");
    }

    #[test]
    fn emit_package_internal_artifact_matches_rendered_body() {
        let mut package = IrPackage {
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
                "tool".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            modules: vec![IrPackageModule {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                symbol_count: 1,
                item_count: 1,
                line_count: 1,
                non_empty_line_count: 1,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: Vec::new(),
            runtime_requirements: Vec::new(),
            entrypoints: Vec::new(),
            routines: vec![IrRoutine {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(0),
                }],
            }],
            owners: Vec::new(),
        };

        sync_exported_function_surface_rows(&mut package);
        let emission =
            emit_package(AotEmitTarget::InternalArtifact, &package).expect("emit should succeed");
        assert_eq!(emission.target, AotEmitTarget::InternalArtifact);
        assert_eq!(emission.artifact.format, AOT_INTERNAL_FORMAT);
        assert_eq!(
            emission.primary_artifact_body,
            render_package_artifact(&emission.artifact)
        );
        assert!(emission.root_artifact_bytes.is_none());
        assert!(emission.support_files.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn native_emit_targets_compile_generated_artifacts() {
        let package = IrPackage {
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
                "tool".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            modules: vec![IrPackageModule {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                symbol_count: 0,
                item_count: 0,
                line_count: 0,
                non_empty_line_count: 0,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: vec!["export:fn:fn main() -> Int:".to_string()],
            }],
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: vec!["module=tool:export:fn:fn main() -> Int:".to_string()],
            runtime_requirements: Vec::new(),
            entrypoints: vec![IrEntrypoint {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![IrRoutine {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(0),
                }],
            }],
            owners: Vec::new(),
        };

        assert_eq!(
            AotEmitTarget::WindowsExeBundle.format(),
            AOT_WINDOWS_EXE_FORMAT
        );
        assert_eq!(
            AotEmitTarget::WindowsDllBundle.format(),
            AOT_WINDOWS_DLL_FORMAT
        );
        let exe = emit_package_with_context(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("windows exe emit should succeed");
        assert!(
            exe.root_artifact_bytes
                .as_ref()
                .is_some_and(|bytes| !bytes.is_empty())
        );
        assert_eq!(
            exe.support_files
                .iter()
                .map(|file| file.relative_path.as_str())
                .collect::<Vec<_>>(),
            vec!["app.exe.arcana-bundle.toml"]
        );
        let exe_manifest = parse_native_bundle_manifest(
            std::str::from_utf8(&exe.support_files[0].bytes)
                .expect("native exe manifest should be utf8"),
        )
        .expect("native exe manifest should parse");
        assert_eq!(exe_manifest.format, NATIVE_BUNDLE_MANIFEST_FORMAT);
        assert_eq!(exe_manifest.launch.kind, "executable");
        assert_eq!(
            exe_manifest.launch.main_routine_key.as_deref(),
            Some("tool#fn-0")
        );
        let dll = emit_package_with_context(
            AotEmitTarget::WindowsDllBundle,
            &package,
            &test_emit_context("lib.dll"),
        )
        .expect("windows dll emit should succeed");
        assert!(
            dll.root_artifact_bytes
                .as_ref()
                .is_some_and(|bytes| !bytes.is_empty())
        );
        assert_eq!(
            dll.support_files
                .iter()
                .map(|file| file.relative_path.as_str())
                .collect::<Vec<_>>(),
            vec!["lib.dll.h", "lib.dll.def", "lib.dll.arcana-bundle.toml"]
        );
        let dll_text =
            std::str::from_utf8(&dll.support_files[0].bytes).expect("dll header should be utf8");
        assert!(dll_text.contains("arcana_cabi_last_error_alloc_v1"));
        assert!(dll_text.contains("arcana_cabi_get_product_api_v1"));
        let dll_def =
            std::str::from_utf8(&dll.support_files[1].bytes).expect("dll def should be utf8");
        assert!(dll_def.contains("EXPORTS"));
        assert!(dll_def.contains("main"));
        let dll_manifest = parse_native_bundle_manifest(
            std::str::from_utf8(&dll.support_files[2].bytes)
                .expect("native dll manifest should be utf8"),
        )
        .expect("native dll manifest should parse");
        assert_eq!(dll_manifest.format, NATIVE_BUNDLE_MANIFEST_FORMAT);
        assert_eq!(dll_manifest.launch.kind, "dynamic-library");
        assert_eq!(dll_manifest.launch.header.as_deref(), Some("lib.dll.h"));
        assert_eq!(
            dll_manifest.launch.definition_file.as_deref(),
            Some("lib.dll.def")
        );
        assert_eq!(dll_manifest.launch.exports.len(), 1);
        assert_eq!(dll_manifest.launch.exports[0].export_name, "main");
    }

    #[test]
    fn native_package_plan_resolves_main_routine_key() {
        let mut package = IrPackage {
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
                "tool".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            modules: vec![IrPackageModule {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                symbol_count: 1,
                item_count: 1,
                line_count: 1,
                non_empty_line_count: 1,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: Vec::new(),
            runtime_requirements: Vec::new(),
            entrypoints: vec![IrEntrypoint {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![IrRoutine {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(0),
                }],
            }],
            owners: Vec::new(),
        };

        sync_exported_function_surface_rows(&mut package);
        let plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native plan should build");
        assert_eq!(plan.root_artifact_file_name, "app.exe");
        assert_eq!(
            plan.launch,
            NativeLaunchPlan::Executable {
                main_routine_key: "tool#fn-0".to_string(),
            }
        );
    }

    #[test]
    fn native_package_plan_rejects_missing_main_entrypoint_for_windows_exe() {
        let package = IrPackage {
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
                "tool".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            modules: vec![IrPackageModule {
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
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: Vec::new(),
            runtime_requirements: Vec::new(),
            entrypoints: Vec::new(),
            routines: Vec::new(),
            owners: Vec::new(),
        };

        let err = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect_err("native plan should reject missing main");
        assert!(
            err.contains("requires exactly one main entrypoint"),
            "{err}"
        );
    }

    #[test]
    fn native_bundle_manifest_roundtrips_windows_dll_export_contract() {
        let mut package = IrPackage {
            package_id: "core".to_string(),
            package_name: "core".to_string(),
            root_module_id: "core".to_string(),
            direct_deps: Vec::new(),
            direct_dep_ids: Vec::new(),
            package_display_names: test_package_display_names_with_deps(
                "core".to_string(),
                "core".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            package_direct_dep_ids: test_package_direct_dep_ids(
                "core".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            modules: vec![IrPackageModule {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                symbol_count: 1,
                item_count: 1,
                line_count: 1,
                non_empty_line_count: 1,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: Vec::new(),
            runtime_requirements: Vec::new(),
            entrypoints: Vec::new(),
            routines: vec![IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "answer".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn answer(value: Int) -> Bool:"),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Bool(true),
                }],
            }],
            owners: Vec::new(),
        };

        sync_exported_function_surface_rows(&mut package);
        let plan = build_native_package_plan(
            AotEmitTarget::WindowsDllBundle,
            &package,
            &test_emit_context("lib.dll"),
        )
        .expect("native plan should build");
        let manifest_text =
            render_native_bundle_manifest(&plan).expect("native manifest should render");
        let manifest =
            parse_native_bundle_manifest(&manifest_text).expect("native manifest should parse");

        assert_eq!(manifest.format, NATIVE_BUNDLE_MANIFEST_FORMAT);
        assert_eq!(manifest.target, "windows-dll");
        assert_eq!(manifest.root_artifact, "lib.dll");
        assert_eq!(manifest.product_name.as_deref(), Some("default"));
        assert_eq!(manifest.product_role.as_deref(), Some("export"));
        assert_eq!(
            manifest.contract_id.as_deref(),
            Some("arcana.cabi.export.v1")
        );
        assert_eq!(manifest.contract_version, Some(1));
        assert_eq!(manifest.launch.kind, "dynamic-library");
        assert_eq!(manifest.launch.header.as_deref(), Some("lib.dll.h"));
        assert_eq!(
            manifest.launch.definition_file.as_deref(),
            Some("lib.dll.def")
        );
        assert_eq!(
            manifest.launch.last_error_alloc_symbol.as_deref(),
            Some("arcana_cabi_last_error_alloc_v1")
        );
        assert_eq!(
            manifest.launch.owned_bytes_free_symbol.as_deref(),
            Some("arcana_cabi_owned_bytes_free_v1")
        );
        assert_eq!(
            manifest.launch.owned_str_free_symbol.as_deref(),
            Some("arcana_cabi_owned_str_free_v1")
        );
        assert_eq!(manifest.launch.exports.len(), 1);
        assert_eq!(manifest.launch.exports[0].export_name, "answer");
        assert_eq!(manifest.launch.exports[0].routine_key, "core#fn-0");
        assert_eq!(manifest.launch.exports[0].symbol_name, "answer");
        assert_eq!(manifest.launch.exports[0].return_type, "Bool");
        assert_eq!(manifest.launch.exports[0].params.len(), 1);
        assert_eq!(manifest.launch.exports[0].params[0].name, "value");
        assert_eq!(manifest.launch.exports[0].params[0].source_mode, "read");
        assert_eq!(manifest.launch.exports[0].params[0].pass_mode, "in");
        assert_eq!(manifest.launch.exports[0].params[0].input_type, "Int");
        assert_eq!(manifest.launch.exports[0].params[0].write_back_type, None);
    }

    #[test]
    fn native_bundle_manifest_preserves_string_and_byte_exports() {
        let mut package = IrPackage {
            package_id: "core".to_string(),
            package_name: "core".to_string(),
            root_module_id: "core".to_string(),
            direct_deps: Vec::new(),
            direct_dep_ids: Vec::new(),
            package_display_names: test_package_display_names_with_deps(
                "core".to_string(),
                "core".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            package_direct_dep_ids: test_package_direct_dep_ids(
                "core".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            modules: vec![IrPackageModule {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                symbol_count: 2,
                item_count: 2,
                line_count: 2,
                non_empty_line_count: 2,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: Vec::new(),
            runtime_requirements: Vec::new(),
            entrypoints: Vec::new(),
            routines: vec![
                IrRoutine {
                    package_id: test_package_id_for_module("core"),
                    module_id: "core".to_string(),
                    routine_key: "core#fn-0".to_string(),
                    symbol_name: "greet".to_string(),
                    symbol_kind: "fn".to_string(),
                    exported: true,
                    is_async: false,
                    type_params: Vec::new(),
                    behavior_attrs: BTreeMap::new(),
                    params: test_params(&["mode=read:name=name:ty=Str".to_string()]),
                    return_type: test_return_type("fn greet(read name: Str) -> Str:"),
                    intrinsic_impl: None,
                    impl_target_type: None,
                    impl_trait_path: None,
                    availability: Vec::new(),
                    foreword_rows: Vec::new(),
                    rollups: Vec::new(),
                    statements: vec![ExecStmt::ReturnValue {
                        value: ExecExpr::Str("hi".to_string()),
                    }],
                },
                IrRoutine {
                    package_id: test_package_id_for_module("core"),
                    module_id: "core".to_string(),
                    routine_key: "core#fn-1".to_string(),
                    symbol_name: "prefix".to_string(),
                    symbol_kind: "fn".to_string(),
                    exported: true,
                    is_async: false,
                    type_params: Vec::new(),
                    behavior_attrs: BTreeMap::new(),
                    params: test_params(&["mode=read:name=bytes:ty=Array[Int]".to_string()]),
                    return_type: test_return_type(
                        "fn prefix(read bytes: Array[Int]) -> Array[Int]:",
                    ),
                    intrinsic_impl: None,
                    impl_target_type: None,
                    impl_trait_path: None,
                    availability: Vec::new(),
                    foreword_rows: Vec::new(),
                    rollups: Vec::new(),
                    statements: vec![ExecStmt::ReturnValue {
                        value: ExecExpr::Path(vec!["bytes".to_string()]),
                    }],
                },
            ],
            owners: Vec::new(),
        };

        sync_exported_function_surface_rows(&mut package);
        let plan = build_native_package_plan(
            AotEmitTarget::WindowsDllBundle,
            &package,
            &test_emit_context("lib.dll"),
        )
        .expect("native plan should build");
        let manifest_text =
            render_native_bundle_manifest(&plan).expect("native manifest should render");
        let manifest =
            parse_native_bundle_manifest(&manifest_text).expect("native manifest should parse");

        assert_eq!(manifest.launch.exports.len(), 2);
        assert_eq!(manifest.launch.exports[0].export_name, "greet");
        assert_eq!(manifest.launch.exports[0].params[0].source_mode, "read");
        assert_eq!(manifest.launch.exports[0].params[0].pass_mode, "in");
        assert_eq!(manifest.launch.exports[0].params[0].input_type, "Str");
        assert_eq!(manifest.launch.exports[0].return_type, "Str");
        assert_eq!(manifest.launch.exports[1].export_name, "prefix");
        assert_eq!(manifest.launch.exports[1].params[0].source_mode, "read");
        assert_eq!(manifest.launch.exports[1].params[0].pass_mode, "in");
        assert_eq!(
            manifest.launch.exports[1].params[0].input_type,
            "Array[Int]"
        );
        assert_eq!(manifest.launch.exports[1].return_type, "Array[Int]");
    }

    #[test]
    fn collect_native_exports_excludes_dependency_generic_surface() {
        let artifact = AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT.to_string(),
            package_id: "core".to_string(),
            package_name: "core".to_string(),
            root_module_id: "core".to_string(),
            direct_deps: vec!["std".to_string()],
            direct_dep_ids: vec!["std".to_string()],
            package_display_names: test_package_display_names_with_deps(
                "core".to_string(),
                "core".to_string(),
                vec!["std".to_string()],
                vec!["std".to_string()],
            ),
            package_direct_dep_ids: test_package_direct_dep_ids(
                test_package_id_for_module("std.array"),
                vec!["std".to_string()],
                vec!["std".to_string()],
            ),
            module_count: 2,
            dependency_edge_count: 1,
            dependency_rows: vec!["source=core:import:std.array:".to_string()],
            exported_surface_rows: vec!["module=core:export:fn:fn answer() -> Int:".to_string()],
            runtime_requirements: Vec::new(),
            entrypoints: Vec::new(),
            routines: vec![
                AotRoutineArtifact {
                    package_id: test_package_id_for_module("core"),
                    module_id: "core".to_string(),
                    routine_key: "core#fn-0".to_string(),
                    symbol_name: "answer".to_string(),
                    symbol_kind: "fn".to_string(),
                    exported: true,
                    is_async: false,
                    type_params: Vec::new(),
                    behavior_attrs: BTreeMap::new(),
                    params: Vec::new(),
                    return_type: test_return_type("fn answer() -> Int:"),
                    intrinsic_impl: None,
                    impl_target_type: None,
                    impl_trait_path: None,
                    availability: Vec::new(),
                    foreword_rows: Vec::new(),
                    rollups: Vec::new(),
                    statements: vec![ExecStmt::ReturnValue {
                        value: ExecExpr::Int(42),
                    }],
                },
                AotRoutineArtifact {
                    package_id: test_package_id_for_module("std.array"),
                    module_id: "std.array".to_string(),
                    routine_key: "std.array#sym-0".to_string(),
                    symbol_name: "len".to_string(),
                    symbol_kind: "fn".to_string(),
                    exported: true,
                    is_async: false,
                    type_params: vec!["T".to_string()],
                    behavior_attrs: BTreeMap::new(),
                    params: test_params(&["mode=read:name=values:ty=Array[T]".to_string()]),
                    return_type: test_return_type("fn len[T](read values: Array[T]) -> Int:"),
                    intrinsic_impl: None,
                    impl_target_type: None,
                    impl_trait_path: None,
                    availability: Vec::new(),
                    foreword_rows: Vec::new(),
                    rollups: Vec::new(),
                    statements: vec![ExecStmt::ReturnValue {
                        value: ExecExpr::Int(0),
                    }],
                },
            ],
            owners: Vec::new(),
            modules: vec![
                AotPackageModuleArtifact {
                    package_id: test_package_id_for_module("core"),
                    module_id: "core".to_string(),
                    symbol_count: 1,
                    item_count: 1,
                    line_count: 1,
                    non_empty_line_count: 1,
                    directive_rows: Vec::new(),
                    lang_item_rows: Vec::new(),
                    exported_surface_rows: vec!["export:fn:fn answer() -> Int:".to_string()],
                },
                AotPackageModuleArtifact {
                    package_id: test_package_id_for_module("std.array"),
                    module_id: "std.array".to_string(),
                    symbol_count: 1,
                    item_count: 1,
                    line_count: 1,
                    non_empty_line_count: 1,
                    directive_rows: Vec::new(),
                    lang_item_rows: Vec::new(),
                    exported_surface_rows: vec![
                        "export:fn:fn len[T](read values: Array[T]) -> Int:".to_string(),
                    ],
                },
            ],
        };

        let exports = crate::native_abi::collect_native_exports(&artifact)
            .expect("dependency generic exports should be ignored");
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].export_name, "answer");
        assert_eq!(exports[0].routine_key, "core#fn-0");
    }

    #[test]
    fn native_bundle_manifest_preserves_pair_exports() {
        let mut package = IrPackage {
            package_id: "core".to_string(),
            package_name: "core".to_string(),
            root_module_id: "core".to_string(),
            direct_deps: Vec::new(),
            direct_dep_ids: Vec::new(),
            package_display_names: test_package_display_names_with_deps(
                "core".to_string(),
                "core".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            package_direct_dep_ids: test_package_direct_dep_ids(
                "core".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            modules: vec![IrPackageModule {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                symbol_count: 1,
                item_count: 1,
                line_count: 1,
                non_empty_line_count: 1,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: Vec::new(),
            runtime_requirements: Vec::new(),
            entrypoints: Vec::new(),
            routines: vec![IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "echo_pair".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=read:name=pair:ty=Pair[Str, Int]".to_string()]),
                return_type: test_return_type(
                    "fn echo_pair(read pair: Pair[Str, Int]) -> Pair[Str, Int]:",
                ),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Path(vec!["pair".to_string()]),
                }],
            }],
            owners: Vec::new(),
        };

        sync_exported_function_surface_rows(&mut package);
        let plan = build_native_package_plan(
            AotEmitTarget::WindowsDllBundle,
            &package,
            &test_emit_context("lib.dll"),
        )
        .expect("native plan should build");
        let manifest_text =
            render_native_bundle_manifest(&plan).expect("native manifest should render");
        let manifest =
            parse_native_bundle_manifest(&manifest_text).expect("native manifest should parse");

        assert_eq!(manifest.launch.exports.len(), 1);
        assert_eq!(manifest.launch.exports[0].export_name, "echo_pair");
        assert_eq!(manifest.launch.exports[0].params[0].source_mode, "read");
        assert_eq!(manifest.launch.exports[0].params[0].pass_mode, "in");
        assert_eq!(
            manifest.launch.exports[0].params[0].input_type,
            "Pair[Str, Int]"
        );
        assert_eq!(manifest.launch.exports[0].return_type, "Pair[Str, Int]");
    }

    #[test]
    fn package_artifact_roundtrips() {
        let artifact = AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT.to_string(),
            package_id: "tool".to_string(),
            package_name: "tool".to_string(),
            root_module_id: "tool".to_string(),
            direct_deps: vec!["std".to_string()],
            direct_dep_ids: vec!["std".to_string()],
            package_display_names: test_package_display_names_with_deps(
                "tool".to_string(),
                "tool".to_string(),
                vec!["std".to_string()],
                vec!["std".to_string()],
            ),
            package_direct_dep_ids: test_package_direct_dep_ids(
                test_package_id_for_module("tool"),
                vec!["std".to_string()],
                vec!["std".to_string()],
            ),
            module_count: 1,
            dependency_edge_count: 1,
            dependency_rows: vec!["source=tool:import:std.io:".to_string()],
            exported_surface_rows: vec![
                "module=tool:export:fn:fn main(x: Int) -> Int:".to_string(),
            ],
            runtime_requirements: vec!["std.io".to_string()],
            entrypoints: vec![AotEntrypointArtifact {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![AotRoutineArtifact {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=:name=x:ty=Int".to_string()]),
                return_type: test_return_type("fn main(x: Int) -> Int:"),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                foreword_rows: vec!["test()".to_string()],
                rollups: vec![ExecPageRollup {
                    kind: "cleanup".to_string(),
                    subject: "page".to_string(),
                    handler_path: vec!["handler".to_string()],
                }],
                statements: vec![ExecStmt::Expr {
                    expr: ExecExpr::Phrase {
                        subject: Box::new(ExecExpr::Path(vec!["x".to_string()])),
                        args: Vec::new(),
                        qualifier_kind: ExecPhraseQualifierKind::BareMethod,
                        qualifier: "is_ok".to_string(),
                        resolved_callable: Some(vec![
                            "std".to_string(),
                            "result".to_string(),
                            "is_ok".to_string(),
                        ]),
                        resolved_routine: Some("std.result#impl-0-method-0".to_string()),
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                    rollups: Vec::new(),
                }],
            }],
            owners: Vec::new(),
            modules: vec![AotPackageModuleArtifact {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                symbol_count: 1,
                item_count: 2,
                line_count: 3,
                non_empty_line_count: 2,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: vec!["export:fn:fn main(x: Int) -> Int:".to_string()],
            }],
        };
        let rendered = render_package_artifact(&artifact);
        let parsed = parse_package_artifact(&rendered).expect("artifact should roundtrip");
        assert_eq!(parsed, artifact);
    }

    #[test]
    fn parse_package_artifact_rejects_mismatched_module_count() {
        let mut artifact = compile_package(&IrPackage {
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
                "tool".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            modules: vec![IrPackageModule {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                symbol_count: 1,
                item_count: 1,
                line_count: 1,
                non_empty_line_count: 1,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: Vec::new(),
            runtime_requirements: Vec::new(),
            entrypoints: Vec::new(),
            routines: vec![IrRoutine {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn helper() -> Int:"),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(0),
                }],
            }],
            owners: Vec::new(),
        });
        artifact.module_count = 2;

        let err = parse_package_artifact(&render_package_artifact(&artifact))
            .expect_err("artifact should reject mismatched module count");
        assert!(
            err.contains("module_count=2 does not match modules.len()=1"),
            "{err}"
        );
    }

    #[test]
    fn validate_package_artifact_rejects_ambiguous_entrypoint_routines() {
        let artifact = AotPackageArtifact {
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
            exported_surface_rows: vec![
                "module=tool:export:fn:fn main() -> Int:".to_string(),
                "module=tool:export:fn:fn main(x: Int) -> Int:".to_string(),
            ],
            runtime_requirements: Vec::new(),
            entrypoints: vec![AotEntrypointArtifact {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![
                AotRoutineArtifact {
                    package_id: test_package_id_for_module("tool"),
                    module_id: "tool".to_string(),
                    routine_key: "tool#fn-0".to_string(),
                    symbol_name: "main".to_string(),
                    symbol_kind: "fn".to_string(),
                    exported: true,
                    is_async: false,
                    type_params: Vec::new(),
                    behavior_attrs: BTreeMap::new(),
                    params: Vec::new(),
                    return_type: test_return_type("fn main() -> Int:"),
                    intrinsic_impl: None,
                    impl_target_type: None,
                    impl_trait_path: None,
                    availability: Vec::new(),
                    foreword_rows: Vec::new(),
                    rollups: Vec::new(),
                    statements: Vec::new(),
                },
                AotRoutineArtifact {
                    package_id: test_package_id_for_module("tool"),
                    module_id: "tool".to_string(),
                    routine_key: "tool#fn-1".to_string(),
                    symbol_name: "main".to_string(),
                    symbol_kind: "fn".to_string(),
                    exported: true,
                    is_async: false,
                    type_params: Vec::new(),
                    behavior_attrs: BTreeMap::new(),
                    params: test_params(&["mode=:name=x:ty=Int".to_string()]),
                    return_type: test_return_type("fn main(x: Int) -> Int:"),
                    intrinsic_impl: None,
                    impl_target_type: None,
                    impl_trait_path: None,
                    availability: Vec::new(),
                    foreword_rows: Vec::new(),
                    rollups: Vec::new(),
                    statements: Vec::new(),
                },
            ],
            owners: Vec::new(),
            modules: vec![AotPackageModuleArtifact {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                symbol_count: 1,
                item_count: 1,
                line_count: 1,
                non_empty_line_count: 1,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: vec![
                    "export:fn:fn main() -> Int:".to_string(),
                    "export:fn:fn main(x: Int) -> Int:".to_string(),
                ],
            }],
        };

        let err = validate_package_artifact(&artifact)
            .expect_err("artifact should reject ambiguous entrypoint routines");
        assert!(
            err.contains("entrypoint `tool::tool.main` is ambiguous across routines"),
            "{err}"
        );
    }

    #[test]
    fn parse_package_artifact_rejects_invalid_structured_params() {
        let mut artifact = compile_package(&IrPackage {
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
                "tool".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            modules: vec![IrPackageModule {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                symbol_count: 1,
                item_count: 1,
                line_count: 1,
                non_empty_line_count: 1,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: Vec::new(),
            runtime_requirements: Vec::new(),
            entrypoints: Vec::new(),
            routines: vec![IrRoutine {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=read:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn helper(read value: Int) -> Int:"),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(0),
                }],
            }],
            owners: Vec::new(),
        });
        artifact.routines[0].params = vec![AotRoutineParamArtifact {
            mode: Some("invalid".to_string()),
            name: "value".to_string(),
            ty: parse_routine_type_text("Int").expect("type should parse"),
        }];

        let err = parse_package_artifact(&render_package_artifact(&artifact))
            .expect_err("artifact should reject invalid structured params");
        assert!(err.contains("unsupported mode"), "{err}");
    }

    #[test]
    fn validate_package_artifact_rejects_malformed_module_directive_rows() {
        let artifact = AotPackageArtifact {
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
            entrypoints: Vec::new(),
            routines: vec![AotRoutineArtifact {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn helper() -> Int:"),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: Vec::new(),
            }],
            owners: Vec::new(),
            modules: vec![AotPackageModuleArtifact {
                package_id: test_package_id_for_module("tool"),
                module_id: "tool".to_string(),
                symbol_count: 1,
                item_count: 1,
                line_count: 1,
                non_empty_line_count: 1,
                directive_rows: vec!["module=tool:import::".to_string()],
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
        };

        let err = validate_package_artifact(&artifact)
            .expect_err("artifact should reject malformed module directive rows");
        assert!(err.contains("invalid path"), "{err}");
    }

    #[test]
    fn parse_package_artifact_rejects_package_qualified_module_surface_rows() {
        let mut artifact = base_surface_validation_artifact();
        artifact.modules[0].exported_surface_rows =
            vec!["module=tool:export:fn:fn main() -> Int:".to_string()];

        let err = parse_package_artifact(&render_package_artifact(&artifact))
            .expect_err("artifact should reject package-qualified module surface rows");
        assert!(
            err.contains("must not use a package `module=` prefix"),
            "{err}"
        );
    }

    #[test]
    fn parse_package_artifact_rejects_unqualified_package_surface_rows() {
        let mut artifact = base_surface_validation_artifact();
        artifact.exported_surface_rows = vec!["export:fn:fn main() -> Int:".to_string()];

        let err = parse_package_artifact(&render_package_artifact(&artifact))
            .expect_err("artifact should reject unqualified package surface rows");
        assert!(
            err.contains("malformed package exported surface row"),
            "{err}"
        );
    }

    #[test]
    fn parse_package_artifact_rejects_package_surface_rows_for_undeclared_modules() {
        let mut artifact = base_surface_validation_artifact();
        artifact.exported_surface_rows =
            vec!["module=ghost:export:fn:fn main() -> Int:".to_string()];

        let err = parse_package_artifact(&render_package_artifact(&artifact))
            .expect_err("artifact should reject package surface rows for undeclared modules");
        assert!(
            err.contains("references undeclared module `ghost`"),
            "{err}"
        );
    }

    #[test]
    fn parse_package_artifact_rejects_invalid_surface_text_escape_sequences() {
        let mut artifact = base_surface_validation_artifact();
        artifact.exported_surface_rows =
            vec!["module=tool:export:fn:fn main() -> Int:\\".to_string()];

        let err = parse_package_artifact(&render_package_artifact(&artifact))
            .expect_err("artifact should reject malformed surface text escapes");
        assert!(err.contains("unterminated escape"), "{err}");
    }

    #[test]
    fn parse_package_artifact_allows_unknown_surface_text_escape_sequences() {
        let mut artifact = base_surface_validation_artifact();
        artifact.exported_surface_rows =
            vec!["module=tool:export:fn:fn main() -> Int:\\q".to_string()];
        artifact.modules[0].exported_surface_rows =
            vec!["export:fn:fn main() -> Int:\\q".to_string()];

        let parsed = parse_package_artifact(&render_package_artifact(&artifact))
            .expect("artifact should allow unknown surface text escapes");
        assert_eq!(
            parsed.exported_surface_rows,
            vec!["module=tool:export:fn:fn main() -> Int:\\q".to_string()]
        );
    }

    #[test]
    fn collect_native_exports_rejects_stale_declared_export_rows() {
        let mut artifact = base_surface_validation_artifact();
        artifact.routines = vec![AotRoutineArtifact {
            package_id: test_package_id_for_module("tool"),
            module_id: "tool".to_string(),
            routine_key: "tool#fn-0".to_string(),
            symbol_name: "answer".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn answer() -> Int:"),
            intrinsic_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            foreword_rows: Vec::new(),
            rollups: Vec::new(),
            statements: Vec::new(),
        }];
        artifact.exported_surface_rows =
            vec!["module=tool:export:fn:fn stale() -> Int:".to_string()];
        artifact.modules[0].exported_surface_rows =
            vec!["export:fn:fn stale() -> Int:".to_string()];

        let err = crate::native_abi::collect_native_exports(&artifact)
            .expect_err("native exports should reject stale declared rows");
        assert!(
            err.contains("native export rows do not match structured routines"),
            "{err}"
        );
    }

    #[test]
    fn collect_native_exports_allows_non_native_package_surface_rows() {
        let mut artifact = base_surface_validation_artifact();
        artifact.exported_surface_rows = vec![
            "module=tool:export:fn:fn main() -> Int:".to_string(),
            "module=tool:export:fn:async fn worker() -> Int:".to_string(),
            "module=tool:export:fn:fn len[T](read values: Array[T]) -> Int:".to_string(),
        ];

        let exports = crate::native_abi::collect_native_exports(&artifact)
            .expect("native exports should ignore async and generic package surface rows");
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].routine_key, "tool#fn-0");
    }
}
