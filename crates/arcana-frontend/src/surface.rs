use arcana_hir::{HirResolvedModule, HirResolvedSymbolRef, HirSymbolKind, HirWorkspaceSummary};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceSymbolUse {
    TypeLike,
    Trait,
}

pub(crate) type ResolvedSymbolRef<'a> = HirResolvedSymbolRef<'a>;

pub(crate) fn lookup_symbol_path<'a>(
    workspace: &'a HirWorkspaceSummary,
    module: &'a HirResolvedModule,
    path: &[String],
) -> Option<ResolvedSymbolRef<'a>> {
    arcana_hir::lookup_symbol_path(workspace, module, path)
}

pub(crate) fn symbol_matches_surface_use(
    kind: HirSymbolKind,
    expected_use: SurfaceSymbolUse,
) -> bool {
    match expected_use {
        SurfaceSymbolUse::TypeLike => {
            matches!(
                kind,
                HirSymbolKind::Record
                    | HirSymbolKind::Struct
                    | HirSymbolKind::Union
                    | HirSymbolKind::Array
                    | HirSymbolKind::Object
                    | HirSymbolKind::Enum
                    | HirSymbolKind::OpaqueType
                    | HirSymbolKind::Trait
            )
        }
        SurfaceSymbolUse::Trait => kind == HirSymbolKind::Trait,
    }
}

pub(crate) fn surface_use_name(expected_use: SurfaceSymbolUse) -> &'static str {
    match expected_use {
        SurfaceSymbolUse::TypeLike => "type",
        SurfaceSymbolUse::Trait => "trait",
    }
}

pub(crate) fn split_simple_path(text: &str) -> Option<Vec<String>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut segments = Vec::new();
    for segment in trimmed.split('.') {
        let segment = segment.trim();
        if segment.is_empty() {
            return None;
        }
        let mut chars = segment.chars();
        let first = chars.next()?;
        if !is_ident_start(first) || !chars.all(is_ident_continue) {
            return None;
        }
        segments.push(segment.to_string());
    }

    if segments.is_empty() {
        None
    } else {
        Some(segments)
    }
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}
