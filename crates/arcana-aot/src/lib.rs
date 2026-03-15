use std::collections::BTreeSet;

use arcana_ir::{
    ExecPageRollup, ExecStmt, IrEntrypoint, IrModule, IrPackage, IrPackageModule, IrRoutine,
};
use serde::{Deserialize, Serialize};

pub const AOT_INTERNAL_FORMAT: &str = "arcana-aot-v4";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotArtifact {
    pub format: String,
    pub symbol_count: usize,
    pub item_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotEntrypointArtifact {
    pub module_id: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub is_async: bool,
    pub exported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotRoutineArtifact {
    pub module_id: String,
    pub routine_key: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub exported: bool,
    pub is_async: bool,
    pub type_param_rows: Vec<String>,
    pub behavior_attr_rows: Vec<String>,
    pub param_rows: Vec<String>,
    pub signature_row: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intrinsic_impl: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impl_target_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impl_trait_path: Option<Vec<String>>,
    pub foreword_rows: Vec<String>,
    pub rollups: Vec<ExecPageRollup>,
    pub statements: Vec<ExecStmt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotPackageArtifact {
    pub format: String,
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
        format: AOT_INTERNAL_FORMAT.to_string(),
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
        routine_key: routine.routine_key.clone(),
        symbol_name: routine.symbol_name.clone(),
        symbol_kind: routine.symbol_kind.clone(),
        exported: routine.exported,
        is_async: routine.is_async,
        type_param_rows: routine.type_param_rows.clone(),
        behavior_attr_rows: routine.behavior_attr_rows.clone(),
        param_rows: routine.param_rows.clone(),
        signature_row: routine.signature_row.clone(),
        intrinsic_impl: routine.intrinsic_impl.clone(),
        impl_target_type: routine.impl_target_type.clone(),
        impl_trait_path: routine.impl_trait_path.clone(),
        foreword_rows: routine.foreword_rows.clone(),
        rollups: routine.rollups.clone(),
        statements: routine.statements.clone(),
    }
}

pub fn compile_package(package: &IrPackage) -> AotPackageArtifact {
    AotPackageArtifact {
        format: AOT_INTERNAL_FORMAT.to_string(),
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

pub fn render_package_artifact(artifact: &AotPackageArtifact) -> String {
    toml::to_string(artifact).expect("backend artifact should serialize")
}

fn strip_prefix_suffix<'a>(text: &'a str, prefix: &str, suffix: &str) -> Result<&'a str, String> {
    text.strip_prefix(prefix)
        .and_then(|value| value.strip_suffix(suffix))
        .ok_or_else(|| format!("malformed backend artifact row `{text}`"))
}

fn decode_escaped_row_text(text: &str, quoted: bool) -> Result<String, String> {
    let inner = if quoted {
        strip_prefix_suffix(text, "\"", "\"")?
    } else {
        text
    };
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let Some(next) = chars.next() else {
                return Err("unterminated escape in backend artifact row".to_string());
            };
            match next {
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                'n' => out.push('\n'),
                't' => out.push('\t'),
                other => out.push(other),
            }
        } else {
            out.push(ch);
        }
    }
    Ok(out)
}

fn decode_row_string(text: &str) -> Result<String, String> {
    decode_escaped_row_text(text, true)
}

fn decode_surface_text(text: &str) -> Result<String, String> {
    decode_escaped_row_text(text, false)
}

fn decode_source_string_literal(text: &str) -> Result<String, String> {
    let source = decode_row_string(text)?;
    if source.starts_with('"') && source.ends_with('"') && source.len() >= 2 {
        decode_row_string(&source)
    } else {
        Ok(source)
    }
}

fn is_identifier_text(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn split_simple_path(text: &str) -> Option<Vec<String>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let segments = trimmed
        .split('.')
        .map(str::trim)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    (!segments.is_empty() && segments.iter().all(|segment| is_identifier_text(segment)))
        .then_some(segments)
}

