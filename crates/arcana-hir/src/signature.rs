use super::{HirSymbol, HirSymbolBody, HirSymbolKind};

pub fn render_symbol_signature(symbol: &HirSymbol) -> String {
    match symbol.kind {
        HirSymbolKind::Fn | HirSymbolKind::System => render_function_signature(symbol),
        HirSymbolKind::Record => render_record_signature(symbol),
        HirSymbolKind::Object => render_object_signature(symbol),
        HirSymbolKind::Owner => render_owner_signature(symbol),
        HirSymbolKind::Enum => render_enum_signature(symbol),
        HirSymbolKind::OpaqueType => render_opaque_signature(symbol),
        HirSymbolKind::Trait => render_trait_signature(symbol),
        HirSymbolKind::Behavior => render_behavior_signature(symbol),
        HirSymbolKind::Const => render_const_signature(symbol),
    }
}

pub(crate) fn render_function_signature(symbol: &HirSymbol) -> String {
    let mut rendered = String::new();
    if symbol.is_async {
        rendered.push_str("async ");
    }
    if symbol.kind == HirSymbolKind::System {
        rendered.push_str("system ");
    } else {
        rendered.push_str("fn ");
    }
    rendered.push_str(&symbol.name);
    if !symbol.type_params.is_empty() || symbol.where_clause.is_some() {
        rendered.push('[');
        let mut parts = symbol.type_params.clone();
        if let Some(where_clause) = &symbol.where_clause {
            parts.push(format!("where {where_clause}"));
        }
        rendered.push_str(&parts.join(", "));
        rendered.push(']');
    }
    rendered.push('(');
    rendered.push_str(
        &symbol
            .params
            .iter()
            .map(|param| {
                let mut piece = String::new();
                if let Some(mode) = param.mode {
                    piece.push_str(mode.as_str());
                    piece.push(' ');
                }
                piece.push_str(&param.name);
                piece.push_str(": ");
                piece.push_str(&param.ty.render());
                piece
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    rendered.push(')');
    if let Some(return_type) = &symbol.return_type {
        rendered.push_str(" -> ");
        rendered.push_str(&return_type.render());
    }
    rendered.push(':');
    rendered
}

fn render_record_signature(symbol: &HirSymbol) -> String {
    let mut lines = vec![render_named_type_header("record", symbol)];
    if let HirSymbolBody::Record { fields } = &symbol.body {
        lines.extend(
            fields
                .iter()
                .map(|field| format!("{}: {}", field.name, field.ty)),
        );
    }
    lines.join("\n")
}

fn render_object_signature(symbol: &HirSymbol) -> String {
    let mut lines = vec![render_named_type_header("obj", symbol)];
    if let HirSymbolBody::Object { fields, methods } = &symbol.body {
        lines.extend(
            fields
                .iter()
                .map(|field| format!("{}: {}", field.name, field.ty)),
        );
        lines.extend(methods.iter().map(render_function_signature));
    }
    lines.join("\n")
}

fn render_owner_signature(symbol: &HirSymbol) -> String {
    let mut rendered = String::new();
    rendered.push_str("create ");
    rendered.push_str(&symbol.name);
    rendered.push('[');
    if let HirSymbolBody::Owner { objects, .. } = &symbol.body {
        rendered.push_str(
            &objects
                .iter()
                .map(|object| format!("{} as {}", object.type_path.join("."), object.local_name))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    rendered.push(']');
    rendered.push_str(" scope-exit:");
    rendered
}

fn render_enum_signature(symbol: &HirSymbol) -> String {
    let mut lines = vec![render_named_type_header("enum", symbol)];
    if let HirSymbolBody::Enum { variants } = &symbol.body {
        lines.extend(variants.iter().map(|variant| match &variant.payload {
            Some(payload) => format!("{}({payload})", variant.name),
            None => variant.name.clone(),
        }));
    }
    lines.join("\n")
}

fn render_trait_signature(symbol: &HirSymbol) -> String {
    let mut lines = vec![render_named_type_header("trait", symbol)];
    if let HirSymbolBody::Trait {
        assoc_types,
        methods,
    } = &symbol.body
    {
        lines.extend(
            assoc_types
                .iter()
                .map(|assoc_type| match &assoc_type.default_ty {
                    Some(default_ty) => format!("type {} = {default_ty}", assoc_type.name),
                    None => format!("type {}", assoc_type.name),
                }),
        );
        lines.extend(methods.iter().map(render_function_signature));
    }
    lines.join("\n")
}

fn render_opaque_signature(symbol: &HirSymbol) -> String {
    let mut rendered = String::new();
    rendered.push_str("opaque type ");
    rendered.push_str(&symbol.name);
    if !symbol.type_params.is_empty() || symbol.where_clause.is_some() {
        rendered.push('[');
        let mut parts = symbol.type_params.clone();
        if let Some(where_clause) = &symbol.where_clause {
            parts.push(format!("where {where_clause}"));
        }
        rendered.push_str(&parts.join(", "));
        rendered.push(']');
    }
    if let Some(policy) = symbol.opaque_policy {
        rendered.push_str(" as ");
        rendered.push_str(policy.ownership.as_str());
        rendered.push_str(", ");
        rendered.push_str(policy.boundary.as_str());
    }
    rendered
}

fn render_behavior_signature(symbol: &HirSymbol) -> String {
    let attrs = symbol
        .behavior_attrs
        .iter()
        .map(|attr| format!("{}={}", attr.name, attr.value))
        .collect::<Vec<_>>()
        .join(", ");
    let mut rendered = String::new();
    rendered.push_str("behavior[");
    rendered.push_str(&attrs);
    rendered.push_str("] ");
    rendered.push_str(&render_function_signature(symbol));
    rendered
}

fn render_const_signature(symbol: &HirSymbol) -> String {
    let mut rendered = String::new();
    rendered.push_str("const ");
    rendered.push_str(&symbol.name);
    if let Some(return_type) = &symbol.return_type {
        rendered.push_str(": ");
        rendered.push_str(&return_type.render());
    }
    rendered.push(':');
    rendered
}

fn render_named_type_header(keyword: &str, symbol: &HirSymbol) -> String {
    let mut rendered = String::new();
    rendered.push_str(keyword);
    rendered.push(' ');
    rendered.push_str(&symbol.name);
    if !symbol.type_params.is_empty() || symbol.where_clause.is_some() {
        rendered.push('[');
        let mut parts = symbol.type_params.clone();
        if let Some(where_clause) = &symbol.where_clause {
            parts.push(format!("where {where_clause}"));
        }
        rendered.push_str(&parts.join(", "));
        rendered.push(']');
    }
    rendered.push(':');
    rendered
}
