use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

mod api_fingerprint;
mod semantic_types;
mod surface;
mod trait_contracts;
mod type_resolve;
mod type_validate;
mod where_clause;

use arcana_hir::{
    HirAssignTarget, HirBinaryOp, HirChainStep, HirExpr, HirHeaderAttachment, HirImplDecl,
    HirLocalTypeLookup, HirMatchPattern, HirModule, HirModuleSummary, HirPhraseArg,
    HirResolvedModule, HirResolvedTarget, HirResolvedWorkspace, HirStatement, HirStatementKind,
    HirSymbol, HirSymbolBody, HirSymbolKind, HirType, HirUnaryOp,
    HirWorkspacePackage, HirWorkspaceSummary, collect_hir_type_refs,
    current_workspace_package_for_module, infer_receiver_expr_type,
    lookup_method_candidates_for_hir_type, lower_module_text,
    match_name_resolves_to_zero_payload_variant, resolve_workspace,
    visible_package_root_for_module,
};
use arcana_ir::{is_runtime_main_entry_symbol, validate_runtime_main_entry_symbol};
use arcana_package::{
    WorkspaceFingerprints, WorkspaceGraph, load_workspace_hir as load_package_workspace_hir,
    load_workspace_hir_from_graph as load_package_workspace_hir_from_graph,
};
use arcana_syntax::{
    BuiltinOwnershipClass, Span, builtin_ownership_class, is_builtin_type_name,
};
use semantic_types::{SemanticArena, SemanticLocalBindingId, TypeId};
use surface::{
    SurfaceSymbolUse, lookup_symbol_path, split_simple_path,
};
use trait_contracts::validate_impl_trait_where_requirements_structured;
use type_resolve::{canonical_symbol_path, canonical_type_from_path};
use type_validate::{
    validate_boundary_symbol_contract, validate_surface_path_kind, validate_trait_surface,
    validate_type_surface,
};
use where_clause::validate_where_clause_surface;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckSummary {
    pub package_count: usize,
    pub module_count: usize,
    pub non_empty_lines: usize,
    pub directive_count: usize,
    pub symbol_count: usize,
}

pub struct CheckedWorkspace {
    summary: CheckSummary,
    pub(crate) workspace: HirWorkspaceSummary,
    pub(crate) resolved_workspace: HirResolvedWorkspace,
}

impl CheckedWorkspace {
    pub fn summary(&self) -> &CheckSummary {
        &self.summary
    }

    pub fn into_workspace_parts(self) -> (HirWorkspaceSummary, HirResolvedWorkspace) {
        (self.workspace, self.resolved_workspace)
    }

    fn into_summary(self) -> CheckSummary {
        self.summary
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Diagnostic {
    path: PathBuf,
    line: usize,
    column: usize,
    message: String,
}

impl Diagnostic {
    fn render(&self) -> String {
        format!(
            "{}:{}:{}: {}",
            self.path.display(),
            self.line,
            self.column,
            self.message
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExprTypeClass {
    Bool,
    Int,
    Str,
    Pair,
    Collection,
}

impl ExprTypeClass {
    const fn label(self) -> &'static str {
        match self {
            Self::Bool => "Bool",
            Self::Int => "Int",
            Self::Str => "Str",
            Self::Pair => "pair",
            Self::Collection => "collection",
        }
    }
}

#[derive(Clone, Debug, Default)]
struct TypeScope {
    type_params: BTreeSet<String>,
    lifetimes: BTreeSet<String>,
    assoc_types: BTreeSet<String>,
    type_param_ids: BTreeMap<String, SemanticLocalBindingId>,
    lifetime_ids: BTreeMap<String, SemanticLocalBindingId>,
    assoc_type_ids: BTreeMap<String, SemanticLocalBindingId>,
    next_binding_id: u32,
    allow_self: bool,
}

impl TypeScope {
    fn with_params(&self, params: &[String]) -> Self {
        let mut next = self.clone();
        for param in params {
            if param.starts_with('\'') {
                next.lifetimes.insert(param.clone());
                next.bind_lifetime(param);
            } else {
                next.type_params.insert(param.clone());
                next.bind_type_param(param);
            }
        }
        next
    }

    fn with_assoc_types<I>(&self, assoc_types: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut next = self.clone();
        for assoc_type in assoc_types {
            next.assoc_types.insert(assoc_type.clone());
            next.bind_assoc_type(&assoc_type);
        }
        next
    }

    fn with_self(&self) -> Self {
        let mut next = self.clone();
        next.allow_self = true;
        next
    }

    fn allows_type_name(&self, name: &str) -> bool {
        self.type_params.contains(name)
            || self.assoc_types.contains(name)
            || (self.allow_self && name == "Self")
    }

    fn lifetime_declared(&self, lifetime: &str) -> bool {
        lifetime == "'static" || self.lifetimes.contains(lifetime)
    }

    fn type_param_id(&self, name: &str) -> Option<SemanticLocalBindingId> {
        self.type_param_ids.get(name).copied()
    }

    fn lifetime_id(&self, name: &str) -> Option<SemanticLocalBindingId> {
        self.lifetime_ids.get(name).copied()
    }

    fn assoc_type_id(&self, name: &str) -> Option<SemanticLocalBindingId> {
        self.assoc_type_ids.get(name).copied()
    }

    fn bind_type_param(&mut self, name: &str) -> SemanticLocalBindingId {
        if let Some(existing) = self.type_param_ids.get(name) {
            return *existing;
        }
        let id = SemanticLocalBindingId(self.next_binding_id);
        self.next_binding_id += 1;
        self.type_param_ids.insert(name.to_string(), id);
        id
    }

    fn bind_lifetime(&mut self, name: &str) -> SemanticLocalBindingId {
        if let Some(existing) = self.lifetime_ids.get(name) {
            return *existing;
        }
        let id = SemanticLocalBindingId(self.next_binding_id);
        self.next_binding_id += 1;
        self.lifetime_ids.insert(name.to_string(), id);
        id
    }

    fn bind_assoc_type(&mut self, name: &str) -> SemanticLocalBindingId {
        if let Some(existing) = self.assoc_type_ids.get(name) {
            return *existing;
        }
        let id = SemanticLocalBindingId(self.next_binding_id);
        self.next_binding_id += 1;
        self.assoc_type_ids.insert(name.to_string(), id);
        id
    }
}

#[derive(Clone, Debug, Default)]
struct ValueScope {
    locals: BTreeSet<String>,
    mutable_locals: BTreeSet<String>,
    params: BTreeSet<String>,
    ownership: BTreeMap<String, OwnershipClass>,
    types: BTreeMap<String, HirType>,
    binding_ids: BTreeMap<String, u64>,
    next_binding_id: u64,
    available_owners: BTreeMap<String, AvailableOwnerBinding>,
    attached_object_names: BTreeSet<String>,
    owner_member_types: BTreeMap<String, BTreeMap<String, HirType>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AvailableOwnerObjectBinding {
    local_name: String,
    ty: HirType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AvailableOwnerBinding {
    local_name: String,
    owner_path: Vec<String>,
    objects: Vec<AvailableOwnerObjectBinding>,
    activation_context_type: Option<HirType>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum LifecycleHookSlot {
    Init,
    InitWithContext,
    Resume,
    ResumeWithContext,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ObjectLifecycleSurface {
    init_context_type: Option<HirType>,
    resume_context_type: Option<HirType>,
}

impl ValueScope {
    fn with_symbol_params(&self, params: &[arcana_hir::HirParam]) -> Self {
        let mut next = self.clone();
        for param in params {
            next.bind_local(
                &param.name,
                matches!(param.mode, Some(arcana_hir::HirParamMode::Edit)),
            );
            next.params.insert(param.name.clone());
        }
        next
    }

    fn with_local(&self, name: &str, mutable: bool) -> Self {
        let mut next = self.clone();
        next.bind_local(name, mutable);
        next
    }

    fn insert(&mut self, name: &str, mutable: bool) {
        self.bind_local(name, mutable);
    }

    fn bind_local(&mut self, name: &str, mutable: bool) -> u64 {
        self.locals.insert(name.to_string());
        if mutable {
            self.mutable_locals.insert(name.to_string());
        }
        let binding_id = self.next_binding_id;
        self.next_binding_id += 1;
        self.binding_ids.insert(name.to_string(), binding_id);
        binding_id
    }

    fn insert_typed(
        &mut self,
        name: &str,
        mutable: bool,
        ownership: OwnershipClass,
        ty: Option<HirType>,
    ) {
        self.insert(name, mutable);
        self.ownership.insert(name.to_string(), ownership);
        if let Some(ty) = ty {
            self.types.insert(name.to_string(), ty);
        } else {
            self.types.remove(name);
        }
    }

    fn contains(&self, name: &str) -> bool {
        self.locals.contains(name)
    }

    fn is_mutable(&self, name: &str) -> bool {
        self.mutable_locals.contains(name)
    }

    fn is_param(&self, name: &str) -> bool {
        self.params.contains(name)
    }

    fn ownership_of(&self, name: &str) -> OwnershipClass {
        self.ownership.get(name).copied().unwrap_or_default()
    }

    fn type_of(&self, name: &str) -> Option<&HirType> {
        self.types.get(name)
    }

    fn binding_id_of(&self, name: &str) -> Option<u64> {
        self.binding_ids.get(name).copied()
    }

    fn attach_owner(&mut self, owner: AvailableOwnerBinding) {
        self.available_owners
            .insert(owner.local_name.clone(), owner);
    }

    fn attach_object_name(&mut self, name: impl Into<String>) {
        self.attached_object_names.insert(name.into());
    }

    fn owner_member_type(&self, owner_name: &str, member: &str) -> Option<&HirType> {
        self.owner_member_types
            .get(owner_name)
            .and_then(|members| members.get(member))
    }

    fn activate_owner(
        &mut self,
        owner: &AvailableOwnerBinding,
        explicit_binding: Option<&str>,
        explicit_binding_mutable: bool,
    ) -> Vec<String> {
        let mut inserted = Vec::new();
        let mut owner_members = BTreeMap::new();
        for object in &owner.objects {
            owner_members.insert(object.local_name.clone(), object.ty.clone());
            if self.attached_object_names.contains(&object.local_name) {
                self.insert_typed(
                    &object.local_name,
                    true,
                    OwnershipClass::Move,
                    Some(object.ty.clone()),
                );
                inserted.push(object.local_name.clone());
            }
        }
        self.insert_typed(
            &owner.local_name,
            false,
            OwnershipClass::Copy,
            None,
        );
        self.owner_member_types
            .insert(owner.local_name.clone(), owner_members.clone());
        inserted.push(owner.local_name.clone());
        if let Some(binding) = explicit_binding {
            self.insert_typed(
                binding,
                explicit_binding_mutable,
                OwnershipClass::Copy,
                None,
            );
            self.owner_member_types
                .insert(binding.to_string(), owner_members);
            inserted.push(binding.to_string());
        }
        inserted
    }
}

impl HirLocalTypeLookup for ValueScope {
    fn contains_local(&self, name: &str) -> bool {
        ValueScope::contains(self, name)
    }

    fn type_text_of(&self, name: &str) -> Option<&str> {
        let _ = name;
        None
    }

    fn type_of(&self, name: &str) -> Option<&HirType> {
        ValueScope::type_of(self, name)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OwnershipClass {
    Copy,
    Move,
    #[default]
    Unknown,
}

impl OwnershipClass {
    const fn is_move_only(self) -> bool {
        matches!(self, Self::Move)
    }
}

fn infer_expr_type(expr: &HirExpr) -> Option<ExprTypeClass> {
    match expr {
        HirExpr::BoolLiteral { .. } => Some(ExprTypeClass::Bool),
        HirExpr::IntLiteral { .. } => Some(ExprTypeClass::Int),
        HirExpr::StrLiteral { .. } => Some(ExprTypeClass::Str),
        HirExpr::Pair { .. } => Some(ExprTypeClass::Pair),
        HirExpr::CollectionLiteral { .. } => Some(ExprTypeClass::Collection),
        HirExpr::Unary { op, .. } => match op {
            HirUnaryOp::Not => Some(ExprTypeClass::Bool),
            HirUnaryOp::Neg | HirUnaryOp::BitNot => Some(ExprTypeClass::Int),
            HirUnaryOp::BorrowRead
            | HirUnaryOp::BorrowMut
            | HirUnaryOp::Deref
            | HirUnaryOp::Weave
            | HirUnaryOp::Split => None,
        },
        HirExpr::Binary { op, .. } => match op {
            HirBinaryOp::And
            | HirBinaryOp::Or
            | HirBinaryOp::EqEq
            | HirBinaryOp::NotEq
            | HirBinaryOp::Lt
            | HirBinaryOp::LtEq
            | HirBinaryOp::Gt
            | HirBinaryOp::GtEq => Some(ExprTypeClass::Bool),
            HirBinaryOp::Sub
            | HirBinaryOp::Mul
            | HirBinaryOp::Div
            | HirBinaryOp::Mod
            | HirBinaryOp::BitOr
            | HirBinaryOp::BitXor
            | HirBinaryOp::BitAnd
            | HirBinaryOp::Shl
            | HirBinaryOp::Shr => Some(ExprTypeClass::Int),
            HirBinaryOp::Add => None,
        },
        _ => None,
    }
}

fn push_type_contract_diagnostic(
    module_path: &Path,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
    message: String,
) {
    diagnostics.push(Diagnostic {
        path: module_path.to_path_buf(),
        line: span.line,
        column: span.column,
        message,
    });
}

fn validate_expected_expr_type(
    module_path: &Path,
    expr: &HirExpr,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
    expected: ExprTypeClass,
    context: &str,
) {
    let Some(actual) = infer_expr_type(expr) else {
        return;
    };
    if actual != expected {
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!(
                "{context} requires {}, found {}",
                expected.label(),
                actual.label()
            ),
        );
    }
}

fn is_tuple_projection_member(member: &str) -> bool {
    matches!(member, "0" | "1")
}

fn binary_op_token(op: HirBinaryOp) -> &'static str {
    match op {
        HirBinaryOp::Or => "or",
        HirBinaryOp::And => "and",
        HirBinaryOp::EqEq => "==",
        HirBinaryOp::NotEq => "!=",
        HirBinaryOp::Lt => "<",
        HirBinaryOp::LtEq => "<=",
        HirBinaryOp::Gt => ">",
        HirBinaryOp::GtEq => ">=",
        HirBinaryOp::BitOr => "|",
        HirBinaryOp::BitXor => "^",
        HirBinaryOp::BitAnd => "&",
        HirBinaryOp::Shl => "<<",
        HirBinaryOp::Shr => "shr",
        HirBinaryOp::Add => "+",
        HirBinaryOp::Sub => "-",
        HirBinaryOp::Mul => "*",
        HirBinaryOp::Div => "/",
        HirBinaryOp::Mod => "%",
    }
}

fn unary_op_token(op: HirUnaryOp) -> &'static str {
    match op {
        HirUnaryOp::Neg => "-",
        HirUnaryOp::Not => "not",
        HirUnaryOp::BitNot => "~",
        HirUnaryOp::BorrowRead => "&",
        HirUnaryOp::BorrowMut => "*",
        HirUnaryOp::Deref => "*",
        HirUnaryOp::Weave => "weave",
        HirUnaryOp::Split => "split",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaceMutability {
    Immutable,
    Mutable,
    Unknown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LocalBorrowState {
    shared_count: usize,
    mutable: bool,
}

#[derive(Clone, Debug, Default)]
struct BorrowFlowState {
    locals: BTreeMap<String, LocalBorrowState>,
    moved_locals: BTreeSet<String>,
    active_cleanup_bindings: BTreeSet<u64>,
}

impl BorrowFlowState {
    fn local_state(&self, name: &str) -> LocalBorrowState {
        self.locals.get(name).copied().unwrap_or_default()
    }

    fn has_shared_borrow(&self, name: &str) -> bool {
        self.local_state(name).shared_count > 0
    }

    fn has_mut_borrow(&self, name: &str) -> bool {
        self.local_state(name).mutable
    }

    fn has_any_borrow(&self, name: &str) -> bool {
        let state = self.local_state(name);
        state.mutable || state.shared_count > 0
    }

    fn has_moved(&self, name: &str) -> bool {
        self.moved_locals.contains(name)
    }

    fn note_shared_borrow(&mut self, name: &str) {
        let state = self.locals.entry(name.to_string()).or_default();
        state.shared_count += 1;
    }

    fn note_mut_borrow(&mut self, name: &str) {
        let state = self.locals.entry(name.to_string()).or_default();
        state.mutable = true;
    }

    fn note_moved(&mut self, name: &str) {
        self.moved_locals.insert(name.to_string());
    }

    fn has_active_cleanup_binding(&self, binding_id: u64) -> bool {
        self.active_cleanup_bindings.contains(&binding_id)
    }

    fn activate_cleanup_binding(&mut self, binding_id: u64) {
        self.active_cleanup_bindings.insert(binding_id);
    }

    fn clear_local(&mut self, name: &str) {
        self.locals.remove(name);
        self.moved_locals.remove(name);
    }

    fn merge_moves_from(&mut self, other: &Self) {
        self.moved_locals.extend(other.moved_locals.iter().cloned());
    }
}

fn expr_place_mutability(expr: &HirExpr, scope: &ValueScope) -> Option<PlaceMutability> {
    match expr {
        HirExpr::Path { segments } if segments.len() == 1 && scope.contains(&segments[0]) => {
            Some(if scope.is_mutable(&segments[0]) {
                PlaceMutability::Mutable
            } else {
                PlaceMutability::Immutable
            })
        }
        HirExpr::MemberAccess { expr, .. } | HirExpr::Index { expr, .. } => {
            expr_place_mutability(expr, scope)
        }
        HirExpr::Unary {
            op: HirUnaryOp::Deref,
            ..
        } => Some(PlaceMutability::Unknown),
        _ => None,
    }
}

fn expr_place_root_local<'a>(expr: &'a HirExpr, scope: &ValueScope) -> Option<&'a str> {
    match expr {
        HirExpr::Path { segments } if segments.len() == 1 && scope.contains(&segments[0]) => {
            Some(segments[0].as_str())
        }
        HirExpr::MemberAccess { expr, .. } | HirExpr::Index { expr, .. } => {
            expr_place_root_local(expr, scope)
        }
        _ => None,
    }
}

fn assign_target_root_local<'a>(
    target: &'a HirAssignTarget,
    scope: &ValueScope,
) -> Option<&'a str> {
    match target {
        HirAssignTarget::Name { text } if scope.contains(text) => Some(text.as_str()),
        HirAssignTarget::MemberAccess { target, .. } | HirAssignTarget::Index { target, .. } => {
            assign_target_root_local(target, scope)
        }
        _ => None,
    }
}

fn ownership_of_builtin_type(name: &str) -> OwnershipClass {
    match builtin_ownership_class(name) {
        Some(BuiltinOwnershipClass::Copy) => OwnershipClass::Copy,
        Some(BuiltinOwnershipClass::Move) => OwnershipClass::Move,
        None => OwnershipClass::Unknown,
    }
}

fn ownership_of_opaque_symbol(symbol: &HirSymbol) -> OwnershipClass {
    match symbol.opaque_policy.map(|policy| policy.ownership) {
        Some(arcana_hir::HirOpaqueOwnershipPolicy::Copy) => OwnershipClass::Copy,
        Some(arcana_hir::HirOpaqueOwnershipPolicy::Move) => OwnershipClass::Move,
        None => OwnershipClass::Unknown,
    }
}

fn opaque_symbol_is_boundary_unsafe(symbol: &HirSymbol) -> bool {
    matches!(
        symbol.opaque_policy.map(|policy| policy.boundary),
        Some(arcana_hir::HirOpaqueBoundaryPolicy::Unsafe)
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpaqueLangFamily {
    FileStreamHandle,
    WindowHandle,
    ImageHandle,
    AppFrameHandle,
    AppSessionHandle,
    WakeHandle,
    AudioDeviceHandle,
    AudioBufferHandle,
    AudioPlaybackHandle,
    ChannelHandle,
    MutexHandle,
    AtomicIntHandle,
    AtomicBoolHandle,
    ArenaHandle,
    ArenaIdHandle,
    FrameArenaHandle,
    FrameIdHandle,
    PoolArenaHandle,
    PoolIdHandle,
    TaskHandle,
    ThreadHandle,
}

impl OpaqueLangFamily {
    const fn name(self) -> &'static str {
        match self {
            Self::FileStreamHandle => "file_stream_handle",
            Self::WindowHandle => "window_handle",
            Self::ImageHandle => "image_handle",
            Self::AppFrameHandle => "app_frame_handle",
            Self::AppSessionHandle => "app_session_handle",
            Self::WakeHandle => "wake_handle",
            Self::AudioDeviceHandle => "audio_device_handle",
            Self::AudioBufferHandle => "audio_buffer_handle",
            Self::AudioPlaybackHandle => "audio_playback_handle",
            Self::ChannelHandle => "channel_handle",
            Self::MutexHandle => "mutex_handle",
            Self::AtomicIntHandle => "atomic_int_handle",
            Self::AtomicBoolHandle => "atomic_bool_handle",
            Self::ArenaHandle => "arena_handle",
            Self::ArenaIdHandle => "arena_id_handle",
            Self::FrameArenaHandle => "frame_arena_handle",
            Self::FrameIdHandle => "frame_id_handle",
            Self::PoolArenaHandle => "pool_arena_handle",
            Self::PoolIdHandle => "pool_id_handle",
            Self::TaskHandle => "task_handle",
            Self::ThreadHandle => "thread_handle",
        }
    }

    const fn expected_ownership(self) -> OwnershipClass {
        match self {
            Self::WakeHandle
            | Self::AtomicIntHandle
            | Self::AtomicBoolHandle
            | Self::ArenaIdHandle
            | Self::FrameIdHandle
            | Self::PoolIdHandle => OwnershipClass::Copy,
            Self::FileStreamHandle
            | Self::WindowHandle
            | Self::ImageHandle
            | Self::AppFrameHandle
            | Self::AppSessionHandle
            | Self::AudioDeviceHandle
            | Self::AudioBufferHandle
            | Self::AudioPlaybackHandle
            | Self::ChannelHandle
            | Self::MutexHandle
            | Self::ArenaHandle
            | Self::FrameArenaHandle
            | Self::PoolArenaHandle
            | Self::TaskHandle
            | Self::ThreadHandle => OwnershipClass::Move,
        }
    }
}

fn opaque_lang_family(name: &str) -> Option<OpaqueLangFamily> {
    match name {
        "file_stream_handle" => Some(OpaqueLangFamily::FileStreamHandle),
        "window_handle" => Some(OpaqueLangFamily::WindowHandle),
        "image_handle" => Some(OpaqueLangFamily::ImageHandle),
        "app_frame_handle" => Some(OpaqueLangFamily::AppFrameHandle),
        "app_session_handle" => Some(OpaqueLangFamily::AppSessionHandle),
        "wake_handle" => Some(OpaqueLangFamily::WakeHandle),
        "audio_device_handle" => Some(OpaqueLangFamily::AudioDeviceHandle),
        "audio_buffer_handle" => Some(OpaqueLangFamily::AudioBufferHandle),
        "audio_playback_handle" => Some(OpaqueLangFamily::AudioPlaybackHandle),
        "channel_handle" => Some(OpaqueLangFamily::ChannelHandle),
        "mutex_handle" => Some(OpaqueLangFamily::MutexHandle),
        "atomic_int_handle" => Some(OpaqueLangFamily::AtomicIntHandle),
        "atomic_bool_handle" => Some(OpaqueLangFamily::AtomicBoolHandle),
        "arena_handle" => Some(OpaqueLangFamily::ArenaHandle),
        "arena_id_handle" => Some(OpaqueLangFamily::ArenaIdHandle),
        "frame_arena_handle" => Some(OpaqueLangFamily::FrameArenaHandle),
        "frame_id_handle" => Some(OpaqueLangFamily::FrameIdHandle),
        "pool_arena_handle" => Some(OpaqueLangFamily::PoolArenaHandle),
        "pool_id_handle" => Some(OpaqueLangFamily::PoolIdHandle),
        "task_handle" => Some(OpaqueLangFamily::TaskHandle),
        "thread_handle" => Some(OpaqueLangFamily::ThreadHandle),
        _ => None,
    }
}

fn validate_package_lang_item_semantics(
    package: &HirWorkspacePackage,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = BTreeMap::<String, (PathBuf, usize, usize)>::new();
    for module in &package.summary.modules {
        let module_path = package
            .module_path(&module.module_id)
            .cloned()
            .unwrap_or_else(|| package.root_dir.join("src").join("unknown.arc"));
        for lang_item in &module.lang_items {
            let Some(family) = opaque_lang_family(&lang_item.name) else {
                continue;
            };
            if let Some((prev_path, prev_line, prev_column)) = seen.insert(
                lang_item.name.clone(),
                (module_path.clone(), lang_item.span.line, lang_item.span.column),
            ) {
                diagnostics.push(Diagnostic {
                    path: module_path.clone(),
                    line: lang_item.span.line,
                    column: lang_item.span.column,
                    message: format!(
                        "opaque family lang item `{}` is declared more than once in package `{}`; first seen at {}:{}:{}",
                        family.name(),
                        package.summary.package_name,
                        prev_path.display(),
                        prev_line,
                        prev_column
                    ),
                });
            }
        }
    }
}

fn infer_type_ownership(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    ty: &HirType,
) -> OwnershipClass {
    match &ty.kind {
        arcana_hir::HirTypeKind::Ref { .. } => OwnershipClass::Copy,
        arcana_hir::HirTypeKind::Path(path) => {
            if path.segments.len() == 1 && type_scope.allows_type_name(&path.segments[0]) {
                return OwnershipClass::Unknown;
            }
            if path.segments.len() == 1 {
                let builtin = ownership_of_builtin_type(&path.segments[0]);
                if builtin != OwnershipClass::Unknown {
                    return builtin;
                }
            }
            let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, &path.segments)
            else {
                return OwnershipClass::Unknown;
            };
            match symbol_ref.symbol.kind {
                HirSymbolKind::OpaqueType => ownership_of_opaque_symbol(symbol_ref.symbol),
                HirSymbolKind::Record | HirSymbolKind::Object | HirSymbolKind::Enum => {
                    OwnershipClass::Move
                }
                _ => OwnershipClass::Unknown,
            }
        }
        arcana_hir::HirTypeKind::Apply { base, .. } => infer_type_ownership(
            workspace,
            resolved_module,
            type_scope,
            &HirType {
                kind: arcana_hir::HirTypeKind::Path(base.clone()),
                span: ty.span,
            },
        ),
        _ => OwnershipClass::Unknown,
    }
}

fn infer_expr_value_type(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    _type_scope: &TypeScope,
    scope: &ValueScope,
    expr: &HirExpr,
) -> Option<HirType> {
    if let HirExpr::MemberAccess { expr, member } = expr {
        if let HirExpr::Path { segments } = expr.as_ref() {
            if segments.len() == 1 && scope.contains(&segments[0]) {
                if let Some(ty) = scope.owner_member_type(&segments[0], member) {
                    return Some(ty.clone());
                }
            }
        }
    }
    infer_receiver_expr_type(workspace, resolved_module, scope, expr)
}

fn infer_expr_ownership(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expr: &HirExpr,
) -> OwnershipClass {
    match expr {
        HirExpr::BoolLiteral { .. } | HirExpr::IntLiteral { .. } => OwnershipClass::Copy,
        HirExpr::StrLiteral { .. } | HirExpr::CollectionLiteral { .. } => OwnershipClass::Move,
        HirExpr::Pair { left, right } => {
            let left_kind =
                infer_expr_ownership(workspace, resolved_module, type_scope, scope, left);
            let right_kind =
                infer_expr_ownership(workspace, resolved_module, type_scope, scope, right);
            if left_kind == OwnershipClass::Copy && right_kind == OwnershipClass::Copy {
                OwnershipClass::Copy
            } else if left_kind.is_move_only() || right_kind.is_move_only() {
                OwnershipClass::Move
            } else {
                OwnershipClass::Unknown
            }
        }
        HirExpr::Path { segments } if segments.len() == 1 && scope.contains(&segments[0]) => {
            scope.ownership_of(&segments[0])
        }
        HirExpr::Unary { op, .. }
            if matches!(op, HirUnaryOp::BorrowRead | HirUnaryOp::BorrowMut) =>
        {
            OwnershipClass::Copy
        }
        _ => infer_expr_value_type(workspace, resolved_module, type_scope, scope, expr)
            .map(|ty| infer_type_ownership(workspace, resolved_module, type_scope, &ty))
            .unwrap_or_default(),
    }
}

fn flatten_callable_expr_path(expr: &HirExpr) -> Option<Vec<String>> {
    match expr {
        HirExpr::GenericApply { expr, .. } => flatten_callable_expr_path(expr),
        _ => flatten_member_expr_path(expr),
    }
}

fn format_bare_method_ambiguity(
    ty: &HirType,
    method_name: &str,
    symbols: &[&HirSymbol],
) -> String {
    let rendered = symbols
        .iter()
        .map(|symbol| symbol.surface_text.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "bare-method qualifier `{method_name}` on `{}` is ambiguous; candidates: {rendered}",
        ty.render()
    )
}

fn lookup_method_symbol_for_type<'a>(
    workspace: &'a HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    ty: &HirType,
    method_name: &str,
) -> Result<Option<&'a HirSymbol>, String> {
    let candidates = lookup_method_candidates_for_hir_type(workspace, resolved_module, ty, method_name)
        .into_iter()
        .map(|candidate| candidate.symbol)
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [] => Ok(None),
        [symbol] => Ok(Some(*symbol)),
        _ => Err(format_bare_method_ambiguity(ty, method_name, &candidates)),
    }
}

fn resolve_qualified_phrase_target_symbol<'a>(
    workspace: &'a HirWorkspaceSummary,
    resolved_module: &'a HirResolvedModule,
    _type_scope: &TypeScope,
    scope: &ValueScope,
    subject: &HirExpr,
    qualifier: &str,
) -> Option<&'a HirSymbol> {
    if qualifier == "call" {
        let path = flatten_callable_expr_path(subject)?;
        return lookup_symbol_path(workspace, resolved_module, &path)
            .map(|resolved| resolved.symbol);
    }

    if let Some(path) = split_simple_path(qualifier) {
        if let Some(resolved) = lookup_symbol_path(workspace, resolved_module, &path) {
            return Some(resolved.symbol);
        }
    }

    if is_identifier_text(qualifier) {
        let subject_ty = infer_receiver_expr_type(workspace, resolved_module, scope, subject)?;
        return lookup_method_symbol_for_type(workspace, resolved_module, &subject_ty, qualifier)
            .ok()
            .flatten();
    }

    None
}

struct ResolvedOwnerActivation<'a> {
    owner: AvailableOwnerBinding,
    context: Option<&'a HirExpr>,
    invalid: Option<String>,
}

