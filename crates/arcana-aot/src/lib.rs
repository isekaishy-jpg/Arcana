use std::collections::BTreeMap;

use arcana_ir::{IrEntrypoint, IrModule, IrPackage, IrPackageModule, IrRoutine};

pub const AOT_INTERNAL_FORMAT: &str = "arcana-aot-v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotArtifact {
    pub format: &'static str,
    pub symbol_count: usize,
    pub item_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotPackageModuleArtifact {
    pub module_id: String,
    pub symbol_count: usize,
    pub item_count: usize,
    pub line_count: usize,
    pub non_empty_line_count: usize,
    pub directive_rows: Vec<String>,
    pub lang_item_rows: Vec<String>,
    pub exported_surface_rows: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotEntrypointArtifact {
    pub module_id: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub is_async: bool,
    pub exported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotRoutineArtifact {
    pub module_id: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub exported: bool,
    pub is_async: bool,
    pub type_param_rows: Vec<String>,
    pub behavior_attr_rows: Vec<String>,
    pub param_rows: Vec<String>,
    pub signature_row: String,
    pub intrinsic_impl: Option<String>,
    pub foreword_rows: Vec<String>,
    pub rollup_rows: Vec<String>,
    pub statement_rows: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotPackageArtifact {
    pub format: &'static str,
    pub package_name: String,
    pub root_module_id: String,
    pub direct_deps: Vec<String>,
    pub module_count: usize,
    pub dependency_edge_count: usize,
    pub dependency_rows: Vec<String>,
    pub exported_surface_rows: Vec<String>,
    pub runtime_requirements: Vec<String>,
    pub entrypoints: Vec<AotEntrypointArtifact>,
    pub routines: Vec<AotRoutineArtifact>,
    pub modules: Vec<AotPackageModuleArtifact>,
}

pub fn compile_module(module: &IrModule) -> AotArtifact {
    AotArtifact {
        format: AOT_INTERNAL_FORMAT,
        symbol_count: module.symbol_count,
        item_count: module.item_count,
    }
}

fn compile_module_artifact(module: &IrPackageModule) -> AotPackageModuleArtifact {
    let compiled = compile_module(&IrModule {
        symbol_count: module.symbol_count,
        item_count: module.item_count,
    });
    AotPackageModuleArtifact {
        module_id: module.module_id.clone(),
        symbol_count: compiled.symbol_count,
        item_count: compiled.item_count,
        line_count: module.line_count,
        non_empty_line_count: module.non_empty_line_count,
        directive_rows: module.directive_rows.clone(),
        lang_item_rows: module.lang_item_rows.clone(),
        exported_surface_rows: module.exported_surface_rows.clone(),
    }
}

fn compile_entrypoint(entrypoint: &IrEntrypoint) -> AotEntrypointArtifact {
    AotEntrypointArtifact {
        module_id: entrypoint.module_id.clone(),
        symbol_name: entrypoint.symbol_name.clone(),
        symbol_kind: entrypoint.symbol_kind.clone(),
        is_async: entrypoint.is_async,
        exported: entrypoint.exported,
    }
}

fn compile_routine(routine: &IrRoutine) -> AotRoutineArtifact {
    AotRoutineArtifact {
        module_id: routine.module_id.clone(),
        symbol_name: routine.symbol_name.clone(),
        symbol_kind: routine.symbol_kind.clone(),
        exported: routine.exported,
        is_async: routine.is_async,
        type_param_rows: routine.type_param_rows.clone(),
        behavior_attr_rows: routine.behavior_attr_rows.clone(),
        param_rows: routine.param_rows.clone(),
        signature_row: routine.signature_row.clone(),
        intrinsic_impl: routine.intrinsic_impl.clone(),
        foreword_rows: routine.foreword_rows.clone(),
        rollup_rows: routine.rollup_rows.clone(),
        statement_rows: routine.statement_rows.clone(),
    }
}

pub fn compile_package(package: &IrPackage) -> AotPackageArtifact {
    AotPackageArtifact {
        format: AOT_INTERNAL_FORMAT,
        package_name: package.package_name.clone(),
        root_module_id: package.root_module_id.clone(),
        direct_deps: package.direct_deps.clone(),
        module_count: package.module_count(),
        dependency_edge_count: package.dependency_edge_count,
        dependency_rows: package.dependency_rows.clone(),
        exported_surface_rows: package.exported_surface_rows.clone(),
        runtime_requirements: package.runtime_requirements.clone(),
        entrypoints: package.entrypoints.iter().map(compile_entrypoint).collect(),
        routines: package.routines.iter().map(compile_routine).collect(),
        modules: package
            .modules
            .iter()
            .map(compile_module_artifact)
            .collect(),
    }
}

fn escape_toml(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn format_string_array(items: &[String]) -> String {
    let rendered = items
        .iter()
        .map(|item| format!("\"{}\"", escape_toml(item)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{rendered}]")
}

fn parse_table(text: &str) -> Result<toml::Value, String> {
    text.parse::<toml::Value>()
        .map_err(|err| format!("failed to parse backend artifact: {err}"))
}

fn require_str(table: &toml::Value, key: &str) -> Result<String, String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("backend artifact missing string `{key}`"))
}

fn require_int(table: &toml::Value, key: &str) -> Result<usize, String> {
    table
        .get(key)
        .and_then(toml::Value::as_integer)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| format!("backend artifact missing integer `{key}`"))
}

fn require_string_array(table: &toml::Value, key: &str) -> Result<Vec<String>, String> {
    let values = table
        .get(key)
        .and_then(toml::Value::as_array)
        .ok_or_else(|| format!("backend artifact missing string array `{key}`"))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToString::to_string)
                .ok_or_else(|| format!("backend artifact array `{key}` must contain only strings"))
        })
        .collect()
}

