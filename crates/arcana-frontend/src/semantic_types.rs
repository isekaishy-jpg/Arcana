use std::collections::BTreeMap;

use arcana_hir::{
    HirPredicate, HirProjection, HirResolvedModule, HirTraitRef, HirType, HirTypeKind,
    HirWorkspaceSummary, lookup_symbol_path,
};
use arcana_syntax::is_builtin_type_name;

use crate::{TypeScope, type_resolve::canonical_symbol_path};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SemanticLocalBindingId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct TypeId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct TraitRefId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ProjectionId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PredicateId(pub u32);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum SemanticLifetimeKey {
    Static,
    Local(SemanticLocalBindingId),
    Named(String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum SemanticTypeKey {
    SelfType,
    TypeParam(SemanticLocalBindingId),
    AssocType(SemanticLocalBindingId),
    Path(Vec<String>),
    Apply { base: Vec<String>, args: Vec<TypeId> },
    Ref {
        lifetime: Option<SemanticLifetimeKey>,
        mutable: bool,
        inner: TypeId,
    },
    Tuple(Vec<TypeId>),
    Projection(ProjectionId),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum SemanticTraitRefKey {
    Path { path: Vec<String>, args: Vec<TypeId> },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum SemanticProjectionKey {
    TraitRef { trait_ref: TraitRefId, assoc: String },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum SemanticPredicateKey {
    TraitBound { trait_ref: TraitRefId },
    ProjectionEq { projection: ProjectionId, value: TypeId },
    LifetimeOutlives {
        longer: SemanticLifetimeKey,
        shorter: SemanticLifetimeKey,
    },
    TypeOutlives {
        ty: TypeId,
        lifetime: SemanticLifetimeKey,
    },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SemanticArena {
    type_ids: BTreeMap<SemanticTypeKey, TypeId>,
    trait_ref_ids: BTreeMap<SemanticTraitRefKey, TraitRefId>,
    projection_ids: BTreeMap<SemanticProjectionKey, ProjectionId>,
    predicate_ids: BTreeMap<SemanticPredicateKey, PredicateId>,
}

impl SemanticArena {
    pub(crate) fn type_id_for_hir(
        &mut self,
        workspace: &HirWorkspaceSummary,
        resolved_module: &HirResolvedModule,
        scope: &TypeScope,
        ty: &HirType,
    ) -> TypeId {
        let key = self.lower_type_key(workspace, resolved_module, scope, ty);
        if let Some(existing) = self.type_ids.get(&key) {
            return *existing;
        }
        let id = TypeId(self.type_ids.len() as u32);
        self.type_ids.insert(key, id);
        id
    }

    pub(crate) fn trait_ref_id_for_hir(
        &mut self,
        workspace: &HirWorkspaceSummary,
        resolved_module: &HirResolvedModule,
        scope: &TypeScope,
        trait_ref: &HirTraitRef,
    ) -> TraitRefId {
        let key = SemanticTraitRefKey::Path {
            path: canonical_path(workspace, resolved_module, scope, &trait_ref.path.segments),
            args: trait_ref
                .args
                .iter()
                .map(|arg| self.type_id_for_hir(workspace, resolved_module, scope, arg))
                .collect(),
        };
        if let Some(existing) = self.trait_ref_ids.get(&key) {
            return *existing;
        }
        let id = TraitRefId(self.trait_ref_ids.len() as u32);
        self.trait_ref_ids.insert(key, id);
        id
    }

    pub(crate) fn projection_id_for_hir(
        &mut self,
        workspace: &HirWorkspaceSummary,
        resolved_module: &HirResolvedModule,
        scope: &TypeScope,
        projection: &HirProjection,
    ) -> ProjectionId {
        let key = SemanticProjectionKey::TraitRef {
            trait_ref: self.trait_ref_id_for_hir(
                workspace,
                resolved_module,
                scope,
                &projection.trait_ref,
            ),
            assoc: projection.assoc.clone(),
        };
        if let Some(existing) = self.projection_ids.get(&key) {
            return *existing;
        }
        let id = ProjectionId(self.projection_ids.len() as u32);
        self.projection_ids.insert(key, id);
        id
    }

    pub(crate) fn predicate_id_for_hir(
        &mut self,
        workspace: &HirWorkspaceSummary,
        resolved_module: &HirResolvedModule,
        scope: &TypeScope,
        predicate: &HirPredicate,
    ) -> PredicateId {
        let key = match predicate {
            HirPredicate::TraitBound { trait_ref, .. } => SemanticPredicateKey::TraitBound {
                trait_ref: self.trait_ref_id_for_hir(workspace, resolved_module, scope, trait_ref),
            },
            HirPredicate::ProjectionEq {
                projection, value, ..
            } => SemanticPredicateKey::ProjectionEq {
                projection: self.projection_id_for_hir(
                    workspace,
                    resolved_module,
                    scope,
                    projection,
                ),
                value: self.type_id_for_hir(workspace, resolved_module, scope, value),
            },
            HirPredicate::LifetimeOutlives {
                longer, shorter, ..
            } => SemanticPredicateKey::LifetimeOutlives {
                longer: lower_lifetime_key(scope, &longer.name),
                shorter: lower_lifetime_key(scope, &shorter.name),
            },
            HirPredicate::TypeOutlives { ty, lifetime, .. } => SemanticPredicateKey::TypeOutlives {
                ty: self.type_id_for_hir(workspace, resolved_module, scope, ty),
                lifetime: lower_lifetime_key(scope, &lifetime.name),
            },
        };
        if let Some(existing) = self.predicate_ids.get(&key) {
            return *existing;
        }
        let id = PredicateId(self.predicate_ids.len() as u32);
        self.predicate_ids.insert(key, id);
        id
    }

    fn lower_type_key(
        &mut self,
        workspace: &HirWorkspaceSummary,
        resolved_module: &HirResolvedModule,
        scope: &TypeScope,
        ty: &HirType,
    ) -> SemanticTypeKey {
        match &ty.kind {
            HirTypeKind::Path(path) => {
                if path.segments.len() == 1 {
                    let name = &path.segments[0];
                    if name == "Self" && scope.allow_self {
                        return SemanticTypeKey::SelfType;
                    }
                    if let Some(id) = scope.type_param_id(name) {
                        return SemanticTypeKey::TypeParam(id);
                    }
                    if let Some(id) = scope.assoc_type_id(name) {
                        return SemanticTypeKey::AssocType(id);
                    }
                }
                SemanticTypeKey::Path(canonical_path(
                    workspace,
                    resolved_module,
                    scope,
                    &path.segments,
                ))
            }
            HirTypeKind::Apply { base, args } => SemanticTypeKey::Apply {
                base: canonical_path(workspace, resolved_module, scope, &base.segments),
                args: args
                    .iter()
                    .map(|arg| self.type_id_for_hir(workspace, resolved_module, scope, arg))
                    .collect(),
            },
            HirTypeKind::Ref {
                lifetime,
                mutable,
                inner,
            } => SemanticTypeKey::Ref {
                lifetime: lifetime
                    .as_ref()
                    .map(|lifetime| lower_lifetime_key(scope, &lifetime.name)),
                mutable: *mutable,
                inner: self.type_id_for_hir(workspace, resolved_module, scope, inner),
            },
            HirTypeKind::Tuple(items) => SemanticTypeKey::Tuple(
                items
                    .iter()
                    .map(|item| self.type_id_for_hir(workspace, resolved_module, scope, item))
                    .collect(),
            ),
            HirTypeKind::Projection(projection) => SemanticTypeKey::Projection(
                self.projection_id_for_hir(workspace, resolved_module, scope, projection),
            ),
        }
    }
}

fn lower_lifetime_key(scope: &TypeScope, lifetime: &str) -> SemanticLifetimeKey {
    if lifetime == "'static" {
        SemanticLifetimeKey::Static
    } else if let Some(id) = scope.lifetime_id(lifetime) {
        SemanticLifetimeKey::Local(id)
    } else {
        SemanticLifetimeKey::Named(lifetime.to_string())
    }
}

fn canonical_path(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &TypeScope,
    path: &[String],
) -> Vec<String> {
    if path.len() == 1
        && (scope.allows_type_name(&path[0]) || is_builtin_type_name(&path[0]) || path[0] == "Self")
    {
        return path.to_vec();
    }
    if let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, path) {
        return canonical_symbol_path(symbol_ref.module_id, &symbol_ref.symbol.name);
    }
    path.to_vec()
}