fn resolve_owner_activation_expr<'a>(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    expr: &'a HirExpr,
) -> Option<ResolvedOwnerActivation<'a>> {
    let HirExpr::QualifiedPhrase {
        subject,
        args,
        qualifier,
        ..
    } = expr
    else {
        return None;
    };
    if qualifier != "call" {
        return None;
    }
    let path = flatten_callable_expr_path(subject)?;
    let resolved = lookup_symbol_path(workspace, resolved_module, &path)?;
    if resolved.symbol.kind != HirSymbolKind::Owner {
        return None;
    }
    let owner =
        resolve_available_owner_binding(workspace, resolved_workspace, resolved_module, &path)?;
    let invalid = if args
        .iter()
        .any(|arg| matches!(arg, HirPhraseArg::Named { .. }))
    {
        Some("owner activation does not support named arguments".to_string())
    } else if args.len() > 1 {
        Some("owner activation accepts at most one context argument".to_string())
    } else {
        None
    };
    let context = args.first().and_then(|arg| match arg {
        HirPhraseArg::Positional(expr) => Some(expr),
        HirPhraseArg::Named { .. } => None,
    });
    Some(ResolvedOwnerActivation {
        owner,
        context,
        invalid,
    })
}

fn validate_owner_activation_context(
    workspace: &HirWorkspaceSummary,
    _resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    owner_activation: &ResolvedOwnerActivation<'_>,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(context) = owner_activation.context else {
        return;
    };
    let Some(expected_context_type) = owner_activation.owner.activation_context_type.as_ref() else {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: span.line,
            column: span.column,
            message: format!(
                "owner activation `{}` does not use an activation context",
                owner_activation.owner.local_name
            ),
        });
        return;
    };
    let Some(actual_context_type) =
        infer_expr_value_type(workspace, resolved_module, type_scope, scope, context)
    else {
        return;
    };
    let mut semantics = SemanticArena::default();
    let expected_context_id =
        semantics.type_id_for_hir(workspace, resolved_module, type_scope, expected_context_type);
    let actual_context_id =
        semantics.type_id_for_hir(workspace, resolved_module, type_scope, &actual_context_type);
    if actual_context_id != expected_context_id {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: span.line,
            column: span.column,
            message: format!(
                "owner activation `{}` expects context `{}`, found `{}`",
                owner_activation.owner.local_name,
                expected_context_type.render(),
                actual_context_type.render()
            ),
        });
    }
}

fn collect_qualified_phrase_param_exprs<'a>(
    symbol: &'a HirSymbol,
    subject: &'a HirExpr,
    args: &'a [arcana_hir::HirPhraseArg],
    qualifier: &str,
) -> Vec<(&'a arcana_hir::HirParam, &'a HirExpr)> {
    let mut bindings = Vec::new();
    let mut next_positional = 0usize;

    if qualifier != "call" {
        if let Some(param) = symbol.params.first() {
            bindings.push((param, subject));
            next_positional = 1;
        }
    }

    for arg in args {
        match arg {
            arcana_hir::HirPhraseArg::Positional(expr) => {
                if let Some(param) = symbol.params.get(next_positional) {
                    bindings.push((param, expr));
                    next_positional += 1;
                }
            }
            arcana_hir::HirPhraseArg::Named { name, value } => {
                if let Some(param) = symbol.params.iter().find(|param| param.name == *name) {
                    bindings.push((param, value));
                }
            }
        }
    }

    bindings
}

fn validate_bare_method_resolution(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    _type_scope: &TypeScope,
    scope: &ValueScope,
    module_path: &Path,
    subject: &HirExpr,
    qualifier: &str,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !is_identifier_text(qualifier) || qualifier == "call" {
        return;
    }
    let Some(subject_ty) = infer_receiver_expr_type(workspace, resolved_module, scope, subject) else {
        return;
    };
    if let Err(message) =
        lookup_method_symbol_for_type(workspace, resolved_module, &subject_ty, qualifier)
    {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: span.line,
            column: span.column,
            message,
        });
    }
}

fn validate_borrow_operand_place(
    module_path: &Path,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
    mutable: bool,
) {
    let op = if mutable { "&mut" } else { "&" };
    let Some(place_mutability) = expr_place_mutability(expr, scope) else {
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!("operand of `{op}` must be a local place expression"),
        );
        return;
    };

    if let Some(name) = expr_place_root_local(expr, scope) {
        if mutable && !scope.is_mutable(name) {
            push_type_contract_diagnostic(
                module_path,
                span,
                diagnostics,
                format!("cannot mutably borrow immutable local `{name}`"),
            );
            return;
        }
    }

    if mutable && matches!(place_mutability, PlaceMutability::Immutable) {
        let name = expr_place_root_local(expr, scope).unwrap_or("value");
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!("cannot mutably borrow immutable local `{name}`"),
        );
    }
}

fn validate_direct_local_place_access(
    module_path: &Path,
    scope: &ValueScope,
    state: &BorrowFlowState,
    expr: &HirExpr,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(name) = expr_place_root_local(expr, scope) else {
        return;
    };
    if state.has_moved(name) {
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!("use of moved local `{name}`"),
        );
        return;
    }
    if state.has_mut_borrow(name) {
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!("cannot access local `{name}` directly while it is mutably borrowed"),
        );
    }
}

fn validate_place_expr_borrow_flow(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    module_path: &Path,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    state: &mut BorrowFlowState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match expr {
        HirExpr::Path { .. } => {}
        HirExpr::MemberAccess { expr, .. } => {
            validate_place_expr_borrow_flow(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                expr,
                span,
                state,
                diagnostics,
            );
        }
        HirExpr::Index { expr, index } => {
            validate_place_expr_borrow_flow(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                expr,
                span,
                state,
                diagnostics,
            );
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                index,
                span,
                state,
                false,
                diagnostics,
            );
        }
        other => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                other,
                span,
                state,
                false,
                diagnostics,
            );
        }
    }
}

fn validate_borrow_op_conflict(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    module_path: &Path,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    state: &mut BorrowFlowState,
    diagnostics: &mut Vec<Diagnostic>,
    mutable: bool,
) {
    validate_place_expr_borrow_flow(
        workspace,
        resolved_module,
        type_scope,
        module_path,
        scope,
        expr,
        span,
        state,
        diagnostics,
    );
    let Some(name) = expr_place_root_local(expr, scope) else {
        return;
    };
    if state.has_moved(name) {
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!("use of moved local `{name}`"),
        );
        return;
    }
    if mutable {
        if state.has_mut_borrow(name) {
            push_type_contract_diagnostic(
                module_path,
                span,
                diagnostics,
                format!("cannot mutably borrow `{name}` while it is already mutably borrowed"),
            );
        } else if state.has_shared_borrow(name) {
            push_type_contract_diagnostic(
                module_path,
                span,
                diagnostics,
                format!("cannot mutably borrow `{name}` while it is already borrowed"),
            );
        }
        state.note_mut_borrow(name);
    } else {
        if state.has_mut_borrow(name) {
            push_type_contract_diagnostic(
                module_path,
                span,
                diagnostics,
                format!("cannot borrow `{name}` while it is mutably borrowed"),
            );
        }
        state.note_shared_borrow(name);
    }
}

fn validate_call_param_mode_flow(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    subject: &HirExpr,
    args: &[arcana_hir::HirPhraseArg],
    qualifier: &str,
    span: Span,
    state: &mut BorrowFlowState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(symbol) = resolve_qualified_phrase_target_symbol(
        workspace,
        resolved_module,
        type_scope,
        scope,
        subject,
        qualifier,
    ) else {
        return;
    };

    for (param, expr) in collect_qualified_phrase_param_exprs(symbol, subject, args, qualifier) {
        match param.mode {
            Some(arcana_hir::HirParamMode::Read) | None => {
                if let Some(name) = expr_place_root_local(expr, scope) {
                    if state.has_moved(name) {
                        push_type_contract_diagnostic(
                            module_path,
                            span,
                            diagnostics,
                            format!("use of moved local `{name}`"),
                        );
                    } else if state.has_mut_borrow(name) {
                        push_type_contract_diagnostic(
                            module_path,
                            span,
                            diagnostics,
                            format!(
                                "cannot pass local `{name}` to read parameter `{}` while it is mutably borrowed",
                                param.name
                            ),
                        );
                    } else {
                        state.note_shared_borrow(name);
                    }
                }
            }
            Some(arcana_hir::HirParamMode::Edit) => {
                let Some(mutability) = expr_place_mutability(expr, scope) else {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!(
                            "argument for edit parameter `{}` must be a local place expression",
                            param.name
                        ),
                    );
                    continue;
                };
                let Some(name) = expr_place_root_local(expr, scope) else {
                    continue;
                };
                if state.has_moved(name) {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!("use of moved local `{name}`"),
                    );
                } else if matches!(mutability, PlaceMutability::Immutable) {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!(
                            "cannot pass immutable local `{name}` to edit parameter `{}`",
                            param.name
                        ),
                    );
                } else if state.has_mut_borrow(name) {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!(
                            "cannot pass local `{name}` to edit parameter `{}` while it is already mutably borrowed",
                            param.name
                        ),
                    );
                } else if state.has_shared_borrow(name) {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!(
                            "cannot pass local `{name}` to edit parameter `{}` while it is already borrowed",
                            param.name
                        ),
                    );
                } else {
                    state.note_mut_borrow(name);
                }
            }
            Some(arcana_hir::HirParamMode::Take) => {
                let Some(name) = expr_place_root_local(expr, scope) else {
                    continue;
                };
                if !scope.ownership_of(name).is_move_only() {
                    continue;
                }
                if state.has_moved(name) {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!("use of moved local `{name}`"),
                    );
                } else if state.has_any_borrow(name) {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!("cannot move local `{name}` while it is borrowed"),
                    );
                } else if scope
                    .binding_id_of(name)
                    .is_some_and(|binding_id| state.has_active_cleanup_binding(binding_id))
                {
                    push_type_contract_diagnostic(
                        module_path,
                        span,
                        diagnostics,
                        format!("cleanup subject `{name}` cannot be moved after activation"),
                    );
                } else {
                    state.note_moved(name);
                }
            }
        }
    }
}

fn note_qualified_phrase_moves(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    scope: &ValueScope,
    subject: &HirExpr,
    args: &[arcana_hir::HirPhraseArg],
    qualifier: &str,
    state: &mut BorrowFlowState,
) {
    let Some(symbol) = resolve_qualified_phrase_target_symbol(
        workspace,
        resolved_module,
        type_scope,
        scope,
        subject,
        qualifier,
    ) else {
        return;
    };

    for (param, expr) in collect_qualified_phrase_param_exprs(symbol, subject, args, qualifier) {
        if !matches!(param.mode, Some(arcana_hir::HirParamMode::Take)) {
            continue;
        }
        let Some(name) = expr_place_root_local(expr, scope) else {
            continue;
        };
        if scope.ownership_of(name).is_move_only()
            && !scope
                .binding_id_of(name)
                .is_some_and(|binding_id| state.has_active_cleanup_binding(binding_id))
        {
            state.note_moved(name);
        }
    }
}

fn validate_expr_borrow_flow(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    module_path: &Path,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    state: &BorrowFlowState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut temp_state = state.clone();
    validate_expr_borrow_flow_inner(
        workspace,
        resolved_module,
        type_scope,
        module_path,
        scope,
        expr,
        span,
        &mut temp_state,
        false,
        diagnostics,
    );
}

