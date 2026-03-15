use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{
    ExecAssignTarget, ExecExpr, ExecHeaderAttachment, ExecPageRollup, ExecPhraseArg, ExecStmt,
    IrEntrypoint, IrPackage, IrRoutine,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeRequirementRoots {
    Entrypoints,
    ExportedRootPackageRoutines,
}

pub fn derive_runtime_requirements(package: &IrPackage) -> Vec<String> {
    derive_runtime_requirements_with_roots(package, RuntimeRequirementRoots::Entrypoints)
}

pub fn derive_runtime_requirements_with_roots(
    package: &IrPackage,
    roots: RuntimeRequirementRoots,
) -> Vec<String> {
    let reachable = reachable_routine_keys(package, roots);
    let routines_by_key = package
        .routines
        .iter()
        .map(|routine| (routine.routine_key.as_str(), routine))
        .collect::<BTreeMap<_, _>>();

    let mut requirements = BTreeSet::new();
    for routine_key in reachable {
        let Some(routine) = routines_by_key.get(routine_key.as_str()) else {
            continue;
        };
        let Some(intrinsic_impl) = &routine.intrinsic_impl else {
            continue;
        };
        if let Some(requirement) = runtime_requirement_for_intrinsic_impl(intrinsic_impl) {
            requirements.insert(requirement.to_string());
        }
    }

    requirements.into_iter().collect()
}

fn reachable_routine_keys(package: &IrPackage, roots: RuntimeRequirementRoots) -> BTreeSet<String> {
    let mut reachable = BTreeSet::new();
    let mut pending = root_routine_keys(package, roots)
        .into_iter()
        .collect::<VecDeque<_>>();
    let routines_by_key = package
        .routines
        .iter()
        .map(|routine| (routine.routine_key.clone(), routine))
        .collect::<BTreeMap<_, _>>();

    while let Some(routine_key) = pending.pop_front() {
        if !reachable.insert(routine_key.clone()) {
            continue;
        }
        let Some(routine) = routines_by_key.get(&routine_key) else {
            continue;
        };
        let mut callees = BTreeSet::new();
        collect_routine_callees(package, routine, &mut callees);
        for callee in callees {
            if !reachable.contains(&callee) {
                pending.push_back(callee);
            }
        }
    }

    reachable
}

fn root_routine_keys(package: &IrPackage, roots: RuntimeRequirementRoots) -> Vec<String> {
    match roots {
        RuntimeRequirementRoots::Entrypoints => entrypoint_routine_keys(package),
        RuntimeRequirementRoots::ExportedRootPackageRoutines => {
            exported_root_package_routine_keys(package)
        }
    }
}

fn entrypoint_routine_keys(package: &IrPackage) -> Vec<String> {
    package
        .entrypoints
        .iter()
        .filter_map(|entrypoint| find_entrypoint_routine_key(package, entrypoint))
        .collect()
}

fn routine_belongs_to_root_package(package: &IrPackage, routine: &IrRoutine) -> bool {
    routine.module_id == package.root_module_id
        || routine
            .module_id
            .starts_with(&(package.root_module_id.clone() + "."))
}

fn exported_root_package_routine_keys(package: &IrPackage) -> Vec<String> {
    package
        .routines
        .iter()
        .filter(|routine| routine.exported && routine_belongs_to_root_package(package, routine))
        .map(|routine| routine.routine_key.clone())
        .collect()
}

fn find_entrypoint_routine_key(package: &IrPackage, entrypoint: &IrEntrypoint) -> Option<String> {
    package
        .routines
        .iter()
        .find(|routine| {
            routine.module_id == entrypoint.module_id
                && routine.symbol_name == entrypoint.symbol_name
        })
        .map(|routine| routine.routine_key.clone())
}

fn collect_routine_callees(package: &IrPackage, routine: &IrRoutine, out: &mut BTreeSet<String>) {
    for rollup in &routine.rollups {
        collect_rollup_callee(package, &routine.module_id, rollup, out);
    }
    for statement in &routine.statements {
        collect_stmt_callees(package, &routine.module_id, statement, out);
    }
}

fn collect_rollup_callee(
    package: &IrPackage,
    current_module_id: &str,
    rollup: &ExecPageRollup,
    out: &mut BTreeSet<String>,
) {
    if let Some(routine_key) = resolve_routine_key(package, current_module_id, &rollup.handler_path)
    {
        out.insert(routine_key);
    }
}

fn collect_stmt_callees(
    package: &IrPackage,
    current_module_id: &str,
    statement: &ExecStmt,
    out: &mut BTreeSet<String>,
) {
    match statement {
        ExecStmt::Let { value, .. } => collect_expr_callees(package, current_module_id, value, out),
        ExecStmt::Expr { expr, rollups } => {
            collect_expr_callees(package, current_module_id, expr, out);
            for rollup in rollups {
                collect_rollup_callee(package, current_module_id, rollup, out);
            }
        }
        ExecStmt::ReturnVoid | ExecStmt::Break | ExecStmt::Continue => {}
        ExecStmt::ReturnValue { value } => {
            collect_expr_callees(package, current_module_id, value, out);
        }
        ExecStmt::If {
            condition,
            then_branch,
            else_branch,
            rollups,
        } => {
            collect_expr_callees(package, current_module_id, condition, out);
            for rollup in rollups {
                collect_rollup_callee(package, current_module_id, rollup, out);
            }
            for statement in then_branch {
                collect_stmt_callees(package, current_module_id, statement, out);
            }
            for statement in else_branch {
                collect_stmt_callees(package, current_module_id, statement, out);
            }
        }
        ExecStmt::While {
            condition,
            body,
            rollups,
        } => {
            collect_expr_callees(package, current_module_id, condition, out);
            for rollup in rollups {
                collect_rollup_callee(package, current_module_id, rollup, out);
            }
            for statement in body {
                collect_stmt_callees(package, current_module_id, statement, out);
            }
        }
        ExecStmt::For {
            iterable,
            body,
            rollups,
            ..
        } => {
            collect_expr_callees(package, current_module_id, iterable, out);
            for rollup in rollups {
                collect_rollup_callee(package, current_module_id, rollup, out);
            }
            for statement in body {
                collect_stmt_callees(package, current_module_id, statement, out);
            }
        }
        ExecStmt::Defer(expr) => collect_expr_callees(package, current_module_id, expr, out),
        ExecStmt::Assign { target, value, .. } => {
            collect_assign_target_callees(package, current_module_id, target, out);
            collect_expr_callees(package, current_module_id, value, out);
        }
    }
}

fn collect_assign_target_callees(
    package: &IrPackage,
    current_module_id: &str,
    target: &ExecAssignTarget,
    out: &mut BTreeSet<String>,
) {
    match target {
        ExecAssignTarget::Name(_) => {}
        ExecAssignTarget::Member { target, .. } => {
            collect_assign_target_callees(package, current_module_id, target, out);
        }
        ExecAssignTarget::Index { target, index } => {
            collect_assign_target_callees(package, current_module_id, target, out);
            collect_expr_callees(package, current_module_id, index, out);
        }
    }
}

fn collect_expr_callees(
    package: &IrPackage,
    current_module_id: &str,
    expr: &ExecExpr,
    out: &mut BTreeSet<String>,
) {
    match expr {
        ExecExpr::Int(_) | ExecExpr::Bool(_) | ExecExpr::Str(_) | ExecExpr::Path(_) => {}
        ExecExpr::Pair { left, right } => {
            collect_expr_callees(package, current_module_id, left, out);
            collect_expr_callees(package, current_module_id, right, out);
        }
        ExecExpr::Collection { items } => {
            for item in items {
                collect_expr_callees(package, current_module_id, item, out);
            }
        }
        ExecExpr::Match { subject, arms } => {
            collect_expr_callees(package, current_module_id, subject, out);
            for arm in arms {
                collect_expr_callees(package, current_module_id, &arm.value, out);
            }
        }
        ExecExpr::Chain { steps, .. } => {
            for step in steps {
                collect_expr_callees(package, current_module_id, &step.stage, out);
                for arg in &step.bind_args {
                    collect_expr_callees(package, current_module_id, arg, out);
                }
            }
        }
        ExecExpr::MemoryPhrase {
            arena,
            init_args,
            constructor,
            attached,
            ..
        } => {
            collect_expr_callees(package, current_module_id, arena, out);
            for arg in init_args {
                collect_phrase_arg_callees(package, current_module_id, arg, out);
            }
            collect_expr_callees(package, current_module_id, constructor, out);
            for attachment in attached {
                collect_attachment_callees(package, current_module_id, attachment, out);
            }
        }
        ExecExpr::Member { expr, .. } => {
            collect_expr_callees(package, current_module_id, expr, out)
        }
        ExecExpr::Index { expr, index } => {
            collect_expr_callees(package, current_module_id, expr, out);
            collect_expr_callees(package, current_module_id, index, out);
        }
        ExecExpr::Slice {
            expr, start, end, ..
        } => {
            collect_expr_callees(package, current_module_id, expr, out);
            if let Some(start) = start {
                collect_expr_callees(package, current_module_id, start, out);
            }
            if let Some(end) = end {
                collect_expr_callees(package, current_module_id, end, out);
            }
        }
        ExecExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                collect_expr_callees(package, current_module_id, start, out);
            }
            if let Some(end) = end {
                collect_expr_callees(package, current_module_id, end, out);
            }
        }
        ExecExpr::Generic { expr, .. } | ExecExpr::Await { expr } => {
            collect_expr_callees(package, current_module_id, expr, out);
        }
        ExecExpr::Phrase {
            subject,
            args,
            resolved_routine,
            attached,
            ..
        } => {
            collect_expr_callees(package, current_module_id, subject, out);
            for arg in args {
                collect_phrase_arg_callees(package, current_module_id, arg, out);
            }
            for attachment in attached {
                collect_attachment_callees(package, current_module_id, attachment, out);
            }
            if let Some(routine_key) = resolved_routine {
                out.insert(routine_key.clone());
            }
        }
        ExecExpr::Unary { expr, .. } => collect_expr_callees(package, current_module_id, expr, out),
        ExecExpr::Binary { left, right, .. } => {
            collect_expr_callees(package, current_module_id, left, out);
            collect_expr_callees(package, current_module_id, right, out);
        }
    }
}