fn split_row<'a>(row: &'a str, expected: usize) -> Result<Vec<&'a str>, String> {
    let parts = row.split(':').collect::<Vec<_>>();
    if parts.len() != expected {
        return Err(format!("malformed backend row `{row}`"));
    }
    Ok(parts)
}

fn split_row_n<'a>(row: &'a str, expected: usize) -> Result<Vec<&'a str>, String> {
    let parts = row.splitn(expected, ':').collect::<Vec<_>>();
    if parts.len() != expected {
        return Err(format!("malformed backend row `{row}`"));
    }
    Ok(parts)
}

fn parse_prefixed_bool(part: &str, prefix: &str) -> Result<bool, String> {
    let value = part
        .strip_prefix(prefix)
        .ok_or_else(|| format!("malformed backend boolean row part `{part}`"))?;
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid backend boolean `{part}`")),
    }
}

fn parse_prefixed_usize(part: &str, prefix: &str) -> Result<usize, String> {
    let value = part
        .strip_prefix(prefix)
        .ok_or_else(|| format!("malformed backend integer row part `{part}`"))?;
    value
        .parse::<usize>()
        .map_err(|err| format!("invalid backend integer `{part}`: {err}"))
}

fn flatten_module_rows<F>(modules: &[AotPackageModuleArtifact], select: F) -> Vec<String>
where
    F: Fn(&AotPackageModuleArtifact) -> &[String],
{
    modules
        .iter()
        .flat_map(|module| {
            select(module)
                .iter()
                .enumerate()
                .map(|(index, row)| format!("{}:{index}:{row}", module.module_id))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn flatten_routine_rows<F>(routines: &[AotRoutineArtifact], select: F) -> Vec<String>
where
    F: Fn(&AotRoutineArtifact) -> &[String],
{
    routines
        .iter()
        .flat_map(|routine| {
            select(routine)
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    format!(
                        "{}:{}:{index}:{row}",
                        routine.module_id, routine.symbol_name
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

pub fn render_package_artifact(artifact: &AotPackageArtifact) -> String {
    let module_rows = artifact
        .modules
        .iter()
        .map(|module| {
            format!(
                "{}:symbols={}:items={}:lines={}:non_empty_lines={}",
                module.module_id,
                module.symbol_count,
                module.item_count,
                module.line_count,
                module.non_empty_line_count
            )
        })
        .collect::<Vec<_>>();
    let module_directive_rows =
        flatten_module_rows(&artifact.modules, |module| &module.directive_rows);
    let module_lang_item_rows =
        flatten_module_rows(&artifact.modules, |module| &module.lang_item_rows);
    let module_surface_rows =
        flatten_module_rows(&artifact.modules, |module| &module.exported_surface_rows);
    let entrypoint_rows = artifact
        .entrypoints
        .iter()
        .map(|entry| {
            format!(
                "{}:{}:{}:async={}:exported={}",
                entry.module_id,
                entry.symbol_kind,
                entry.symbol_name,
                entry.is_async,
                entry.exported
            )
        })
        .collect::<Vec<_>>();
    let routine_rows = artifact
        .routines
        .iter()
        .map(|routine| {
            format!(
                "{}:{}:{}:async={}:exported={}:intrinsic={}:signature={}",
                routine.module_id,
                routine.symbol_kind,
                routine.symbol_name,
                routine.is_async,
                routine.exported,
                routine.intrinsic_impl.as_deref().unwrap_or(""),
                routine.signature_row
            )
        })
        .collect::<Vec<_>>();
    let routine_param_rows =
        flatten_routine_rows(&artifact.routines, |routine| &routine.param_rows);
    let routine_type_param_rows =
        flatten_routine_rows(&artifact.routines, |routine| &routine.type_param_rows);
    let routine_behavior_attr_rows =
        flatten_routine_rows(&artifact.routines, |routine| &routine.behavior_attr_rows);
    let routine_foreword_rows =
        flatten_routine_rows(&artifact.routines, |routine| &routine.foreword_rows);
    let routine_rollup_rows =
        flatten_routine_rows(&artifact.routines, |routine| &routine.rollup_rows);
    let statement_rows =
        flatten_routine_rows(&artifact.routines, |routine| &routine.statement_rows);
    format!(
        concat!(
            "format = \"{}\"\n",
            "package = \"{}\"\n",
            "root_module = \"{}\"\n",
            "module_count = {}\n",
            "dependency_edge_count = {}\n",
            "direct_deps = {}\n",
            "dependency_rows = {}\n",
            "runtime_requirements = {}\n",
            "entrypoint_rows = {}\n",
            "routine_rows = {}\n",
            "routine_type_param_rows = {}\n",
            "routine_behavior_attr_rows = {}\n",
            "routine_param_rows = {}\n",
            "routine_foreword_rows = {}\n",
            "routine_rollup_rows = {}\n",
            "statement_rows = {}\n",
            "surface_rows = {}\n",
            "module_rows = {}\n",
            "module_directive_rows = {}\n",
            "module_lang_item_rows = {}\n",
            "module_surface_rows = {}\n"
        ),
        artifact.format,
        artifact.package_name,
        artifact.root_module_id,
        artifact.module_count,
        artifact.dependency_edge_count,
        format_string_array(&artifact.direct_deps),
        format_string_array(&artifact.dependency_rows),
        format_string_array(&artifact.runtime_requirements),
        format_string_array(&entrypoint_rows),
        format_string_array(&routine_rows),
        format_string_array(&routine_type_param_rows),
        format_string_array(&routine_behavior_attr_rows),
        format_string_array(&routine_param_rows),
        format_string_array(&routine_foreword_rows),
        format_string_array(&routine_rollup_rows),
        format_string_array(&statement_rows),
        format_string_array(&artifact.exported_surface_rows),
        format_string_array(&module_rows),
        format_string_array(&module_directive_rows),
        format_string_array(&module_lang_item_rows),
        format_string_array(&module_surface_rows),
    )
}

pub fn parse_package_artifact(text: &str) -> Result<AotPackageArtifact, String> {
    let table = parse_table(text)?;
    let format = require_str(&table, "format")?;
    if format != AOT_INTERNAL_FORMAT {
        return Err(format!(
            "unsupported backend artifact format `{format}`; expected `{AOT_INTERNAL_FORMAT}`"
        ));
    }

    let module_rows = require_string_array(&table, "module_rows")?;
    let mut modules = Vec::new();
    let mut module_indexes = BTreeMap::new();
    for row in module_rows {
        let parts = split_row(&row, 5)?;
        let module_id = parts[0].to_string();
        let module = AotPackageModuleArtifact {
            module_id: module_id.clone(),
            symbol_count: parse_prefixed_usize(parts[1], "symbols=")?,
            item_count: parse_prefixed_usize(parts[2], "items=")?,
            line_count: parse_prefixed_usize(parts[3], "lines=")?,
            non_empty_line_count: parse_prefixed_usize(parts[4], "non_empty_lines=")?,
            directive_rows: Vec::new(),
            lang_item_rows: Vec::new(),
            exported_surface_rows: Vec::new(),
        };
        module_indexes.insert(module_id, modules.len());
        modules.push(module);
    }

    for row in require_string_array(&table, "module_directive_rows")? {
        let parts = split_row_n(&row, 3)?;
        let module_index = *module_indexes
            .get(parts[0])
            .ok_or_else(|| format!("unknown module `{}` in backend artifact", parts[0]))?;
        modules[module_index]
            .directive_rows
            .push(parts[2].to_string());
    }
    for row in require_string_array(&table, "module_lang_item_rows")? {
        let parts = split_row_n(&row, 3)?;
        let module_index = *module_indexes
            .get(parts[0])
            .ok_or_else(|| format!("unknown module `{}` in backend artifact", parts[0]))?;
        modules[module_index]
            .lang_item_rows
            .push(parts[2].to_string());
    }
    for row in require_string_array(&table, "module_surface_rows")? {
        let parts = split_row_n(&row, 3)?;
        let module_index = *module_indexes
            .get(parts[0])
            .ok_or_else(|| format!("unknown module `{}` in backend artifact", parts[0]))?;
        modules[module_index]
            .exported_surface_rows
            .push(parts[2].to_string());
    }

    let entrypoints = require_string_array(&table, "entrypoint_rows")?
        .into_iter()
        .map(|row| {
            let parts = split_row(&row, 5)?;
            Ok(AotEntrypointArtifact {
                module_id: parts[0].to_string(),
                symbol_kind: parts[1].to_string(),
                symbol_name: parts[2].to_string(),
                is_async: parse_prefixed_bool(parts[3], "async=")?,
                exported: parse_prefixed_bool(parts[4], "exported=")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let routine_rows = require_string_array(&table, "routine_rows")?;
    let mut routines = Vec::new();
    let mut routine_indexes = BTreeMap::new();
    for row in routine_rows {
        let parts = split_row_n(&row, 7)?;
        let module_id = parts[0].to_string();
        let symbol_name = parts[2].to_string();
        let routine = AotRoutineArtifact {
            module_id: module_id.clone(),
            symbol_kind: parts[1].to_string(),
            symbol_name: symbol_name.clone(),
            is_async: parse_prefixed_bool(parts[3], "async=")?,
            exported: parse_prefixed_bool(parts[4], "exported=")?,
            intrinsic_impl: {
                let value = parts[5]
                    .strip_prefix("intrinsic=")
                    .ok_or_else(|| format!("malformed backend intrinsic row `{row}`"))?;
                if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            },
            type_param_rows: Vec::new(),
            behavior_attr_rows: Vec::new(),
            param_rows: Vec::new(),
            signature_row: parts[6]
                .strip_prefix("signature=")
                .ok_or_else(|| format!("malformed backend signature row `{row}`"))?
                .to_string(),
            foreword_rows: Vec::new(),
            rollup_rows: Vec::new(),
            statement_rows: Vec::new(),
        };
        routine_indexes.insert((module_id, symbol_name), routines.len());
        routines.push(routine);
    }

    for row in require_string_array(&table, "routine_type_param_rows")? {
        let parts = split_row_n(&row, 4)?;
        let routine_index = *routine_indexes
            .get(&(parts[0].to_string(), parts[1].to_string()))
            .ok_or_else(|| {
                format!(
                    "unknown routine `{}:{}` in backend artifact",
                    parts[0], parts[1]
                )
            })?;
        routines[routine_index]
            .type_param_rows
            .push(parts[3].to_string());
    }

    for row in require_string_array(&table, "routine_behavior_attr_rows")? {
        let parts = split_row_n(&row, 4)?;
        let routine_index = *routine_indexes
            .get(&(parts[0].to_string(), parts[1].to_string()))
            .ok_or_else(|| {
                format!(
                    "unknown routine `{}:{}` in backend artifact",
                    parts[0], parts[1]
                )
            })?;
        routines[routine_index]
            .behavior_attr_rows
            .push(parts[3].to_string());
    }

    for row in require_string_array(&table, "routine_param_rows")? {
        let parts = split_row_n(&row, 4)?;
        let routine_index = *routine_indexes
            .get(&(parts[0].to_string(), parts[1].to_string()))
            .ok_or_else(|| {
                format!(
                    "unknown routine `{}:{}` in backend artifact",
                    parts[0], parts[1]
                )
            })?;
        routines[routine_index]
            .param_rows
            .push(parts[3].to_string());
    }
    for row in require_string_array(&table, "routine_foreword_rows")? {
        let parts = split_row_n(&row, 4)?;
        let routine_index = *routine_indexes
            .get(&(parts[0].to_string(), parts[1].to_string()))
            .ok_or_else(|| {
                format!(
                    "unknown routine `{}:{}` in backend artifact",
                    parts[0], parts[1]
                )
            })?;
        routines[routine_index]
            .foreword_rows
            .push(parts[3].to_string());
    }
    for row in require_string_array(&table, "routine_rollup_rows")? {
        let parts = split_row_n(&row, 4)?;
        let routine_index = *routine_indexes
            .get(&(parts[0].to_string(), parts[1].to_string()))
            .ok_or_else(|| {
                format!(
                    "unknown routine `{}:{}` in backend artifact",
                    parts[0], parts[1]
                )
            })?;
        routines[routine_index]
            .rollup_rows
            .push(parts[3].to_string());
    }
    for row in require_string_array(&table, "statement_rows")? {
        let parts = split_row_n(&row, 4)?;
        let routine_index = *routine_indexes
            .get(&(parts[0].to_string(), parts[1].to_string()))
            .ok_or_else(|| {
                format!(
                    "unknown routine `{}:{}` in backend artifact",
                    parts[0], parts[1]
                )
            })?;
        routines[routine_index]
            .statement_rows
            .push(parts[3].to_string());
    }

    Ok(AotPackageArtifact {
        format: AOT_INTERNAL_FORMAT,
        package_name: require_str(&table, "package")?,
        root_module_id: require_str(&table, "root_module")?,
        direct_deps: require_string_array(&table, "direct_deps")?,
        module_count: require_int(&table, "module_count")?,
        dependency_edge_count: require_int(&table, "dependency_edge_count")?,
        dependency_rows: require_string_array(&table, "dependency_rows")?,
        exported_surface_rows: require_string_array(&table, "surface_rows")?,
        runtime_requirements: require_string_array(&table, "runtime_requirements")?,
        entrypoints,
        routines,
        modules,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AOT_INTERNAL_FORMAT, AotEntrypointArtifact, AotPackageArtifact, AotPackageModuleArtifact,
        AotRoutineArtifact, compile_module, compile_package, parse_package_artifact,
        render_package_artifact,
    };
    use arcana_ir::{IrEntrypoint, IrModule, IrPackage, IrPackageModule, IrRoutine};

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
                symbol_name: "open".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn open() -> Int:".to_string(),
                intrinsic_impl: None,
                foreword_rows: Vec::new(),
                rollup_rows: Vec::new(),
                statement_rows: vec![
                    "stmt(core=return(int(0)),forewords=[],rollups=[])".to_string(),
                ],
            }],
        });
        assert_eq!(artifact.format, AOT_INTERNAL_FORMAT);
        assert_eq!(artifact.package_name, "winspell");
        assert_eq!(artifact.root_module_id, "winspell");
        assert_eq!(artifact.direct_deps, vec!["std".to_string()]);
        assert_eq!(
            artifact.runtime_requirements,
            vec!["std.canvas".to_string()]
        );
        assert_eq!(artifact.entrypoints.len(), 1);
        assert_eq!(artifact.routines.len(), 1);
        assert_eq!(artifact.modules.len(), 2);
    }

    #[test]
    fn render_and_parse_package_roundtrip() {
        let artifact = AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT,
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
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: vec!["name=phase:value=update".to_string()],
                param_rows: vec!["mode=:name=path:ty=Str".to_string()],
                signature_row: "fn main() -> Int:".to_string(),
                intrinsic_impl: None,
                foreword_rows: vec!["test()".to_string()],
                rollup_rows: vec!["cleanup:page:handler".to_string()],
                statement_rows: vec![
                    "stmt(core=return(int(0)),forewords=[],rollups=[])".to_string(),
                ],
            }],
            modules: vec![AotPackageModuleArtifact {
                module_id: "tool".to_string(),
                symbol_count: 1,
                item_count: 3,
                line_count: 3,
                non_empty_line_count: 3,
                directive_rows: vec!["module=tool:import:std.io:".to_string()],
                lang_item_rows: vec!["module=tool:lang:entry:main".to_string()],
                exported_surface_rows: vec!["module=tool:export:fn:fn main() -> Int:".to_string()],
            }],
        };

        let rendered = render_package_artifact(&artifact);
        let parsed = parse_package_artifact(&rendered).expect("artifact should roundtrip");
        assert_eq!(parsed, artifact);
    }
}