fn validate_expr_borrow_flow_inner(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    module_path: &Path,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    state: &mut BorrowFlowState,
    within_place: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match expr {
        HirExpr::Path { segments } => {
            if !within_place && segments.len() == 1 && scope.contains(&segments[0]) {
                validate_direct_local_place_access(
                    module_path,
                    scope,
                    state,
                    expr,
                    span,
                    diagnostics,
                );
            }
        }
        HirExpr::BoolLiteral { .. } | HirExpr::IntLiteral { .. } | HirExpr::StrLiteral { .. } => {}
        HirExpr::Pair { left, right } => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                left,
                span,
                state,
                false,
                diagnostics,
            );
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                right,
                span,
                state,
                false,
                diagnostics,
            );
        }
        HirExpr::CollectionLiteral { items } => {
            for item in items {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    item,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
        }
        HirExpr::Match { subject, arms } => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                subject,
                span,
                state,
                false,
                diagnostics,
            );
            for arm in arms {
                let mut arm_state = state.clone();
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    &arm.value,
                    arm.span,
                    &mut arm_state,
                    false,
                    diagnostics,
                );
            }
        }
        HirExpr::Chain { steps, .. } => {
            for step in steps {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    &step.stage,
                    span,
                    state,
                    false,
                    diagnostics,
                );
                for arg in &step.bind_args {
                    validate_expr_borrow_flow_inner(
                        workspace,
                        resolved_module,
                        type_scope,
                        module_path,
                        scope,
                        arg,
                        span,
                        state,
                        false,
                        diagnostics,
                    );
                }
            }
        }
        HirExpr::MemoryPhrase {
            arena,
            init_args,
            constructor,
            attached,
            ..
        } => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                arena,
                span,
                state,
                false,
                diagnostics,
            );
            for arg in init_args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        validate_expr_borrow_flow_inner(
                            workspace,
                            resolved_module,
                            type_scope,
                            module_path,
                            scope,
                            expr,
                            span,
                            state,
                            false,
                            diagnostics,
                        );
                    }
                }
            }
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                constructor,
                span,
                state,
                false,
                diagnostics,
            );
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, span, .. }
                    | HirHeaderAttachment::Chain {
                        expr: value, span, ..
                    } => validate_expr_borrow_flow_inner(
                        workspace,
                        resolved_module,
                        type_scope,
                        module_path,
                        scope,
                        value,
                        *span,
                        state,
                        false,
                        diagnostics,
                    ),
                }
            }
        }
        HirExpr::GenericApply { expr, .. } | HirExpr::Await { expr } => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                expr,
                span,
                state,
                false,
                diagnostics,
            );
        }
        HirExpr::QualifiedPhrase {
            subject,
            args,
            qualifier,
            attached,
            ..
        } => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                subject,
                span,
                state,
                false,
                diagnostics,
            );
            for arg in args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        validate_expr_borrow_flow_inner(
                            workspace,
                            resolved_module,
                            type_scope,
                            module_path,
                            scope,
                            expr,
                            span,
                            state,
                            false,
                            diagnostics,
                        );
                    }
                }
            }
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, span, .. }
                    | HirHeaderAttachment::Chain {
                        expr: value, span, ..
                    } => validate_expr_borrow_flow_inner(
                        workspace,
                        resolved_module,
                        type_scope,
                        module_path,
                        scope,
                        value,
                        *span,
                        state,
                        false,
                        diagnostics,
                    ),
                }
            }
            validate_call_param_mode_flow(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                subject,
                args,
                qualifier,
                span,
                state,
                diagnostics,
            );
        }
        HirExpr::Unary { op, expr } => match op {
            HirUnaryOp::BorrowRead => validate_borrow_op_conflict(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                expr,
                span,
                state,
                diagnostics,
                false,
            ),
            HirUnaryOp::BorrowMut => validate_borrow_op_conflict(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                expr,
                span,
                state,
                diagnostics,
                true,
            ),
            HirUnaryOp::Deref | HirUnaryOp::Neg | HirUnaryOp::Not | HirUnaryOp::BitNot => {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    expr,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
            HirUnaryOp::Weave | HirUnaryOp::Split => {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    expr,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
        },
        HirExpr::Binary { left, right, .. } => {
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                left,
                span,
                state,
                false,
                diagnostics,
            );
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                right,
                span,
                state,
                false,
                diagnostics,
            );
        }
        HirExpr::MemberAccess { expr: target, .. } => {
            if !within_place {
                validate_direct_local_place_access(
                    module_path,
                    scope,
                    state,
                    expr,
                    span,
                    diagnostics,
                );
            }
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                target,
                span,
                state,
                true,
                diagnostics,
            );
        }
        HirExpr::Index {
            expr: target,
            index,
        } => {
            if !within_place {
                validate_direct_local_place_access(
                    module_path,
                    scope,
                    state,
                    expr,
                    span,
                    diagnostics,
                );
            }
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                target,
                span,
                state,
                true,
                diagnostics,
            );
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                index,
                span,
                state,
                false,
                diagnostics,
            );
        }
        HirExpr::Slice {
            expr: target,
            start,
            end,
            ..
        } => {
            if !within_place {
                validate_direct_local_place_access(
                    module_path,
                    scope,
                    state,
                    expr,
                    span,
                    diagnostics,
                );
            }
            validate_expr_borrow_flow_inner(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                target,
                span,
                state,
                true,
                diagnostics,
            );
            if let Some(start) = start {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    start,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
            if let Some(end) = end {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    end,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
        }
        HirExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    start,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
            if let Some(end) = end {
                validate_expr_borrow_flow_inner(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    end,
                    span,
                    state,
                    false,
                    diagnostics,
                );
            }
        }
    }
}

fn collect_expr_local_borrows(
    expr: &HirExpr,
    scope: &ValueScope,
    borrows: &mut Vec<(String, bool)>,
) {
    match expr {
        HirExpr::Unary { op, expr } => {
            if matches!(op, HirUnaryOp::BorrowRead | HirUnaryOp::BorrowMut) {
                if let Some(name) = expr_place_root_local(expr, scope) {
                    borrows.push((name.to_string(), matches!(op, HirUnaryOp::BorrowMut)));
                }
            }
            collect_expr_local_borrows(expr, scope, borrows);
        }
        HirExpr::Pair { left, right } | HirExpr::Binary { left, right, .. } => {
            collect_expr_local_borrows(left, scope, borrows);
            collect_expr_local_borrows(right, scope, borrows);
        }
        HirExpr::CollectionLiteral { items } => {
            for item in items {
                collect_expr_local_borrows(item, scope, borrows);
            }
        }
        HirExpr::Match { subject, arms } => {
            collect_expr_local_borrows(subject, scope, borrows);
            for arm in arms {
                collect_expr_local_borrows(&arm.value, scope, borrows);
            }
        }
        HirExpr::Chain { steps, .. } => {
            for step in steps {
                collect_expr_local_borrows(&step.stage, scope, borrows);
                for arg in &step.bind_args {
                    collect_expr_local_borrows(arg, scope, borrows);
                }
            }
        }
        HirExpr::MemoryPhrase {
            arena,
            init_args,
            constructor,
            attached,
            ..
        } => {
            collect_expr_local_borrows(arena, scope, borrows);
            for arg in init_args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        collect_expr_local_borrows(expr, scope, borrows);
                    }
                }
            }
            collect_expr_local_borrows(constructor, scope, borrows);
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, .. }
                    | HirHeaderAttachment::Chain { expr: value, .. } => {
                        collect_expr_local_borrows(value, scope, borrows);
                    }
                }
            }
        }
        HirExpr::GenericApply { expr, .. }
        | HirExpr::Await { expr }
        | HirExpr::MemberAccess { expr, .. } => collect_expr_local_borrows(expr, scope, borrows),
        HirExpr::QualifiedPhrase {
            subject,
            args,
            attached,
            ..
        } => {
            collect_expr_local_borrows(subject, scope, borrows);
            for arg in args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        collect_expr_local_borrows(expr, scope, borrows);
                    }
                }
            }
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, .. }
                    | HirHeaderAttachment::Chain { expr: value, .. } => {
                        collect_expr_local_borrows(value, scope, borrows);
                    }
                }
            }
        }
        HirExpr::Index { expr, index } => {
            collect_expr_local_borrows(expr, scope, borrows);
            collect_expr_local_borrows(index, scope, borrows);
        }
        HirExpr::Slice {
            expr, start, end, ..
        } => {
            collect_expr_local_borrows(expr, scope, borrows);
            if let Some(start) = start {
                collect_expr_local_borrows(start, scope, borrows);
            }
            if let Some(end) = end {
                collect_expr_local_borrows(end, scope, borrows);
            }
        }
        HirExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                collect_expr_local_borrows(start, scope, borrows);
            }
            if let Some(end) = end {
                collect_expr_local_borrows(end, scope, borrows);
            }
        }
        HirExpr::Path { .. }
        | HirExpr::BoolLiteral { .. }
        | HirExpr::IntLiteral { .. }
        | HirExpr::StrLiteral { .. } => {}
    }
}

fn note_escaping_expr_borrows(state: &mut BorrowFlowState, expr: &HirExpr, scope: &ValueScope) {
    let mut borrows = Vec::new();
    collect_expr_local_borrows(expr, scope, &mut borrows);
    for (name, mutable) in borrows {
        if mutable {
            state.note_mut_borrow(&name);
        } else {
            state.note_shared_borrow(&name);
        }
    }
}

fn note_expr_moves(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expr: &HirExpr,
    state: &mut BorrowFlowState,
) {
    match expr {
        HirExpr::QualifiedPhrase {
            subject,
            args,
            qualifier,
            ..
        } => {
            note_qualified_phrase_moves(
                workspace,
                resolved_module,
                type_scope,
                scope,
                subject,
                args,
                qualifier,
                state,
            );
            note_expr_moves(
                workspace,
                resolved_module,
                type_scope,
                scope,
                subject,
                state,
            );
            for arg in args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        note_expr_moves(workspace, resolved_module, type_scope, scope, expr, state);
                    }
                }
            }
        }
        HirExpr::GenericApply { expr, .. }
        | HirExpr::Await { expr }
        | HirExpr::MemberAccess { expr, .. } => {
            note_expr_moves(workspace, resolved_module, type_scope, scope, expr, state);
        }
        HirExpr::Unary { expr, .. } => {
            note_expr_moves(workspace, resolved_module, type_scope, scope, expr, state);
        }
        HirExpr::Pair { left, right } | HirExpr::Binary { left, right, .. } => {
            note_expr_moves(workspace, resolved_module, type_scope, scope, left, state);
            note_expr_moves(workspace, resolved_module, type_scope, scope, right, state);
        }
        HirExpr::CollectionLiteral { items } => {
            for item in items {
                note_expr_moves(workspace, resolved_module, type_scope, scope, item, state);
            }
        }
        HirExpr::Match { subject, arms } => {
            note_expr_moves(
                workspace,
                resolved_module,
                type_scope,
                scope,
                subject,
                state,
            );
            for arm in arms {
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    &arm.value,
                    state,
                );
            }
        }
        HirExpr::Chain { steps, .. } => {
            for step in steps {
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    &step.stage,
                    state,
                );
                for arg in &step.bind_args {
                    note_expr_moves(workspace, resolved_module, type_scope, scope, arg, state);
                }
            }
        }
        HirExpr::MemoryPhrase {
            arena,
            init_args,
            constructor,
            attached,
            ..
        } => {
            note_expr_moves(workspace, resolved_module, type_scope, scope, arena, state);
            for arg in init_args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        note_expr_moves(workspace, resolved_module, type_scope, scope, expr, state);
                    }
                }
            }
            note_expr_moves(
                workspace,
                resolved_module,
                type_scope,
                scope,
                constructor,
                state,
            );
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, .. }
                    | HirHeaderAttachment::Chain { expr: value, .. } => {
                        note_expr_moves(
                            workspace,
                            resolved_module,
                            type_scope,
                            scope,
                            value,
                            state,
                        );
                    }
                }
            }
        }
        HirExpr::Index { expr, index } => {
            note_expr_moves(workspace, resolved_module, type_scope, scope, expr, state);
            note_expr_moves(workspace, resolved_module, type_scope, scope, index, state);
        }
        HirExpr::Slice {
            expr, start, end, ..
        } => {
            note_expr_moves(workspace, resolved_module, type_scope, scope, expr, state);
            if let Some(start) = start {
                note_expr_moves(workspace, resolved_module, type_scope, scope, start, state);
            }
            if let Some(end) = end {
                note_expr_moves(workspace, resolved_module, type_scope, scope, end, state);
            }
        }
        HirExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                note_expr_moves(workspace, resolved_module, type_scope, scope, start, state);
            }
            if let Some(end) = end {
                note_expr_moves(workspace, resolved_module, type_scope, scope, end, state);
            }
        }
        HirExpr::Path { .. }
        | HirExpr::BoolLiteral { .. }
        | HirExpr::IntLiteral { .. }
        | HirExpr::StrLiteral { .. } => {}
    }
}

fn collect_returned_local_borrows(
    expr: &HirExpr,
    scope: &ValueScope,
    roots: &mut BTreeSet<String>,
) {
    match expr {
        HirExpr::Unary { op, expr } => {
            if matches!(op, HirUnaryOp::BorrowRead | HirUnaryOp::BorrowMut) {
                if let Some(name) = expr_place_root_local(expr, scope) {
                    roots.insert(name.to_string());
                }
            }
            collect_returned_local_borrows(expr, scope, roots);
        }
        HirExpr::Pair { left, right } | HirExpr::Binary { left, right, .. } => {
            collect_returned_local_borrows(left, scope, roots);
            collect_returned_local_borrows(right, scope, roots);
        }
        HirExpr::CollectionLiteral { items } => {
            for item in items {
                collect_returned_local_borrows(item, scope, roots);
            }
        }
        HirExpr::Match { subject, arms } => {
            collect_returned_local_borrows(subject, scope, roots);
            for arm in arms {
                collect_returned_local_borrows(&arm.value, scope, roots);
            }
        }
        HirExpr::Chain { steps, .. } => {
            for step in steps {
                collect_returned_local_borrows(&step.stage, scope, roots);
                for arg in &step.bind_args {
                    collect_returned_local_borrows(arg, scope, roots);
                }
            }
        }
        HirExpr::MemoryPhrase {
            arena,
            init_args,
            constructor,
            attached,
            ..
        } => {
            collect_returned_local_borrows(arena, scope, roots);
            for arg in init_args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        collect_returned_local_borrows(expr, scope, roots);
                    }
                }
            }
            collect_returned_local_borrows(constructor, scope, roots);
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, .. }
                    | HirHeaderAttachment::Chain { expr: value, .. } => {
                        collect_returned_local_borrows(value, scope, roots);
                    }
                }
            }
        }
        HirExpr::GenericApply { expr, .. }
        | HirExpr::Await { expr }
        | HirExpr::MemberAccess { expr, .. } => collect_returned_local_borrows(expr, scope, roots),
        HirExpr::QualifiedPhrase {
            subject,
            args,
            attached,
            ..
        } => {
            collect_returned_local_borrows(subject, scope, roots);
            for arg in args {
                match arg {
                    arcana_hir::HirPhraseArg::Positional(expr)
                    | arcana_hir::HirPhraseArg::Named { value: expr, .. } => {
                        collect_returned_local_borrows(expr, scope, roots);
                    }
                }
            }
            for attachment in attached {
                match attachment {
                    HirHeaderAttachment::Named { value, .. }
                    | HirHeaderAttachment::Chain { expr: value, .. } => {
                        collect_returned_local_borrows(value, scope, roots);
                    }
                }
            }
        }
        HirExpr::Index { expr, index } => {
            collect_returned_local_borrows(expr, scope, roots);
            collect_returned_local_borrows(index, scope, roots);
        }
        HirExpr::Slice {
            expr, start, end, ..
        } => {
            collect_returned_local_borrows(expr, scope, roots);
            if let Some(start) = start {
                collect_returned_local_borrows(start, scope, roots);
            }
            if let Some(end) = end {
                collect_returned_local_borrows(end, scope, roots);
            }
        }
        HirExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                collect_returned_local_borrows(start, scope, roots);
            }
            if let Some(end) = end {
                collect_returned_local_borrows(end, scope, roots);
            }
        }
        HirExpr::Path { .. }
        | HirExpr::BoolLiteral { .. }
        | HirExpr::IntLiteral { .. }
        | HirExpr::StrLiteral { .. } => {}
    }
}

fn validate_return_borrow_ties(
    module_path: &Path,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut roots = BTreeSet::new();
    collect_returned_local_borrows(expr, scope, &mut roots);
    for root in roots {
        if scope.is_param(&root) {
            continue;
        }
        push_type_contract_diagnostic(
            module_path,
            span,
            diagnostics,
            format!(
                "returned reference must be tied to input lifetimes; local `{root}` does not live long enough"
            ),
        );
    }
}

fn validate_opaque_constructor_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    subject: &HirExpr,
    qualifier: &str,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if qualifier != "call" {
        return;
    }
    let Some(symbol) = resolve_qualified_phrase_target_symbol(
        workspace,
        resolved_module,
        type_scope,
        scope,
        subject,
        qualifier,
    ) else {
        return;
    };
    if symbol.kind != HirSymbolKind::OpaqueType {
        return;
    }
    diagnostics.push(Diagnostic {
        path: module_path.to_path_buf(),
        line: span.line,
        column: span.column,
        message: format!("opaque type `{}` is not constructible", symbol.name),
    });
}

pub fn check_sources<'a, I>(sources: I) -> Result<CheckSummary, String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut summary = CheckSummary::default();
    for (index, source) in sources.into_iter().enumerate() {
        let hir = lower_module_text(format!("memory.module.{index}"), source)?;
        summary.module_count += 1;
        summary.non_empty_lines += hir.non_empty_line_count;
        summary.directive_count += hir.directives.len();
        summary.symbol_count += hir.symbols.len();
    }
    Ok(summary)
}

pub fn check_path(path: &Path) -> Result<CheckSummary, String> {
    let metadata =
        fs::metadata(path).map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
    if metadata.is_file() {
        return check_file(path);
    }
    if !metadata.is_dir() {
        return Err(format!("`{}` is not a file or directory", path.display()));
    }

    Ok(check_workspace_path(path)?.into_summary())
}

pub fn load_workspace_hir(path: &Path) -> Result<HirWorkspaceSummary, String> {
    let root_dir = canonicalize_workspace_dir(path)?;
    load_package_workspace_hir(&root_dir)
}

pub fn check_workspace_path(path: &Path) -> Result<CheckedWorkspace, String> {
    let root_dir = canonicalize_workspace_dir(path)?;
    let workspace = load_package_workspace_hir(&root_dir)?;
    validate_packages(workspace)
}

pub fn check_workspace_graph(graph: &WorkspaceGraph) -> Result<CheckedWorkspace, String> {
    let workspace = load_package_workspace_hir_from_graph(&graph.root_dir, graph)?;
    validate_packages(workspace)
}

pub fn compute_member_fingerprints_for_checked_workspace(
    graph: &WorkspaceGraph,
    checked: &CheckedWorkspace,
) -> Result<WorkspaceFingerprints, String> {
    api_fingerprint::compute_member_fingerprints_for_checked_workspace(graph, checked)
}

pub fn compute_member_fingerprints(
    graph: &WorkspaceGraph,
) -> Result<WorkspaceFingerprints, String> {
    let workspace = load_package_workspace_hir_from_graph(&graph.root_dir, graph)?;
    let resolved_workspace = resolve_workspace(&workspace)
        .map_err(|errors| render_resolution_errors(&workspace, errors))?;
    api_fingerprint::compute_member_fingerprints_for_workspace(
        graph,
        &workspace,
        &resolved_workspace,
    )
}

pub fn lower_to_hir(summary: &CheckSummary) -> HirModule {
    HirModule {
        symbol_count: summary.symbol_count.max(summary.module_count),
        item_count: summary.non_empty_lines + summary.directive_count,
    }
}

fn check_file(path: &Path) -> Result<CheckSummary, String> {
    let source = fs::read_to_string(path)
        .map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
    let hir = lower_module_text(path.display().to_string(), &source)
        .map_err(|err| format!("{}: {err}", path.display()))?;
    Ok(CheckSummary {
        package_count: 0,
        module_count: 1,
        non_empty_lines: hir.non_empty_line_count,
        directive_count: hir.directives.len(),
        symbol_count: hir.symbols.len(),
    })
}

fn canonicalize_workspace_dir(path: &Path) -> Result<PathBuf, String> {
    let metadata =
        fs::metadata(path).map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
    if !metadata.is_dir() {
        return Err(format!(
            "workspace HIR requires a grimoire or workspace directory, got `{}`",
            path.display()
        ));
    }

    let root_dir = fs::canonicalize(path)
        .map_err(|err| format!("failed to open `{}`: {err}", path.display()))?;
    let manifest_path = root_dir.join("book.toml");
    if !manifest_path.is_file() {
        return Err(format!(
            "`{}` does not contain a `book.toml` manifest",
            root_dir.display()
        ));
    }

    Ok(root_dir)
}

fn render_resolution_errors(
    workspace: &HirWorkspaceSummary,
    errors: Vec<arcana_hir::HirResolutionError>,
) -> String {
    errors
        .into_iter()
        .map(|error| {
            let package = workspace.package(&error.package_name);
            let path = package
                .and_then(|package| package.module_path(&error.source_module_id))
                .cloned()
                .unwrap_or_else(|| {
                    package
                        .map(|package| package.root_dir.join("src").join("unknown.arc"))
                        .unwrap_or_else(|| PathBuf::from("unknown.arc"))
                });
            Diagnostic {
                path,
                line: error.span.line,
                column: error.span.column,
                message: error.message,
            }
            .render()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn validate_packages(workspace: HirWorkspaceSummary) -> Result<CheckedWorkspace, String> {
    let summary = summarize_workspace(&workspace);
    let resolved_workspace = resolve_workspace(&workspace)
        .map_err(|errors| render_resolution_errors(&workspace, errors))?;
    let diagnostics = validate_hir_semantics(&workspace, &resolved_workspace);

    if diagnostics.is_empty() {
        return Ok(CheckedWorkspace {
            summary,
            workspace,
            resolved_workspace,
        });
    }

    Err(render_diagnostics(diagnostics))
}

fn summarize_workspace(workspace: &HirWorkspaceSummary) -> CheckSummary {
    let mut summary = CheckSummary {
        package_count: workspace.package_count(),
        ..CheckSummary::default()
    };

    for package in workspace.packages.values() {
        for module in &package.summary.modules {
            summary.module_count += 1;
            summary.non_empty_lines += module.non_empty_line_count;
            summary.directive_count += module.directives.len();
            summary.symbol_count += module.symbols.len();
        }
    }

    summary
}

fn render_diagnostics(mut diagnostics: Vec<Diagnostic>) -> String {
    diagnostics.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.column.cmp(&right.column))
            .then_with(|| left.message.cmp(&right.message))
    });
    diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.render())
        .collect::<Vec<_>>()
        .join("\n")
}