fn collect_phrase_arg_callees(
    package: &IrPackage,
    current_module_id: &str,
    arg: &ExecPhraseArg,
    out: &mut BTreeSet<String>,
) {
    collect_expr_callees(package, current_module_id, &arg.value, out);
}

fn collect_attachment_callees(
    package: &IrPackage,
    current_module_id: &str,
    attachment: &ExecHeaderAttachment,
    out: &mut BTreeSet<String>,
) {
    match attachment {
        ExecHeaderAttachment::Named { value, .. } => {
            collect_expr_callees(package, current_module_id, value, out);
        }
        ExecHeaderAttachment::Chain { expr } => {
            collect_expr_callees(package, current_module_id, expr, out);
        }
    }
}

fn resolve_routine_key(
    package: &IrPackage,
    current_module_id: &str,
    callable_path: &[String],
) -> Option<String> {
    let (module_id, symbol_name) = match callable_path {
        [] => return None,
        [symbol_name] => (current_module_id.to_string(), symbol_name.clone()),
        _ => (
            callable_path[..callable_path.len() - 1].join("."),
            callable_path.last()?.clone(),
        ),
    };
    package
        .routines
        .iter()
        .find(|routine| routine.module_id == module_id && routine.symbol_name == symbol_name)
        .or_else(|| {
            let prefixed_module = if module_id == package.root_module_id
                || module_id.starts_with(&(package.root_module_id.clone() + "."))
            {
                module_id.clone()
            } else {
                format!("{}.{}", package.root_module_id, module_id)
            };
            package.routines.iter().find(|routine| {
                routine.module_id == prefixed_module && routine.symbol_name == symbol_name
            })
        })
        .map(|routine| routine.routine_key.clone())
}

