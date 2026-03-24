use std::path::Path;

use arcana_hir::{
    HirPredicate, HirResolvedModule, HirSymbolBody, HirTraitRef, HirType, HirWhereClause,
    HirWorkspaceSummary,
};
use arcana_syntax::Span;

use crate::semantic_types::SemanticArena;
use crate::trait_contracts::resolve_trait_symbol_from_trait_ref;
use crate::type_validate::{validate_trait_surface, validate_type_surface};
use crate::{Diagnostic, TypeScope};

pub(crate) fn validate_where_clause_surface(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    scope: &TypeScope,
    where_clause: &HirWhereClause,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut semantics = SemanticArena::default();
    for predicate in &where_clause.predicates {
        let _ = semantics.predicate_id_for_hir(workspace, resolved_module, scope, predicate);
        match predicate {
            HirPredicate::TraitBound { trait_ref, .. } => validate_trait_surface(
                workspace,
                resolved_module,
                module_path,
                scope,
                trait_ref,
                span,
                &format!("where predicate `{}`", trait_ref.render()),
                diagnostics,
            ),
            HirPredicate::ProjectionEq {
                projection, value, ..
            } => {
                validate_projection_predicate(
                    workspace,
                    resolved_module,
                    module_path,
                    scope,
                    &projection.trait_ref,
                    &projection.assoc,
                    value,
                    span,
                    diagnostics,
                );
            }
            HirPredicate::LifetimeOutlives {
                longer, shorter, ..
            } => {
                for lifetime in [&longer.name, &shorter.name] {
                    if !scope.lifetime_declared(lifetime) {
                        diagnostics.push(Diagnostic {
                            path: module_path.to_path_buf(),
                            line: span.line,
                            column: span.column,
                            message: format!(
                                "undeclared lifetime `{lifetime}` in where predicate `{}`",
                                predicate.render()
                            ),
                        });
                    }
                }
            }
            HirPredicate::TypeOutlives { ty, lifetime, .. } => {
                validate_type_surface(
                    workspace,
                    resolved_module,
                    module_path,
                    scope,
                    ty,
                    span,
                    &format!("where predicate `{}`", predicate.render()),
                    diagnostics,
                );
                if !scope.lifetime_declared(&lifetime.name) {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: span.line,
                        column: span.column,
                        message: format!(
                            "undeclared lifetime `{}` in where predicate `{}`",
                            lifetime.name,
                            predicate.render()
                        ),
                    });
                }
            }
        }
    }
}

fn validate_projection_predicate(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    scope: &TypeScope,
    trait_ref: &HirTraitRef,
    assoc: &str,
    value: &HirType,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let context = format!("projection base `{}.{}`", trait_ref.render(), assoc);
    validate_trait_surface(
        workspace,
        resolved_module,
        module_path,
        scope,
        trait_ref,
        span,
        &context,
        diagnostics,
    );
    if let Some(trait_symbol_ref) =
        resolve_trait_symbol_from_trait_ref(workspace, resolved_module, trait_ref)
    {
        if let HirSymbolBody::Trait { assoc_types, .. } = &trait_symbol_ref.symbol.body {
            if !assoc_types.iter().any(|item| item.name == assoc) {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: span.line,
                    column: span.column,
                    message: format!(
                        "trait `{}` does not declare associated type `{}`",
                        trait_ref.render(),
                        assoc
                    ),
                });
            }
        }
    }
    validate_type_surface(
        workspace,
        resolved_module,
        module_path,
        scope,
        value,
        span,
        &format!("projection value `{}`", value.render()),
        diagnostics,
    );
}