fn validate_hir_semantics(
    workspace: &HirWorkspaceSummary,
    resolved: &HirResolvedWorkspace,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (package_name, package) in &workspace.packages {
        let Some(resolved_package) = resolved.package(package_name) else {
            continue;
        };
        validate_package_lang_item_semantics(package, &mut diagnostics);
        for module in &package.summary.modules {
            let Some(resolved_module) = resolved_package.module(&module.module_id) else {
                continue;
            };
            validate_module_semantics(
                workspace,
                resolved,
                package,
                module,
                resolved_module,
                &mut diagnostics,
            );
        }
    }
    diagnostics
}

fn validate_module_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    resolved_module: &HirResolvedModule,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let module_path = package
        .module_path(&module.module_id)
        .cloned()
        .unwrap_or_else(|| package.root_dir.join("src").join("unknown.arc"));

    for lang_item in &module.lang_items {
        let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, lang_item.target.as_slice()) else {
            diagnostics.push(Diagnostic {
                path: module_path.clone(),
                line: lang_item.span.line,
                column: lang_item.span.column,
                message: format!(
                    "unresolved `lang` item target `{}` for `{}`",
                    lang_item.target.join("."),
                    lang_item.name
                ),
            });
            continue;
        };
        if let Some(family) = opaque_lang_family(&lang_item.name) {
            if symbol_ref.symbol.kind != HirSymbolKind::OpaqueType {
                diagnostics.push(Diagnostic {
                    path: module_path.clone(),
                    line: lang_item.span.line,
                    column: lang_item.span.column,
                    message: format!(
                        "opaque family lang item `{}` must target an opaque type, found `{}`",
                        family.name(),
                        symbol_ref.symbol.kind.as_str()
                    ),
                });
                continue;
            }
            let actual_ownership = ownership_of_opaque_symbol(symbol_ref.symbol);
            let expected_ownership = family.expected_ownership();
            if actual_ownership != expected_ownership {
                diagnostics.push(Diagnostic {
                    path: module_path.clone(),
                    line: lang_item.span.line,
                    column: lang_item.span.column,
                    message: format!(
                        "opaque family lang item `{}` must target a {} opaque type",
                        family.name(),
                        match expected_ownership {
                            OwnershipClass::Copy => "copy",
                            OwnershipClass::Move => "move",
                            OwnershipClass::Unknown => "known-ownership",
                        }
                    ),
                });
            }
            if !opaque_symbol_is_boundary_unsafe(symbol_ref.symbol) {
                diagnostics.push(Diagnostic {
                    path: module_path.clone(),
                    line: lang_item.span.line,
                    column: lang_item.span.column,
                    message: format!(
                        "opaque family lang item `{}` must target a boundary_unsafe opaque type",
                        family.name()
                    ),
                });
            }
        }
    }

    for symbol in &module.symbols {
        if is_runtime_main_entry_symbol(&package.summary.package_name, &module.module_id, symbol) {
            if let Err(message) = validate_runtime_main_entry_symbol(symbol) {
                diagnostics.push(Diagnostic {
                    path: module_path.clone(),
                    line: symbol.span.line,
                    column: symbol.span.column,
                    message,
                });
            }
        }
        if symbol.kind == HirSymbolKind::OpaqueType && is_builtin_type_name(&symbol.name) {
            diagnostics.push(Diagnostic {
                path: module_path.clone(),
                line: symbol.span.line,
                column: symbol.span.column,
                message: format!(
                    "opaque type `{}` conflicts with reserved builtin type name",
                    symbol.name
                ),
            });
        }
        validate_symbol_surface_types(
            workspace,
            resolved_workspace,
            resolved_module,
            &module_path,
            symbol,
            &TypeScope::default(),
            diagnostics,
        );
        let symbol_scope = TypeScope::default().with_params(&symbol.type_params);
        validate_boundary_symbol_contract(
            workspace,
            resolved_workspace,
            resolved_module,
            &module_path,
            symbol,
            &symbol_scope,
            None,
            &BTreeMap::new(),
            diagnostics,
        );
        validate_symbol_value_semantics(
            workspace,
            resolved_workspace,
            resolved_module,
            &module_path,
            symbol,
            &TypeScope::default(),
            &ValueScope::default(),
            diagnostics,
        );
    }
    for impl_decl in &module.impls {
        validate_impl_surface_types(
            workspace,
            resolved_workspace,
            resolved_module,
            &module_path,
            impl_decl,
            diagnostics,
        );
        validate_impl_value_semantics(
            workspace,
            resolved_workspace,
            resolved_module,
            &module_path,
            impl_decl,
            diagnostics,
        );
    }
}

fn validate_symbol_surface_types(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    symbol: &HirSymbol,
    inherited_scope: &TypeScope,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let scope = inherited_scope.with_params(&symbol.type_params);
    if symbol.kind == HirSymbolKind::OpaqueType && symbol.opaque_policy.is_none() {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: symbol.span.line,
            column: symbol.span.column,
            message: format!(
                "opaque type `{}` is missing required policy atoms",
                symbol.name
            ),
        });
    }
    for param in &symbol.params {
        validate_type_surface(
            workspace,
            resolved_module,
            module_path,
            &scope,
            &param.ty,
            symbol.span,
            &format!("parameter type `{}`", param.name),
            diagnostics,
        );
    }
    if let Some(return_type) = &symbol.return_type {
        validate_type_surface(
            workspace,
            resolved_module,
            module_path,
            &scope,
            return_type,
            symbol.span,
            "return type",
            diagnostics,
        );
        for lifetime in collect_hir_type_refs(return_type).lifetimes {
            if lifetime != "'static" && symbol.params.is_empty() {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: symbol.span.line,
                    column: symbol.span.column,
                    message: format!(
                        "return lifetime `{lifetime}` must be tied to an input parameter"
                    ),
                });
            }
        }
    }
    if let Some(where_clause) = &symbol.where_clause {
        validate_where_clause_surface(
            workspace,
            resolved_module,
            module_path,
            &scope,
            where_clause,
            symbol.span,
            diagnostics,
        );
    }
    match &symbol.body {
        HirSymbolBody::None => {}
        HirSymbolBody::Record { fields } => {
            for field in fields {
                validate_type_surface(
                    workspace,
                    resolved_module,
                    module_path,
                    &scope,
                    &field.ty,
                    field.span,
                    &format!("field type `{}`", field.name),
                    diagnostics,
                );
            }
        }
        HirSymbolBody::Object { fields, methods } => {
            let object_scope = scope.with_self();
            for field in fields {
                validate_type_surface(
                    workspace,
                    resolved_module,
                    module_path,
                    &object_scope,
                    &field.ty,
                    field.span,
                    &format!("object field type `{}`", field.name),
                    diagnostics,
                );
            }
            for method in methods {
                validate_symbol_surface_types(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    method,
                    &object_scope,
                    diagnostics,
                );
            }
            let _ = collect_object_lifecycle_surface(
                workspace,
                resolved_module,
                module_path,
                symbol,
                diagnostics,
            );
        }
        HirSymbolBody::Enum { variants } => {
            for variant in variants {
                if let Some(payload) = &variant.payload {
                    validate_type_surface(
                        workspace,
                        resolved_module,
                        module_path,
                        &scope,
                        payload,
                        variant.span,
                        &format!("enum variant payload `{}`", variant.name),
                        diagnostics,
                    );
                }
            }
        }
        HirSymbolBody::Owner { objects, exits } => {
            if exits.is_empty() {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: symbol.span.line,
                    column: symbol.span.column,
                    message: format!(
                        "owner `{}` must declare at least one scope-exit",
                        symbol.name
                    ),
                });
            }
            let mut seen_owned_names = BTreeSet::new();
            for object in objects {
                if !seen_owned_names.insert(object.local_name.clone()) {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: object.span.line,
                        column: object.span.column,
                        message: format!(
                            "owner object `{}` is declared more than once",
                            object.local_name
                        ),
                    });
                }
                validate_surface_path_kind(
                    workspace,
                    resolved_module,
                    module_path,
                    &scope,
                    &object.type_path,
                    object.span,
                    &format!("owner object type `{}`", object.local_name),
                    SurfaceSymbolUse::TypeLike,
                    diagnostics,
                );
                if let Some(resolved_object) =
                    lookup_symbol_path(workspace, resolved_module, &object.type_path)
                {
                    let object_resolved_module = resolved_workspace
                        .package(resolved_object.package_name)
                        .and_then(|package| package.module(resolved_object.module_id))
                        .unwrap_or(resolved_module);
                    let object_module_path = workspace
                        .package(resolved_object.package_name)
                        .and_then(|package| package.module_path(resolved_object.module_id))
                        .cloned()
                        .unwrap_or_else(|| module_path.to_path_buf());
                    if resolved_object.symbol.kind != HirSymbolKind::Object {
                        diagnostics.push(Diagnostic {
                            path: module_path.to_path_buf(),
                            line: object.span.line,
                            column: object.span.column,
                            message: format!(
                                "owner object `{}` must resolve to an `obj`, found `{}`",
                                object.local_name,
                                resolved_object.symbol.kind.as_str()
                            ),
                        });
                    } else {
                        let _ = collect_object_lifecycle_surface(
                            workspace,
                            object_resolved_module,
                            &object_module_path,
                            resolved_object.symbol,
                            diagnostics,
                        );
                    }
                }
            }
            let owner_context_types = collect_owner_activation_context_types(
                workspace,
                resolved_workspace,
                resolved_module,
                objects,
            );
            if owner_context_types.len() > 1 {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: symbol.span.line,
                    column: symbol.span.column,
                    message: format!(
                        "owner `{}` uses incompatible lifecycle context types across owned objects: {}",
                        symbol.name,
                        owner_context_types
                            .into_iter()
                            .map(|ty| ty.render())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                });
            }
            let mut seen_exit_names = BTreeSet::new();
            for owner_exit in exits {
                if !seen_exit_names.insert(owner_exit.name.clone()) {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: owner_exit.span.line,
                        column: owner_exit.span.column,
                        message: format!(
                            "owner exit `{}` is declared more than once",
                            owner_exit.name
                        ),
                    });
                }
                for hold in &owner_exit.holds {
                    if !objects.iter().any(|object| object.local_name == *hold) {
                        diagnostics.push(Diagnostic {
                            path: module_path.to_path_buf(),
                            line: owner_exit.span.line,
                            column: owner_exit.span.column,
                            message: format!(
                                "owner exit `{}` holds unknown object `{hold}`",
                                owner_exit.name
                            ),
                        });
                    }
                }
            }
        }
        HirSymbolBody::Trait {
            assoc_types,
            methods,
        } => {
            let trait_scope = scope
                .with_assoc_types(assoc_types.iter().map(|assoc_type| assoc_type.name.clone()))
                .with_self();
            for assoc_type in assoc_types {
                if let Some(default_ty) = &assoc_type.default_ty {
                    validate_type_surface(
                        workspace,
                        resolved_module,
                        module_path,
                        &trait_scope,
                        default_ty,
                        assoc_type.span,
                        &format!("associated type default `{}`", assoc_type.name),
                        diagnostics,
                    );
                }
            }
            for method in methods {
                validate_symbol_surface_types(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    method,
                    &trait_scope,
                    diagnostics,
                );
            }
        }
    }
}

fn validate_impl_surface_types(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    impl_decl: &HirImplDecl,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let scope = TypeScope::default()
        .with_params(&impl_decl.type_params)
        .with_assoc_types(
            impl_decl
                .assoc_types
                .iter()
                .map(|assoc_type| assoc_type.name.clone()),
        )
        .with_self();
    let assoc_bindings = impl_decl
        .assoc_types
        .iter()
        .filter_map(|assoc_type| {
            assoc_type
                .value_ty
                .as_ref()
                .map(|value_ty| (assoc_type.name.clone(), value_ty.clone()))
        })
        .collect::<BTreeMap<_, _>>();

    if let Some(trait_path) = &impl_decl.trait_path {
        validate_trait_surface(
            workspace,
            resolved_module,
            module_path,
            &scope,
            trait_path,
            impl_decl.span,
            "impl trait path",
            diagnostics,
        );
    }
    validate_type_surface(
        workspace,
        resolved_module,
        module_path,
        &scope,
        &impl_decl.target_type,
        impl_decl.span,
        "impl target type",
        diagnostics,
    );
    for assoc_type in &impl_decl.assoc_types {
        if let Some(value_ty) = &assoc_type.value_ty {
            validate_type_surface(
                workspace,
                resolved_module,
                module_path,
                &scope,
                value_ty,
                assoc_type.span,
                &format!("associated type binding `{}`", assoc_type.name),
                diagnostics,
            );
        }
    }
    validate_impl_trait_where_requirements_structured(
        workspace,
        resolved_workspace,
        resolved_module,
        module_path,
        impl_decl,
        &scope,
        diagnostics,
    );
    for method in &impl_decl.methods {
        validate_symbol_surface_types(
            workspace,
            resolved_workspace,
            resolved_module,
            module_path,
            method,
            &scope,
            diagnostics,
        );
        let method_scope = scope.with_params(&method.type_params);
        validate_boundary_symbol_contract(
            workspace,
            resolved_workspace,
            resolved_module,
            module_path,
            method,
            &method_scope,
            Some(impl_decl.target_type.clone()),
            &assoc_bindings,
            diagnostics,
        );
    }
}

fn render_object_declared_type(symbol: &HirSymbol) -> HirType {
    let base = arcana_hir::HirPath {
        segments: vec![symbol.name.clone()],
        span: symbol.span,
    };
    if symbol.type_params.is_empty() {
        HirType {
            kind: arcana_hir::HirTypeKind::Path(base),
            span: symbol.span,
        }
    } else {
        HirType {
            kind: arcana_hir::HirTypeKind::Apply {
                base,
                args: symbol
                    .type_params
                    .iter()
                    .map(|param| HirType {
                        kind: arcana_hir::HirTypeKind::Path(arcana_hir::HirPath {
                            segments: vec![param.clone()],
                            span: symbol.span,
                        }),
                        span: symbol.span,
                    })
                    .collect(),
            },
            span: symbol.span,
        }
    }
}

fn classify_object_lifecycle_method(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    object_symbol: &HirSymbol,
    object_scope: &TypeScope,
    method: &HirSymbol,
) -> Result<Option<(LifecycleHookSlot, Option<HirType>)>, String> {
    let slot = match method.name.as_str() {
        "init" => {
            if method.params.len() == 1 {
                LifecycleHookSlot::Init
            } else if method.params.len() == 2 {
                LifecycleHookSlot::InitWithContext
            } else {
                return Err(
                    "object lifecycle hook `init` must take `edit self` with optional `read ctx`"
                        .to_string(),
                );
            }
        }
        "resume" => {
            if method.params.len() == 1 {
                LifecycleHookSlot::Resume
            } else if method.params.len() == 2 {
                LifecycleHookSlot::ResumeWithContext
            } else {
                return Err(
                    "object lifecycle hook `resume` must take `edit self` with optional `read ctx`"
                        .to_string(),
                );
            }
        }
        _ => return Ok(None),
    };

    if method.is_async {
        return Err(format!(
            "object lifecycle hook `{}` must not be async",
            method.name
        ));
    }
    if !method.type_params.is_empty() {
        return Err(format!(
            "object lifecycle hook `{}` must not declare type parameters",
            method.name
        ));
    }
    if let Some(return_type) = &method.return_type {
        let mut semantics = SemanticArena::default();
        let unit_ty = HirType {
            kind: arcana_hir::HirTypeKind::Path(arcana_hir::HirPath {
                segments: vec!["Unit".to_string()],
                span: method.span,
            }),
            span: method.span,
        };
        if semantics.type_id_for_hir(workspace, resolved_module, object_scope, return_type)
            != semantics.type_id_for_hir(workspace, resolved_module, object_scope, &unit_ty)
        {
            return Err(format!(
                "object lifecycle hook `{}` must return Unit",
                method.name
            ));
        }
    }
    let Some(receiver) = method.params.first() else {
        return Err(format!(
            "object lifecycle hook `{}` must declare `edit self`",
            method.name
        ));
    };
    if receiver.name != "self" {
        return Err(format!(
            "object lifecycle hook `{}` must use `self` as its first parameter",
            method.name
        ));
    }
    if receiver.mode != Some(arcana_hir::HirParamMode::Edit) {
        return Err(format!(
            "object lifecycle hook `{}` must take `edit self`",
            method.name
        ));
    }
    let mut semantics = SemanticArena::default();
    let self_ty = HirType {
        kind: arcana_hir::HirTypeKind::Path(arcana_hir::HirPath {
            segments: vec!["Self".to_string()],
            span: method.span,
        }),
        span: method.span,
    };
    let expected_self = render_object_declared_type(object_symbol);
    let actual_self = semantics.type_id_for_hir(workspace, resolved_module, object_scope, &receiver.ty);
    let self_id = semantics.type_id_for_hir(workspace, resolved_module, object_scope, &self_ty);
    let expected_self_id =
        semantics.type_id_for_hir(workspace, resolved_module, object_scope, &expected_self);
    if actual_self != self_id && actual_self != expected_self_id {
        return Err(format!(
            "object lifecycle hook `{}` must use receiver type `Self` or `{}`",
            method.name,
            expected_self.render()
        ));
    }

    let context_type = match method.params.get(1) {
        Some(context) => {
            if context.mode != Some(arcana_hir::HirParamMode::Read) {
                return Err(format!(
                    "object lifecycle hook `{}` context parameter must be `read`",
                    method.name
                ));
            }
            Some(context.ty.clone())
        }
        None => None,
    };

    Ok(Some((slot, context_type)))
}

fn collect_object_lifecycle_surface(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    symbol: &HirSymbol,
    diagnostics: &mut Vec<Diagnostic>,
) -> ObjectLifecycleSurface {
    let object_scope = TypeScope::default()
        .with_params(&symbol.type_params)
        .with_self();
    let HirSymbolBody::Object { methods, .. } = &symbol.body else {
        return ObjectLifecycleSurface::default();
    };

    let mut surface = ObjectLifecycleSurface::default();
    let mut seen_slots = BTreeSet::new();
    for method in methods {
        match classify_object_lifecycle_method(
            workspace,
            resolved_module,
            symbol,
            &object_scope,
            method,
        ) {
            Ok(Some((slot, context_type))) => {
                if !seen_slots.insert(slot) {
                    diagnostics.push(Diagnostic {
                        path: module_path.to_path_buf(),
                        line: method.span.line,
                        column: method.span.column,
                        message: format!(
                            "object lifecycle hook `{}` is declared more than once for the same activation shape",
                            method.name
                        ),
                    });
                    continue;
                }
                match slot {
                    LifecycleHookSlot::InitWithContext => {
                        surface.init_context_type = context_type;
                    }
                    LifecycleHookSlot::ResumeWithContext => {
                        surface.resume_context_type = context_type;
                    }
                    LifecycleHookSlot::Init | LifecycleHookSlot::Resume => {}
                }
            }
            Ok(None) => {}
            Err(message) => diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: method.span.line,
                column: method.span.column,
                message,
            }),
        }
    }
    surface
}

fn collect_owner_activation_context_types(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    objects: &[arcana_hir::HirOwnerObject],
) -> Vec<HirType> {
    let mut semantics = SemanticArena::default();
    let mut context_types = BTreeMap::<TypeId, HirType>::new();
    for object in objects {
        let Some(resolved_object) =
            lookup_symbol_path(workspace, resolved_module, &object.type_path)
        else {
            continue;
        };
        let object_resolved_module = resolved_workspace
            .package(resolved_object.package_name)
            .and_then(|package| package.module(resolved_object.module_id))
            .unwrap_or(resolved_module);
        let HirSymbolBody::Object { methods, .. } = &resolved_object.symbol.body else {
            continue;
        };
        let object_scope = TypeScope::default()
            .with_params(&resolved_object.symbol.type_params)
            .with_self();
        for method in methods {
            let Ok(classified) = classify_object_lifecycle_method(
                workspace,
                object_resolved_module,
                resolved_object.symbol,
                &object_scope,
                method,
            ) else {
                continue;
            };
            if let Some((_, Some(context_type))) = classified {
                let type_id = semantics.type_id_for_hir(
                    workspace,
                    object_resolved_module,
                    &object_scope,
                    &context_type,
                );
                context_types.entry(type_id).or_insert(context_type);
            }
        }
    }
    context_types.into_values().collect()
}

fn resolve_owner_activation_context_type(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    objects: &[arcana_hir::HirOwnerObject],
) -> Option<HirType> {
    let context_types = collect_owner_activation_context_types(
        workspace,
        resolved_workspace,
        resolved_module,
        objects,
    );
    if context_types.len() == 1 {
        context_types.into_iter().next()
    } else {
        None
    }
}

fn resolve_available_owner_binding(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    path: &[String],
) -> Option<AvailableOwnerBinding> {
    let resolved = lookup_symbol_path(workspace, resolved_module, path)?;
    let HirSymbolBody::Owner { objects, .. } = &resolved.symbol.body else {
        return None;
    };
    Some(AvailableOwnerBinding {
        local_name: resolved.symbol.name.clone(),
        owner_path: canonical_symbol_path(resolved.module_id, &resolved.symbol.name),
        objects: objects
            .iter()
            .map(|object| AvailableOwnerObjectBinding {
                local_name: object.local_name.clone(),
                ty: canonical_type_from_path(
                    workspace,
                    resolved_module,
                    &object.type_path,
                    resolved.symbol.span,
                ),
            })
            .collect(),
        activation_context_type: resolve_owner_activation_context_type(
            workspace,
            resolved_workspace,
            resolved_module,
            objects,
        ),
    })
}

fn apply_availability_attachments_to_scope(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    attachments: &[arcana_hir::HirAvailabilityAttachment],
    scope: &mut ValueScope,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for attachment in attachments {
        let Some(resolved) = lookup_symbol_path(workspace, resolved_module, &attachment.path)
        else {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: attachment.span.line,
                column: attachment.span.column,
                message: format!(
                    "unresolved availability attachment `{}`",
                    attachment.path.join(".")
                ),
            });
            continue;
        };
        match resolved.symbol.kind {
            HirSymbolKind::Owner => {
                if let Some(owner) = resolve_available_owner_binding(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    &attachment.path,
                ) {
                    scope.attach_owner(owner);
                }
            }
            HirSymbolKind::Object => {
                scope.attach_object_name(resolved.symbol.name.clone());
            }
            other => diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: attachment.span.line,
                column: attachment.span.column,
                message: format!(
                    "availability attachment `{}` must resolve to an owner or object, found `{}`",
                    attachment.path.join("."),
                    other.as_str()
                ),
            }),
        }
    }
}