fn runtime_requirement_for_intrinsic_impl(intrinsic_impl: &str) -> Option<&'static str> {
    let requirement = match intrinsic_impl {
        "IoPrint" | "IoEprint" | "IoFlushStdout" | "IoFlushStderr" | "IoStdinReadLineTry" => {
            "std.kernel.io"
        }
        name if name.starts_with("HostArg") => "std.kernel.args",
        name if name.starts_with("HostEnv") => "std.kernel.env",
        name if name.starts_with("HostPath") => "std.kernel.path",
        name if name.starts_with("HostFs") => "std.kernel.fs",
        name if name.starts_with("Window")
            || name.starts_with("Canvas")
            || name.starts_with("Image")
            || name.starts_with("Input") =>
        {
            "std.kernel.gfx"
        }
        name if name.starts_with("Events") => "std.kernel.events",
        name if name.starts_with("HostTime") => "std.kernel.time",
        name if name.starts_with("Concurrent") => "std.kernel.concurrency",
        name if name.starts_with("Memory") => "std.kernel.memory",
        name if name.starts_with("Audio") => "std.kernel.audio",
        name if name.starts_with("HostProcess") => "std.kernel.process",
        name if name.starts_with("HostText") || name.starts_with("HostBytes") => "std.kernel.text",
        "ListNew" | "ListLen" | "ListPush" | "ListPop" | "ListTryPopOr" | "ArrayNew"
        | "ArrayLen" | "ArrayFromList" | "ArrayToList" | "MapNew" | "MapLen" | "MapHas"
        | "MapGet" | "MapSet" | "MapRemove" | "MapTryGetOr" => "std.kernel.collections",
        name if name.starts_with("Ecs") => "std.kernel.ecs",
        _ => return None,
    };
    Some(requirement)
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeRequirementRoots, derive_runtime_requirements,
        derive_runtime_requirements_with_roots,
    };
    use crate::{
        ExecExpr, ExecPageRollup, ExecPhraseQualifierKind, ExecStmt, IrEntrypoint, IrPackage,
        IrPackageModule, IrRoutine,
    };

    fn routine(
        module_id: &str,
        routine_key: &str,
        symbol_name: &str,
        intrinsic_impl: Option<&str>,
        statements: Vec<ExecStmt>,
    ) -> IrRoutine {
        IrRoutine {
            module_id: module_id.to_string(),
            routine_key: routine_key.to_string(),
            symbol_name: symbol_name.to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_param_rows: Vec::new(),
            behavior_attr_rows: Vec::new(),
            param_rows: Vec::new(),
            signature_row: format!("fn {symbol_name}() -> Int:"),
            intrinsic_impl: intrinsic_impl.map(ToString::to_string),
            impl_target_type: None,
            impl_trait_path: None,
            foreword_rows: Vec::new(),
            rollups: Vec::new(),
            statements,
        }
    }

    fn call(routine_key: &str, callable: &[&str]) -> ExecStmt {
        ExecStmt::ReturnValue {
            value: ExecExpr::Phrase {
                subject: Box::new(ExecExpr::Path(
                    callable.iter().map(|segment| segment.to_string()).collect(),
                )),
                args: Vec::new(),
                qualifier_kind: ExecPhraseQualifierKind::Call,
                qualifier: "call".to_string(),
                resolved_callable: Some(
                    callable.iter().map(|segment| segment.to_string()).collect(),
                ),
                resolved_routine: Some(routine_key.to_string()),
                dynamic_dispatch: None,
                attached: Vec::new(),
            },
        }
    }

    #[test]
    fn derives_canonical_requirements_from_reachable_intrinsics() {
        let package = IrPackage {
            package_name: "app".to_string(),
            root_module_id: "app".to_string(),
            direct_deps: vec!["std".to_string()],
            modules: vec![IrPackageModule {
                module_id: "app".to_string(),
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
                module_id: "app".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![
                routine(
                    "app",
                    "app#sym-0",
                    "main",
                    None,
                    vec![call("std.text#sym-0", &["std", "text", "len_bytes"])],
                ),
                routine(
                    "std.text",
                    "std.text#sym-0",
                    "len_bytes",
                    Some("HostTextLenBytes"),
                    Vec::new(),
                ),
                routine(
                    "std.audio",
                    "std.audio#sym-0",
                    "default_output",
                    Some("AudioDefaultOutputTry"),
                    Vec::new(),
                ),
            ],
        };

        assert_eq!(
            derive_runtime_requirements(&package),
            vec!["std.kernel.text".to_string()]
        );
    }

    #[test]
    fn derives_transitive_dependency_requirements_without_unrelated_std_union() {
        let package = IrPackage {
            package_name: "app".to_string(),
            root_module_id: "app".to_string(),
            direct_deps: vec!["core".to_string(), "std".to_string()],
            modules: vec![IrPackageModule {
                module_id: "app".to_string(),
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
                module_id: "app".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![
                routine(
                    "app",
                    "app#sym-0",
                    "main",
                    None,
                    vec![call("core#sym-0", &["core", "read_file"])],
                ),
                routine(
                    "core",
                    "core#sym-0",
                    "read_file",
                    None,
                    vec![call("std.fs#sym-0", &["std", "fs", "read_text"])],
                ),
                routine(
                    "std.fs",
                    "std.fs#sym-0",
                    "read_text",
                    Some("HostFsReadTextTry"),
                    Vec::new(),
                ),
                routine(
                    "std.window",
                    "std.window#sym-0",
                    "open",
                    Some("WindowOpenTry"),
                    Vec::new(),
                ),
            ],
        };

        assert_eq!(
            derive_runtime_requirements(&package),
            vec!["std.kernel.fs".to_string()]
        );
    }

    #[test]
    fn derives_exported_library_requirements_without_entrypoints() {
        let package = IrPackage {
            package_name: "core".to_string(),
            root_module_id: "core".to_string(),
            direct_deps: vec!["std".to_string()],
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
            routines: vec![
                routine(
                    "core",
                    "core#sym-0",
                    "announce",
                    None,
                    vec![call("std.io#sym-0", &["std", "io", "print"])],
                ),
                routine(
                    "std.io",
                    "std.io#sym-0",
                    "print",
                    Some("IoPrint"),
                    Vec::new(),
                ),
            ],
        };

        assert_eq!(
            derive_runtime_requirements_with_roots(
                &package,
                RuntimeRequirementRoots::ExportedRootPackageRoutines
            ),
            vec!["std.kernel.io".to_string()]
        );
    }

    #[test]
    fn exported_library_roots_ignore_unrelated_dependency_exports() {
        let package = IrPackage {
            package_name: "core".to_string(),
            root_module_id: "core".to_string(),
            direct_deps: vec!["std".to_string()],
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
            routines: vec![
                routine(
                    "core",
                    "core#sym-0",
                    "announce",
                    None,
                    vec![call("std.io#sym-0", &["std", "io", "print"])],
                ),
                routine(
                    "std.io",
                    "std.io#sym-0",
                    "print",
                    Some("IoPrint"),
                    Vec::new(),
                ),
                routine(
                    "std.audio",
                    "std.audio#sym-0",
                    "default_output",
                    Some("AudioDefaultOutputTry"),
                    Vec::new(),
                ),
            ],
        };

        assert_eq!(
            derive_runtime_requirements_with_roots(
                &package,
                RuntimeRequirementRoots::ExportedRootPackageRoutines
            ),
            vec!["std.kernel.io".to_string()]
        );
    }

    #[test]
    fn rollup_handlers_do_not_use_global_unique_name_fallback() {
        let package = IrPackage {
            package_name: "app".to_string(),
            root_module_id: "app".to_string(),
            direct_deps: vec!["std".to_string()],
            modules: vec![
                IrPackageModule {
                    module_id: "app".to_string(),
                    symbol_count: 1,
                    item_count: 1,
                    line_count: 1,
                    non_empty_line_count: 1,
                    directive_rows: Vec::new(),
                    lang_item_rows: Vec::new(),
                    exported_surface_rows: Vec::new(),
                },
                IrPackageModule {
                    module_id: "helpers".to_string(),
                    symbol_count: 1,
                    item_count: 1,
                    line_count: 1,
                    non_empty_line_count: 1,
                    directive_rows: Vec::new(),
                    lang_item_rows: Vec::new(),
                    exported_surface_rows: Vec::new(),
                },
            ],
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: Vec::new(),
            runtime_requirements: Vec::new(),
            entrypoints: vec![IrEntrypoint {
                module_id: "app".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![
                IrRoutine {
                    module_id: "app".to_string(),
                    routine_key: "app#fn-0".to_string(),
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
                    foreword_rows: Vec::new(),
                    rollups: vec![ExecPageRollup {
                        kind: "cleanup".to_string(),
                        subject: "scope".to_string(),
                        handler_path: vec!["cleanup".to_string()],
                    }],
                    statements: vec![ExecStmt::ReturnValue {
                        value: ExecExpr::Int(0),
                    }],
                },
                IrRoutine {
                    module_id: "helpers".to_string(),
                    routine_key: "helpers#fn-0".to_string(),
                    symbol_name: "cleanup".to_string(),
                    symbol_kind: "fn".to_string(),
                    exported: false,
                    is_async: false,
                    type_param_rows: Vec::new(),
                    behavior_attr_rows: Vec::new(),
                    param_rows: vec!["mode=:name=scope:ty=Int".to_string()],
                    signature_row: "fn cleanup(scope: Int) -> Int:".to_string(),
                    intrinsic_impl: Some("IoPrint".to_string()),
                    impl_target_type: None,
                    impl_trait_path: None,
                    foreword_rows: Vec::new(),
                    rollups: Vec::new(),
                    statements: Vec::new(),
                },
            ],
        };

        assert!(derive_runtime_requirements(&package).is_empty());
    }
}
