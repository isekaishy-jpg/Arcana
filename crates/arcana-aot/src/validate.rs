use std::collections::BTreeSet;

use crate::artifact::AotPackageArtifact;
use crate::native_abi::{
    collect_binding_layouts, collect_native_binding_callbacks, collect_native_binding_imports,
};
use arcana_cabi::{validate_binding_callbacks, validate_binding_imports, validate_binding_layouts};
use arcana_ir::{
    IrForewordRetention, IrRoutineType, IrRoutineTypeKind, parse_memory_spec_surface_row,
    parse_struct_bitfield_layout_row,
};

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
                'r' => out.push('\r'),
                '|' => out.push('|'),
                other => out.push(other),
            }
        } else {
            out.push(ch);
        }
    }
    Ok(out)
}

fn validate_surface_text_encoding(text: &str) -> Result<(), String> {
    decode_escaped_row_text(text, false).map(|_| ())
}

fn is_identifier_text(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_generic_param_text(text: &str) -> bool {
    if let Some(rest) = text.strip_prefix('\'') {
        !rest.is_empty() && is_identifier_text(rest)
    } else {
        is_identifier_text(text)
    }
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
    if let Some(layout) = parse_struct_bitfield_layout_row(row)? {
        let expected_prefix = format!("{expected_module_id}.");
        if !layout.type_name.starts_with(&expected_prefix) {
            return Err(format!(
                "struct bitfield layout row `{row}` does not match containing module `{expected_module_id}`"
            ));
        }
        if layout.fields.is_empty() {
            return Err(format!(
                "struct bitfield layout row `{row}` must contain at least one field"
            ));
        }
        return Ok(());
    }
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
    if parse_memory_spec_surface_row(row)?.is_some() {
        return Ok(());
    }
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
        if parts[1].is_empty() {
            return Err(format!(
                "exported surface row `{row}` is missing a signature"
            ));
        }
        validate_surface_text_encoding(parts[1])?;
        return Ok(());
    }
    if let Some(payload) = row.strip_prefix("impl:target=") {
        let Some((target, rest)) = payload.split_once(":trait=") else {
            return Err(format!("malformed impl surface row `{row}`"));
        };
        let Some((trait_path, methods)) = rest.split_once(":methods=") else {
            return Err(format!("malformed impl surface row `{row}`"));
        };
        if target.is_empty() {
            return Err(format!("impl surface row `{row}` is missing a target type"));
        }
        validate_surface_text_encoding(target)?;
        validate_surface_text_encoding(trait_path)?;
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
            if signature.is_empty() {
                return Err(format!(
                    "impl surface row `{row}` has a method with no signature"
                ));
            }
            validate_surface_text_encoding(signature)?;
        }
        return Ok(());
    }
    if let Some(signature) = row.strip_prefix("native_callback:") {
        if signature.is_empty() {
            return Err(format!(
                "native callback surface row `{row}` is missing a signature"
            ));
        }
        validate_surface_text_encoding(signature)?;
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

fn validate_routine_param(
    routine_key: &str,
    index: usize,
    mode: Option<&str>,
    name: &str,
    ty: &IrRoutineType,
) -> Result<(), String> {
    if let Some(mode) = mode
        && !matches!(
            mode,
            "read" | "edit" | "take" | "hold" | "copy" | "borrow" | "move"
        )
    {
        return Err(format!(
            "backend artifact routine `{routine_key}` param {index} has unsupported mode `{mode}`"
        ));
    }
    if !is_identifier_text(name) {
        return Err(format!(
            "backend artifact routine `{routine_key}` param {index} has invalid name"
        ));
    }
    if !validate_routine_type(ty) {
        return Err(format!(
            "backend artifact routine `{routine_key}` param {index} has an invalid type"
        ));
    }
    Ok(())
}

fn validate_routine_type(ty: &IrRoutineType) -> bool {
    match &ty.kind {
        IrRoutineTypeKind::Path(path) => {
            !path.segments.is_empty() && path.segments.iter().all(|segment| !segment.is_empty())
        }
        IrRoutineTypeKind::Apply { base, args } => {
            !base.segments.is_empty()
                && base.segments.iter().all(|segment| !segment.is_empty())
                && args.iter().all(validate_routine_type)
        }
        IrRoutineTypeKind::Ref {
            lifetime, inner, ..
        } => {
            lifetime
                .as_ref()
                .is_none_or(|lifetime| !lifetime.name.is_empty())
                && validate_routine_type(inner)
        }
        IrRoutineTypeKind::Tuple(items) => items.iter().all(validate_routine_type),
        IrRoutineTypeKind::Projection(projection) => {
            !projection.assoc.is_empty()
                && !projection.trait_ref.path.segments.is_empty()
                && projection
                    .trait_ref
                    .path
                    .segments
                    .iter()
                    .all(|segment| !segment.is_empty())
                && projection.trait_ref.args.iter().all(validate_routine_type)
        }
    }
}

fn validate_foreword_metadata_entry(
    module_keys: &BTreeSet<(&str, &str)>,
    entry: &arcana_ir::IrForewordMetadata,
) -> Result<(), String> {
    if entry.qualified_name.is_empty() {
        return Err("backend artifact foreword index entry names must not be empty".to_string());
    }
    if entry.package_id.is_empty() || entry.module_id.is_empty() {
        return Err(format!(
            "backend artifact foreword index entry `{}` must reference a non-empty package and module",
            entry.qualified_name
        ));
    }
    if !module_keys.contains(&(entry.package_id.as_str(), entry.module_id.as_str())) {
        return Err(format!(
            "backend artifact foreword index entry `{}` references undeclared module `{}::{}`",
            entry.qualified_name, entry.package_id, entry.module_id
        ));
    }
    if entry.target_kind.is_empty() || entry.target_path.is_empty() {
        return Err(format!(
            "backend artifact foreword index entry `{}` must declare a target kind and path",
            entry.qualified_name
        ));
    }
    match entry.retention {
        IrForewordRetention::Compile
        | IrForewordRetention::Tooling
        | IrForewordRetention::Runtime => {}
    }
    for arg in &entry.args {
        if arg.name.as_deref().is_some_and(str::is_empty) {
            return Err(format!(
                "backend artifact foreword index entry `{}` contains an empty payload field name",
                entry.qualified_name
            ));
        }
    }
    if let Some(generated_by) = &entry.generated_by {
        validate_generated_by(
            &format!(
                "backend artifact foreword index entry `{}`",
                entry.qualified_name
            ),
            generated_by,
        )?;
    }
    Ok(())
}

fn validate_generated_by(
    context: &str,
    generated_by: &arcana_ir::IrForewordGeneratedBy,
) -> Result<(), String> {
    if generated_by.applied_name.is_empty() || generated_by.resolved_name.is_empty() {
        return Err(format!(
            "{context} has incomplete generating foreword names"
        ));
    }
    if generated_by.provider_package_id.is_empty() {
        return Err(format!(
            "{context} must declare a generating provider package id"
        ));
    }
    if generated_by.owner_kind.is_empty() || generated_by.owner_path.is_empty() {
        return Err(format!(
            "{context} must declare a generating owner kind and path"
        ));
    }
    match generated_by.retention {
        IrForewordRetention::Compile
        | IrForewordRetention::Tooling
        | IrForewordRetention::Runtime => {}
    }
    for arg in &generated_by.args {
        if arg.name.as_deref().is_some_and(str::is_empty) {
            return Err(format!(
                "{context} contains an empty generating payload field name"
            ));
        }
    }
    Ok(())
}

fn validate_foreword_registration_row(
    row: &arcana_ir::IrForewordRegistrationRow,
) -> Result<(), String> {
    if split_simple_path(&row.namespace).is_none() {
        return Err(format!(
            "backend artifact foreword registration row `{}` must declare a valid namespace",
            row.key
        ));
    }
    if row.key.is_empty() {
        return Err(
            "backend artifact foreword registration rows must declare a non-empty key".to_string(),
        );
    }
    if row.target_kind.is_empty() || row.target_path.is_empty() {
        return Err(format!(
            "backend artifact foreword registration row `{}:{}` must declare a target kind and path",
            row.namespace, row.key
        ));
    }
    validate_generated_by(
        &format!(
            "backend artifact foreword registration row `{}:{}`",
            row.namespace, row.key
        ),
        &row.generated_by,
    )?;
    Ok(())
}

pub fn validate_package_artifact(artifact: &AotPackageArtifact) -> Result<(), String> {
    if artifact.package_id.is_empty() {
        return Err("backend artifact package id must not be empty".to_string());
    }
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
    let mut direct_dep_ids = BTreeSet::new();
    for dep_id in &artifact.direct_dep_ids {
        if dep_id.is_empty() {
            return Err("backend artifact direct dependency ids must not be empty".to_string());
        }
        if dep_id == &artifact.package_id {
            return Err(format!(
                "backend artifact package `{}` cannot list itself as a direct dependency id",
                artifact.package_id
            ));
        }
        if !direct_dep_ids.insert(dep_id.as_str()) {
            return Err(format!(
                "backend artifact package `{}` lists duplicate direct dependency id `{dep_id}`",
                artifact.package_id
            ));
        }
    }
    if artifact.package_display_names.is_empty() {
        return Err("backend artifact package display-name table must not be empty".to_string());
    }
    if !artifact
        .package_display_names
        .contains_key(&artifact.package_id)
    {
        return Err(format!(
            "backend artifact package display-name table is missing root package `{}`",
            artifact.package_id
        ));
    }
    for (package_id, package_name) in &artifact.package_display_names {
        if package_id.is_empty() || package_name.is_empty() {
            return Err("backend artifact package display names must not be empty".to_string());
        }
    }
    for (package_id, dep_ids) in &artifact.package_direct_dep_ids {
        if !artifact.package_display_names.contains_key(package_id) {
            return Err(format!(
                "backend artifact dependency-id table references unknown package `{package_id}`"
            ));
        }
        for (package_name, dep_id) in dep_ids {
            if package_name.is_empty() || dep_id.is_empty() {
                return Err(format!(
                    "backend artifact package `{package_id}` has an empty dependency mapping"
                ));
            }
            if !artifact.package_display_names.contains_key(dep_id) {
                return Err(format!(
                    "backend artifact package `{package_id}` references unknown dependency id `{dep_id}`"
                ));
            }
        }
    }

    let mut module_ids = BTreeSet::new();
    let mut module_keys = BTreeSet::new();
    for module in &artifact.modules {
        if module.package_id.is_empty() {
            return Err("backend artifact module package ids must not be empty".to_string());
        }
        if !artifact
            .package_display_names
            .contains_key(&module.package_id)
        {
            return Err(format!(
                "backend artifact module `{}` references unknown package `{}`",
                module.module_id, module.package_id
            ));
        }
        if module.module_id.is_empty() {
            return Err("backend artifact module ids must not be empty".to_string());
        }
        module_ids.insert(module.module_id.as_str());
        if !module_keys.insert((module.package_id.as_str(), module.module_id.as_str())) {
            return Err(format!(
                "backend artifact contains duplicate module `{}::{}`",
                module.package_id, module.module_id
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
    if !module_keys.contains(&(
        artifact.package_id.as_str(),
        artifact.root_module_id.as_str(),
    )) {
        return Err(format!(
            "backend artifact root module `{}` is missing from the root package `{}`",
            artifact.root_module_id, artifact.package_id
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
        if routine.package_id.is_empty() {
            return Err(format!(
                "backend artifact routine `{}` has an empty package id",
                routine.routine_key
            ));
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
        if !artifact
            .package_display_names
            .contains_key(&routine.package_id)
        {
            return Err(format!(
                "backend artifact routine `{}` references unknown package `{}`",
                routine.routine_key, routine.package_id
            ));
        }
        if !module_keys.contains(&(routine.package_id.as_str(), routine.module_id.as_str())) {
            return Err(format!(
                "backend artifact routine `{}` references undeclared module `{}::{}`",
                routine.routine_key, routine.package_id, routine.module_id
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
        if routine
            .impl_target_type
            .as_ref()
            .is_some_and(|ty| !validate_routine_type(ty))
        {
            return Err(format!(
                "backend artifact routine `{}` has an invalid impl target type",
                routine.routine_key
            ));
        }
        if let Some(trait_path) = &routine.impl_trait_path
            && (trait_path.is_empty() || trait_path.iter().any(|segment| segment.is_empty()))
        {
            return Err(format!(
                "backend artifact routine `{}` has an invalid impl trait path",
                routine.routine_key
            ));
        }
        if routine
            .return_type
            .as_ref()
            .is_some_and(|ty| !validate_routine_type(ty))
        {
            return Err(format!(
                "backend artifact routine `{}` has an invalid return type",
                routine.routine_key
            ));
        }
        for name in &routine.type_params {
            if name.is_empty() || !is_generic_param_text(name) {
                return Err(format!(
                    "backend artifact routine `{}` has an invalid type param `{name}`",
                    routine.routine_key,
                ));
            }
        }
        for (name, value) in &routine.behavior_attrs {
            if name.is_empty() || value.is_empty() {
                return Err(format!(
                    "backend artifact routine `{}` has an empty behavior attr",
                    routine.routine_key
                ));
            }
        }
        for (index, param) in routine.params.iter().enumerate() {
            validate_routine_param(
                &routine.routine_key,
                index,
                param.mode.as_deref(),
                &param.name,
                &param.ty,
            )?;
        }
    }

    for entry in &artifact.foreword_index {
        validate_foreword_metadata_entry(&module_keys, entry)?;
    }
    for row in &artifact.foreword_registrations {
        validate_foreword_registration_row(row)?;
    }

    let mut entrypoint_keys = BTreeSet::new();
    for entrypoint in &artifact.entrypoints {
        if entrypoint.package_id.is_empty() {
            return Err("backend artifact entrypoint package ids must not be empty".to_string());
        }
        if entrypoint.module_id.is_empty() {
            return Err("backend artifact entrypoint module ids must not be empty".to_string());
        }
        if entrypoint.symbol_name.is_empty() {
            return Err("backend artifact entrypoint names must not be empty".to_string());
        }
        if entrypoint.symbol_kind.is_empty() {
            return Err("backend artifact entrypoint kinds must not be empty".to_string());
        }
        if !artifact
            .package_display_names
            .contains_key(&entrypoint.package_id)
        {
            return Err(format!(
                "backend artifact entrypoint `{}::{}` references unknown package",
                entrypoint.package_id, entrypoint.symbol_name
            ));
        }
        if !module_keys.contains(&(
            entrypoint.package_id.as_str(),
            entrypoint.module_id.as_str(),
        )) {
            return Err(format!(
                "backend artifact entrypoint `{}::{}.{}` references undeclared module",
                entrypoint.package_id, entrypoint.module_id, entrypoint.symbol_name
            ));
        }
        let key = format!(
            "{}:{}:{}:{}",
            entrypoint.package_id,
            entrypoint.module_id,
            entrypoint.symbol_kind,
            entrypoint.symbol_name
        );
        if !entrypoint_keys.insert(key) {
            return Err(format!(
                "backend artifact contains duplicate entrypoint `{}::{}.{}`",
                entrypoint.package_id, entrypoint.module_id, entrypoint.symbol_name
            ));
        }
        let matches = artifact
            .routines
            .iter()
            .filter(|routine| {
                routine.package_id == entrypoint.package_id
                    && routine.module_id == entrypoint.module_id
                    && routine.symbol_name == entrypoint.symbol_name
                    && routine.symbol_kind == entrypoint.symbol_kind
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => {
                return Err(format!(
                    "backend artifact entrypoint `{}::{}.{}` has no matching routine",
                    entrypoint.package_id, entrypoint.module_id, entrypoint.symbol_name
                ));
            }
            [routine] => {
                if routine.is_async != entrypoint.is_async {
                    return Err(format!(
                        "backend artifact entrypoint `{}::{}.{}` async metadata does not match its routine",
                        entrypoint.package_id, entrypoint.module_id, entrypoint.symbol_name
                    ));
                }
                if routine.exported != entrypoint.exported {
                    return Err(format!(
                        "backend artifact entrypoint `{}::{}.{}` export metadata does not match its routine",
                        entrypoint.package_id, entrypoint.module_id, entrypoint.symbol_name
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "backend artifact entrypoint `{}::{}.{}` is ambiguous across routines",
                    entrypoint.package_id, entrypoint.module_id, entrypoint.symbol_name
                ));
            }
        }
    }

    for decl in &artifact.shackle_decls {
        match decl.kind.as_str() {
            "type" | "struct" | "union" | "callback" => {
                if decl.raw_layout.is_none() {
                    return Err(format!(
                        "backend artifact shackle {} `{}` is missing typed raw layout metadata",
                        decl.kind, decl.name
                    ));
                }
            }
            "flags" if decl.binding.is_some() => {
                if decl.raw_layout.is_none() {
                    return Err(format!(
                        "backend artifact shackle flags `{}` is missing typed raw layout metadata",
                        decl.name
                    ));
                }
            }
            "import fn" | "import_fn" => {
                if decl.import_target.is_none() {
                    return Err(format!(
                        "backend artifact shackle import fn `{}` is missing typed import metadata",
                        decl.name
                    ));
                }
            }
            "thunk" => {
                if decl.thunk_target.is_none() {
                    return Err(format!(
                        "backend artifact shackle thunk `{}` is missing typed thunk metadata",
                        decl.name
                    ));
                }
            }
            _ => {}
        }
    }

    let binding_imports = collect_native_binding_imports(artifact)?;
    validate_binding_imports(&binding_imports)?;
    let binding_callbacks = collect_native_binding_callbacks(artifact)?;
    validate_binding_callbacks(&binding_callbacks)?;
    validate_binding_layouts(&artifact.binding_layouts)?;
    let collected_layouts = collect_binding_layouts(artifact)?;
    if collected_layouts != artifact.binding_layouts {
        return Err(
            "backend artifact binding layout table does not match typed shackle metadata"
                .to_string(),
        );
    }

    Ok(())
}