fn validate_symbol_value_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    symbol: &HirSymbol,
    inherited_type_scope: &TypeScope,
    inherited_scope: &ValueScope,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let type_scope = inherited_type_scope.with_params(&symbol.type_params);
    let mut scope = inherited_scope.with_symbol_params(&symbol.params);
    apply_availability_attachments_to_scope(
        workspace,
        resolved_workspace,
        resolved_module,
        module_path,
        &symbol.availability,
        &mut scope,
        diagnostics,
    );
    let symbol_cleanup_subjects = collect_cleanup_subject_names(&symbol.rollups);
    for param in &symbol.params {
        let ownership = infer_type_ownership(workspace, resolved_module, &type_scope, &param.ty);
        scope.ownership.insert(param.name.clone(), ownership);
        scope.types.insert(param.name.clone(), param.ty.clone());
    }
    let mut borrow_state = BorrowFlowState::default();
    for name in &symbol_cleanup_subjects {
        if let Some(binding_id) = scope.binding_id_of(name) {
            borrow_state.activate_cleanup_binding(binding_id);
        }
    }
    validate_rollup_handlers(
        workspace,
        resolved_module,
        module_path,
        &symbol.rollups,
        diagnostics,
    );
    validate_statement_block_semantics(
        workspace,
        resolved_workspace,
        resolved_module,
        module_path,
        &symbol.statements,
        &type_scope,
        &mut scope,
        &mut borrow_state,
        &symbol_cleanup_subjects,
        diagnostics,
    );

    if let HirSymbolBody::Owner { objects, exits } = &symbol.body {
        let owner_path = canonical_symbol_path(&resolved_module.module_id, &symbol.name);
        let mut owner_scope = scope.clone();
        owner_scope.insert_typed(
            &symbol.name,
            false,
            OwnershipClass::Copy,
            None,
        );
        let available_owner = AvailableOwnerBinding {
            local_name: symbol.name.clone(),
            owner_path,
            objects: objects
                .iter()
                .map(|object| AvailableOwnerObjectBinding {
                    local_name: object.local_name.clone(),
                    ty: canonical_type_from_path(
                        workspace,
                        resolved_module,
                        &object.type_path,
                        symbol.span,
                    ),
                })
                .collect(),
            activation_context_type: resolve_owner_activation_context_type(
                workspace,
                resolved_workspace,
                resolved_module,
                objects,
            ),
        };
        let _ = owner_scope.activate_owner(&available_owner, None, false);
        for owner_exit in exits {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                &type_scope,
                &owner_scope,
                &owner_exit.condition,
                owner_exit.span,
                diagnostics,
            );
            validate_expected_expr_type(
                module_path,
                &owner_exit.condition,
                owner_exit.span,
                diagnostics,
                ExprTypeClass::Bool,
                "owner exit condition",
            );
        }
    }

    if let HirSymbolBody::Trait {
        assoc_types,
        methods,
    } = &symbol.body
    {
        let trait_scope = type_scope
            .with_assoc_types(assoc_types.iter().map(|assoc_type| assoc_type.name.clone()))
            .with_self();
        for method in methods {
            validate_symbol_value_semantics(
                workspace,
                resolved_workspace,
                resolved_module,
                module_path,
                method,
                &trait_scope,
                &ValueScope::default(),
                diagnostics,
            );
        }
    }
    if let HirSymbolBody::Object { methods, .. } = &symbol.body {
        let object_scope = type_scope.with_self();
        for method in methods {
            validate_symbol_value_semantics(
                workspace,
                resolved_workspace,
                resolved_module,
                module_path,
                method,
                &object_scope,
                &ValueScope::default(),
                diagnostics,
            );
        }
    }
}

fn validate_impl_value_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    impl_decl: &HirImplDecl,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let scope = TypeScope::default()
        .with_params(&impl_decl.type_params)
        .with_assoc_types(
            impl_decl
                .assoc_types
                .iter()
                .map(|assoc_type| assoc_type.name.clone()),
        )
        .with_self();
    for method in &impl_decl.methods {
        validate_symbol_value_semantics(
            workspace,
            resolved_workspace,
            resolved_module,
            module_path,
            method,
            &scope,
            &ValueScope::default(),
            diagnostics,
        );
    }
}

fn validate_rollup_handlers(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    rollups: &[arcana_hir::HirPageRollup],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for rollup in rollups {
        let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, &rollup.handler_path)
        else {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: rollup.span.line,
                column: rollup.span.column,
                message: format!(
                    "unresolved page rollup handler `{}`",
                    rollup.handler_path.join(".")
                ),
            });
            continue;
        };
        if !matches!(
            symbol_ref.symbol.kind,
            arcana_hir::HirSymbolKind::Fn
                | arcana_hir::HirSymbolKind::System
                | arcana_hir::HirSymbolKind::Behavior
                | arcana_hir::HirSymbolKind::Const
        ) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: rollup.span.line,
                column: rollup.span.column,
                message: format!(
                    "page rollup handler `{}` must resolve to a callable symbol",
                    rollup.handler_path.join(".")
                ),
            });
            continue;
        }
        if symbol_ref.symbol.is_async {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: rollup.span.line,
                column: rollup.span.column,
                message: format!(
                    "page rollup handler `{}` cannot be async in v1",
                    rollup.handler_path.join(".")
                ),
            });
            continue;
        }
        if symbol_ref.symbol.params.len() != 1 {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: rollup.span.line,
                column: rollup.span.column,
                message: format!(
                    "page rollup handler `{}` must accept exactly one parameter in v1",
                    rollup.handler_path.join(".")
                ),
            });
        }
    }
}

fn collect_cleanup_subject_names(rollups: &[arcana_hir::HirPageRollup]) -> BTreeSet<String> {
    rollups
        .iter()
        .map(|rollup| rollup.subject.clone())
        .collect::<BTreeSet<_>>()
}

fn activate_current_cleanup_binding(
    borrow_state: &mut BorrowFlowState,
    scope: &ValueScope,
    current_block_cleanup_subjects: &BTreeSet<String>,
    name: &str,
) {
    if !current_block_cleanup_subjects.contains(name) {
        return;
    }
    if let Some(binding_id) = scope.binding_id_of(name) {
        borrow_state.activate_cleanup_binding(binding_id);
    }
}

fn validate_statement_block_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    statements: &[HirStatement],
    type_scope: &TypeScope,
    scope: &mut ValueScope,
    borrow_state: &mut BorrowFlowState,
    current_block_cleanup_subjects: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for statement in statements {
        validate_rollup_handlers(
            workspace,
            resolved_module,
            module_path,
            &statement.rollups,
            diagnostics,
        );
        match &statement.kind {
            HirStatementKind::Let {
                mutable,
                name,
                value,
            } => {
                if let Some(owner_activation) = resolve_owner_activation_expr(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    value,
                ) {
                    if let Some(ref message) = owner_activation.invalid {
                        diagnostics.push(Diagnostic {
                            path: module_path.to_path_buf(),
                            line: statement.span.line,
                            column: statement.span.column,
                            message: message.clone(),
                        });
                    }
                    if let Some(context) = owner_activation.context {
                        validate_expr_semantics(
                            workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            scope,
                            context,
                            statement.span,
                            diagnostics,
                        );
                        validate_expr_borrow_flow(
                            workspace,
                            resolved_module,
                            type_scope,
                            module_path,
                            scope,
                            context,
                            statement.span,
                            borrow_state,
                            diagnostics,
                        );
                        note_expr_moves(
                            workspace,
                            resolved_module,
                            type_scope,
                            scope,
                            context,
                            borrow_state,
                        );
                        note_escaping_expr_borrows(borrow_state, context, scope);
                    }
                    validate_owner_activation_context(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        scope,
                        &owner_activation,
                        statement.span,
                        diagnostics,
                    );
                    let inserted =
                        scope.activate_owner(&owner_activation.owner, Some(name), *mutable);
                    for inserted_name in inserted {
                        activate_current_cleanup_binding(
                            borrow_state,
                            scope,
                            current_block_cleanup_subjects,
                            &inserted_name,
                        );
                    }
                    continue;
                }
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    value,
                    statement.span,
                    diagnostics,
                );
                validate_expr_borrow_flow(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    value,
                    statement.span,
                    borrow_state,
                    diagnostics,
                );
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    value,
                    borrow_state,
                );
                note_escaping_expr_borrows(borrow_state, value, scope);
                borrow_state.clear_local(name);
                let ownership =
                    infer_expr_ownership(workspace, resolved_module, type_scope, scope, value);
                let ty =
                    infer_expr_value_type(workspace, resolved_module, type_scope, scope, value);
                scope.insert_typed(name, *mutable, ownership, ty);
                activate_current_cleanup_binding(
                    borrow_state,
                    scope,
                    current_block_cleanup_subjects,
                    name,
                );
            }
            HirStatementKind::Return { value } => {
                if let Some(value) = value {
                    validate_expr_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        scope,
                        value,
                        statement.span,
                        diagnostics,
                    );
                    validate_expr_borrow_flow(
                        workspace,
                        resolved_module,
                        type_scope,
                        module_path,
                        scope,
                        value,
                        statement.span,
                        borrow_state,
                        diagnostics,
                    );
                    validate_return_borrow_ties(
                        module_path,
                        scope,
                        value,
                        statement.span,
                        diagnostics,
                    );
                }
            }
            HirStatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let nested_cleanup_subjects = collect_cleanup_subject_names(&statement.rollups);
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    condition,
                    statement.span,
                    diagnostics,
                );
                validate_expr_borrow_flow(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    condition,
                    statement.span,
                    borrow_state,
                    diagnostics,
                );
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    condition,
                    borrow_state,
                );
                validate_expected_expr_type(
                    module_path,
                    condition,
                    statement.span,
                    diagnostics,
                    ExprTypeClass::Bool,
                    "if condition",
                );
                let mut then_scope = scope.clone();
                apply_availability_attachments_to_scope(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    &statement.availability,
                    &mut then_scope,
                    diagnostics,
                );
                let mut then_borrows = borrow_state.clone();
                validate_statement_block_semantics(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    then_branch,
                    type_scope,
                    &mut then_scope,
                    &mut then_borrows,
                    &nested_cleanup_subjects,
                    diagnostics,
                );
                borrow_state.merge_moves_from(&then_borrows);
                if let Some(else_branch) = else_branch {
                    let mut else_scope = scope.clone();
                    apply_availability_attachments_to_scope(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        module_path,
                        &statement.availability,
                        &mut else_scope,
                        diagnostics,
                    );
                    let mut else_borrows = borrow_state.clone();
                    validate_statement_block_semantics(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        module_path,
                        else_branch,
                        type_scope,
                        &mut else_scope,
                        &mut else_borrows,
                        &nested_cleanup_subjects,
                        diagnostics,
                    );
                    borrow_state.merge_moves_from(&else_borrows);
                }
            }
            HirStatementKind::While { condition, body } => {
                let nested_cleanup_subjects = collect_cleanup_subject_names(&statement.rollups);
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    condition,
                    statement.span,
                    diagnostics,
                );
                validate_expr_borrow_flow(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    condition,
                    statement.span,
                    borrow_state,
                    diagnostics,
                );
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    condition,
                    borrow_state,
                );
                validate_expected_expr_type(
                    module_path,
                    condition,
                    statement.span,
                    diagnostics,
                    ExprTypeClass::Bool,
                    "while condition",
                );
                let mut body_scope = scope.clone();
                apply_availability_attachments_to_scope(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    &statement.availability,
                    &mut body_scope,
                    diagnostics,
                );
                let mut body_borrows = borrow_state.clone();
                validate_statement_block_semantics(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    body,
                    type_scope,
                    &mut body_scope,
                    &mut body_borrows,
                    &nested_cleanup_subjects,
                    diagnostics,
                );
                borrow_state.merge_moves_from(&body_borrows);
            }
            HirStatementKind::For {
                binding,
                iterable,
                body,
            } => {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    iterable,
                    statement.span,
                    diagnostics,
                );
                validate_expr_borrow_flow(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    iterable,
                    statement.span,
                    borrow_state,
                    diagnostics,
                );
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    iterable,
                    borrow_state,
                );
                let nested_cleanup_subjects = collect_cleanup_subject_names(&statement.rollups);
                let mut body_scope = scope.with_local(binding, false);
                apply_availability_attachments_to_scope(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    &statement.availability,
                    &mut body_scope,
                    diagnostics,
                );
                let mut body_borrows = borrow_state.clone();
                activate_current_cleanup_binding(
                    &mut body_borrows,
                    &body_scope,
                    &nested_cleanup_subjects,
                    binding,
                );
                validate_statement_block_semantics(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    module_path,
                    body,
                    type_scope,
                    &mut body_scope,
                    &mut body_borrows,
                    &nested_cleanup_subjects,
                    diagnostics,
                );
                borrow_state.merge_moves_from(&body_borrows);
            }
            HirStatementKind::Defer { expr } | HirStatementKind::Expr { expr } => {
                if let HirStatementKind::Expr { .. } = &statement.kind {
                    if let Some(owner_activation) = resolve_owner_activation_expr(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        expr,
                    ) {
                        if let Some(ref message) = owner_activation.invalid {
                            diagnostics.push(Diagnostic {
                                path: module_path.to_path_buf(),
                                line: statement.span.line,
                                column: statement.span.column,
                                message: message.clone(),
                            });
                        }
                        if let Some(context) = owner_activation.context {
                            validate_expr_semantics(
                                workspace,
                                resolved_module,
                                module_path,
                                type_scope,
                                scope,
                                context,
                                statement.span,
                                diagnostics,
                            );
                            validate_expr_borrow_flow(
                                workspace,
                                resolved_module,
                                type_scope,
                                module_path,
                                scope,
                                context,
                                statement.span,
                                borrow_state,
                                diagnostics,
                            );
                            note_expr_moves(
                                workspace,
                                resolved_module,
                                type_scope,
                                scope,
                                context,
                                borrow_state,
                            );
                            note_escaping_expr_borrows(borrow_state, context, scope);
                        }
                        validate_owner_activation_context(
                            workspace,
                            resolved_workspace,
                            resolved_module,
                            module_path,
                            type_scope,
                            scope,
                            &owner_activation,
                            statement.span,
                            diagnostics,
                        );
                        let inserted = scope.activate_owner(&owner_activation.owner, None, false);
                        for inserted_name in inserted {
                            activate_current_cleanup_binding(
                                borrow_state,
                                scope,
                                current_block_cleanup_subjects,
                                &inserted_name,
                            );
                        }
                        continue;
                    }
                }
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    expr,
                    statement.span,
                    diagnostics,
                );
                validate_expr_borrow_flow(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    expr,
                    statement.span,
                    borrow_state,
                    diagnostics,
                );
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    expr,
                    borrow_state,
                );
            }
            HirStatementKind::Assign { target, value, .. } => {
                validate_assign_target_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    target,
                    statement.span,
                    diagnostics,
                );
                validate_assign_target_borrow_flow(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    target,
                    statement.span,
                    borrow_state,
                    diagnostics,
                );
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    value,
                    statement.span,
                    diagnostics,
                );
                validate_expr_borrow_flow(
                    workspace,
                    resolved_module,
                    type_scope,
                    module_path,
                    scope,
                    value,
                    statement.span,
                    borrow_state,
                    diagnostics,
                );
                note_expr_moves(
                    workspace,
                    resolved_module,
                    type_scope,
                    scope,
                    value,
                    borrow_state,
                );
                if let HirAssignTarget::Name { text } = target {
                    if scope.contains(text) {
                        borrow_state.clear_local(text);
                        let ownership = infer_expr_ownership(
                            workspace,
                            resolved_module,
                            type_scope,
                            scope,
                            value,
                        );
                        let ty = infer_expr_value_type(
                            workspace,
                            resolved_module,
                            type_scope,
                            scope,
                            value,
                        );
                        scope.ownership.insert(text.clone(), ownership);
                        if let Some(ty) = ty {
                            scope.types.insert(text.clone(), ty);
                        } else {
                            scope.types.remove(text);
                        }
                    }
                }
                if matches!(target, HirAssignTarget::Name { text } if scope.contains(text)) {
                    note_escaping_expr_borrows(borrow_state, value, scope);
                }
            }
            HirStatementKind::Break | HirStatementKind::Continue => {}
        }
    }
}

fn validate_assign_target_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    target: &HirAssignTarget,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match target {
        HirAssignTarget::Name { text } => validate_value_path_segments(
            workspace,
            resolved_module,
            module_path,
            scope,
            &[text.clone()],
            span,
            "assignment target",
            diagnostics,
        ),
        target @ HirAssignTarget::MemberAccess {
            target: inner_target,
            ..
        } => {
            if let Some(path) = flatten_assign_target_path(target) {
                if should_resolve_member_path_as_namespace(workspace, resolved_module, scope, &path)
                {
                    validate_value_path_segments(
                        workspace,
                        resolved_module,
                        module_path,
                        scope,
                        &path,
                        span,
                        "assignment target",
                        diagnostics,
                    );
                    return;
                }
            }
            validate_assign_target_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                inner_target,
                span,
                diagnostics,
            );
        }
        HirAssignTarget::Index { target, index } => {
            validate_assign_target_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                target,
                span,
                diagnostics,
            );
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                index,
                span,
                diagnostics,
            );
        }
    }
}

fn validate_assign_target_borrow_flow(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_scope: &TypeScope,
    module_path: &Path,
    scope: &ValueScope,
    target: &HirAssignTarget,
    span: Span,
    state: &BorrowFlowState,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(name) = assign_target_root_local(target, scope) {
        if !matches!(target, HirAssignTarget::Name { .. }) && state.has_moved(name) {
            push_type_contract_diagnostic(
                module_path,
                span,
                diagnostics,
                format!("use of moved local `{name}`"),
            );
        } else if state.has_any_borrow(name) {
            push_type_contract_diagnostic(
                module_path,
                span,
                diagnostics,
                format!("cannot assign to local `{name}` while it is borrowed"),
            );
        }
    }

    match target {
        HirAssignTarget::Name { .. } => {}
        HirAssignTarget::MemberAccess { target, .. } => {
            validate_assign_target_borrow_flow(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                target,
                span,
                state,
                diagnostics,
            );
        }
        HirAssignTarget::Index { target, index } => {
            validate_assign_target_borrow_flow(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                target,
                span,
                state,
                diagnostics,
            );
            validate_expr_borrow_flow(
                workspace,
                resolved_module,
                type_scope,
                module_path,
                scope,
                index,
                span,
                state,
                diagnostics,
            );
        }
    }
}