fn split_top_level_items(text: &str, delimiter: char) -> Vec<String> {
    let mut items = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    let mut in_string = false;
    let mut escape = false;
    for ch in text.chars() {
        if in_string {
            current.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                current.push(ch);
            }
            '[' | '(' => {
                depth += 1;
                current.push(ch);
            }
            ']' | ')' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            _ if ch == delimiter && depth == 0 => {
                let item = current.trim();
                if !item.is_empty() {
                    items.push(item.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let tail = current.trim();
    if !tail.is_empty() {
        items.push(tail.to_string());
    }
    items
}

fn validate_module_directive_row(row: &str, expected_module_id: &str) -> Result<(), String> {
    let payload = row
        .strip_prefix("module=")
        .ok_or_else(|| format!("malformed module directive row `{row}`"))?;
    let parts = payload.split(':').collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(format!("malformed module directive row `{row}`"));
    }
    if parts[0] != expected_module_id {
        return Err(format!(
            "module directive row `{row}` does not match containing module `{expected_module_id}`"
        ));
    }
    if parts[1] != "import" && parts[1] != "use" && parts[1] != "reexport" {
        return Err(format!(
            "unsupported module directive kind `{}` in `{row}`",
            parts[1]
        ));
    }
    if split_simple_path(parts[2]).is_none() {
        return Err(format!("module directive row `{row}` has invalid path"));
    }
    if !parts[3].is_empty() && !is_identifier_text(parts[3]) {
        return Err(format!("module directive row `{row}` has invalid alias"));
    }
    Ok(())
}

fn validate_lang_item_row(row: &str, expected_module_id: &str) -> Result<(), String> {
    let payload = row
        .strip_prefix("module=")
        .ok_or_else(|| format!("malformed lang item row `{row}`"))?;
    let parts = payload.split(':').collect::<Vec<_>>();
    if parts.len() != 4 || parts[1] != "lang" {
        return Err(format!("malformed lang item row `{row}`"));
    }
    if parts[0] != expected_module_id {
        return Err(format!(
            "lang item row `{row}` does not match containing module `{expected_module_id}`"
        ));
    }
    if !is_identifier_text(parts[2]) {
        return Err(format!("lang item row `{row}` has invalid name"));
    }
    if split_simple_path(parts[3]).is_none() {
        return Err(format!("lang item row `{row}` has invalid target path"));
    }
    Ok(())
}

fn validate_dependency_row(row: &str) -> Result<(), String> {
    let payload = row
        .strip_prefix("source=")
        .ok_or_else(|| format!("malformed dependency row `{row}`"))?;
    let parts = payload.split(':').collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(format!("malformed dependency row `{row}`"));
    }
    if parts[0].is_empty() {
        return Err(format!(
            "dependency row `{row}` is missing a source module id"
        ));
    }
    if parts[1] != "import" && parts[1] != "use" && parts[1] != "reexport" {
        return Err(format!(
            "unsupported dependency kind `{}` in `{row}`",
            parts[1]
        ));
    }
    if split_simple_path(parts[2]).is_none() {
        return Err(format!("dependency row `{row}` has invalid target path"));
    }
    if !parts[3].is_empty() && !is_identifier_text(parts[3]) {
        return Err(format!("dependency row `{row}` has invalid alias"));
    }
    Ok(())
}

fn validate_surface_row_payload(row: &str) -> Result<(), String> {
    if let Some(path) = row.strip_prefix("reexport:") {
        if split_simple_path(path).is_none() {
            return Err(format!("malformed exported reexport row `{row}`"));
        }
        return Ok(());
    }
    if let Some(payload) = row.strip_prefix("export:") {
        let parts = payload.splitn(2, ':').collect::<Vec<_>>();
        if parts.len() != 2 || parts[0].is_empty() {
            return Err(format!("malformed exported surface row `{row}`"));
        }
        let signature = decode_surface_text(parts[1])?;
        if signature.is_empty() {
            return Err(format!(
                "exported surface row `{row}` is missing a signature"
            ));
        }
        return Ok(());
    }
    if let Some(payload) = row.strip_prefix("impl:target=") {
        let Some((target, rest)) = payload.split_once(":trait=") else {
            return Err(format!("malformed impl surface row `{row}`"));
        };
        let Some((trait_path, methods)) = rest.split_once(":methods=") else {
            return Err(format!("malformed impl surface row `{row}`"));
        };
        let target = decode_surface_text(target)?;
        if target.is_empty() {
            return Err(format!("impl surface row `{row}` is missing a target type"));
        }
        let _trait_path = decode_surface_text(trait_path)?;
        let methods = strip_prefix_suffix(methods, "[", "]")
            .map_err(|_| format!("malformed impl surface row `{row}`"))?;
        for method in split_top_level_items(methods, ',') {
            let Some((kind, signature)) = method.split_once(':') else {
                return Err(format!("malformed impl surface row `{row}`"));
            };
            if kind.is_empty() {
                return Err(format!(
                    "impl surface row `{row}` has a method with no kind"
                ));
            }
            let signature = decode_surface_text(signature)?;
            if signature.is_empty() {
                return Err(format!(
                    "impl surface row `{row}` has a method with no signature"
                ));
            }
        }
        return Ok(());
    }
    Err(format!("unsupported exported surface row `{row}`"))
}

fn validate_module_surface_row(row: &str) -> Result<(), String> {
    if row.starts_with("module=") {
        return Err(format!(
            "module exported surface row `{row}` must not use a package `module=` prefix"
        ));
    }
    validate_surface_row_payload(row)
}

fn validate_package_surface_row(row: &str, module_ids: &BTreeSet<&str>) -> Result<(), String> {
    let payload = row
        .strip_prefix("module=")
        .ok_or_else(|| format!("malformed package exported surface row `{row}`"))?;
    let Some((module_id, module_row)) = payload.split_once(':') else {
        return Err(format!("malformed package exported surface row `{row}`"));
    };
    if module_id.is_empty() {
        return Err(format!("malformed package exported surface row `{row}`"));
    }
    if !module_ids.contains(module_id) {
        return Err(format!(
            "package exported surface row `{row}` references undeclared module `{module_id}`"
        ));
    }
    validate_surface_row_payload(module_row)
}

fn validate_param_row(text: &str) -> Result<(), String> {
    let parts = text.splitn(3, ':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(format!("malformed backend artifact param row `{text}`"));
    }
    let mode = parts[0]
        .strip_prefix("mode=")
        .ok_or_else(|| format!("backend artifact param row missing mode in `{text}`"))?;
    let name = parts[1]
        .strip_prefix("name=")
        .ok_or_else(|| format!("backend artifact param row missing name in `{text}`"))?;
    let ty = parts[2]
        .strip_prefix("ty=")
        .ok_or_else(|| format!("backend artifact param row missing ty in `{text}`"))?;
    if mode != "read" && mode != "edit" && mode != "take" && !mode.is_empty() {
        return Err(format!(
            "backend artifact param row has invalid mode in `{text}`"
        ));
    }
    if name.is_empty() {
        return Err(format!(
            "backend artifact param row missing name in `{text}`"
        ));
    }
    if ty.is_empty() {
        return Err(format!("backend artifact param row missing ty in `{text}`"));
    }
    Ok(())
}

fn validate_type_param_row(text: &str) -> Result<(), String> {
    let name = text
        .strip_prefix("name=")
        .ok_or_else(|| format!("backend artifact type param row missing name in `{text}`"))?;
    if name.is_empty() {
        return Err(format!(
            "backend artifact type param row missing name in `{text}`"
        ));
    }
    Ok(())
}

fn validate_behavior_attr_row(text: &str) -> Result<(), String> {
    let payload = text
        .strip_prefix("name=")
        .ok_or_else(|| format!("backend artifact behavior attr row missing name in `{text}`"))?;
    let Some((name, value)) = payload.split_once(":value=") else {
        return Err(format!(
            "malformed backend artifact behavior attr row `{text}`"
        ));
    };
    let decode_part = |part: &str| {
        if part.starts_with('"') {
            decode_row_string(part)
        } else {
            Ok(part.to_string())
        }
    };
    let name = decode_part(name)?;
    let value = decode_part(value)?;
    if name.is_empty() {
        return Err(format!(
            "backend artifact behavior attr row missing name in `{text}`"
        ));
    }
    if value.is_empty() {
        return Err(format!(
            "backend artifact behavior attr row missing value in `{text}`"
        ));
    }
    Ok(())
}

fn validate_foreword_row(text: &str) -> Result<(), String> {
    let Some(open) = text.find('(') else {
        return Err(format!("malformed backend artifact foreword row `{text}`"));
    };
    if !text.ends_with(')') {
        return Err(format!("malformed backend artifact foreword row `{text}`"));
    }
    let name = text[..open].trim();
    if !is_identifier_text(name) {
        return Err(format!("malformed backend artifact foreword row `{text}`"));
    }
    let args = &text[open + 1..text.len() - 1];
    for arg in split_top_level_items(args, ',') {
        if let Some((name, value)) = arg.split_once('=') {
            if !is_identifier_text(name.trim()) {
                return Err(format!("malformed backend artifact foreword row `{text}`"));
            }
            decode_source_string_literal(value.trim())?;
        } else {
            decode_source_string_literal(arg.trim())?;
        }
    }
    Ok(())
}

pub fn validate_package_artifact(artifact: &AotPackageArtifact) -> Result<(), String> {
    if artifact.package_name.is_empty() {
        return Err("backend artifact package name must not be empty".to_string());
    }
    if artifact.root_module_id.is_empty() {
        return Err("backend artifact root module id must not be empty".to_string());
    }
    if artifact.module_count != artifact.modules.len() {
        return Err(format!(
            "backend artifact module_count={} does not match modules.len()={}",
            artifact.module_count,
            artifact.modules.len()
        ));
    }
    if artifact.dependency_edge_count != artifact.dependency_rows.len() {
        return Err(format!(
            "backend artifact dependency_edge_count={} does not match dependency_rows.len()={}",
            artifact.dependency_edge_count,
            artifact.dependency_rows.len()
        ));
    }

    let mut direct_deps = BTreeSet::new();
    for dep in &artifact.direct_deps {
        if dep.is_empty() {
            return Err("backend artifact direct dependency names must not be empty".to_string());
        }
        if dep == &artifact.package_name {
            return Err(format!(
                "backend artifact package `{}` cannot list itself as a direct dependency",
                artifact.package_name
            ));
        }
        if !direct_deps.insert(dep.as_str()) {
            return Err(format!(
                "backend artifact package `{}` lists duplicate direct dependency `{dep}`",
                artifact.package_name
            ));
        }
    }

    let mut module_ids = BTreeSet::new();
    for module in &artifact.modules {
        if module.module_id.is_empty() {
            return Err("backend artifact module ids must not be empty".to_string());
        }
        if !module_ids.insert(module.module_id.as_str()) {
            return Err(format!(
                "backend artifact contains duplicate module `{}`",
                module.module_id
            ));
        }
        for row in &module.directive_rows {
            if row.is_empty() {
                return Err(format!(
                    "backend artifact module `{}` contains an empty directive row",
                    module.module_id
                ));
            }
            validate_module_directive_row(row, &module.module_id)?;
        }
        for row in &module.lang_item_rows {
            if row.is_empty() {
                return Err(format!(
                    "backend artifact module `{}` contains an empty lang item row",
                    module.module_id
                ));
            }
            validate_lang_item_row(row, &module.module_id)?;
        }
        for row in &module.exported_surface_rows {
            if row.is_empty() {
                return Err(format!(
                    "backend artifact module `{}` contains an empty exported surface row",
                    module.module_id
                ));
            }
            validate_module_surface_row(row)?;
        }
    }
    if !module_ids.contains(artifact.root_module_id.as_str()) {
        return Err(format!(
            "backend artifact root module `{}` is missing from module table",
            artifact.root_module_id
        ));
    }

    let mut dependency_rows = BTreeSet::new();
    for row in &artifact.dependency_rows {
        if row.is_empty() {
            return Err("backend artifact dependency rows must not be empty".to_string());
        }
        validate_dependency_row(row)?;
        if !dependency_rows.insert(row.as_str()) {
            return Err(format!(
                "backend artifact contains duplicate dependency row `{row}`"
            ));
        }
    }

    let mut surface_rows = BTreeSet::new();
    for row in &artifact.exported_surface_rows {
        if row.is_empty() {
            return Err("backend artifact exported surface rows must not be empty".to_string());
        }
        validate_package_surface_row(row, &module_ids)?;
        if !surface_rows.insert(row.as_str()) {
            return Err(format!(
                "backend artifact contains duplicate exported surface row `{row}`"
            ));
        }
    }

    let mut runtime_requirements = BTreeSet::new();
    for requirement in &artifact.runtime_requirements {
        if requirement.is_empty() {
            return Err("backend artifact runtime requirements must not be empty".to_string());
        }
        if !runtime_requirements.insert(requirement.as_str()) {
            return Err(format!(
                "backend artifact contains duplicate runtime requirement `{requirement}`"
            ));
        }
    }

    let mut routine_keys = BTreeSet::new();
    for routine in &artifact.routines {
        if routine.routine_key.is_empty() {
            return Err("backend artifact routine keys must not be empty".to_string());
        }
        if routine.module_id.is_empty() {
            return Err(format!(
                "backend artifact routine `{}` has an empty module id",
                routine.routine_key
            ));
        }
        if routine.symbol_name.is_empty() {
            return Err(format!(
                "backend artifact routine `{}` has an empty symbol name",
                routine.routine_key
            ));
        }
        if routine.symbol_kind.is_empty() {
            return Err(format!(
                "backend artifact routine `{}` has an empty symbol kind",
                routine.routine_key
            ));
        }
        if routine.signature_row.is_empty() {
            return Err(format!(
                "backend artifact routine `{}` has an empty signature row",
                routine.routine_key
            ));
        }
        if !module_ids.contains(routine.module_id.as_str()) {
            return Err(format!(
                "backend artifact routine `{}` references undeclared module `{}`",
                routine.routine_key, routine.module_id
            ));
        }
        if !routine_keys.insert(routine.routine_key.as_str()) {
            return Err(format!(
                "backend artifact contains duplicate routine key `{}`",
                routine.routine_key
            ));
        }
        if matches!(routine.intrinsic_impl.as_deref(), Some("")) {
            return Err(format!(
                "backend artifact routine `{}` has an empty intrinsic implementation",
                routine.routine_key
            ));
        }
        if matches!(routine.impl_target_type.as_deref(), Some("")) {
            return Err(format!(
                "backend artifact routine `{}` has an empty impl target type",
                routine.routine_key
            ));
        }
        if let Some(trait_path) = &routine.impl_trait_path {
            if trait_path.is_empty() || trait_path.iter().any(|segment| segment.is_empty()) {
                return Err(format!(
                    "backend artifact routine `{}` has an invalid impl trait path",
                    routine.routine_key
                ));
            }
        }
        for row in &routine.type_param_rows {
            if row.is_empty() {
                return Err(format!(
                    "backend artifact routine `{}` contains an empty type param row",
                    routine.routine_key
                ));
            }
            validate_type_param_row(row)?;
        }
        for row in &routine.behavior_attr_rows {
            if row.is_empty() {
                return Err(format!(
                    "backend artifact routine `{}` contains an empty behavior attr row",
                    routine.routine_key
                ));
            }
            validate_behavior_attr_row(row)?;
        }
        for row in &routine.param_rows {
            if row.is_empty() {
                return Err(format!(
                    "backend artifact routine `{}` contains an empty param row",
                    routine.routine_key
                ));
            }
            validate_param_row(row)?;
        }
        for row in &routine.foreword_rows {
            if row.is_empty() {
                return Err(format!(
                    "backend artifact routine `{}` contains an empty foreword row",
                    routine.routine_key
                ));
            }
            validate_foreword_row(row)?;
        }
    }

    let mut entrypoint_keys = BTreeSet::new();
    for entrypoint in &artifact.entrypoints {
        if entrypoint.module_id.is_empty() {
            return Err("backend artifact entrypoint module ids must not be empty".to_string());
        }
        if entrypoint.symbol_name.is_empty() {
            return Err("backend artifact entrypoint names must not be empty".to_string());
        }
        if entrypoint.symbol_kind.is_empty() {
            return Err("backend artifact entrypoint kinds must not be empty".to_string());
        }
        if !module_ids.contains(entrypoint.module_id.as_str()) {
            return Err(format!(
                "backend artifact entrypoint `{}.{}` references undeclared module",
                entrypoint.module_id, entrypoint.symbol_name
            ));
        }
        let key = format!(
            "{}:{}:{}",
            entrypoint.module_id, entrypoint.symbol_kind, entrypoint.symbol_name
        );
        if !entrypoint_keys.insert(key) {
            return Err(format!(
                "backend artifact contains duplicate entrypoint `{}.{}`",
                entrypoint.module_id, entrypoint.symbol_name
            ));
        }
        let matches = artifact
            .routines
            .iter()
            .filter(|routine| {
                routine.module_id == entrypoint.module_id
                    && routine.symbol_name == entrypoint.symbol_name
                    && routine.symbol_kind == entrypoint.symbol_kind
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => {
                return Err(format!(
                    "backend artifact entrypoint `{}.{}` has no matching routine",
                    entrypoint.module_id, entrypoint.symbol_name
                ));
            }
            [routine] => {
                if routine.is_async != entrypoint.is_async {
                    return Err(format!(
                        "backend artifact entrypoint `{}.{}` async metadata does not match its routine",
                        entrypoint.module_id, entrypoint.symbol_name
                    ));
                }
                if routine.exported != entrypoint.exported {
                    return Err(format!(
                        "backend artifact entrypoint `{}.{}` export metadata does not match its routine",
                        entrypoint.module_id, entrypoint.symbol_name
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "backend artifact entrypoint `{}.{}` is ambiguous across routines",
                    entrypoint.module_id, entrypoint.symbol_name
                ));
            }
        }
    }

    Ok(())
}

pub fn parse_package_artifact(text: &str) -> Result<AotPackageArtifact, String> {
    let artifact = toml::from_str::<AotPackageArtifact>(text)
        .map_err(|err| format!("failed to parse backend artifact: {err}"))?;
    if artifact.format != AOT_INTERNAL_FORMAT {
        return Err(format!(
            "unsupported backend artifact format `{}`; expected `{AOT_INTERNAL_FORMAT}`",
            artifact.format
        ));
    }
    validate_package_artifact(&artifact)?;
    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use super::{
        AOT_INTERNAL_FORMAT, AotEntrypointArtifact, AotPackageArtifact, AotPackageModuleArtifact,
        AotRoutineArtifact, compile_module, compile_package, parse_package_artifact,
        render_package_artifact, validate_package_artifact,
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
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(0),
                }],
            }],
        });
        assert_eq!(artifact.format, AOT_INTERNAL_FORMAT);
        assert_eq!(artifact.module_count, 2);
        assert_eq!(artifact.modules[0].module_id, "winspell");
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
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(0),
                }],
            }],
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
                    foreword_rows: Vec::new(),
                    rollups: Vec::new(),
                    statements: Vec::new(),
                },
            ],
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
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(0),
                }],
            }],
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
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: Vec::new(),
            }],
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
        assert!(err.contains("must not use a package `module=` prefix"), "{err}");
    }

    #[test]
    fn parse_package_artifact_rejects_unqualified_package_surface_rows() {
        let mut artifact = base_surface_validation_artifact();
        artifact.exported_surface_rows = vec!["export:fn:fn main() -> Int:".to_string()];

        let err = parse_package_artifact(&render_package_artifact(&artifact))
            .expect_err("artifact should reject unqualified package surface rows");
        assert!(err.contains("malformed package exported surface row"), "{err}");
    }

    #[test]
    fn parse_package_artifact_rejects_package_surface_rows_for_undeclared_modules() {
        let mut artifact = base_surface_validation_artifact();
        artifact.exported_surface_rows = vec!["module=ghost:export:fn:fn main() -> Int:".to_string()];

        let err = parse_package_artifact(&render_package_artifact(&artifact))
            .expect_err("artifact should reject package surface rows for undeclared modules");
        assert!(err.contains("references undeclared module `ghost`"), "{err}");
    }
}
