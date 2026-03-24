use arcana_hir::{HirPath, HirResolvedModule, HirType, HirTypeKind, HirWorkspaceSummary};
use arcana_syntax::Span;

use crate::surface::lookup_symbol_path;

pub(crate) fn canonical_symbol_path(module_id: &str, symbol_name: &str) -> Vec<String> {
    let mut path = module_id.split('.').map(str::to_string).collect::<Vec<_>>();
    path.push(symbol_name.to_string());
    path
}

pub(crate) fn canonical_type_from_path(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    path: &[String],
    span: Span,
) -> HirType {
    let segments = lookup_symbol_path(workspace, resolved_module, path)
        .map(|resolved| canonical_symbol_path(resolved.module_id, &resolved.symbol.name))
        .unwrap_or_else(|| path.to_vec());
    HirType {
        kind: HirTypeKind::Path(HirPath { segments, span }),
        span,
    }
}