fn validate_expr_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match expr {
        HirExpr::Path { segments } => validate_value_path_segments(
            workspace,
            resolved_module,
            module_path,
            scope,
            segments,
            span,
            "value expression",
            diagnostics,
        ),
        HirExpr::BoolLiteral { .. } | HirExpr::IntLiteral { .. } | HirExpr::StrLiteral { .. } => {}
        HirExpr::Pair { left, right } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                left,
                span,
                diagnostics,
            );
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                right,
                span,
                diagnostics,
            );
        }
        HirExpr::CollectionLiteral { items } => {
            for item in items {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    item,
                    span,
                    diagnostics,
                );
            }
        }
        HirExpr::Match { subject, arms } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                subject,
                span,
                diagnostics,
            );
            for arm in arms {
                let mut arm_scope = scope.clone();
                for pattern in &arm.patterns {
                    validate_match_pattern_semantics(
                        workspace,
                        resolved_module,
                        Some(subject),
                        pattern,
                        &mut arm_scope,
                    );
                }
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &arm_scope,
                    &arm.value,
                    arm.span,
                    diagnostics,
                );
            }
        }
        HirExpr::Chain { steps, .. } => {
            for step in steps {
                validate_chain_step_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    step,
                    span,
                    diagnostics,
                );
            }
        }
        HirExpr::MemoryPhrase {
            arena,
            init_args,
            constructor,
            attached,
            ..
        } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                arena,
                span,
                diagnostics,
            );
            for arg in init_args {
                validate_phrase_arg_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    arg,
                    span,
                    diagnostics,
                );
            }
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                constructor,
                span,
                diagnostics,
            );
            if let Some(path) = flatten_callable_expr_path(constructor) {
                validate_value_path_segments(
                    workspace,
                    resolved_module,
                    module_path,
                    scope,
                    &path,
                    span,
                    "memory constructor",
                    diagnostics,
                );
            }
            for attachment in attached {
                validate_header_attachment_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    attachment,
                    diagnostics,
                );
            }
        }
        HirExpr::GenericApply { expr, type_args } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                diagnostics,
            );
            for type_arg in type_args {
                validate_type_surface(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    type_arg,
                    span,
                    &format!("expression generic argument `{type_arg}`"),
                    diagnostics,
                );
            }
        }
        HirExpr::QualifiedPhrase {
            subject,
            args,
            qualifier,
            attached,
            ..
        } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                subject,
                span,
                diagnostics,
            );
            validate_bare_method_resolution(
                workspace,
                resolved_module,
                type_scope,
                scope,
                module_path,
                subject,
                qualifier,
                span,
                diagnostics,
            );
            for arg in args {
                validate_phrase_arg_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    arg,
                    span,
                    diagnostics,
                );
            }
            for attachment in attached {
                validate_header_attachment_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    attachment,
                    diagnostics,
                );
            }
            validate_opaque_constructor_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                subject,
                qualifier,
                span,
                diagnostics,
            );
        }
        HirExpr::Await { expr } => validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            expr,
            span,
            diagnostics,
        ),
        HirExpr::Unary { op, expr } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                diagnostics,
            );
            match op {
                HirUnaryOp::Not => validate_expected_expr_type(
                    module_path,
                    expr,
                    span,
                    diagnostics,
                    ExprTypeClass::Bool,
                    &format!("operand of `{}`", unary_op_token(*op)),
                ),
                HirUnaryOp::Neg | HirUnaryOp::BitNot => validate_expected_expr_type(
                    module_path,
                    expr,
                    span,
                    diagnostics,
                    ExprTypeClass::Int,
                    &format!("operand of `{}`", unary_op_token(*op)),
                ),
                HirUnaryOp::BorrowRead => {
                    validate_borrow_operand_place(
                        module_path,
                        scope,
                        expr,
                        span,
                        diagnostics,
                        false,
                    );
                }
                HirUnaryOp::BorrowMut => {
                    validate_borrow_operand_place(
                        module_path,
                        scope,
                        expr,
                        span,
                        diagnostics,
                        true,
                    );
                }
                HirUnaryOp::Deref | HirUnaryOp::Weave | HirUnaryOp::Split => {}
            }
        }
        HirExpr::Binary { left, op, right } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                left,
                span,
                diagnostics,
            );
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                right,
                span,
                diagnostics,
            );
            match op {
                HirBinaryOp::And | HirBinaryOp::Or => {
                    validate_expected_expr_type(
                        module_path,
                        left,
                        span,
                        diagnostics,
                        ExprTypeClass::Bool,
                        &format!("left operand of `{}`", binary_op_token(*op)),
                    );
                    validate_expected_expr_type(
                        module_path,
                        right,
                        span,
                        diagnostics,
                        ExprTypeClass::Bool,
                        &format!("right operand of `{}`", binary_op_token(*op)),
                    );
                }
                HirBinaryOp::Sub
                | HirBinaryOp::Mul
                | HirBinaryOp::Div
                | HirBinaryOp::Mod
                | HirBinaryOp::BitOr
                | HirBinaryOp::BitXor
                | HirBinaryOp::BitAnd
                | HirBinaryOp::Shl
                | HirBinaryOp::Shr => {
                    validate_expected_expr_type(
                        module_path,
                        left,
                        span,
                        diagnostics,
                        ExprTypeClass::Int,
                        &format!("left operand of `{}`", binary_op_token(*op)),
                    );
                    validate_expected_expr_type(
                        module_path,
                        right,
                        span,
                        diagnostics,
                        ExprTypeClass::Int,
                        &format!("right operand of `{}`", binary_op_token(*op)),
                    );
                }
                HirBinaryOp::Add
                | HirBinaryOp::EqEq
                | HirBinaryOp::NotEq
                | HirBinaryOp::Lt
                | HirBinaryOp::LtEq
                | HirBinaryOp::Gt
                | HirBinaryOp::GtEq => {}
            }
        }
        member_expr @ HirExpr::MemberAccess { expr, .. } => {
            if let Some(path) = flatten_member_expr_path(member_expr) {
                if should_resolve_member_path_as_namespace(workspace, resolved_module, scope, &path)
                {
                    validate_value_path_segments(
                        workspace,
                        resolved_module,
                        module_path,
                        scope,
                        &path,
                        span,
                        "value expression",
                        diagnostics,
                    );
                    return;
                }
            }
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                diagnostics,
            );
            if let HirExpr::MemberAccess { member, .. } = member_expr {
                if is_tuple_projection_member(member) {
                    if let Some(actual) = infer_expr_type(expr) {
                        if actual != ExprTypeClass::Pair {
                            push_type_contract_diagnostic(
                                module_path,
                                span,
                                diagnostics,
                                format!(
                                    "tuple field access `.{member}` requires a pair value, found {}",
                                    actual.label()
                                ),
                            );
                        }
                    }
                }
            }
        }
        HirExpr::Index { expr, index } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                diagnostics,
            );
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                index,
                span,
                diagnostics,
            );
        }
        HirExpr::Slice {
            expr, start, end, ..
        } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                diagnostics,
            );
            if let Some(start) = start {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    start,
                    span,
                    diagnostics,
                );
                validate_expected_expr_type(
                    module_path,
                    start,
                    span,
                    diagnostics,
                    ExprTypeClass::Int,
                    "slice start",
                );
            }
            if let Some(end) = end {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    end,
                    span,
                    diagnostics,
                );
                validate_expected_expr_type(
                    module_path,
                    end,
                    span,
                    diagnostics,
                    ExprTypeClass::Int,
                    "slice end",
                );
            }
        }
        HirExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    start,
                    span,
                    diagnostics,
                );
            }
            if let Some(end) = end {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    end,
                    span,
                    diagnostics,
                );
            }
        }
    }
}

fn validate_phrase_arg_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    arg: &arcana_hir::HirPhraseArg,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match arg {
        arcana_hir::HirPhraseArg::Positional(expr)
        | arcana_hir::HirPhraseArg::Named { value: expr, .. } => validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            expr,
            span,
            diagnostics,
        ),
    }
}

fn validate_chain_step_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    step: &HirChainStep,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_chain_stage_semantics(
        workspace,
        resolved_module,
        module_path,
        type_scope,
        scope,
        &step.stage,
        span,
        &step.text,
        diagnostics,
    );
    for arg in &step.bind_args {
        validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            arg,
            span,
            diagnostics,
        );
    }
}

fn validate_chain_stage_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    stage: &HirExpr,
    span: Span,
    text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match stage {
        HirExpr::Path { segments } => validate_value_path_segments(
            workspace,
            resolved_module,
            module_path,
            scope,
            segments,
            span,
            &format!("chain step `{text}`"),
            diagnostics,
        ),
        stage @ HirExpr::MemberAccess { .. } => {
            if let Some(path) = flatten_member_expr_path(stage) {
                validate_value_path_segments(
                    workspace,
                    resolved_module,
                    module_path,
                    scope,
                    &path,
                    span,
                    &format!("chain step `{text}`"),
                    diagnostics,
                );
            } else {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    stage,
                    span,
                    diagnostics,
                );
            }
        }
        HirExpr::GenericApply { expr, type_args } => {
            validate_chain_stage_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                text,
                diagnostics,
            );
            for type_arg in type_args {
                validate_type_surface(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    type_arg,
                    span,
                    &format!("chain step generic argument `{type_arg}` in `{text}`"),
                    diagnostics,
                );
            }
        }
        _ => validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            stage,
            span,
            diagnostics,
        ),
    }
}

fn validate_header_attachment_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    attachment: &HirHeaderAttachment,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match attachment {
        HirHeaderAttachment::Named { value, span, .. }
        | HirHeaderAttachment::Chain {
            expr: value, span, ..
        } => validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            value,
            *span,
            diagnostics,
        ),
    }
}

fn validate_match_pattern_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    subject: Option<&HirExpr>,
    pattern: &HirMatchPattern,
    scope: &mut ValueScope,
) {
    match pattern {
        HirMatchPattern::Wildcard | HirMatchPattern::Literal { .. } => {}
        HirMatchPattern::Name { text } => {
            let is_binding = match split_simple_path(text) {
                Some(path) => {
                    path.len() == 1
                        && !subject.is_some_and(|subject| {
                            match_name_resolves_to_zero_payload_variant(
                                workspace,
                                resolved_module,
                                scope,
                                subject,
                                text.trim(),
                            )
                        })
                }
                None => true,
            };
            if is_binding {
                scope.insert(text.trim(), false);
            }
        }
        HirMatchPattern::Variant { args, .. } => {
            for arg in args {
                validate_match_pattern_semantics(workspace, resolved_module, None, arg, scope);
            }
        }
    }
}

fn validate_value_path_segments(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    scope: &ValueScope,
    path: &[String],
    span: Span,
    context: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if path.len() == 1 && scope.contains(&path[0]) {
        return;
    }
    if value_path_exists(workspace, resolved_module, path) {
        return;
    }
    diagnostics.push(Diagnostic {
        path: module_path.to_path_buf(),
        line: span.line,
        column: span.column,
        message: format!(
            "unresolved value reference `{}` in {context}",
            path.join(".")
        ),
    });
}

fn value_path_exists(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    path: &[String],
) -> bool {
    if path.is_empty() {
        return false;
    }
    if let Some(binding) = resolved_module.bindings.get(&path[0]) {
        if target_path_exists(workspace, &binding.target, &path[1..]) {
            return true;
        }
    }
    let Some(package) = visible_package_root_for_module(workspace, resolved_module, &path[0])
    else {
        let Some(package) = current_workspace_package_for_module(workspace, resolved_module) else {
            return false;
        };
        return package_path_exists(package, path);
    };
    package_path_exists(package, &path[1..])
}

fn target_path_exists(
    workspace: &HirWorkspaceSummary,
    target: &HirResolvedTarget,
    tail: &[String],
) -> bool {
    match target {
        HirResolvedTarget::Symbol {
            package_name,
            module_id,
            symbol_name,
        } => {
            let Some(package) = workspace.package(package_name) else {
                return false;
            };
            let Some(module) = package.module(module_id) else {
                return false;
            };
            let Some(symbol) = module
                .symbols
                .iter()
                .find(|symbol| symbol.name == *symbol_name)
            else {
                return false;
            };
            symbol_tail_exists(symbol, tail)
        }
        HirResolvedTarget::Module {
            package_name,
            module_id,
        } => {
            let Some(package) = workspace.package(package_name) else {
                return false;
            };
            let Some(module) = package.module(module_id) else {
                return false;
            };
            module_path_exists(package, module, tail)
        }
    }
}

fn package_path_exists(package: &HirWorkspacePackage, path: &[String]) -> bool {
    if path.is_empty() {
        return true;
    }
    let Some(module) = package.module(&package.summary.package_name) else {
        return false;
    };
    module_path_exists(package, module, path)
}

fn module_path_exists(
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    path: &[String],
) -> bool {
    if path.is_empty() {
        return true;
    }
    if module_value_member_exists(module, &path[0], &path[1..]) {
        return true;
    }
    let base_relative = module
        .module_id
        .split('.')
        .skip(1)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut module_candidate = base_relative.clone();
    module_candidate.extend_from_slice(path);
    if package.resolve_relative_module(&module_candidate).is_some() {
        return true;
    }
    for split_index in 1..path.len() {
        let mut symbol_module_path = base_relative.clone();
        symbol_module_path.extend_from_slice(&path[..split_index]);
        let Some(target_module) = package.resolve_relative_module(&symbol_module_path) else {
            continue;
        };
        if module_value_member_exists(target_module, &path[split_index], &path[split_index + 1..]) {
            return true;
        }
    }
    false
}

fn module_value_member_exists(module: &HirModuleSummary, member: &str, tail: &[String]) -> bool {
    if let Some(symbol) = module.symbols.iter().find(|symbol| symbol.name == member) {
        if symbol_tail_exists(symbol, tail) {
            return true;
        }
    }
    tail.is_empty()
        && module
            .impls
            .iter()
            .flat_map(|impl_decl| impl_decl.methods.iter())
            .any(|method| method.name == member)
}

fn symbol_tail_exists(symbol: &HirSymbol, tail: &[String]) -> bool {
    if tail.is_empty() {
        return true;
    }
    match &symbol.body {
        HirSymbolBody::Enum { variants } => {
            tail.len() == 1 && variants.iter().any(|variant| variant.name == tail[0])
        }
        _ => false,
    }
}

fn should_resolve_member_path_as_namespace(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &ValueScope,
    path: &[String],
) -> bool {
    if path.len() < 2 || scope.contains(&path[0]) {
        return false;
    }
    if let Some(binding) = resolved_module.bindings.get(&path[0]) {
        return match &binding.target {
            HirResolvedTarget::Module { .. } => true,
            HirResolvedTarget::Symbol { .. } => {
                target_supports_member_namespace(workspace, &binding.target)
            }
        };
    }
    if visible_package_root_for_module(workspace, resolved_module, &path[0]).is_some() {
        return true;
    }
    let Some(package) = current_workspace_package_for_module(workspace, resolved_module) else {
        return false;
    };
    package
        .resolve_relative_module(&[path[0].clone()])
        .is_some()
}

fn target_supports_member_namespace(
    workspace: &HirWorkspaceSummary,
    target: &HirResolvedTarget,
) -> bool {
    match target {
        HirResolvedTarget::Module { .. } => true,
        HirResolvedTarget::Symbol {
            package_name,
            module_id,
            symbol_name,
        } => workspace
            .package(package_name)
            .and_then(|package| package.module(module_id))
            .and_then(|module| {
                module
                    .symbols
                    .iter()
                    .find(|symbol| symbol.name == *symbol_name)
            })
            .map(|symbol| matches!(symbol.body, HirSymbolBody::Enum { .. }))
            .unwrap_or(false),
    }
}

fn flatten_member_expr_path(expr: &HirExpr) -> Option<Vec<String>> {
    match expr {
        HirExpr::Path { segments } => Some(segments.clone()),
        HirExpr::MemberAccess { expr, member } if is_identifier_text(member) => {
            let mut path = flatten_member_expr_path(expr)?;
            path.push(member.clone());
            Some(path)
        }
        _ => None,
    }
}

