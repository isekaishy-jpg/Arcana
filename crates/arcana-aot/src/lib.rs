mod artifact;
mod codec;
mod compile;
mod emit;
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
    AotRoutineArtifact,
};
pub use codec::{parse_package_artifact, render_package_artifact};
pub use compile::{compile_module, compile_package};
pub use emit::{
    AOT_WINDOWS_DLL_FORMAT, AOT_WINDOWS_EXE_FORMAT, AotEmissionFile, AotEmitContext, AotEmitTarget,
    AotPackageEmission, emit_package, emit_package_with_context,
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
        AotRoutineArtifact, NATIVE_BUNDLE_MANIFEST_FORMAT, NativeLaunchPlan,
        build_native_package_plan, compile_module, compile_package, emit_package,
        emit_package_with_context, parse_native_bundle_manifest, parse_package_artifact,
        render_native_bundle_manifest, render_package_artifact, validate_package_artifact,
    };
    use arcana_ir::{
        ExecExpr, ExecPageRollup, ExecPhraseQualifierKind, ExecStmt, IrEntrypoint, IrModule,
        IrPackage, IrPackageModule, IrRoutine,
    };

    fn base_surface_validation_artifact() -> AotPackageArtifact {
        AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT.to_string(),
            package_name: "tool".to_string(),
            root_module_id: "tool".to_string(),
            direct_deps: Vec::new(),
            module_count: 1,
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: vec!["module=tool:export:fn:fn main() -> Int:".to_string()],
            runtime_requirements: Vec::new(),
            entrypoints: Vec::new(),
            routines: Vec::new(),
            owners: Vec::new(),
            modules: vec![AotPackageModuleArtifact {
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
            package_name: "winspell".to_string(),
            root_module_id: "winspell".to_string(),
            direct_deps: vec!["std".to_string()],
            modules: vec![
                IrPackageModule {
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
                module_id: "winspell".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![IrRoutine {
                module_id: "winspell".to_string(),
                routine_key: "winspell#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn main() -> Int:".to_string(),
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
        let package = IrPackage {
            package_name: "tool".to_string(),
            root_module_id: "tool".to_string(),
            direct_deps: Vec::new(),
            modules: vec![IrPackageModule {
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
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn main() -> Int:".to_string(),
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
            package_name: "tool".to_string(),
            root_module_id: "tool".to_string(),
            direct_deps: Vec::new(),
            modules: vec![IrPackageModule {
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
            entrypoints: vec![IrEntrypoint {
                module_id: "tool".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![IrRoutine {
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn main() -> Int:".to_string(),
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
            &AotEmitContext {
                root_artifact_file_name: Some("app.exe".to_string()),
            },
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
            vec!["app.exe.arcana-native.toml"]
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
            &AotEmitContext {
                root_artifact_file_name: Some("lib.dll".to_string()),
            },
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
            vec!["lib.dll.h", "lib.dll.def", "lib.dll.arcana-native.toml"]
        );
        let dll_text =
            std::str::from_utf8(&dll.support_files[0].bytes).expect("dll header should be utf8");
        assert!(dll_text.contains("arcana_last_error_alloc"));
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
        let package = IrPackage {
            package_name: "tool".to_string(),
            root_module_id: "tool".to_string(),
            direct_deps: Vec::new(),
            modules: vec![IrPackageModule {
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
                module_id: "tool".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![IrRoutine {
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn main() -> Int:".to_string(),
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

        let plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &AotEmitContext {
                root_artifact_file_name: Some("app.exe".to_string()),
            },
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
            package_name: "tool".to_string(),
            root_module_id: "tool".to_string(),
            direct_deps: Vec::new(),
            modules: vec![IrPackageModule {
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
            &AotEmitContext {
                root_artifact_file_name: Some("app.exe".to_string()),
            },
        )
        .expect_err("native plan should reject missing main");
        assert!(
            err.contains("requires exactly one main entrypoint"),
            "{err}"
        );
    }

    #[test]
    fn native_bundle_manifest_roundtrips_windows_dll_export_contract() {
        let package = IrPackage {
            package_name: "core".to_string(),
            root_module_id: "core".to_string(),
            direct_deps: Vec::new(),
            modules: vec![IrPackageModule {
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
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "answer".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: vec!["mode=:name=value:ty=Int".to_string()],
                signature_row: "fn answer(value: Int) -> Bool:".to_string(),
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

        let plan = build_native_package_plan(
            AotEmitTarget::WindowsDllBundle,
            &package,
            &AotEmitContext {
                root_artifact_file_name: Some("lib.dll".to_string()),
            },
        )
        .expect("native plan should build");
        let manifest_text =
            render_native_bundle_manifest(&plan).expect("native manifest should render");
        let manifest =
            parse_native_bundle_manifest(&manifest_text).expect("native manifest should parse");

        assert_eq!(manifest.format, NATIVE_BUNDLE_MANIFEST_FORMAT);
        assert_eq!(manifest.target, "windows-dll");
        assert_eq!(manifest.root_artifact, "lib.dll");
        assert_eq!(manifest.launch.kind, "dynamic-library");
        assert_eq!(manifest.launch.header.as_deref(), Some("lib.dll.h"));
        assert_eq!(
            manifest.launch.definition_file.as_deref(),
            Some("lib.dll.def")
        );
        assert_eq!(manifest.launch.exports.len(), 1);
        assert_eq!(manifest.launch.exports[0].export_name, "answer");
        assert_eq!(manifest.launch.exports[0].routine_key, "core#fn-0");
        assert_eq!(manifest.launch.exports[0].return_type, "Bool");
        assert_eq!(manifest.launch.exports[0].params.len(), 1);
        assert_eq!(manifest.launch.exports[0].params[0].name, "value");
        assert_eq!(manifest.launch.exports[0].params[0].ty, "Int");
    }

    #[test]
    fn native_bundle_manifest_preserves_string_and_byte_exports() {
        let package = IrPackage {
            package_name: "core".to_string(),
            root_module_id: "core".to_string(),
            direct_deps: Vec::new(),
            modules: vec![IrPackageModule {
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
                    module_id: "core".to_string(),
                    routine_key: "core#fn-0".to_string(),
                    symbol_name: "greet".to_string(),
                    symbol_kind: "fn".to_string(),
                    exported: true,
                    is_async: false,
                    type_param_rows: Vec::new(),
                    behavior_attr_rows: Vec::new(),
                    param_rows: vec!["mode=read:name=name:ty=Str".to_string()],
                    signature_row: "fn greet(read name: Str) -> Str:".to_string(),
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
                    module_id: "core".to_string(),
                    routine_key: "core#fn-1".to_string(),
                    symbol_name: "prefix".to_string(),
                    symbol_kind: "fn".to_string(),
                    exported: true,
                    is_async: false,
                    type_param_rows: Vec::new(),
                    behavior_attr_rows: Vec::new(),
                    param_rows: vec!["mode=read:name=bytes:ty=Array[Int]".to_string()],
                    signature_row: "fn prefix(read bytes: Array[Int]) -> Array[Int]:".to_string(),
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

        let plan = build_native_package_plan(
            AotEmitTarget::WindowsDllBundle,
            &package,
            &AotEmitContext {
                root_artifact_file_name: Some("lib.dll".to_string()),
            },
        )
        .expect("native plan should build");
        let manifest_text =
            render_native_bundle_manifest(&plan).expect("native manifest should render");
        let manifest =
            parse_native_bundle_manifest(&manifest_text).expect("native manifest should parse");

        assert_eq!(manifest.launch.exports.len(), 2);
        assert_eq!(manifest.launch.exports[0].export_name, "greet");
        assert_eq!(manifest.launch.exports[0].params[0].ty, "Str");
        assert_eq!(manifest.launch.exports[0].return_type, "Str");
        assert_eq!(manifest.launch.exports[1].export_name, "prefix");
        assert_eq!(manifest.launch.exports[1].params[0].ty, "Array[Int]");
        assert_eq!(manifest.launch.exports[1].return_type, "Array[Int]");
    }

    #[test]
    fn native_bundle_manifest_preserves_pair_exports() {
        let package = IrPackage {
            package_name: "core".to_string(),
            root_module_id: "core".to_string(),
            direct_deps: Vec::new(),
            modules: vec![IrPackageModule {
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
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "echo_pair".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: vec!["mode=read:name=pair:ty=Pair[Str, Int]".to_string()],
                signature_row: "fn echo_pair(read pair: Pair[Str, Int]) -> Pair[Str, Int]:"
                    .to_string(),
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

        let plan = build_native_package_plan(
            AotEmitTarget::WindowsDllBundle,
            &package,
            &AotEmitContext {
                root_artifact_file_name: Some("lib.dll".to_string()),
            },
        )
        .expect("native plan should build");
        let manifest_text =
            render_native_bundle_manifest(&plan).expect("native manifest should render");
        let manifest =
            parse_native_bundle_manifest(&manifest_text).expect("native manifest should parse");

        assert_eq!(manifest.launch.exports.len(), 1);
        assert_eq!(manifest.launch.exports[0].export_name, "echo_pair");
        assert_eq!(manifest.launch.exports[0].params[0].ty, "Pair[Str, Int]");
        assert_eq!(manifest.launch.exports[0].return_type, "Pair[Str, Int]");
    }

    #[test]
    fn package_artifact_roundtrips() {
        let artifact = AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT.to_string(),
            package_name: "tool".to_string(),
            root_module_id: "tool".to_string(),
            direct_deps: vec!["std".to_string()],
            module_count: 1,
            dependency_edge_count: 1,
            dependency_rows: vec!["source=tool:import:std.io:".to_string()],
            exported_surface_rows: vec!["module=tool:export:fn:fn main() -> Int:".to_string()],
            runtime_requirements: vec!["std.io".to_string()],
            entrypoints: vec![AotEntrypointArtifact {
                module_id: "tool".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![AotRoutineArtifact {
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: vec!["mode=:name=x:ty=Int".to_string()],
                signature_row: "fn main(x: Int) -> Int:".to_string(),
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
                module_id: "tool".to_string(),
                symbol_count: 1,
                item_count: 2,
                line_count: 3,
                non_empty_line_count: 2,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: vec!["export:fn:fn main() -> Int:".to_string()],
            }],
        };
        let rendered = render_package_artifact(&artifact);
        let parsed = parse_package_artifact(&rendered).expect("artifact should roundtrip");
        assert_eq!(parsed, artifact);
    }

    #[test]
    fn parse_package_artifact_rejects_mismatched_module_count() {
        let mut artifact = compile_package(&IrPackage {
            package_name: "tool".to_string(),
            root_module_id: "tool".to_string(),
            direct_deps: Vec::new(),
            modules: vec![IrPackageModule {
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
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn helper() -> Int:".to_string(),
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
            package_name: "tool".to_string(),
            root_module_id: "tool".to_string(),
            direct_deps: Vec::new(),
            module_count: 1,
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: Vec::new(),
            runtime_requirements: Vec::new(),
            entrypoints: vec![AotEntrypointArtifact {
                module_id: "tool".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![
                AotRoutineArtifact {
                    module_id: "tool".to_string(),
                    routine_key: "tool#fn-0".to_string(),
                    symbol_name: "main".to_string(),
                    symbol_kind: "fn".to_string(),
                    exported: true,
                    is_async: false,
                    type_param_rows: Vec::new(),
                    behavior_attr_rows: Vec::new(),
                    param_rows: Vec::new(),
                    signature_row: "fn main() -> Int:".to_string(),
                    intrinsic_impl: None,
                    impl_target_type: None,
                    impl_trait_path: None,
                    availability: Vec::new(),
                    foreword_rows: Vec::new(),
                    rollups: Vec::new(),
                    statements: Vec::new(),
                },
                AotRoutineArtifact {
                    module_id: "tool".to_string(),
                    routine_key: "tool#fn-1".to_string(),
                    symbol_name: "main".to_string(),
                    symbol_kind: "fn".to_string(),
                    exported: true,
                    is_async: false,
                    type_param_rows: Vec::new(),
                    behavior_attr_rows: Vec::new(),
                    param_rows: vec!["mode=:name=x:ty=Int".to_string()],
                    signature_row: "fn main(x: Int) -> Int:".to_string(),
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
                module_id: "tool".to_string(),
                symbol_count: 1,
                item_count: 1,
                line_count: 1,
                non_empty_line_count: 1,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
        };

        let err = validate_package_artifact(&artifact)
            .expect_err("artifact should reject ambiguous entrypoint routines");
        assert!(err.contains("entrypoint `tool.main` is ambiguous"), "{err}");
    }

    #[test]
    fn parse_package_artifact_rejects_malformed_param_rows() {
        let mut artifact = compile_package(&IrPackage {
            package_name: "tool".to_string(),
            root_module_id: "tool".to_string(),
            direct_deps: Vec::new(),
            modules: vec![IrPackageModule {
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
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: vec!["mode=read:name=value:ty=Int".to_string()],
                signature_row: "fn helper(read value: Int) -> Int:".to_string(),
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
        artifact.routines[0].param_rows = vec!["mode=borrow:name=value:ty=Int".to_string()];

        let err = parse_package_artifact(&render_package_artifact(&artifact))
            .expect_err("artifact should reject malformed param rows");
        assert!(err.contains("param row has invalid mode"), "{err}");
    }

    #[test]
    fn validate_package_artifact_rejects_malformed_module_directive_rows() {
        let artifact = AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT.to_string(),
            package_name: "tool".to_string(),
            root_module_id: "tool".to_string(),
            direct_deps: Vec::new(),
            module_count: 1,
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: Vec::new(),
            runtime_requirements: Vec::new(),
            entrypoints: Vec::new(),
            routines: vec![AotRoutineArtifact {
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn helper() -> Int:".to_string(),
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
}