fn flatten_assign_target_path(target: &HirAssignTarget) -> Option<Vec<String>> {
    match target {
        HirAssignTarget::Name { text } => Some(vec![text.clone()]),
        HirAssignTarget::MemberAccess { target, member } if is_identifier_text(member) => {
            let mut path = flatten_assign_target_path(target)?;
            path.push(member.clone());
            Some(path)
        }
        _ => None,
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

#[cfg(test)]
mod tests {
    use super::{
        check_path, check_sources, check_workspace_graph, compute_member_fingerprints,
        compute_member_fingerprints_for_checked_workspace, load_workspace_hir, lower_to_hir,
    };
    use arcana_package::{
        BuildDisposition, execute_build, load_workspace_graph, plan_workspace, prepare_build,
        read_lockfile, write_lockfile,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

    fn plan_build(
        graph: &arcana_package::WorkspaceGraph,
        order: &[String],
        _fingerprints: &arcana_package::WorkspaceFingerprints,
        existing_lock: Option<&arcana_package::Lockfile>,
    ) -> Result<Vec<arcana_package::BuildStatus>, String> {
        let prepared = prepare_build(graph)?;
        arcana_package::plan_build(graph, order, &prepared, existing_lock)
    }

    fn execute_planned_build(
        graph: &arcana_package::WorkspaceGraph,
        _fingerprints: &arcana_package::WorkspaceFingerprints,
        statuses: &[arcana_package::BuildStatus],
    ) {
        let prepared = prepare_build(graph).expect("prepare build");
        execute_build(graph, &prepared, statuses).expect("execute");
    }

    #[test]
    fn check_sources_counts_modules() {
        let summary = check_sources(
            ["import std.io\nfn main() -> Int:\n    return 0\n"]
                .iter()
                .copied(),
        )
        .expect("check should pass");
        assert_eq!(summary.module_count, 1);
        assert_eq!(summary.directive_count, 1);
        assert!(summary.symbol_count >= 1);

        let hir = lower_to_hir(&summary);
        assert!(hir.symbol_count >= 1);
    }

    #[test]
    fn check_path_reports_unresolved_import() {
        let root = make_temp_package(
            "broken_app",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "import missing.module\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("expected unresolved import");
        assert!(err.contains("missing.module"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_workspace_root_package_with_members_and_deps() {
        let root = make_temp_workspace(
            "workspace_root_package",
            &["app"],
            &[
                (
                    "src/shelf.arc",
                    "import core\nfn main() -> Int:\n    return core.value :: :: call\n",
                ),
                ("src/types.arc", ""),
                ("app/book.toml", "name = \"app\"\nkind = \"app\"\n"),
                ("app/src/shelf.arc", "fn main() -> Int:\n    return 0\n"),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn value() -> Int:\n    return 7\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );
        fs::write(
            root.join("book.toml"),
            concat!(
                "name = \"workspace\"\n",
                "kind = \"app\"\n",
                "[workspace]\n",
                "members = [\"app\"]\n",
                "[deps]\n",
                "core = { path = \"core\" }\n",
            ),
        )
        .expect("workspace manifest should be writable");

        let summary = check_path(&root).expect("workspace root package should check");
        assert_eq!(summary.package_count, 3);
        assert_eq!(summary.module_count, 6);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_dependency_alias_imports() {
        let root = make_temp_workspace(
            "dependency_alias_imports",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\nutil = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import util\nfn main() -> Int:\n    return util.value :: :: call\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn value() -> Int:\n    return 7\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );

        let summary = check_path(&root.join("app")).expect("dependency alias import should check");
        assert_eq!(summary.package_count, 2);
        assert!(summary.module_count >= 4);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_workspace_root_dependency_alias_imports() {
        let root = make_temp_workspace(
            "workspace_root_dependency_aliases",
            &["app"],
            &[
                (
                    "src/shelf.arc",
                    "import util\nfn main() -> Int:\n    return util.value :: :: call\n",
                ),
                ("src/types.arc", ""),
                ("app/book.toml", "name = \"app\"\nkind = \"app\"\n"),
                ("app/src/shelf.arc", "fn main() -> Int:\n    return 0\n"),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn value() -> Int:\n    return 7\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );
        fs::write(
            root.join("book.toml"),
            concat!(
                "name = \"workspace\"\n",
                "kind = \"app\"\n",
                "[workspace]\n",
                "members = [\"app\"]\n",
                "[deps]\n",
                "util = { path = \"core\" }\n",
            ),
        )
        .expect("workspace manifest should be writable");

        let summary =
            check_path(&root).expect("workspace root dependency alias import should check");
        assert_eq!(summary.package_count, 3);
        assert_eq!(summary.module_count, 6);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_graph_allows_local_modules_named_like_unrelated_members() {
        let root = make_temp_workspace(
            "workspace_local_module_shadowing_member",
            &["app", "core"],
            &[
                ("app/book.toml", "name = \"app\"\nkind = \"app\"\n"),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    return core.value :: :: call\n",
                ),
                (
                    "app/src/core.arc",
                    "export fn value() -> Int:\n    return 7\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn value() -> Int:\n    return 0\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );

        let graph = load_workspace_graph(&root).expect("workspace graph should load");
        let summary = check_workspace_graph(&graph)
            .expect("workspace check should prefer app-local modules over unrelated members");
        assert_eq!(summary.summary().package_count, 2);
        assert!(summary.summary().module_count >= 5);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_main_with_parameters() {
        let root = make_temp_package(
            "main_with_parameters",
            "app",
            &[],
            &[
                ("src/shelf.arc", "fn main(x: Int) -> Int:\n    return x\n"),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("parameterized main should fail");
        assert!(
            err.contains("main must not take parameters in the current runtime lane"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_main_with_non_runtime_return_type() {
        let root = make_temp_package(
            "main_with_bool_return",
            "app",
            &[],
            &[
                ("src/shelf.arc", "fn main() -> Bool:\n    return true\n"),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("non-runtime main return should fail");
        assert!(
            err.contains("main must return Int or Unit in the current runtime lane"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_sources_rejects_tuple_contract_fixtures() {
        let repo_root = repo_root();
        for (fixture, expected) in [
            (
                "tuple_field_out_of_range.arc",
                "tuple field access only supports `.0` and `.1` in v1",
            ),
            (
                "tuple_destructure_let.arc",
                "tuple destructuring is not allowed in `let` statements",
            ),
            (
                "tuple_triple_type.arc",
                "tuple types must have exactly 2 elements in v1",
            ),
            (
                "tuple_field_assignment.arc",
                "tuple field assignment is not allowed in v1",
            ),
        ] {
            let source = fs::read_to_string(
                repo_root
                    .join("conformance")
                    .join("check_parity_fixtures")
                    .join(fixture),
            )
            .expect("fixture should be readable");
            let err = check_sources([source.as_str()]).expect_err("fixture should fail");
            assert!(err.contains(expected), "{fixture}: {err}");
        }
    }

    #[test]
    fn check_sources_rejects_page_rollup_contract_fixtures() {
        let repo_root = repo_root();
        for (fixture, expected) in [
            (
                "page_rollup_stray.arc",
                "page rollup without a valid owning header",
            ),
            (
                "page_rollup_bad_subject.arc",
                "cleanup subject must be a binding name",
            ),
            (
                "page_rollup_unknown_subject.arc",
                "cleanup subject `missing` is not available in the owning header scope",
            ),
            (
                "page_rollup_reassign.arc",
                "cleanup subject `local` cannot be reassigned after activation",
            ),
        ] {
            let source = fs::read_to_string(
                repo_root
                    .join("conformance")
                    .join("check_parity_fixtures")
                    .join(fixture),
            )
            .expect("fixture should be readable");
            let err = check_sources([source.as_str()]).expect_err("fixture should fail");
            assert!(err.contains(expected), "{fixture}: {err}");
        }
    }

    #[test]
    fn check_sources_rejects_foreword_and_intrinsic_contract_fixtures() {
        let repo_root = repo_root();
        for (fixture, expected) in [
            (
                "invalid_statement_foreword.arc",
                "`#inline` is not a valid statement-level contract",
            ),
            (
                "phrase_arg_arity.arc",
                "qualified phrase allows at most 3 top-level arguments",
            ),
            (
                "phrase_arg_shape.arc",
                "trailing comma is not allowed before phrase qualifier",
            ),
            (
                "memory_phrase_arg_arity.arc",
                "memory phrase allows at most 3 top-level arguments",
            ),
            (
                "unknown_memory_type.arc",
                "unknown memory type `weird`; supported now: arena, frame, pool (reserved for future expansion)",
            ),
            (
                "unknown_chain_style.arc",
                "unknown chain style `mystery`; supported: forward, lazy, parallel, async, plan, broadcast, collect",
            ),
            (
                "reverse_parallel_chain.arc",
                "chain style `parallel` does not support reverse-introduced chains",
            ),
            (
                "unknown_memory_type.arc",
                "unknown memory type `weird`; supported now: arena, frame, pool (reserved for future expansion)",
            ),
            (
                "invalid_boundary_payload.arc",
                "invalid payload for foreword `#boundary`: `target` must be a string or symbol",
            ),
            (
                "test_payload.arc",
                "invalid payload for foreword `#test`: expected no payload",
            ),
            (
                "invalid_stage_contract_key.arc",
                "invalid #stage contract key 'bad_key'",
            ),
            (
                "invalid_chain_contract_key.arc",
                "invalid #chain contract key 'bad_key'",
            ),
            (
                "invalid_chain_contract_phase.arc",
                "invalid payload for `phase`",
            ),
            (
                "malformed_intrinsic.arc",
                "malformed intrinsic function declaration",
            ),
        ] {
            let source = fs::read_to_string(
                repo_root
                    .join("conformance")
                    .join("check_parity_fixtures")
                    .join(fixture),
            )
            .expect("fixture should be readable");
            let err = check_sources([source.as_str()]).expect_err("fixture should fail");
            assert!(err.contains(expected), "{fixture}: {err}");
        }
    }

    #[test]
    fn check_path_rejects_unresolved_tuple_value_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("unresolved_tuple_value_ref"),
        )
        .expect_err("fixture should fail");
        assert!(
            err.contains("unresolved value reference `missing` in value expression"),
            "{err}"
        );
    }

    #[test]
    fn check_path_resolves_local_use_symbols() {
        let root = make_temp_package(
            "counter_app",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "import types\nuse types.Counter\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", "export record Counter:\n    value: Int\n"),
            ],
        );

        let summary = check_path(&root).expect("local symbols should resolve");
        assert_eq!(summary.module_count, 2);
        assert_eq!(summary.package_count, 1);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn typed_api_fingerprint_ignores_equivalent_export_type_spelling() {
        let root = make_temp_workspace(
            "typed_api_surface_spelling",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    let value = core.make_counter :: :: call\n    return value.value\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "import types\nuse types.Counter\nexport fn make_counter() -> Counter:\n    return pool: entities :> value = 0 <: Counter\n",
                ),
                (
                    "core/src/types.arc",
                    "export record Counter:\n    value: Int\n",
                ),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_planned_build(&graph, &first_fingerprints, &first_statuses);
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        fs::write(
            root.join("core/src/book.arc"),
            "import types\nexport fn make_counter() -> types.Counter:\n    return pool: entities :> value = 0 <: types.Counter\n",
        )
        .expect("rewrite should succeed");

        let second_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let second_statuses =
            plan_build(&graph, &order, &second_fingerprints, Some(&existing)).expect("plan");
        assert_eq!(second_statuses[0].member(), "core");
        assert_eq!(second_statuses[0].disposition(), BuildDisposition::Built);
        assert_eq!(second_statuses[1].member(), "app");
        assert_eq!(second_statuses[1].disposition(), BuildDisposition::Built);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn compute_member_fingerprints_reuses_checked_workspace_state() {
        let root = make_temp_workspace(
            "typed_api_checked_workspace",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    return core.value :: :: call\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn value() -> Int:\n    return 0\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let direct = compute_member_fingerprints(&graph).expect("direct fingerprints");
        let reused = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("reused fingerprints");
        assert_eq!(direct, reused);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn member_source_fingerprint_ignores_whitespace_only_edits() {
        let root = make_temp_workspace(
            "typed_api_whitespace_source",
            &["app"],
            &[
                ("app/book.toml", "name = \"app\"\nkind = \"app\"\n"),
                ("app/src/shelf.arc", "fn main() -> Int:\n    return 0\n"),
                ("app/src/types.arc", ""),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_planned_build(&graph, &first_fingerprints, &first_statuses);
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        fs::write(
            root.join("app/src/shelf.arc"),
            "fn main() -> Int:\n\n    return 0\n",
        )
        .expect("rewrite should succeed");

        let second_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        assert_eq!(
            first_fingerprints
                .get("app")
                .map(|fingerprint| &fingerprint.source),
            second_fingerprints
                .get("app")
                .map(|fingerprint| &fingerprint.source)
        );
        assert_eq!(
            first_fingerprints
                .get("app")
                .map(|fingerprint| &fingerprint.api),
            second_fingerprints
                .get("app")
                .map(|fingerprint| &fingerprint.api)
        );

        let second_statuses =
            plan_build(&graph, &order, &second_fingerprints, Some(&existing)).expect("plan");
        assert!(
            second_statuses
                .iter()
                .all(|status| status.disposition() == BuildDisposition::CacheHit)
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn typed_api_fingerprint_ignores_private_dependency_code_edits() {
        let root = make_temp_workspace(
            "typed_api_private_dependency_code",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    return core.shared_value :: :: call\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn shared_value() -> Int:\n    return helper :: :: call\n",
                ),
                ("core/src/helper.arc", "fn helper() -> Int:\n    return 0\n"),
                ("core/src/types.arc", ""),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_planned_build(&graph, &first_fingerprints, &first_statuses);
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        fs::write(
            root.join("core/src/helper.arc"),
            "fn helper() -> Int:\n    return 1\n",
        )
        .expect("rewrite should succeed");

        let second_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        assert_ne!(
            first_fingerprints
                .get("core")
                .map(|fingerprint| &fingerprint.source),
            second_fingerprints
                .get("core")
                .map(|fingerprint| &fingerprint.source)
        );
        assert_eq!(
            first_fingerprints
                .get("core")
                .map(|fingerprint| &fingerprint.api),
            second_fingerprints
                .get("core")
                .map(|fingerprint| &fingerprint.api)
        );

        let second_statuses =
            plan_build(&graph, &order, &second_fingerprints, Some(&existing)).expect("plan");
        assert_eq!(second_statuses[0].member(), "core");
        assert_eq!(second_statuses[0].disposition(), BuildDisposition::Built);
        assert_eq!(second_statuses[1].member(), "app");
        assert_eq!(second_statuses[1].disposition(), BuildDisposition::Built);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_rejects_private_dependency_symbol_use() {
        let root = make_temp_workspace(
            "private_dependency_symbol_use",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "use core.hidden\nfn main() -> Int:\n    return hidden :: :: call\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn shared() -> Int:\n    return 1\nfn hidden() -> Int:\n    return 0\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let err = match check_workspace_graph(&graph) {
            Ok(_) => panic!("workspace should not check"),
            Err(err) => err,
        };
        assert!(
            err.contains("unresolved symbol `hidden` in module `core`"),
            "unexpected error: {err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_workspace_rejects_trait_where_impls_satisfied_only_by_unrelated_members() {
        let root = make_temp_workspace(
            "where_impl_visibility_scope",
            &["app", "core", "stray"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    concat!(
                        "use core.Ord\n",
                        "impl Ord[Int] for Int:\n",
                        "    fn cmp(read self: Int) -> Int:\n",
                        "        return 0\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    concat!(
                        "export trait Eq[T]:\n",
                        "    fn ok(read self: T) -> Int:\n",
                        "        return 0\n",
                        "export trait Ord[T, where Eq[T]]:\n",
                        "    fn cmp(read self: T) -> Int:\n",
                        "        return 0\n",
                    ),
                ),
                ("core/src/types.arc", ""),
                (
                    "stray/book.toml",
                    "name = \"stray\"\nkind = \"lib\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "stray/src/book.arc",
                    concat!(
                        "use core.Eq\n",
                        "impl Eq[Int] for Int:\n",
                        "    fn ok(read self: Int) -> Int:\n",
                        "        return 0\n",
                    ),
                ),
                ("stray/src/types.arc", ""),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let err = match check_workspace_graph(&graph) {
            Ok(_) => panic!("workspace should not check"),
            Err(err) => err,
        };
        assert!(
            err.contains("impl requires satisfying where-bound `Eq[Int]`"),
            "unexpected error: {err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn typed_api_fingerprint_rebuilds_dependents_for_boundary_contract_changes() {
        let root = make_temp_workspace(
            "typed_api_boundary_contract",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    return core.boundary_value :: 1 :: call\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "#boundary[target = \"lua\"]\nexport fn boundary_value(value: Int) -> Int:\n    return value\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_planned_build(&graph, &first_fingerprints, &first_statuses);
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        fs::write(
            root.join("core/src/book.arc"),
            "#boundary[target = \"sql\"]\nexport fn boundary_value(value: Int) -> Int:\n    return value\n",
        )
        .expect("rewrite should succeed");

        let second_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let second_statuses =
            plan_build(&graph, &order, &second_fingerprints, Some(&existing)).expect("plan");
        assert_eq!(second_statuses[0].member(), "core");
        assert_eq!(second_statuses[0].disposition(), BuildDisposition::Built);
        assert_eq!(second_statuses[1].member(), "app");
        assert_eq!(second_statuses[1].disposition(), BuildDisposition::Built);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn typed_api_fingerprint_rebuilds_dependents_for_public_impl_method_changes() {
        let root = make_temp_workspace(
            "typed_api_impl_methods",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    let counter = core.make_counter :: :: call\n    return counter :: 1 :: add\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "import types\nuse types.Counter\nexport fn make_counter() -> Counter:\n    return pool: entities :> value = 0 <: Counter\n\nimpl Counter:\n    fn add(self: Counter, value: Int) -> Int:\n        return self.value + value\n",
                ),
                (
                    "core/src/types.arc",
                    "export record Counter:\n    value: Int\n",
                ),
            ],
        );

        let graph = load_workspace_graph(&root).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_planned_build(&graph, &first_fingerprints, &first_statuses);
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        fs::write(
            root.join("core/src/book.arc"),
            "import types\nuse types.Counter\nexport fn make_counter() -> Counter:\n    return pool: entities :> value = 0 <: Counter\n\nimpl Counter:\n    fn add(self: Counter, value: Int, scale: Int) -> Int:\n        return self.value + value + scale\n",
        )
        .expect("rewrite should succeed");

        let second_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let second_statuses =
            plan_build(&graph, &order, &second_fingerprints, Some(&existing)).expect("plan");
        assert_eq!(second_statuses[0].member(), "core");
        assert_eq!(second_statuses[0].disposition(), BuildDisposition::Built);
        assert_eq!(second_statuses[1].member(), "app");
        assert_eq!(second_statuses[1].disposition(), BuildDisposition::Built);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_handles_owned_desktop_grimoire() {
        let summary = check_path(&owned_app_root().join("arcana-desktop"))
            .expect("owned desktop grimoire should check");
        assert!(summary.package_count >= 2);
        assert!(summary.module_count >= 6);
    }

    #[test]
    fn check_path_handles_owned_graphics_grimoire() {
        let summary = check_path(&owned_app_root().join("arcana-graphics"))
            .expect("owned graphics grimoire should check");
        assert!(summary.package_count >= 2);
        assert!(summary.module_count >= 5);
    }

    #[test]
    fn check_path_handles_owned_text_grimoire() {
        let summary = check_path(&owned_app_root().join("arcana-text"))
            .expect("owned text grimoire should check");
        assert!(summary.package_count >= 2);
        assert!(summary.module_count >= 4);
    }

    #[test]
    fn check_path_handles_owned_audio_grimoire() {
        let summary = check_path(&owned_app_root().join("arcana-audio"))
            .expect("owned audio grimoire should check");
        assert!(summary.package_count >= 2);
        assert!(summary.module_count >= 5);
    }

    #[test]
    fn check_path_accepts_builtin_foreword_package() {
        let root = make_temp_package(
            "builtin_foreword_positive",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "#test\nfn smoke() -> Int:\n    return 0\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("builtin foreword package should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_boundary_interop_package() {
        let root = make_temp_package(
            "boundary_interop_positive",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "import types\n#boundary[target = \"lua\"]\nexport fn bridge(read payload: types.Payload) -> Int:\n    return payload.value\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", "export record Payload:\n    value: Int\n"),
            ],
        );

        let summary = check_path(&root).expect("boundary interop package should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_handles_std_intrinsics() {
        let summary = check_path(&repo_root().join("std")).expect("std should check");
        assert!(summary.package_count >= 1);
        assert!(summary.module_count >= 10);
    }

    #[test]
    fn check_path_accepts_page_rollup_package() {
        let root = make_temp_package(
            "page_rollup_positive",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn cleanup(value: Int):\n    return\nfn run(seed: Int) -> Int:\n    let local = seed\n    while local > 0:\n        let scratch = local\n        local -= 1\n    [scratch, cleanup]#cleanup\n    return local\n[seed, cleanup]#cleanup\nfn main() -> Int:\n    return run :: 1 :: call\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("page rollup package should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_async_page_rollup_handler() {
        let root = make_temp_package(
            "page_rollup_async_handler",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "async fn cleanup(value: Int):\n    return\nfn main() -> Int:\n    let value = 1\n    return 0\n[value, cleanup]#cleanup\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("async rollup handler should fail");
        assert!(
            err.contains("page rollup handler `cleanup` cannot be async in v1"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_non_callable_page_rollup_handler() {
        let root = make_temp_package(
            "page_rollup_non_callable_handler",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "record Cleaner:\n    id: Int\nfn main() -> Int:\n    let value = 1\n    return 0\n[value, Cleaner]#cleanup\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("non-callable rollup handler should fail");
        assert!(
            err.contains("page rollup handler `Cleaner` must resolve to a callable symbol"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_wrong_arity_page_rollup_handler() {
        let root = make_temp_package(
            "page_rollup_wrong_arity_handler",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn cleanup() -> Int:\n    return 0\nfn main() -> Int:\n    let value = 1\n    return 0\n[value, cleanup]#cleanup\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("wrong-arity rollup handler should fail");
        assert!(
            err.contains("page rollup handler `cleanup` must accept exactly one parameter in v1"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_cleanup_subject_move_after_activation() {
        let root = make_temp_package(
            "page_rollup_moved_subject",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn cleanup(value: Str):\n    return\nfn consume(take value: Str):\n    return\nfn main() -> Int:\n    let text = \"hi\"\n    consume :: text :: call\n    return 0\n[text, cleanup]#cleanup\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("moved cleanup subject should fail");
        assert!(
            err.contains("cleanup subject `text` cannot be moved after activation"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_unresolved_lang_item_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("unresolved_lang_item"),
        )
        .expect_err("unresolved lang item package should fail");
        assert!(
            err.contains("unresolved `lang` item target `Missing` for `result`"),
            "{err}"
        );
    }

    #[test]
    fn check_path_rejects_invalid_boundary_and_test_packages() {
        let repo_root = repo_root()
            .join("conformance")
            .join("check_parity_packages");
        for (package, expected) in [
            (
                "invalid_boundary_signature",
                "`#boundary` target `lua` does not allow mutable borrows",
            ),
            (
                "nested_boundary_unsafe",
                "type `types.Payload` is not boundary-safe for target `lua`",
            ),
            (
                "invalid_test_signature",
                "`#test` functions must have zero parameters",
            ),
        ] {
            let err = check_path(&repo_root.join(package)).expect_err("package should fail");
            assert!(err.contains(expected), "{package}: {err}");
        }
    }

    #[test]
    fn check_path_rejects_unresolved_type_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("unresolved_type_ref"),
        )
        .expect_err("unresolved type package should fail");
        assert!(
            err.contains("unresolved type reference `MissingType` in field type `value`"),
            "{err}"
        );
    }

    #[test]
    fn check_path_rejects_undeclared_lifetime_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("undeclared_lifetime"),
        )
        .expect_err("undeclared lifetime package should fail");
        assert!(
            err.contains("undeclared lifetime `'a` in parameter type `value`"),
            "{err}"
        );
    }

    #[test]
    fn check_path_rejects_unresolved_body_value_packages() {
        let repo_root = repo_root()
            .join("conformance")
            .join("check_parity_packages");
        for (package, expected) in [
            (
                "unresolved_value_ref",
                "unresolved value reference `missing` in value expression",
            ),
            (
                "unresolved_namespace_member_ref",
                "unresolved value reference `std.kernel.text.missing` in value expression",
            ),
            (
                "unresolved_expr_type_arg",
                "unresolved type reference `Missing` in expression generic argument `Missing`",
            ),
            (
                "unresolved_chain_step",
                "unresolved value reference `missing` in chain step `missing`",
            ),
            (
                "unresolved_rollup_handler",
                "unresolved page rollup handler `missing.cleanup`",
            ),
        ] {
            let err = check_path(&repo_root.join(package))
                .expect_err("unresolved body semantic package should fail");
            assert!(err.contains(expected), "{package}: {err}");
        }
    }

    #[test]
    fn check_path_rejects_non_bool_if_condition() {
        let root = make_temp_package(
            "typed_if_condition",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    if 1:\n        return 0\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("non-bool if condition should fail");
        assert!(
            err.contains("if condition requires Bool, found Int"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_non_bool_not_operand() {
        let root = make_temp_package(
            "typed_not_operand",
            "app",
            &[],
            &[
                ("src/shelf.arc", "fn main() -> Bool:\n    return not 1\n"),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("non-bool `not` operand should fail");
        assert!(
            err.contains("operand of `not` requires Bool, found Int"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_non_int_shift_operand() {
        let root = make_temp_package(
            "typed_shift_operand",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    return 1 << \"x\"\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("non-int shift operand should fail");
        assert!(
            err.contains("right operand of `<<` requires Int, found Str"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_tuple_projection_on_non_pair() {
        let root = make_temp_package(
            "typed_tuple_projection",
            "app",
            &[],
            &[
                ("src/shelf.arc", "fn main() -> Int:\n    return (\"x\").0\n"),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("tuple projection on non-pair should fail");
        assert!(
            err.contains("tuple field access `.0` requires a pair value, found Str"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_borrow_of_non_place_expression() {
        let root = make_temp_package(
            "typed_non_place_borrow",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    let x = &(1 + 2)\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("borrow of non-place should fail");
        assert!(
            err.contains("operand of `&` must be a local place expression"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_mutable_borrow_of_immutable_local() {
        let root = make_temp_package(
            "typed_immutable_mut_borrow",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    let x = 1\n    let y = &mut x\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("mutable borrow of immutable local should fail");
        assert!(
            err.contains("cannot mutably borrow immutable local `x`"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_mutable_borrow_while_shared_borrow_active() {
        let root = make_temp_package(
            "typed_borrow_conflict",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    let mut x = 1\n    let a = &x\n    let b = &mut x\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("conflicting borrows should fail");
        assert!(
            err.contains("cannot mutably borrow `x` while it is already borrowed"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_assignment_while_local_borrowed() {
        let root = make_temp_package(
            "typed_assign_while_borrowed",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    let mut x = 1\n    let a = &x\n    x = 2\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("assignment while borrowed should fail");
        assert!(
            err.contains("cannot assign to local `x` while it is borrowed"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_direct_access_while_mutably_borrowed() {
        let root = make_temp_package(
            "typed_mut_borrow_direct_access",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main() -> Int:\n    let mut x = 1\n    let a = &mut x\n    let y = x + 1\n    return y\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("direct access while mutably borrowed should fail");
        assert!(
            err.contains("cannot access local `x` directly while it is mutably borrowed"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_allows_assignment_to_edit_param() {
        let root = make_temp_package(
            "typed_edit_param_assign",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn bump(edit n: Int):\n    n = n + 1\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("assignment to edit param should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_untied_return_lifetime() {
        let root = make_temp_package(
            "typed_untied_return_lifetime",
            "app",
            &[],
            &[
                ("src/shelf.arc", "fn bad['a]() -> &'a Int:\n    return 1\n"),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("untied return lifetime should fail");
        assert!(
            err.contains("return lifetime `'a` must be tied to an input parameter"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_structured_where_predicates() {
        let root = make_temp_package(
            "typed_where_ok",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "trait Iterator[I]:\n    type Item\nfn main['a, I, U, where Iterator[I], Iterator[I].Item = U, U: 'a]() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("structured where predicates should check");
        assert_eq!(summary.package_count, 1);
        assert!(summary.module_count >= 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_non_trait_where_predicates() {
        let root = make_temp_package(
            "typed_where_non_trait",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "record NotTrait:\n    value: Int\nfn main[T, where NotTrait]() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("non-trait where predicate should fail");
        assert!(err.contains("does not resolve to a valid trait"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_malformed_projection_equality_where_predicates() {
        let root = make_temp_package(
            "typed_where_bad_projection",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "trait Iterator[I]:\n    type Item\nfn main[I, U, where Iterator[I] = U]() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("bad projection equality should fail");
        assert!(
            err.contains("unsupported top-level syntax")
                || err.contains("projection-equality predicate `Iterator[I]`"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_undeclared_outlives_where_lifetimes() {
        let root = make_temp_package(
            "typed_where_bad_outlives",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main['a, T, where T: 'b]() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("undeclared outlives lifetime should fail");
        assert!(
            err.contains("undeclared lifetime `'b` in where predicate `T: 'b`"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_projection_equality_unknown_assoc_types() {
        let root = make_temp_package(
            "typed_where_unknown_assoc",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "trait Iterator[I]:\n    type Item\nfn main[I, U, where Iterator[I].Missing = U]() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err =
            check_path(&root).expect_err("projection equality with unknown assoc type should fail");
        assert!(
            err.contains("does not declare associated type `Missing`"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_projection_equality_where_predicates() {
        let root = make_temp_package(
            "typed_where_projection_ok",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "trait Iterator[I]:\n    type Item\nfn main[I, U, where Iterator[I].Item = U]() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        check_path(&root).expect("projection equality with known assoc type should check");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_declared_outlives_where_predicates() {
        let root = make_temp_package(
            "typed_where_outlives_ok",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn main['a, 'b, T, where 'a: 'b, T: 'a]() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        check_path(&root).expect("declared outlives predicates should check");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_impls_missing_trait_where_requirements() {
        let root = make_temp_package(
            "typed_where_missing_impl_req",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "trait Eq[T]:\n",
                        "    fn ok(read self: T) -> Int:\n",
                        "        return 0\n",
                        "trait Ord[T, where Eq[T]]:\n",
                        "    fn cmp(read self: T) -> Int:\n",
                        "        return 0\n",
                        "impl Ord[Int] for Int:\n",
                        "    fn cmp(read self: Int) -> Int:\n",
                        "        return 0\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("missing required trait impl should fail");
        assert!(
            err.contains("impl requires satisfying where-bound `Eq[Int]`"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_impls_with_trait_where_requirements_satisfied() {
        let root = make_temp_package(
            "typed_where_impl_req_ok",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "trait Eq[T]:\n",
                        "    fn ok(read self: T) -> Int:\n",
                        "        return 0\n",
                        "trait Ord[T, where Eq[T]]:\n",
                        "    fn cmp(read self: T) -> Int:\n",
                        "        return 0\n",
                        "impl Eq[Int] for Int:\n",
                        "    fn ok(read self: Int) -> Int:\n",
                        "        return 0\n",
                        "impl Ord[Int] for Int:\n",
                        "    fn cmp(read self: Int) -> Int:\n",
                        "        return 0\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );

        check_path(&root).expect("satisfied trait where requirements should check");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_ambiguous_concrete_bare_methods() {
        let root = make_temp_package(
            "ambiguous_bare_method_concrete",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "import types\n",
                        "use types.Counter\n",
                        "fn main() -> Int:\n",
                        "    let counter = Counter :: value = 1 :: call\n",
                        "    return counter :: :: tap\n",
                    ),
                ),
                ("src/types.arc", "export record Counter:\n    value: Int\n"),
                (
                    "src/left.arc",
                    concat!(
                        "import types\n",
                        "use types.Counter\n",
                        "impl Counter:\n",
                        "    fn tap(read self: Counter) -> Int:\n",
                        "        return self.value + 1\n",
                    ),
                ),
                (
                    "src/right.arc",
                    concat!(
                        "import types\n",
                        "use types.Counter\n",
                        "impl Counter:\n",
                        "    fn tap(read self: Counter) -> Int:\n",
                        "        return self.value + 2\n",
                    ),
                ),
            ],
        );

        let err = check_path(&root).expect_err("ambiguous concrete bare method should fail");
        assert!(
            err.contains("bare-method qualifier `tap` on `ambiguous_bare_method_concrete.types.Counter` is ambiguous"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_ignores_unrelated_workspace_methods_for_bare_resolution() {
        let root = make_temp_workspace(
            "unrelated_bare_method_scope",
            &["app", "core", "extra"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    concat!(
                        "import core.types\n",
                        "use core.types.Counter\n",
                        "fn main() -> Int:\n",
                        "    let counter = Counter :: value = 1 :: call\n",
                        "    return counter :: :: tap\n",
                    ),
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "import types\nuse types.Counter\nimpl Counter:\n    fn tap(read self: Counter) -> Int:\n        return self.value + 1\n",
                ),
                (
                    "core/src/types.arc",
                    "export record Counter:\n    value: Int\n",
                ),
                ("extra/book.toml", "name = \"extra\"\nkind = \"lib\"\n"),
                (
                    "extra/src/book.arc",
                    "import types\nuse types.Counter\nimpl Counter:\n    fn tap(read self: Counter) -> Int:\n        return self.value + 99\n",
                ),
                (
                    "extra/src/types.arc",
                    "export record Counter:\n    value: Int\n",
                ),
            ],
        );

        check_path(&root.join("app"))
            .expect("unrelated workspace package should not affect bare methods");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_duplicate_top_level_symbols() {
        let root = make_temp_package(
            "typed_duplicate_top_level_symbol",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "export fn mouse_in_window(read win: Window) -> Bool:\n    return false\nexport fn mouse_in_window(read win: Window) -> Bool:\n    return true\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("duplicate top-level symbol should fail");
        assert!(err.contains("duplicate symbol `mouse_in_window`"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_duplicate_directive_bindings() {
        let root = make_temp_workspace(
            "typed_duplicate_directive_binding",
            &["app", "io", "text"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\nio = { path = \"../io\" }\ntext = { path = \"../text\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "use io as io\nuse text as io\nfn main() -> Int:\n    return 0\n",
                ),
                ("app/src/types.arc", ""),
                ("io/book.toml", "name = \"io\"\nkind = \"lib\"\n"),
                (
                    "io/src/book.arc",
                    "export fn print() -> Int:\n    return 0\n",
                ),
                ("io/src/types.arc", ""),
                ("text/book.toml", "name = \"text\"\nkind = \"lib\"\n"),
                (
                    "text/src/book.arc",
                    "export fn print() -> Int:\n    return 0\n",
                ),
                ("text/src/types.arc", ""),
            ],
        );

        let err =
            check_path(&root.join("app")).expect_err("duplicate directive binding should fail");
        assert!(err.contains("duplicate binding `io`"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_return_borrow_of_local() {
        let root = make_temp_package(
            "typed_return_local_borrow",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn bad['a](read value: &'a Int) -> &'a Int:\n    let x = 1\n    return &x\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("return borrow of local should fail");
        assert!(
            err.contains(
                "returned reference must be tied to input lifetimes; local `x` does not live long enough"
            ),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_use_after_take_move() {
        let root = make_temp_package(
            "typed_take_use_after_move",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn consume(take value: Str):\n    return\nfn main() -> Int:\n    let s = \"hi\"\n    consume :: s :: call\n    s :: :: std.io.print\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("use after take-move should fail");
        assert!(err.contains("use of moved local `s`"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_take_move_while_borrowed() {
        let root = make_temp_package(
            "typed_take_while_borrowed",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn consume(take value: Str):\n    return\nfn main() -> Int:\n    let s = \"hi\"\n    let r = &s\n    consume :: s :: call\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("moving borrowed local should fail");
        assert!(
            err.contains("cannot move local `s` while it is borrowed"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_allows_copy_value_after_take_param_call() {
        let root = make_temp_package(
            "typed_take_copy_ok",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn consume(take value: Int):\n    return\nfn main() -> Int:\n    let x = 1\n    consume :: x :: call\n    return x\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("copy value after take call should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_non_place_edit_param_call() {
        let root = make_temp_package(
            "typed_edit_non_place",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn bump(edit n: Int):\n    n = n + 1\nfn main() -> Int:\n    bump :: (1 + 2) :: call\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("edit call on non-place should fail");
        assert!(
            err.contains("argument for edit parameter `n` must be a local place expression"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_immutable_local_edit_param_call() {
        let root = make_temp_package(
            "typed_edit_immutable_local",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn bump(edit n: Int):\n    n = n + 1\nfn main() -> Int:\n    let x = 1\n    bump :: x :: call\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("edit call on immutable local should fail");
        assert!(
            err.contains("cannot pass immutable local `x` to edit parameter `n`"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_method_style_take_use_after_move() {
        let root = make_temp_package(
            "typed_method_take_move",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "record Bag:\n    n: Int\nimpl Bag:\n    fn push(edit self: Bag, take value: Str):\n        self.n = self.n + 1\nfn main() -> Int:\n    let bag = Bag :: n = 0 :: call\n    let s = \"hi\"\n    bag :: s :: push\n    let t = s\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("method-style take move should fail");
        assert!(err.contains("use of moved local `s`"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_window_use_after_close() {
        let std_dep = repo_root().join("std").to_string_lossy().replace('\\', "/");
        let root = make_temp_package(
            "typed_window_use_after_close",
            "app",
            &[("std", std_dep.as_str())],
            &[
                (
                    "src/shelf.arc",
                    "import std.window\nuse std.window.Window\nfn bad(take win: Window) -> Int:\n    std.window.close :: win :: call\n    let alive = std.window.alive :: win :: call\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("window use after close should fail");
        assert!(err.contains("use of moved local `win`"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_stream_use_after_close() {
        let std_dep = repo_root().join("std").to_string_lossy().replace('\\', "/");
        let root = make_temp_package(
            "typed_stream_use_after_close",
            "app",
            &[("std", std_dep.as_str())],
            &[
                (
                    "src/shelf.arc",
                    "import std.fs\nuse std.fs.FileStream\nfn bad(take stream: FileStream) -> Int:\n    std.fs.stream_close :: stream :: call\n    let done = std.fs.stream_eof :: stream :: call\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("stream use after close should fail");
        assert!(err.contains("use of moved local `stream`"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_opaque_type_outside_std() {
        let root = make_temp_package(
            "opaque_type_outside_std",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "export opaque type Token as move, boundary_safe\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        check_path(&root).expect("opaque types outside std should check");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_grimoire_opaque_family_lang_item() {
        let root = make_temp_package(
            "desktop",
            "lib",
            &[],
            &[
                (
                    "src/types.arc",
                    concat!(
                        "export opaque type Window as move, boundary_unsafe\n",
                        "lang window_handle = Window\n",
                    ),
                ),
                ("src/book.arc", "reexport desktop.types\n"),
            ],
        );

        check_path(&root).expect("grimoire opaque family binding should check");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_duplicate_opaque_family_lang_item_in_package() {
        let root = make_temp_package(
            "desktop",
            "lib",
            &[],
            &[
                (
                    "src/types.arc",
                    concat!(
                        "export opaque type Window as move, boundary_unsafe\n",
                        "lang window_handle = Window\n",
                    ),
                ),
                (
                    "src/extra.arc",
                    concat!(
                        "export opaque type AltWindow as move, boundary_unsafe\n",
                        "lang window_handle = AltWindow\n",
                    ),
                ),
                ("src/book.arc", "reexport desktop.types\nreexport desktop.extra\n"),
            ],
        );

        let err = check_path(&root).expect_err("duplicate opaque family binding should fail");
        assert!(
            err.contains("opaque family lang item `window_handle` is declared more than once"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_std_owned_opaque_type_impl_target() {
        let root = make_temp_package(
            "std",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "export opaque type Token[T] as move, boundary_safe\nimpl[T] Token[T]:\n    fn id(read self: Token[T]) -> Int:\n        return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        check_path(&root).expect("std opaque type impl target should check");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_std_opaque_type_builtin_name_collision() {
        let root = make_temp_package(
            "std",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "export opaque type Int as move, boundary_safe\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("builtin-name opaque type should fail");
        assert!(
            err.contains("opaque type `Int` conflicts with reserved builtin type name"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_opaque_type_constructor_use() {
        let std_dep = repo_root().join("std").to_string_lossy().replace('\\', "/");
        let root = make_temp_package(
            "opaque_type_constructor_use",
            "app",
            &[("std", std_dep.as_str())],
            &[
                (
                    "src/shelf.arc",
                    "use std.window.Window\nfn bad() -> Int:\n    let win = Window :: :: call\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("opaque constructors should fail");
        assert!(
            err.contains("opaque type `Window` is not constructible"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_rejects_boundary_unsafe_std_opaque_type() {
        let std_dep = repo_root().join("std").to_string_lossy().replace('\\', "/");
        let root = make_temp_package(
            "opaque_type_boundary_contract",
            "app",
            &[("std", std_dep.as_str())],
            &[
                (
                    "src/shelf.arc",
                    "use std.window.Window\n#boundary[target = \"lua\"]\nexport fn bad(read win: Window) -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("boundary-unsafe opaque type should fail");
        assert!(
            err.contains("type `Window` is not boundary-safe for target `lua`"),
            "{err}"
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_return_lifetime_tied_to_param() {
        let root = make_temp_package(
            "typed_tied_return_lifetime",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "fn keep['a](read value: &'a Int) -> &'a Int:\n    return value\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("tied return lifetime should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_handles_local_member_field_access() {
        let root = make_temp_package(
            "local_member_access",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "record Box:\n    value: Int\nfn main() -> Int:\n    let item = Box :: value = 1 :: call\n    return item.value\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("local member access should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_sources_accepts_object_owner_activation_flow() {
        let summary = check_sources(
            [concat!(
                "obj Counter:\n",
                "    value: Int\n",
                "\n",
                "create Session [Counter] scope-exit:\n",
                "    done: when Counter.value > 0 hold [Counter]\n",
                "\n",
                "Session\n",
                "Counter\n",
                "fn main() -> Int:\n",
                "    let active = Session :: :: call\n",
                "    Counter.value = 1\n",
                "    return Counter.value\n",
            )]
            .iter()
            .copied(),
        )
        .expect("object/owner flow should check");
        assert!(summary.symbol_count >= 3);
    }

    #[test]
    fn check_sources_accepts_object_only_attached_owner_flow() {
        let summary = check_sources(
            [concat!(
                "obj Counter:\n",
                "    value: Int\n",
                "\n",
                "create Session [Counter] scope-exit:\n",
                "    done: when Counter.value > 0 hold [Counter]\n",
                "\n",
                "Counter\n",
                "fn bump() -> Int:\n",
                "    Counter.value += 1\n",
                "    return Counter.value\n",
                "\n",
                "Session\n",
                "Counter\n",
                "fn main() -> Int:\n",
                "    let active = Session :: :: call\n",
                "    Counter.value = 4\n",
                "    return bump :: :: call\n",
            )]
            .iter()
            .copied(),
        )
        .expect("object-only attached owner flow should check");
        assert!(summary.symbol_count >= 3);
    }

    #[test]
    fn check_sources_rejects_owner_without_scope_exit_clause() {
        let root = make_temp_package(
            "owner_missing_exit",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "obj Counter:\n",
                        "    value: Int\n",
                        "\n",
                        "create Session [Counter] scope-exit:\n",
                        "\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );
        let err = check_path(&root).expect_err("owner without exit should fail");
        assert!(
            err.contains("must declare at least one scope-exit"),
            "{err}"
        );
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_sources_rejects_non_bool_owner_exit_condition() {
        let root = make_temp_package(
            "owner_non_bool_exit",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "obj Counter:\n",
                        "    value: Int\n",
                        "\n",
                        "create Session [Counter] scope-exit:\n",
                        "    exit when 1\n",
                        "\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );
        let err = check_path(&root).expect_err("non-bool owner exit condition should fail");
        assert!(
            err.contains("owner exit condition requires Bool, found Int"),
            "{err}"
        );
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_accepts_object_owner_lifecycle_conformance_package() {
        let summary = check_path(
            &repo_root()
                .join("conformance")
                .join("fixtures")
                .join("object_owner_lifecycle_workspace")
                .join("app"),
        )
        .expect("lifecycle conformance fixture should check");
        assert!(summary.package_count >= 1);
        assert!(summary.module_count >= 2);
    }

    #[test]
    fn check_path_rejects_owner_invalid_lifecycle_hook_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("owner_invalid_lifecycle_hook"),
        )
        .expect_err("invalid lifecycle hook fixture should fail");
        assert!(err.contains("must take `edit self`"), "{err}");
    }

    #[test]
    fn check_path_rejects_owner_wrong_context_type_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("owner_wrong_context_type"),
        )
        .expect_err("wrong context type fixture should fail");
        assert!(err.contains("expects context"), "{err}");
    }

    #[test]
    fn check_sources_rejects_invalid_object_lifecycle_hook_signature() {
        let root = make_temp_package(
            "owner_invalid_hook",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "obj Counter:\n",
                        "    value: Int\n",
                        "    fn init(read self: Self):\n",
                        "        return\n",
                        "\n",
                        "create Session [Counter] scope-exit:\n",
                        "    done: when false hold [Counter]\n",
                        "\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );
        let err = check_path(&root).expect_err("invalid lifecycle hook should fail");
        assert!(err.contains("must take `edit self`"), "{err}");
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_sources_rejects_owner_activation_with_wrong_context_type() {
        let root = make_temp_package(
            "owner_wrong_context",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    concat!(
                        "obj SessionCtx:\n",
                        "    base: Int\n",
                        "\n",
                        "obj Counter:\n",
                        "    value: Int\n",
                        "    fn init(edit self: Self, read ctx: SessionCtx):\n",
                        "        self.value = ctx.base\n",
                        "\n",
                        "create Session [Counter] scope-exit:\n",
                        "    done: when Counter.value > 10 hold [Counter]\n",
                        "\n",
                        "Session\n",
                        "Counter\n",
                        "fn main() -> Int:\n",
                        "    Session :: 1 :: call\n",
                        "    return 0\n",
                    ),
                ),
                ("src/types.arc", ""),
            ],
        );
        let err = check_path(&root).expect_err("wrong owner activation context should fail");
        assert!(err.contains("expects context"), "{err}");
        assert!(err.contains("Int"), "{err}");
        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_handles_result_variant_constructor_package() {
        let std_dep = repo_root().join("std").to_string_lossy().replace('\\', "/");
        let root = make_temp_package(
            "result_variant_positive",
            "app",
            &[("std", std_dep.as_str())],
            &[
                (
                    "src/shelf.arc",
                    "import std.result\nuse std.result.Result\nfn parse(flag: Bool) -> Result[Int, Str]:\n    if flag:\n        return Result.Ok[Int, Str] :: 1 :: call\n    return Result.Err[Int, Str] :: \"bad\" :: call\nfn main() -> Int:\n    let parsed = parse :: true :: call\n    return match parsed:\n        Result.Ok(value) => value\n        Result.Err(_) => 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("result variant package should resolve");
        assert_eq!(summary.package_count, 2);
        assert!(summary.module_count >= 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_handles_mixed_chain_package() {
        let root = make_temp_package(
            "mixed_chain_positive",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "import stage\nfn main() -> Int:\n    let seed = 1\n    let score = forward :=> stage.seed with (seed) => stage.inc <= stage.dec <= stage.emit\n    return score\n",
                ),
                (
                    "src/stage.arc",
                    "export fn seed(seed: Int) -> Int:\n    return seed\nexport fn inc(value: Int) -> Int:\n    return value + 1\nexport fn dec(value: Int) -> Int:\n    return value - 1\nexport fn emit(value: Int) -> Int:\n    return value\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("mixed chain package should resolve");
        assert_eq!(summary.package_count, 1);
        assert!(summary.module_count >= 3);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_handles_bound_chain_workspace() {
        let root = make_temp_workspace(
            "bound_chain_workspace",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    let seed = 1\n    let score = forward :=> core.seed with (seed) => core.inc <= core.dec <= core.emit\n    return score\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn seed(seed: Int) -> Int:\n    return seed\nexport fn inc(value: Int) -> Int:\n    return value + 1\nexport fn dec(value: Int) -> Int:\n    return value - 1\nexport fn emit(value: Int) -> Int:\n    return value\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("bound chain workspace should resolve");
        assert_eq!(summary.package_count, 2);
        assert!(summary.module_count >= 4);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_filters_only_forewords_for_current_target() {
        let root = make_temp_package(
            "only_filter_app",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "#only[os = \"definitely_not_host\"]\nfn skipped() -> MissingType:\n    return 0\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("#only should exclude non-matching declarations");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn load_workspace_hir_exposes_package_summaries() {
        let root = make_temp_workspace(
            "workspace_hir_summary",
            &["app", "core"],
            &[
                (
                    "app/book.toml",
                    "name = \"app\"\nkind = \"app\"\n\n[deps]\ncore = { path = \"../core\" }\n",
                ),
                (
                    "app/src/shelf.arc",
                    "import core\nfn main() -> Int:\n    return core.id :: 1 :: call\n",
                ),
                ("app/src/types.arc", ""),
                ("core/book.toml", "name = \"core\"\nkind = \"lib\"\n"),
                (
                    "core/src/book.arc",
                    "export fn id(x: Int) -> Int:\n    return x\n",
                ),
                ("core/src/types.arc", ""),
            ],
        );

        let workspace = load_workspace_hir(&root).expect("workspace hir should load");
        assert!(workspace.package("app").is_some());
        assert!(workspace.package("core").is_some());
        assert!(
            workspace
                .package("app")
                .expect("app package should exist")
                .summary
                .dependency_edges
                .iter()
                .any(|edge| edge.target_path == vec!["core".to_string()])
        );

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    fn make_temp_package(
        name: &str,
        kind: &str,
        deps: &[(&str, &str)],
        files: &[(&str, &str)],
    ) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "arcana-frontend-test-{}-{}",
            unique_test_id(),
            name
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("stale temp dir should be removable");
        }

        fs::create_dir_all(root.join("src")).expect("src dir should be creatable");
        let mut manifest = format!("name = \"{name}\"\nkind = \"{kind}\"\n");
        if !deps.is_empty() {
            manifest.push_str("\n[deps]\n");
            for (dep_name, dep_path) in deps {
                manifest.push_str(&format!("{dep_name} = {{ path = \"{dep_path}\" }}\n"));
            }
        }
        fs::write(root.join("book.toml"), manifest).expect("manifest should be writable");

        for (rel_path, contents) in files {
            let path = root.join(rel_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("parent dirs should be creatable");
            }
            fs::write(path, contents).expect("source file should be writable");
        }

        root
    }

    fn make_temp_workspace(name: &str, members: &[&str], files: &[(&str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "arcana-frontend-workspace-test-{}-{}",
            unique_test_id(),
            name
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("stale temp dir should be removable");
        }
        fs::create_dir_all(&root).expect("workspace dir should be creatable");
        let workspace_members = members
            .iter()
            .map(|member| format!("\"{member}\""))
            .collect::<Vec<_>>()
            .join(", ");
        fs::write(
            root.join("book.toml"),
            format!(
                "name = \"workspace\"\nkind = \"app\"\n[workspace]\nmembers = [{workspace_members}]\n"
            ),
        )
        .expect("workspace manifest should be writable");
        for (rel_path, contents) in files {
            let path = root.join(rel_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("parent dirs should be creatable");
            }
            fs::write(path, contents).expect("file should be writable");
        }
        root
    }

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root should resolve")
    }

    fn owned_root() -> PathBuf {
        repo_root().join("grimoires").join("owned")
    }

    fn owned_app_root() -> PathBuf {
        let libs = owned_root().join("libs");
        if libs.is_dir() {
            libs
        } else {
            owned_root().join("app")
        }
    }

    fn unique_test_id() -> u64 {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos() as u64;
        time ^ NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
    }
}
