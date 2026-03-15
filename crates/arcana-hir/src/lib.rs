pub mod freeze;
mod lookup;

pub use lookup::{
    current_workspace_package_for_module, impl_target_is_public_from_package,
    lookup_method_candidates_for_type, lookup_symbol_path, visible_method_package_names_for_module,
    visible_package_root_for_module,
};
pub(crate) use lookup::{
    lookup_symbol_path_in_module_context, visible_symbol_refs_in_module_for_package,
};

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use arcana_syntax::{
    AssignOp as ParsedAssignOp, DirectiveKind as ParsedDirectiveKind, Expr as ParsedExpr,
    OpaqueBoundaryPolicy as ParsedOpaqueBoundaryPolicy,
    OpaqueOwnershipPolicy as ParsedOpaqueOwnershipPolicy,
    OpaqueTypePolicy as ParsedOpaqueTypePolicy, ParamMode as ParsedParamMode, ParsedModule, Span,
    StatementKind as ParsedStatementKind, SymbolKind as ParsedSymbolKind, builtin_type_info,
    parse_module,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HirModule {
    pub symbol_count: usize,
    pub item_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HirDirectiveKind {
    Import,
    Use,
    Reexport,
}

impl HirDirectiveKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Use => "use",
            Self::Reexport => "reexport",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirDirective {
    pub kind: HirDirectiveKind,
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub forewords: Vec<HirForewordApp>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HirSymbolKind {
    Fn,
    System,
    Record,
    Enum,
    OpaqueType,
    Trait,
    Behavior,
    Const,
}

impl HirSymbolKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Fn => "fn",
            Self::System => "system",
            Self::Record => "record",
            Self::Enum => "enum",
            Self::OpaqueType => "opaque_type",
            Self::Trait => "trait",
            Self::Behavior => "behavior",
            Self::Const => "const",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirOpaqueOwnershipPolicy {
    Copy,
    Move,
}

impl HirOpaqueOwnershipPolicy {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Move => "move",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirOpaqueBoundaryPolicy {
    Safe,
    Unsafe,
}

impl HirOpaqueBoundaryPolicy {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "boundary_safe",
            Self::Unsafe => "boundary_unsafe",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HirOpaqueTypePolicy {
    pub ownership: HirOpaqueOwnershipPolicy,
    pub boundary: HirOpaqueBoundaryPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirSymbol {
    pub kind: HirSymbolKind,
    pub name: String,
    pub exported: bool,
    pub is_async: bool,
    pub type_params: Vec<String>,
    pub where_clause: Option<String>,
    pub params: Vec<HirParam>,
    pub return_type: Option<String>,
    pub behavior_attrs: Vec<HirBehaviorAttr>,
    pub opaque_policy: Option<HirOpaqueTypePolicy>,
    pub forewords: Vec<HirForewordApp>,
    pub intrinsic_impl: Option<String>,
    pub body: HirSymbolBody,
    pub statements: Vec<HirStatement>,
    pub rollups: Vec<HirPageRollup>,
    pub surface_text: String,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirParamMode {
    Read,
    Edit,
    Take,
}

impl HirParamMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Edit => "edit",
            Self::Take => "take",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirParam {
    pub mode: Option<HirParamMode>,
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirField {
    pub name: String,
    pub ty: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirBehaviorAttr {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirForewordArg {
    pub name: Option<String>,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirForewordApp {
    pub name: String,
    pub args: Vec<HirForewordArg>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirLangItem {
    pub name: String,
    pub target: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirPageRollupKind {
    Cleanup,
}

impl HirPageRollupKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Cleanup => "cleanup",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirPageRollup {
    pub kind: HirPageRollupKind,
    pub subject: String,
    pub handler_path: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirMatchArm {
    pub patterns: Vec<HirMatchPattern>,
    pub value: HirExpr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirMatchPattern {
    Wildcard,
    Literal {
        text: String,
    },
    Name {
        text: String,
    },
    Variant {
        path: String,
        args: Vec<HirMatchPattern>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirPhraseArg {
    Positional(HirExpr),
    Named { name: String, value: HirExpr },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirChainConnector {
    Forward,
    Reverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirChainIntroducer {
    Forward,
    Reverse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirChainStep {
    pub incoming: Option<HirChainConnector>,
    pub stage: HirExpr,
    pub bind_args: Vec<HirExpr>,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirHeaderAttachment {
    Named {
        name: String,
        value: HirExpr,
        forewords: Vec<HirForewordApp>,
        span: Span,
    },
    Chain {
        expr: HirExpr,
        forewords: Vec<HirForewordApp>,
        span: Span,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirUnaryOp {
    Neg,
    Not,
    BitNot,
    BorrowRead,
    BorrowMut,
    Deref,
    Weave,
    Split,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirBinaryOp {
    Or,
    And,
    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    BitOr,
    BitXor,
    BitAnd,
    Shl,
    Shr,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirExpr {
    Path {
        segments: Vec<String>,
    },
    BoolLiteral {
        value: bool,
    },
    IntLiteral {
        text: String,
    },
    StrLiteral {
        text: String,
    },
    Pair {
        left: Box<HirExpr>,
        right: Box<HirExpr>,
    },
    CollectionLiteral {
        items: Vec<HirExpr>,
    },
    Match {
        subject: Box<HirExpr>,
        arms: Vec<HirMatchArm>,
    },
    Chain {
        style: String,
        introducer: HirChainIntroducer,
        steps: Vec<HirChainStep>,
    },
    MemoryPhrase {
        family: String,
        arena: Box<HirExpr>,
        init_args: Vec<HirPhraseArg>,
        constructor: Box<HirExpr>,
        attached: Vec<HirHeaderAttachment>,
    },
    GenericApply {
        expr: Box<HirExpr>,
        type_args: Vec<String>,
    },
    QualifiedPhrase {
        subject: Box<HirExpr>,
        args: Vec<HirPhraseArg>,
        qualifier: String,
        attached: Vec<HirHeaderAttachment>,
    },
    Await {
        expr: Box<HirExpr>,
    },
    Unary {
        op: HirUnaryOp,
        expr: Box<HirExpr>,
    },
    Binary {
        left: Box<HirExpr>,
        op: HirBinaryOp,
        right: Box<HirExpr>,
    },
    MemberAccess {
        expr: Box<HirExpr>,
        member: String,
    },
    Index {
        expr: Box<HirExpr>,
        index: Box<HirExpr>,
    },
    Slice {
        expr: Box<HirExpr>,
        start: Option<Box<HirExpr>>,
        end: Option<Box<HirExpr>>,
        inclusive_end: bool,
    },
    Range {
        start: Option<Box<HirExpr>>,
        end: Option<Box<HirExpr>>,
        inclusive_end: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirAssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
}

impl HirAssignOp {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::AddAssign => "+=",
            Self::SubAssign => "-=",
            Self::MulAssign => "*=",
            Self::DivAssign => "/=",
            Self::ModAssign => "%=",
            Self::BitAndAssign => "&=",
            Self::BitOrAssign => "|=",
            Self::BitXorAssign => "^=",
            Self::ShlAssign => "<<=",
            Self::ShrAssign => "shr=",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirAssignTarget {
    Name {
        text: String,
    },
    MemberAccess {
        target: Box<HirAssignTarget>,
        member: String,
    },
    Index {
        target: Box<HirAssignTarget>,
        index: HirExpr,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirStatementKind {
    Let {
        mutable: bool,
        name: String,
        value: HirExpr,
    },
    Return {
        value: Option<HirExpr>,
    },
    If {
        condition: HirExpr,
        then_branch: Vec<HirStatement>,
        else_branch: Option<Vec<HirStatement>>,
    },
    While {
        condition: HirExpr,
        body: Vec<HirStatement>,
    },
    For {
        binding: String,
        iterable: HirExpr,
        body: Vec<HirStatement>,
    },
    Defer {
        expr: HirExpr,
    },
    Break,
    Continue,
    Assign {
        target: HirAssignTarget,
        op: HirAssignOp,
        value: HirExpr,
    },
    Expr {
        expr: HirExpr,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirStatement {
    pub kind: HirStatementKind,
    pub forewords: Vec<HirForewordApp>,
    pub rollups: Vec<HirPageRollup>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirEnumVariant {
    pub name: String,
    pub payload: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirTraitAssocType {
    pub name: String,
    pub default_ty: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirSymbolBody {
    None,
    Record {
        fields: Vec<HirField>,
    },
    Enum {
        variants: Vec<HirEnumVariant>,
    },
    Trait {
        assoc_types: Vec<HirTraitAssocType>,
        methods: Vec<HirSymbol>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirImplDecl {
    pub type_params: Vec<String>,
    pub trait_path: Option<String>,
    pub target_type: String,
    pub assoc_types: Vec<HirImplAssocTypeBinding>,
    pub methods: Vec<HirSymbol>,
    pub body_entries: Vec<String>,
    pub surface_text: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirImplAssocTypeBinding {
    pub name: String,
    pub value_ty: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirModuleSummary {
    pub module_id: String,
    pub line_count: usize,
    pub non_empty_line_count: usize,
    pub directives: Vec<HirDirective>,
    pub lang_items: Vec<HirLangItem>,
    pub symbols: Vec<HirSymbol>,
    pub impls: Vec<HirImplDecl>,
}

impl HirModuleSummary {
    pub fn has_symbol(&self, name: &str) -> bool {
        self.symbols.iter().any(|symbol| symbol.name == name)
    }

    pub fn symbol_count(&self, name: &str) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.name == name)
            .count()
    }

    pub fn summary_surface_rows(&self) -> Vec<String> {
        self.summary_api_fingerprint_rows()
    }

    pub fn summary_api_fingerprint_rows(&self) -> Vec<String> {
        let mut rows = self
            .directives
            .iter()
            .filter(|directive| directive.kind == HirDirectiveKind::Reexport)
            .map(|directive| format!("reexport:{}", directive.path.join(".")))
            .collect::<Vec<_>>();
        rows.extend(
            self.symbols
                .iter()
                .filter(|symbol| symbol.exported)
                .map(|symbol| {
                    format!(
                        "export:{}:{}",
                        symbol.kind.as_str(),
                        encode_surface_text(&symbol.api_signature_text())
                    )
                }),
        );
        rows.sort();
        rows
    }

    pub fn hir_fingerprint_rows(&self) -> Vec<String> {
        let mut rows = Vec::new();
        rows.extend(self.directives.iter().map(render_directive_fingerprint));
        rows.extend(self.lang_items.iter().map(render_lang_item_fingerprint));
        rows.extend(self.symbols.iter().map(render_symbol_fingerprint));
        rows.extend(self.impls.iter().map(render_impl_fingerprint));
        rows
    }
}

impl HirSymbol {
    fn api_signature_text(&self) -> String {
        match self.kind {
            HirSymbolKind::Fn | HirSymbolKind::System => render_function_signature(self),
            HirSymbolKind::Record => render_record_signature(self),
            HirSymbolKind::Enum => render_enum_signature(self),
            HirSymbolKind::OpaqueType => render_opaque_signature(self),
            HirSymbolKind::Trait => render_trait_signature(self),
            HirSymbolKind::Behavior => render_behavior_signature(self),
            _ => self.surface_text.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirModuleDependency {
    pub source_module_id: String,
    pub kind: HirDirectiveKind,
    pub target_path: Vec<String>,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirPackageSummary {
    pub package_name: String,
    pub modules: Vec<HirModuleSummary>,
    pub dependency_edges: Vec<HirModuleDependency>,
}

impl HirPackageSummary {
    pub fn module(&self, module_id: &str) -> Option<&HirModuleSummary> {
        self.modules
            .iter()
            .find(|module| module.module_id == module_id)
    }

    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    pub fn summary_surface_rows(&self) -> Vec<String> {
        self.summary_api_fingerprint_rows()
    }

    pub fn summary_api_fingerprint_rows(&self) -> Vec<String> {
        let mut rows = Vec::new();
        for module in &self.modules {
            for row in module.summary_api_fingerprint_rows() {
                rows.push(format!("module={}:{}", module.module_id, row));
            }
        }
        rows.sort();
        rows
    }

    pub fn hir_fingerprint_rows(&self) -> Vec<String> {
        let mut rows = vec![format!(
            "package_name={}",
            quote_fingerprint_text(&self.package_name)
        )];
        rows.extend(
            self.dependency_edges
                .iter()
                .map(render_dependency_edge_fingerprint),
        );
        for module in &self.modules {
            for row in module.hir_fingerprint_rows() {
                rows.push(format!("module={}:{}", module.module_id, row));
            }
        }
        rows
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirPackageLayout {
    pub module_paths: BTreeMap<String, PathBuf>,
    pub relative_modules: BTreeMap<String, String>,
    pub absolute_modules: BTreeMap<String, usize>,
}

impl HirPackageLayout {
    pub fn module<'a>(
        &self,
        summary: &'a HirPackageSummary,
        module_id: &str,
    ) -> Option<&'a HirModuleSummary> {
        self.absolute_modules
            .get(module_id)
            .and_then(|index| summary.modules.get(*index))
    }

    pub fn module_path(&self, module_id: &str) -> Option<&PathBuf> {
        self.module_paths.get(module_id)
    }

    pub fn resolve_relative_module<'a>(
        &self,
        summary: &'a HirPackageSummary,
        path: &[String],
    ) -> Option<&'a HirModuleSummary> {
        let key = path.join(".");
        self.relative_modules
            .get(&key)
            .and_then(|module_id| self.module(summary, module_id))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirWorkspacePackage {
    pub root_dir: PathBuf,
    pub direct_deps: BTreeSet<String>,
    pub direct_dep_packages: BTreeMap<String, String>,
    pub summary: HirPackageSummary,
    pub layout: HirPackageLayout,
}

impl HirWorkspacePackage {
    pub fn module(&self, module_id: &str) -> Option<&HirModuleSummary> {
        self.layout.module(&self.summary, module_id)
    }

    pub fn module_path(&self, module_id: &str) -> Option<&PathBuf> {
        self.layout.module_path(module_id)
    }

    pub fn resolve_relative_module(&self, path: &[String]) -> Option<&HirModuleSummary> {
        self.layout.resolve_relative_module(&self.summary, path)
    }

    pub fn dependency_package_name(&self, visible_name: &str) -> Option<&str> {
        self.direct_dep_packages
            .get(visible_name)
            .map(String::as_str)
    }

    pub fn dependency_module_id(&self, path: &[String]) -> Option<String> {
        let (visible_name, suffix) = path.split_first()?;
        let dependency_name = self.dependency_package_name(visible_name)?;
        let mut segments = Vec::with_capacity(path.len());
        segments.push(dependency_name.to_string());
        segments.extend(suffix.iter().cloned());
        Some(segments.join("."))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HirWorkspaceSummary {
    pub packages: BTreeMap<String, HirWorkspacePackage>,
}

impl HirWorkspaceSummary {
    pub fn package(&self, name: &str) -> Option<&HirWorkspacePackage> {
        self.packages.get(name)
    }

    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    pub fn module_count(&self) -> usize {
        self.packages
            .values()
            .map(|package| package.summary.modules.len())
            .sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirResolvedTarget {
    Module {
        package_name: String,
        module_id: String,
    },
    Symbol {
        package_name: String,
        module_id: String,
        symbol_name: String,
    },
}

impl HirResolvedTarget {
    pub fn binding_name(&self) -> String {
        match self {
            Self::Module { module_id, .. } => module_id
                .rsplit('.')
                .next()
                .unwrap_or(module_id)
                .to_string(),
            Self::Symbol { symbol_name, .. } => symbol_name.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirBindingOrigin {
    LocalSymbol,
    Directive(HirDirectiveKind),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirResolvedBinding {
    pub local_name: String,
    pub origin: HirBindingOrigin,
    pub target: HirResolvedTarget,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirResolvedDirective {
    pub source_module_id: String,
    pub local_name: String,
    pub kind: HirDirectiveKind,
    pub target: HirResolvedTarget,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirResolvedModule {
    pub module_id: String,
    pub bindings: BTreeMap<String, HirResolvedBinding>,
    pub directives: Vec<HirResolvedDirective>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirResolvedPackage {
    pub package_name: String,
    pub modules: BTreeMap<String, HirResolvedModule>,
}

impl HirResolvedPackage {
    pub fn module(&self, module_id: &str) -> Option<&HirResolvedModule> {
        self.modules.get(module_id)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HirResolvedWorkspace {
    pub packages: BTreeMap<String, HirResolvedPackage>,
}

impl HirResolvedWorkspace {
    pub fn package(&self, package_name: &str) -> Option<&HirResolvedPackage> {
        self.packages.get(package_name)
    }
}

#[derive(Clone, Debug)]
pub struct HirMethodCandidate<'a> {
    pub module_id: &'a str,
    pub symbol: &'a HirSymbol,
    pub declared_receiver_type: &'a str,
    pub routine_key: String,
}

pub fn routine_key_for_symbol(module_id: &str, symbol_index: usize) -> String {
    format!("{module_id}#sym-{symbol_index}")
}

pub fn routine_key_for_impl_method(
    module_id: &str,
    impl_index: usize,
    method_index: usize,
) -> String {
    format!("{module_id}#impl-{impl_index}-method-{method_index}")
}

fn canonicalize_method_lookup_base(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    text: &str,
) -> String {
    split_simple_path(text)
        .and_then(|path| lookup_symbol_path(workspace, resolved_module, &path))
        .map(|symbol_ref| format!("{}.{}", symbol_ref.module_id, symbol_ref.symbol.name))
        .or_else(|| canonical_ambient_type_root(text).map(str::to_string))
        .unwrap_or_else(|| text.trim().to_string())
}

fn canonical_ambient_type_root(text: &str) -> Option<&'static str> {
    match text.trim() {
        "List" => Some("std.collections.list.List"),
        "Array" => Some("std.collections.array.Array"),
        "Map" => Some("std.collections.map.Map"),
        "Set" => Some("std.collections.set.Set"),
        "Option" => Some("std.option.Option"),
        "Result" => Some("std.result.Result"),
        "Arena" => Some("std.memory.Arena"),
        "ArenaId" => Some("std.memory.ArenaId"),
        "FrameArena" => Some("std.memory.FrameArena"),
        "FrameId" => Some("std.memory.FrameId"),
        "PoolArena" => Some("std.memory.PoolArena"),
        "PoolId" => Some("std.memory.PoolId"),
        "Task" => Some("std.concurrent.Task"),
        "Thread" => Some("std.concurrent.Thread"),
        "Channel" => Some("std.concurrent.Channel"),
        "Mutex" => Some("std.concurrent.Mutex"),
        "AtomicInt" => Some("std.concurrent.AtomicInt"),
        "AtomicBool" => Some("std.concurrent.AtomicBool"),
        _ => None,
    }
}

fn canonicalize_method_lookup_base_in_module(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    text: &str,
) -> String {
    split_simple_path(text)
        .and_then(|path| lookup_symbol_path_in_module_context(workspace, package, module, &path))
        .map(|symbol_ref| format!("{}.{}", symbol_ref.module_id, symbol_ref.symbol.name))
        .or_else(|| canonical_ambient_type_root(text).map(str::to_string))
        .unwrap_or_else(|| text.trim().to_string())
}

fn canonicalize_method_lookup_type_text_in_module(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    text: &str,
) -> String {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("&mut") {
        return format!(
            "&mut {}",
            canonicalize_method_lookup_type_text_in_module(workspace, package, module, rest)
        );
    }
    if let Some(rest) = trimmed.strip_prefix('&') {
        return format!(
            "& {}",
            canonicalize_method_lookup_type_text_in_module(workspace, package, module, rest)
        );
    }
    if let Some((base, args)) = parse_surface_type_application(trimmed) {
        let base = canonicalize_method_lookup_base_in_module(workspace, package, module, &base);
        if args.is_empty() {
            return base;
        }
        let args = args
            .into_iter()
            .map(|arg| {
                canonicalize_method_lookup_type_text_in_module(workspace, package, module, &arg)
            })
            .collect::<Vec<_>>()
            .join(", ");
        return format!("{base}[{args}]");
    }
    canonicalize_method_lookup_base_in_module(workspace, package, module, trimmed)
}

fn canonicalize_method_lookup_type_text(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    text: &str,
) -> String {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("&mut") {
        return format!(
            "&mut {}",
            canonicalize_method_lookup_type_text(workspace, resolved_module, rest)
        );
    }
    if let Some(rest) = trimmed.strip_prefix('&') {
        return format!(
            "& {}",
            canonicalize_method_lookup_type_text(workspace, resolved_module, rest)
        );
    }
    if let Some((base, args)) = parse_surface_type_application(trimmed) {
        let base = canonicalize_method_lookup_base(workspace, resolved_module, &base);
        if args.is_empty() {
            return base;
        }
        let args = args
            .into_iter()
            .map(|arg| canonicalize_method_lookup_type_text(workspace, resolved_module, &arg))
            .collect::<Vec<_>>()
            .join(", ");
        return format!("{base}[{args}]");
    }
    canonicalize_method_lookup_base(workspace, resolved_module, trimmed)
}

fn is_simple_type_placeholder(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        && builtin_type_info(text).is_none()
}

fn method_target_type_matches(
    declared: &str,
    actual: &str,
    substitutions: &mut BTreeMap<String, String>,
) -> bool {
    let declared = strip_reference_prefix(declared);
    let actual = strip_reference_prefix(actual);
    if declared == actual {
        return true;
    }
    if is_simple_type_placeholder(declared) {
        if let Some(existing) = substitutions.get(declared) {
            return existing == actual;
        }
        substitutions.insert(declared.to_string(), actual.to_string());
        return true;
    }
    match (
        parse_surface_type_application(declared),
        parse_surface_type_application(actual),
    ) {
        (Some((decl_base, decl_args)), Some((actual_base, actual_args))) => {
            if decl_base != actual_base || decl_args.len() != actual_args.len() {
                return false;
            }
            decl_args
                .iter()
                .zip(actual_args.iter())
                .all(|(decl_arg, actual_arg)| {
                    method_target_type_matches(decl_arg, actual_arg, substitutions)
                })
        }
        _ => declared == actual,
    }
}

pub trait HirLocalTypeLookup {
    fn contains_local(&self, name: &str) -> bool;
    fn type_text_of(&self, name: &str) -> Option<&str>;
}

#[derive(Clone, Copy, Debug)]
pub struct HirResolvedSymbolRef<'a> {
    pub package_name: &'a str,
    pub module_id: &'a str,
    pub symbol_index: usize,
    pub symbol: &'a HirSymbol,
}

fn hir_is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn hir_is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn split_simple_path(text: &str) -> Option<Vec<String>> {
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
        if !hir_is_ident_start(first) || !chars.all(hir_is_ident_continue) {
            return None;
        }
        segments.push(segment.to_string());
    }

    (!segments.is_empty()).then_some(segments)
}

fn split_top_level_surface_items(text: &str, separator: char) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut square_depth = 0usize;
    let mut paren_depth = 0usize;
    for ch in text.chars() {
        match ch {
            '[' => {
                square_depth += 1;
                current.push(ch);
            }
            ']' => {
                square_depth = square_depth.saturating_sub(1);
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            _ if ch == separator && square_depth == 0 && paren_depth == 0 => {
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

fn strip_reference_prefix(text: &str) -> &str {
    let trimmed = text.trim_start();
    if let Some(rest) = trimmed.strip_prefix("&mut") {
        return rest.trim_start();
    }
    if let Some(rest) = trimmed.strip_prefix('&') {
        return rest.trim_start();
    }
    trimmed
}

fn parse_surface_type_application(text: &str) -> Option<(String, Vec<String>)> {
    let trimmed = text.trim();
    if let Some(path) = split_simple_path(trimmed) {
        return Some((path.join("."), Vec::new()));
    }
    let mut depth = 0usize;
    let mut open = None;
    for (index, ch) in trimmed.char_indices() {
        match ch {
            '[' if depth == 0 => {
                open = Some(index);
                break;
            }
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let open = open?;
    if !trimmed.ends_with(']') || open == 0 {
        return None;
    }
    let base = trimmed[..open].trim();
    let path = split_simple_path(base)?;
    let args = split_top_level_surface_items(&trimmed[open + 1..trimmed.len() - 1], ',');
    Some((path.join("."), args))
}

fn substitute_type_params(text: &str, substitutions: &BTreeMap<String, String>) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let mut out = String::new();
    let mut index = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        if ch == '\'' {
            out.push(ch);
            index += 1;
            while index < chars.len() {
                let current = chars[index];
                out.push(current);
                index += 1;
                if !(current == '_' || current.is_ascii_alphanumeric()) {
                    break;
                }
            }
            continue;
        }
        if ch == '_' || ch.is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index] == '_' || chars[index].is_ascii_alphanumeric())
            {
                index += 1;
            }
            let ident = chars[start..index].iter().collect::<String>();
            if let Some(replacement) = substitutions.get(&ident) {
                out.push_str(replacement);
            } else {
                out.push_str(&ident);
            }
            continue;
        }
        out.push(ch);
        index += 1;
    }
    out
}

fn build_receiver_type_substitutions(
    actual_type: &str,
    declared_type: &str,
) -> BTreeMap<String, String> {
    let mut substitutions = BTreeMap::new();
    let Some((_, actual_args)) =
        parse_surface_type_application(strip_reference_prefix(actual_type))
    else {
        return substitutions;
    };
    let Some((_, declared_args)) =
        parse_surface_type_application(strip_reference_prefix(declared_type))
    else {
        return substitutions;
    };
    for (formal, actual) in declared_args.into_iter().zip(actual_args.into_iter()) {
        substitutions.insert(formal, actual);
    }
    substitutions
}

fn substitute_surface_type_params(text: &str, substitutions: &BTreeMap<String, String>) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let mut out = String::new();
    let mut index = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        if ch == '\'' {
            out.push(ch);
            index += 1;
            while index < chars.len() {
                let current = chars[index];
                out.push(current);
                index += 1;
                if !(current == '_' || current.is_ascii_alphanumeric()) {
                    break;
                }
            }
            continue;
        }
        if ch == '_' || ch.is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index] == '_' || chars[index].is_ascii_alphanumeric())
            {
                index += 1;
            }
            let ident = chars[start..index].iter().collect::<String>();
            if let Some(replacement) = substitutions.get(&ident) {
                out.push_str(replacement);
            } else {
                out.push_str(&ident);
            }
            continue;
        }
        out.push(ch);
        index += 1;
    }
    out
}

fn flatten_member_expr_path(expr: &HirExpr) -> Option<Vec<String>> {
    match expr {
        HirExpr::Path { segments } => Some(segments.clone()),
        HirExpr::MemberAccess { expr, member } => {
            let mut path = flatten_member_expr_path(expr)?;
            path.push(member.clone());
            Some(path)
        }
        _ => None,
    }
}

fn flatten_callable_expr_path(expr: &HirExpr) -> Option<Vec<String>> {
    match expr {
        HirExpr::GenericApply { expr, .. } => flatten_callable_expr_path(expr),
        _ => flatten_member_expr_path(expr),
    }
}

fn extract_expr_generic_type_args(expr: &HirExpr) -> Vec<String> {
    match expr {
        HirExpr::GenericApply { expr, type_args } => {
            let mut inherited = extract_expr_generic_type_args(expr);
            inherited.extend(type_args.iter().cloned());
            inherited
        }
        _ => Vec::new(),
    }
}

fn symbol_return_type_text(
    workspace: &HirWorkspaceSummary,
    symbol_ref: HirResolvedSymbolRef<'_>,
) -> Option<String> {
    if let Some(return_type) = &symbol_ref.symbol.return_type {
        let package = workspace.package(symbol_ref.package_name)?;
        let module = package.module(symbol_ref.module_id)?;
        return Some(canonicalize_method_lookup_type_text_in_module(
            workspace,
            package,
            module,
            return_type,
        ));
    }
    None.or_else(|| {
        matches!(
            symbol_ref.symbol.kind,
            HirSymbolKind::Record | HirSymbolKind::Enum | HirSymbolKind::OpaqueType
        )
        .then(|| format!("{}.{}", symbol_ref.module_id, symbol_ref.symbol.name))
    })
}

fn symbol_call_return_type_text(
    workspace: &HirWorkspaceSummary,
    symbol_ref: HirResolvedSymbolRef<'_>,
    generic_args: &[String],
) -> Option<String> {
    if matches!(
        symbol_ref.symbol.kind,
        HirSymbolKind::Record | HirSymbolKind::Enum | HirSymbolKind::OpaqueType
    ) {
        let base = format!("{}.{}", symbol_ref.module_id, symbol_ref.symbol.name);
        if generic_args.is_empty() {
            return Some(base);
        }
        return Some(format!("{base}[{}]", generic_args.join(", ")));
    }
    if let Some(return_type) = symbol_return_type_text(workspace, symbol_ref) {
        if generic_args.is_empty() || symbol_ref.symbol.type_params.is_empty() {
            return Some(return_type);
        }
        let substitutions = symbol_ref
            .symbol
            .type_params
            .iter()
            .cloned()
            .zip(generic_args.iter().cloned())
            .collect::<BTreeMap<_, _>>();
        return Some(substitute_surface_type_params(&return_type, &substitutions));
    }
    None
}

fn infer_call_target_return_type<L: HirLocalTypeLookup>(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    locals: &L,
    subject: &HirExpr,
) -> Option<String> {
    let path = flatten_callable_expr_path(subject)?;
    let generic_args = extract_expr_generic_type_args(subject)
        .into_iter()
        .map(|arg| canonicalize_method_lookup_type_text(workspace, resolved_module, &arg))
        .collect::<Vec<_>>();
    if let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, &path) {
        return symbol_call_return_type_text(workspace, symbol_ref, &generic_args);
    }
    if path.len() >= 2 {
        let enum_path = path[..path.len() - 1].to_vec();
        if let Some(enum_ref) = lookup_symbol_path(workspace, resolved_module, &enum_path) {
            if matches!(enum_ref.symbol.kind, HirSymbolKind::Enum) {
                return symbol_call_return_type_text(workspace, enum_ref, &generic_args);
            }
        }
    }
    let _ = locals;
    None
}

fn infer_member_access_type<L: HirLocalTypeLookup>(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    locals: &L,
    expr: &HirExpr,
    member: &str,
) -> Option<String> {
    let base_ty = infer_receiver_expr_type_text(workspace, resolved_module, locals, expr)?;
    let (base, _) = parse_surface_type_application(strip_reference_prefix(&base_ty))?;
    let path = split_simple_path(&base)?;
    let symbol_ref = lookup_symbol_path(workspace, resolved_module, &path)?;
    match &symbol_ref.symbol.body {
        HirSymbolBody::Record { fields } => fields
            .iter()
            .find(|field| field.name == member)
            .map(|field| field.ty.clone()),
        _ => None,
    }
}

fn infer_index_type_text<L: HirLocalTypeLookup>(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    locals: &L,
    expr: &HirExpr,
) -> Option<String> {
    let base_ty = infer_receiver_expr_type_text(workspace, resolved_module, locals, expr)?;
    let (base, args) = parse_surface_type_application(strip_reference_prefix(&base_ty))?;
    match base.as_str() {
        "List" | "Array" => args.first().cloned(),
        "Map" => args.get(1).cloned(),
        _ => None,
    }
}

fn infer_slice_type_text<L: HirLocalTypeLookup>(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    locals: &L,
    expr: &HirExpr,
) -> Option<String> {
    let base_ty = infer_receiver_expr_type_text(workspace, resolved_module, locals, expr)?;
    let (base, args) = parse_surface_type_application(strip_reference_prefix(&base_ty))?;
    match base.as_str() {
        "List" => Some(format!(
            "List[{}]",
            args.first().cloned().unwrap_or_else(|| "_".to_string())
        )),
        "Array" => Some(format!(
            "Array[{}]",
            args.first().cloned().unwrap_or_else(|| "_".to_string())
        )),
        _ => Some(base_ty),
    }
}

pub fn infer_receiver_expr_type_text<L: HirLocalTypeLookup>(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    locals: &L,
    expr: &HirExpr,
) -> Option<String> {
    match expr {
        HirExpr::BoolLiteral { .. } => Some("Bool".to_string()),
        HirExpr::IntLiteral { .. } => Some("Int".to_string()),
        HirExpr::StrLiteral { .. } => Some("Str".to_string()),
        HirExpr::CollectionLiteral { .. } => Some("List[_]".to_string()),
        HirExpr::Range { .. } => Some("RangeInt".to_string()),
        HirExpr::Path { segments }
            if segments.len() == 1 && locals.contains_local(&segments[0]) =>
        {
            locals.type_text_of(&segments[0]).map(ToOwned::to_owned)
        }
        HirExpr::Path { segments } => {
            let symbol_ref = lookup_symbol_path(workspace, resolved_module, segments)?;
            symbol_return_type_text(workspace, symbol_ref)
        }
        HirExpr::Unary { op, expr }
            if matches!(op, HirUnaryOp::BorrowRead | HirUnaryOp::BorrowMut) =>
        {
            infer_receiver_expr_type_text(workspace, resolved_module, locals, expr)
                .map(|text| format!("& {text}"))
        }
        HirExpr::Unary {
            op: HirUnaryOp::Weave,
            expr,
        } => infer_receiver_expr_type_text(workspace, resolved_module, locals, expr)
            .map(|text| format!("std.concurrent.Task[{text}]")),
        HirExpr::Unary {
            op: HirUnaryOp::Split,
            expr,
        } => infer_receiver_expr_type_text(workspace, resolved_module, locals, expr)
            .map(|text| format!("std.concurrent.Thread[{text}]")),
        HirExpr::Unary {
            op: HirUnaryOp::Deref,
            expr,
        } => infer_receiver_expr_type_text(workspace, resolved_module, locals, expr).map(|text| {
            let stripped = strip_reference_prefix(&text);
            if stripped == text.trim() {
                text
            } else {
                stripped.to_string()
            }
        }),
        HirExpr::GenericApply { expr, .. } => {
            infer_receiver_expr_type_text(workspace, resolved_module, locals, expr)
        }
        HirExpr::QualifiedPhrase {
            subject, qualifier, ..
        } if qualifier == "call" => {
            infer_call_target_return_type(workspace, resolved_module, locals, subject)
        }
        HirExpr::QualifiedPhrase { qualifier, .. } if qualifier.contains('.') => {
            let path = split_simple_path(qualifier)?;
            lookup_symbol_path(workspace, resolved_module, &path)
                .and_then(|symbol_ref| symbol_return_type_text(workspace, symbol_ref))
        }
        HirExpr::QualifiedPhrase {
            subject, qualifier, ..
        } if split_simple_path(qualifier).is_some() => {
            let subject_ty =
                infer_receiver_expr_type_text(workspace, resolved_module, locals, subject)?;
            let candidates = lookup_method_candidates_for_type(
                workspace,
                resolved_module,
                &subject_ty,
                qualifier,
            );
            match candidates.as_slice() {
                [candidate] => candidate.symbol.return_type.as_ref().map(|text| {
                    let substitutions = build_receiver_type_substitutions(
                        &subject_ty,
                        candidate.declared_receiver_type,
                    );
                    substitute_type_params(text, &substitutions)
                }),
                _ => None,
            }
        }
        HirExpr::MemberAccess { expr, member } => {
            infer_member_access_type(workspace, resolved_module, locals, expr, member)
        }
        HirExpr::Index { expr, .. } => {
            infer_index_type_text(workspace, resolved_module, locals, expr)
        }
        HirExpr::Slice { expr, .. } => {
            infer_slice_type_text(workspace, resolved_module, locals, expr)
        }
        HirExpr::Match { arms, .. } => {
            let inferred = arms
                .iter()
                .filter_map(|arm| {
                    infer_receiver_expr_type_text(workspace, resolved_module, locals, &arm.value)
                })
                .collect::<Vec<_>>();
            let first = inferred.first()?.clone();
            inferred
                .iter()
                .all(|candidate| candidate == &first)
                .then_some(first)
        }
        HirExpr::Await { expr } => {
            let awaited = infer_receiver_expr_type_text(workspace, resolved_module, locals, expr)?;
            let (base, args) = parse_surface_type_application(strip_reference_prefix(&awaited))?;
            match base.as_str() {
                "std.concurrent.Task" | "std.concurrent.Thread" | "Task" | "Thread" => {
                    args.first().cloned()
                }
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn match_name_resolves_to_zero_payload_variant<L: HirLocalTypeLookup>(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    locals: &L,
    subject: &HirExpr,
    name: &str,
) -> bool {
    if name.contains('.') {
        return false;
    }
    let Some(subject_type) =
        infer_receiver_expr_type_text(workspace, resolved_module, locals, subject)
    else {
        return false;
    };
    let stripped = strip_reference_prefix(&subject_type);
    let base = parse_surface_type_application(stripped)
        .map(|(base, _)| base)
        .unwrap_or_else(|| stripped.trim().to_string());
    let Some(path) = split_simple_path(&base) else {
        return false;
    };
    let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, &path) else {
        return false;
    };
    let HirSymbolBody::Enum { variants } = &symbol_ref.symbol.body else {
        return false;
    };
    variants
        .iter()
        .any(|variant| variant.name == name && variant.payload.is_none())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirResolutionError {
    pub package_name: String,
    pub source_module_id: String,
    pub span: Span,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceModulePath {
    pub relative_segments: Vec<String>,
    pub module_id: String,
}

pub fn lower_module_text(
    module_id: impl Into<String>,
    source: &str,
) -> Result<HirModuleSummary, String> {
    let parsed = parse_module(source)?;
    Ok(lower_parsed_module(module_id, &parsed))
}

pub fn lower_parsed_module(
    module_id: impl Into<String>,
    parsed: &ParsedModule,
) -> HirModuleSummary {
    HirModuleSummary {
        module_id: module_id.into(),
        line_count: parsed.line_count,
        non_empty_line_count: parsed.non_empty_line_count,
        directives: parsed
            .directives
            .iter()
            .map(|directive| HirDirective {
                kind: lower_directive_kind(&directive.kind),
                path: directive.path.clone(),
                alias: directive.alias.clone(),
                forewords: lower_forewords(&directive.forewords),
                span: directive.span,
            })
            .collect(),
        lang_items: parsed
            .lang_items
            .iter()
            .map(|item| HirLangItem {
                name: item.name.clone(),
                target: item.target.clone(),
                span: item.span,
            })
            .collect(),
        symbols: parsed
            .symbols
            .iter()
            .map(|symbol| HirSymbol {
                kind: lower_symbol_kind(&symbol.kind),
                name: symbol.name.clone(),
                exported: symbol.exported,
                is_async: symbol.is_async,
                type_params: symbol.type_params.clone(),
                where_clause: symbol.where_clause.clone(),
                params: symbol
                    .params
                    .iter()
                    .map(|param| HirParam {
                        mode: param.mode.as_ref().map(lower_param_mode),
                        name: param.name.clone(),
                        ty: param.ty.clone(),
                    })
                    .collect(),
                return_type: symbol.return_type.clone(),
                behavior_attrs: symbol
                    .behavior_attrs
                    .iter()
                    .map(|attr| HirBehaviorAttr {
                        name: attr.name.clone(),
                        value: attr.value.clone(),
                    })
                    .collect(),
                opaque_policy: symbol.opaque_policy.as_ref().map(lower_opaque_policy),
                forewords: lower_forewords(&symbol.forewords),
                intrinsic_impl: symbol.intrinsic_impl.clone(),
                body: lower_symbol_body(&symbol.body),
                statements: lower_statements(&symbol.statements),
                rollups: lower_rollups(&symbol.rollups),
                surface_text: symbol.surface_text.clone(),
                span: symbol.span,
            })
            .collect(),
        impls: parsed
            .impls
            .iter()
            .map(|impl_decl| HirImplDecl {
                type_params: impl_decl.type_params.clone(),
                trait_path: impl_decl.trait_path.clone(),
                target_type: impl_decl.target_type.clone(),
                assoc_types: impl_decl
                    .assoc_types
                    .iter()
                    .map(|assoc_type| HirImplAssocTypeBinding {
                        name: assoc_type.name.clone(),
                        value_ty: assoc_type.value_ty.clone(),
                        span: assoc_type.span,
                    })
                    .collect(),
                methods: impl_decl
                    .methods
                    .iter()
                    .map(lower_trait_or_impl_method)
                    .collect(),
                body_entries: impl_decl.body_entries.clone(),
                surface_text: impl_decl.surface_text.clone(),
                span: impl_decl.span,
            })
            .collect(),
    }
}

fn encode_surface_text(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\n', "\\n")
}

fn quote_fingerprint_text(text: &str) -> String {
    format!("{text:?}")
}

fn render_function_signature(symbol: &HirSymbol) -> String {
    let mut rendered = String::new();
    if symbol.is_async {
        rendered.push_str("async ");
    }
    rendered.push_str("fn ");
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
            .map(render_param)
            .collect::<Vec<_>>()
            .join(", "),
    );
    rendered.push(')');
    if let Some(return_type) = &symbol.return_type {
        rendered.push_str(" -> ");
        rendered.push_str(return_type);
    }
    rendered.push(':');
    rendered
}

fn render_param(param: &HirParam) -> String {
    match param.mode {
        Some(mode) => format!("{} {}: {}", mode.as_str(), param.name, param.ty),
        None => format!("{}: {}", param.name, param.ty),
    }
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

fn render_directive_fingerprint(directive: &HirDirective) -> String {
    format!(
        "directive(kind={}|path=[{}]|alias={}|forewords=[{}])",
        directive.kind.as_str(),
        directive
            .path
            .iter()
            .map(|segment| quote_fingerprint_text(segment))
            .collect::<Vec<_>>()
            .join(","),
        directive
            .alias
            .as_ref()
            .map(|alias| quote_fingerprint_text(alias))
            .unwrap_or_else(|| "none".to_string()),
        directive
            .forewords
            .iter()
            .map(render_foreword_fingerprint)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_lang_item_fingerprint(lang_item: &HirLangItem) -> String {
    format!(
        "lang(name={}|target=[{}])",
        quote_fingerprint_text(&lang_item.name),
        lang_item
            .target
            .iter()
            .map(|segment| quote_fingerprint_text(segment))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_symbol_fingerprint(symbol: &HirSymbol) -> String {
    format!(
        concat!(
            "symbol(",
            "kind={}|name={}|exported={}|async={}|signature={}|type_params=[{}]|",
            "where_clause={}|behavior_attrs=[{}]|forewords=[{}]|intrinsic={}|body={}|",
            "statements=[{}]|rollups=[{}])"
        ),
        symbol.kind.as_str(),
        quote_fingerprint_text(&symbol.name),
        symbol.exported,
        symbol.is_async,
        quote_fingerprint_text(&symbol.api_signature_text()),
        symbol
            .type_params
            .iter()
            .map(|param| quote_fingerprint_text(param))
            .collect::<Vec<_>>()
            .join(","),
        symbol
            .where_clause
            .as_ref()
            .map(|clause| quote_fingerprint_text(clause))
            .unwrap_or_else(|| "none".to_string()),
        symbol
            .behavior_attrs
            .iter()
            .map(render_behavior_attr_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        symbol
            .forewords
            .iter()
            .map(render_foreword_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        symbol
            .intrinsic_impl
            .as_ref()
            .map(|intrinsic| quote_fingerprint_text(intrinsic))
            .unwrap_or_else(|| "none".to_string()),
        render_symbol_body_fingerprint(&symbol.body),
        symbol
            .statements
            .iter()
            .map(render_statement_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        symbol
            .rollups
            .iter()
            .map(render_rollup_fingerprint)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_behavior_attr_fingerprint(attr: &HirBehaviorAttr) -> String {
    format!(
        "attr(name={}|value={})",
        quote_fingerprint_text(&attr.name),
        quote_fingerprint_text(&attr.value)
    )
}

fn render_foreword_fingerprint(foreword: &HirForewordApp) -> String {
    format!(
        "foreword(name={}|args=[{}])",
        quote_fingerprint_text(&foreword.name),
        foreword
            .args
            .iter()
            .map(render_foreword_arg_fingerprint)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_foreword_arg_fingerprint(arg: &HirForewordArg) -> String {
    format!(
        "arg(name={}|value={})",
        arg.name
            .as_ref()
            .map(|name| quote_fingerprint_text(name))
            .unwrap_or_else(|| "none".to_string()),
        quote_fingerprint_text(&arg.value)
    )
}

fn render_symbol_body_fingerprint(body: &HirSymbolBody) -> String {
    match body {
        HirSymbolBody::None => "none".to_string(),
        HirSymbolBody::Record { fields } => format!(
            "record([{}])",
            fields
                .iter()
                .map(|field| format!(
                    "field(name={}|ty={})",
                    quote_fingerprint_text(&field.name),
                    quote_fingerprint_text(&field.ty)
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirSymbolBody::Enum { variants } => format!(
            "enum([{}])",
            variants
                .iter()
                .map(|variant| format!(
                    "variant(name={}|payload={})",
                    quote_fingerprint_text(&variant.name),
                    variant
                        .payload
                        .as_ref()
                        .map(|payload| quote_fingerprint_text(payload))
                        .unwrap_or_else(|| "none".to_string())
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirSymbolBody::Trait {
            assoc_types,
            methods,
        } => format!(
            "trait(assoc_types=[{}]|methods=[{}])",
            assoc_types
                .iter()
                .map(|assoc_type| format!(
                    "assoc_type(name={}|default={})",
                    quote_fingerprint_text(&assoc_type.name),
                    assoc_type
                        .default_ty
                        .as_ref()
                        .map(|default_ty| quote_fingerprint_text(default_ty))
                        .unwrap_or_else(|| "none".to_string())
                ))
                .collect::<Vec<_>>()
                .join(","),
            methods
                .iter()
                .map(render_symbol_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn render_impl_fingerprint(impl_decl: &HirImplDecl) -> String {
    format!(
        concat!(
            "impl(type_params=[{}]|trait={}|target={}|assoc_types=[{}]|methods=[{}]|",
            "body_entries=[{}]|surface={})"
        ),
        impl_decl
            .type_params
            .iter()
            .map(|param| quote_fingerprint_text(param))
            .collect::<Vec<_>>()
            .join(","),
        impl_decl
            .trait_path
            .as_ref()
            .map(|trait_path| quote_fingerprint_text(trait_path))
            .unwrap_or_else(|| "none".to_string()),
        quote_fingerprint_text(&impl_decl.target_type),
        impl_decl
            .assoc_types
            .iter()
            .map(|assoc_type| format!(
                "assoc(name={}|value={})",
                quote_fingerprint_text(&assoc_type.name),
                assoc_type
                    .value_ty
                    .as_ref()
                    .map(|value_ty| quote_fingerprint_text(value_ty))
                    .unwrap_or_else(|| "none".to_string())
            ))
            .collect::<Vec<_>>()
            .join(","),
        impl_decl
            .methods
            .iter()
            .map(render_symbol_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        impl_decl
            .body_entries
            .iter()
            .map(|entry| quote_fingerprint_text(entry))
            .collect::<Vec<_>>()
            .join(","),
        quote_fingerprint_text(&impl_decl.surface_text)
    )
}

fn render_rollup_fingerprint(rollup: &HirPageRollup) -> String {
    format!(
        "rollup(kind={}|subject={}|handler=[{}])",
        rollup.kind.as_str(),
        quote_fingerprint_text(&rollup.subject),
        rollup
            .handler_path
            .iter()
            .map(|segment| quote_fingerprint_text(segment))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_statement_fingerprint(statement: &HirStatement) -> String {
    format!(
        "stmt(forewords=[{}]|rollups=[{}]|kind={})",
        statement
            .forewords
            .iter()
            .map(render_foreword_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        statement
            .rollups
            .iter()
            .map(render_rollup_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        render_statement_kind_fingerprint(&statement.kind)
    )
}

fn render_statement_kind_fingerprint(kind: &HirStatementKind) -> String {
    match kind {
        HirStatementKind::Let {
            mutable,
            name,
            value,
        } => format!(
            "let(mutable={}|name={}|value={})",
            mutable,
            quote_fingerprint_text(name),
            render_expr_fingerprint(value)
        ),
        HirStatementKind::Return { value } => format!(
            "return({})",
            value
                .as_ref()
                .map(render_expr_fingerprint)
                .unwrap_or_else(|| "none".to_string())
        ),
        HirStatementKind::If {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "if(cond={}|then=[{}]|else={})",
            render_expr_fingerprint(condition),
            then_branch
                .iter()
                .map(render_statement_fingerprint)
                .collect::<Vec<_>>()
                .join(","),
            else_branch
                .as_ref()
                .map(|branch| format!(
                    "[{}]",
                    branch
                        .iter()
                        .map(render_statement_fingerprint)
                        .collect::<Vec<_>>()
                        .join(",")
                ))
                .unwrap_or_else(|| "none".to_string())
        ),
        HirStatementKind::While { condition, body } => format!(
            "while(cond={}|body=[{}])",
            render_expr_fingerprint(condition),
            body.iter()
                .map(render_statement_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirStatementKind::For {
            binding,
            iterable,
            body,
        } => format!(
            "for(binding={}|iterable={}|body=[{}])",
            quote_fingerprint_text(binding),
            render_expr_fingerprint(iterable),
            body.iter()
                .map(render_statement_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirStatementKind::Defer { expr } => {
            format!("defer({})", render_expr_fingerprint(expr))
        }
        HirStatementKind::Break => "break".to_string(),
        HirStatementKind::Continue => "continue".to_string(),
        HirStatementKind::Assign { target, op, value } => format!(
            "assign(target={}|op={}|value={})",
            render_assign_target_fingerprint(target),
            op.as_str(),
            render_expr_fingerprint(value)
        ),
        HirStatementKind::Expr { expr } => {
            format!("expr({})", render_expr_fingerprint(expr))
        }
    }
}

fn render_assign_target_fingerprint(target: &HirAssignTarget) -> String {
    match target {
        HirAssignTarget::Name { text } => {
            format!("name({})", quote_fingerprint_text(text))
        }
        HirAssignTarget::MemberAccess { target, member } => format!(
            "member(base={}|member={})",
            render_assign_target_fingerprint(target),
            quote_fingerprint_text(member)
        ),
        HirAssignTarget::Index { target, index } => format!(
            "index(base={}|index={})",
            render_assign_target_fingerprint(target),
            render_expr_fingerprint(index)
        ),
    }
}

fn render_expr_fingerprint(expr: &HirExpr) -> String {
    match expr {
        HirExpr::Path { segments } => format!(
            "path([{}])",
            segments
                .iter()
                .map(|segment| quote_fingerprint_text(segment))
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::BoolLiteral { value } => format!("bool({value})"),
        HirExpr::IntLiteral { text } => format!("int({})", quote_fingerprint_text(text)),
        HirExpr::StrLiteral { text } => format!("str({})", quote_fingerprint_text(text)),
        HirExpr::Pair { left, right } => format!(
            "pair(left={}|right={})",
            render_expr_fingerprint(left),
            render_expr_fingerprint(right)
        ),
        HirExpr::CollectionLiteral { items } => format!(
            "collection([{}])",
            items
                .iter()
                .map(render_expr_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::Match { subject, arms } => format!(
            "match(subject={}|arms=[{}])",
            render_expr_fingerprint(subject),
            arms.iter()
                .map(render_match_arm_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::Chain {
            style,
            introducer,
            steps,
        } => format!(
            "chain(style={}|introducer={}|steps=[{}])",
            quote_fingerprint_text(style),
            render_chain_introducer_fingerprint(*introducer),
            steps
                .iter()
                .map(render_chain_step_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
            attached,
        } => format!(
            "memory(family={}|arena={}|args=[{}]|constructor={}|attached=[{}])",
            quote_fingerprint_text(family),
            render_expr_fingerprint(arena),
            init_args
                .iter()
                .map(render_phrase_arg_fingerprint)
                .collect::<Vec<_>>()
                .join(","),
            render_expr_fingerprint(constructor),
            attached
                .iter()
                .map(render_header_attachment_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::GenericApply { expr, type_args } => format!(
            "generic_apply(expr={}|type_args=[{}])",
            render_expr_fingerprint(expr),
            type_args
                .iter()
                .map(|arg| quote_fingerprint_text(arg))
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::QualifiedPhrase {
            subject,
            args,
            qualifier,
            attached,
        } => format!(
            "qualified(subject={}|args=[{}]|qualifier={}|attached=[{}])",
            render_expr_fingerprint(subject),
            args.iter()
                .map(render_phrase_arg_fingerprint)
                .collect::<Vec<_>>()
                .join(","),
            quote_fingerprint_text(qualifier),
            attached
                .iter()
                .map(render_header_attachment_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::Await { expr } => format!("await({})", render_expr_fingerprint(expr)),
        HirExpr::Unary { op, expr } => format!(
            "unary(op={}|expr={})",
            render_unary_op_fingerprint(*op),
            render_expr_fingerprint(expr)
        ),
        HirExpr::Binary { left, op, right } => format!(
            "binary(left={}|op={}|right={})",
            render_expr_fingerprint(left),
            render_binary_op_fingerprint(*op),
            render_expr_fingerprint(right)
        ),
        HirExpr::MemberAccess { expr, member } => format!(
            "member(expr={}|member={})",
            render_expr_fingerprint(expr),
            quote_fingerprint_text(member)
        ),
        HirExpr::Index { expr, index } => format!(
            "index(expr={}|index={})",
            render_expr_fingerprint(expr),
            render_expr_fingerprint(index)
        ),
        HirExpr::Slice {
            expr,
            start,
            end,
            inclusive_end,
        } => format!(
            "slice(expr={}|start={}|end={}|inclusive_end={})",
            render_expr_fingerprint(expr),
            start
                .as_ref()
                .map(|expr| render_expr_fingerprint(expr))
                .unwrap_or_else(|| "none".to_string()),
            end.as_ref()
                .map(|expr| render_expr_fingerprint(expr))
                .unwrap_or_else(|| "none".to_string()),
            inclusive_end
        ),
        HirExpr::Range {
            start,
            end,
            inclusive_end,
        } => format!(
            "range(start={}|end={}|inclusive_end={})",
            start
                .as_ref()
                .map(|expr| render_expr_fingerprint(expr))
                .unwrap_or_else(|| "none".to_string()),
            end.as_ref()
                .map(|expr| render_expr_fingerprint(expr))
                .unwrap_or_else(|| "none".to_string()),
            inclusive_end
        ),
    }
}

fn render_match_arm_fingerprint(arm: &HirMatchArm) -> String {
    format!(
        "arm(patterns=[{}]|value={})",
        arm.patterns
            .iter()
            .map(render_match_pattern_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        render_expr_fingerprint(&arm.value)
    )
}

fn render_match_pattern_fingerprint(pattern: &HirMatchPattern) -> String {
    match pattern {
        HirMatchPattern::Wildcard => "wildcard".to_string(),
        HirMatchPattern::Literal { text } => {
            format!("literal({})", quote_fingerprint_text(text))
        }
        HirMatchPattern::Name { text } => format!("name({})", quote_fingerprint_text(text)),
        HirMatchPattern::Variant { path, args } => format!(
            "variant(path={}|args=[{}])",
            quote_fingerprint_text(path),
            args.iter()
                .map(render_match_pattern_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn render_chain_step_fingerprint(step: &HirChainStep) -> String {
    format!(
        "step(incoming={}|stage={}|bind_args=[{}]|text={})",
        step.incoming
            .map(render_chain_connector_fingerprint)
            .unwrap_or("none"),
        render_expr_fingerprint(&step.stage),
        step.bind_args
            .iter()
            .map(render_expr_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        quote_fingerprint_text(&step.text)
    )
}

fn render_phrase_arg_fingerprint(arg: &HirPhraseArg) -> String {
    match arg {
        HirPhraseArg::Positional(expr) => {
            format!("pos({})", render_expr_fingerprint(expr))
        }
        HirPhraseArg::Named { name, value } => format!(
            "named(name={}|value={})",
            quote_fingerprint_text(name),
            render_expr_fingerprint(value)
        ),
    }
}

fn render_header_attachment_fingerprint(attachment: &HirHeaderAttachment) -> String {
    match attachment {
        HirHeaderAttachment::Named {
            name,
            value,
            forewords,
            ..
        } => format!(
            "named(name={}|value={}|forewords=[{}])",
            quote_fingerprint_text(name),
            render_expr_fingerprint(value),
            forewords
                .iter()
                .map(render_foreword_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirHeaderAttachment::Chain {
            expr, forewords, ..
        } => format!(
            "chain(expr={}|forewords=[{}])",
            render_expr_fingerprint(expr),
            forewords
                .iter()
                .map(render_foreword_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn render_dependency_edge_fingerprint(edge: &HirModuleDependency) -> String {
    format!(
        "dep(source={}|kind={}|target=[{}]|alias={})",
        quote_fingerprint_text(&edge.source_module_id),
        edge.kind.as_str(),
        edge.target_path
            .iter()
            .map(|segment| quote_fingerprint_text(segment))
            .collect::<Vec<_>>()
            .join(","),
        edge.alias
            .as_ref()
            .map(|alias| quote_fingerprint_text(alias))
            .unwrap_or_else(|| "none".to_string())
    )
}

fn render_chain_connector_fingerprint(connector: HirChainConnector) -> &'static str {
    match connector {
        HirChainConnector::Forward => "forward",
        HirChainConnector::Reverse => "reverse",
    }
}

fn render_chain_introducer_fingerprint(introducer: HirChainIntroducer) -> &'static str {
    match introducer {
        HirChainIntroducer::Forward => "forward",
        HirChainIntroducer::Reverse => "reverse",
    }
}

fn render_unary_op_fingerprint(op: HirUnaryOp) -> &'static str {
    match op {
        HirUnaryOp::Neg => "neg",
        HirUnaryOp::Not => "not",
        HirUnaryOp::BitNot => "bit_not",
        HirUnaryOp::BorrowRead => "borrow_read",
        HirUnaryOp::BorrowMut => "borrow_mut",
        HirUnaryOp::Deref => "deref",
        HirUnaryOp::Weave => "weave",
        HirUnaryOp::Split => "split",
    }
}

fn render_binary_op_fingerprint(op: HirBinaryOp) -> &'static str {
    match op {
        HirBinaryOp::Or => "or",
        HirBinaryOp::And => "and",
        HirBinaryOp::EqEq => "eqeq",
        HirBinaryOp::NotEq => "noteq",
        HirBinaryOp::Lt => "lt",
        HirBinaryOp::LtEq => "lteq",
        HirBinaryOp::Gt => "gt",
        HirBinaryOp::GtEq => "gteq",
        HirBinaryOp::BitOr => "bitor",
        HirBinaryOp::BitXor => "bitxor",
        HirBinaryOp::BitAnd => "bitand",
        HirBinaryOp::Shl => "shl",
        HirBinaryOp::Shr => "shr",
        HirBinaryOp::Add => "add",
        HirBinaryOp::Sub => "sub",
        HirBinaryOp::Mul => "mul",
        HirBinaryOp::Div => "div",
        HirBinaryOp::Mod => "mod",
    }
}

fn resolve_module_target(
    package: &HirWorkspacePackage,
    workspace: &HirWorkspaceSummary,
    path: &[String],
) -> Result<(String, String), String> {
    if path.is_empty() {
        return Err("missing module path".to_string());
    }

    let key = path.join(".");
    let first = &path[0];
    if first == &package.summary.package_name {
        return package
            .module(&key)
            .map(|module| {
                (
                    package.summary.package_name.clone(),
                    module.module_id.clone(),
                )
            })
            .ok_or_else(|| format!("unresolved module `{key}`"));
    }

    if first == "std" {
        return workspace
            .package("std")
            .ok_or_else(|| "implicit package `std` is not available".to_string())
            .and_then(|std_package| {
                std_package
                    .module(&key)
                    .map(|module| {
                        (
                            std_package.summary.package_name.clone(),
                            module.module_id.clone(),
                        )
                    })
                    .ok_or_else(|| format!("unresolved module `{key}`"))
            });
    }

    if let Some(dependency_name) = package.dependency_package_name(first) {
        let dependency_module_id = package
            .dependency_module_id(path)
            .ok_or_else(|| format!("unresolved module `{key}`"))?;
        return workspace
            .package(dependency_name)
            .ok_or_else(|| {
                format!(
                    "dependency `{first}` is not loaded for `{}`",
                    package.summary.package_name
                )
            })
            .and_then(|dependency| {
                dependency
                    .module(&dependency_module_id)
                    .map(|module| {
                        (
                            dependency.summary.package_name.clone(),
                            module.module_id.clone(),
                        )
                    })
                    .ok_or_else(|| format!("unresolved module `{key}`"))
            });
    }

    if workspace.package(first).is_some() {
        return Err(format!(
            "package `{first}` is not a direct dependency of `{}`",
            package.summary.package_name
        ));
    }

    package
        .resolve_relative_module(path)
        .map(|module| {
            (
                package.summary.package_name.clone(),
                module.module_id.clone(),
            )
        })
        .ok_or_else(|| format!("unresolved module `{key}`"))
}

enum ResolvedUseTarget {
    Module {
        package_name: String,
        module_id: String,
    },
    Symbol {
        package_name: String,
        module_id: String,
        symbol_name: String,
    },
}

fn resolve_use_target(
    package: &HirWorkspacePackage,
    workspace: &HirWorkspaceSummary,
    path: &[String],
) -> Result<ResolvedUseTarget, String> {
    if path.is_empty() {
        return Err("missing use target".to_string());
    }

    for prefix_len in (1..=path.len()).rev() {
        let prefix = &path[..prefix_len];
        let Ok((package_name, module_id)) = resolve_module_target(package, workspace, prefix)
        else {
            continue;
        };
        if prefix_len == path.len() {
            return Ok(ResolvedUseTarget::Module {
                package_name,
                module_id,
            });
        }

        let suffix = &path[prefix_len..];
        if suffix.len() != 1 {
            return Err(format!(
                "nested symbol path `{}` is not supported yet",
                path.join(".")
            ));
        }

        let symbol_name = &suffix[0];
        let visible_symbols = visible_symbol_refs_in_module_for_package(
            workspace,
            &package.summary.package_name,
            &package_name,
            &module_id,
            symbol_name,
        );
        if visible_symbols.len() == 1 {
            return Ok(ResolvedUseTarget::Symbol {
                package_name,
                module_id,
                symbol_name: symbol_name.clone(),
            });
        }
        if visible_symbols.len() > 1 {
            return Err(format!(
                "symbol `{symbol_name}` is defined multiple times in module `{module_id}`"
            ));
        }
        return Err(format!(
            "unresolved symbol `{symbol_name}` in module `{module_id}`"
        ));
    }

    if let Some(first) = path.first() {
        if workspace.package(first).is_some()
            && first != &package.summary.package_name
            && first != "std"
            && package.dependency_package_name(first).is_none()
        {
            return Err(format!(
                "package `{first}` is not a direct dependency of `{}`",
                package.summary.package_name
            ));
        }
    }

    Err(format!("unresolved module path `{}`", path.join(".")))
}

pub fn build_package_summary(
    package_name: impl Into<String>,
    mut modules: Vec<HirModuleSummary>,
) -> HirPackageSummary {
    modules.sort_by(|left, right| left.module_id.cmp(&right.module_id));

    let mut dependency_edges = modules
        .iter()
        .flat_map(|module| {
            module
                .directives
                .iter()
                .map(move |directive| HirModuleDependency {
                    source_module_id: module.module_id.clone(),
                    kind: directive.kind,
                    target_path: directive.path.clone(),
                    alias: directive.alias.clone(),
                    span: directive.span,
                })
        })
        .collect::<Vec<_>>();
    dependency_edges.sort_by(|left, right| {
        left.source_module_id
            .cmp(&right.source_module_id)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.target_path.cmp(&right.target_path))
            .then_with(|| left.alias.cmp(&right.alias))
            .then_with(|| left.span.line.cmp(&right.span.line))
            .then_with(|| left.span.column.cmp(&right.span.column))
    });

    HirPackageSummary {
        package_name: package_name.into(),
        modules,
        dependency_edges,
    }
}

pub fn build_package_layout(
    summary: &HirPackageSummary,
    module_paths: BTreeMap<String, PathBuf>,
    relative_modules: BTreeMap<String, String>,
) -> Result<HirPackageLayout, String> {
    let mut absolute_modules = BTreeMap::new();
    for (index, module) in summary.modules.iter().enumerate() {
        if absolute_modules
            .insert(module.module_id.clone(), index)
            .is_some()
        {
            return Err(format!(
                "duplicate module id `{}` in package `{}`",
                module.module_id, summary.package_name
            ));
        }
    }

    for module_id in absolute_modules.keys() {
        if !module_paths.contains_key(module_id) {
            return Err(format!(
                "missing source path for module `{module_id}` in package `{}`",
                summary.package_name
            ));
        }
    }
    for module_id in module_paths.keys() {
        if !absolute_modules.contains_key(module_id) {
            return Err(format!(
                "source path provided for unknown module `{module_id}` in package `{}`",
                summary.package_name
            ));
        }
    }
    for module_id in relative_modules.values() {
        if !absolute_modules.contains_key(module_id) {
            return Err(format!(
                "relative module mapping targets unknown module `{module_id}` in package `{}`",
                summary.package_name
            ));
        }
    }

    Ok(HirPackageLayout {
        module_paths,
        relative_modules,
        absolute_modules,
    })
}

pub fn build_workspace_package(
    root_dir: PathBuf,
    direct_deps: BTreeSet<String>,
    summary: HirPackageSummary,
    layout: HirPackageLayout,
) -> Result<HirWorkspacePackage, String> {
    let direct_dep_packages = direct_deps
        .iter()
        .map(|name| (name.clone(), name.clone()))
        .collect();
    build_workspace_package_with_dep_packages(root_dir, direct_dep_packages, summary, layout)
}

pub fn build_workspace_package_with_dep_packages(
    root_dir: PathBuf,
    direct_dep_packages: BTreeMap<String, String>,
    summary: HirPackageSummary,
    layout: HirPackageLayout,
) -> Result<HirWorkspacePackage, String> {
    if summary.modules.len() != layout.absolute_modules.len() {
        return Err(format!(
            "package `{}` has {} modules but layout indexes {}",
            summary.package_name,
            summary.modules.len(),
            layout.absolute_modules.len()
        ));
    }

    let direct_deps = direct_dep_packages.keys().cloned().collect();
    Ok(HirWorkspacePackage {
        root_dir,
        direct_deps,
        direct_dep_packages,
        summary,
        layout,
    })
}

pub fn build_workspace_summary(
    packages: Vec<HirWorkspacePackage>,
) -> Result<HirWorkspaceSummary, String> {
    let mut package_map = BTreeMap::new();
    for package in packages {
        let name = package.summary.package_name.clone();
        if package_map.insert(name.clone(), package).is_some() {
            return Err(format!("duplicate package `{name}` in workspace summary"));
        }
    }
    Ok(HirWorkspaceSummary {
        packages: package_map,
    })
}

pub fn resolve_workspace(
    workspace: &HirWorkspaceSummary,
) -> Result<HirResolvedWorkspace, Vec<HirResolutionError>> {
    let mut packages = BTreeMap::new();
    let mut errors = Vec::new();

    for package in workspace.packages.values() {
        let mut modules = BTreeMap::new();
        for module in &package.summary.modules {
            let mut bindings: BTreeMap<String, HirResolvedBinding> = BTreeMap::new();
            for symbol in &module.symbols {
                if let Some(existing) = bindings.get(&symbol.name) {
                    errors.push(HirResolutionError {
                        package_name: package.summary.package_name.clone(),
                        source_module_id: module.module_id.clone(),
                        span: symbol.span,
                        message: format!(
                            "duplicate symbol `{}` in module `{}`; first declared at {}:{}",
                            symbol.name, module.module_id, existing.span.line, existing.span.column
                        ),
                    });
                    continue;
                }
                bindings.insert(
                    symbol.name.clone(),
                    HirResolvedBinding {
                        local_name: symbol.name.clone(),
                        origin: HirBindingOrigin::LocalSymbol,
                        target: HirResolvedTarget::Symbol {
                            package_name: package.summary.package_name.clone(),
                            module_id: module.module_id.clone(),
                            symbol_name: symbol.name.clone(),
                        },
                        span: symbol.span,
                    },
                );
            }

            let mut directives = Vec::new();
            for directive in &module.directives {
                let target = match directive.kind {
                    HirDirectiveKind::Import | HirDirectiveKind::Reexport => {
                        resolve_module_target(package, workspace, &directive.path).map(
                            |(package_name, module_id)| HirResolvedTarget::Module {
                                package_name,
                                module_id,
                            },
                        )
                    }
                    HirDirectiveKind::Use => {
                        resolve_use_target(package, workspace, &directive.path).map(
                            |resolved_target| match resolved_target {
                                ResolvedUseTarget::Module {
                                    package_name,
                                    module_id,
                                } => HirResolvedTarget::Module {
                                    package_name,
                                    module_id,
                                },
                                ResolvedUseTarget::Symbol {
                                    package_name,
                                    module_id,
                                    symbol_name,
                                } => HirResolvedTarget::Symbol {
                                    package_name,
                                    module_id,
                                    symbol_name,
                                },
                            },
                        )
                    }
                };

                match target {
                    Ok(target) => {
                        let local_name = directive
                            .alias
                            .clone()
                            .unwrap_or_else(|| target.binding_name());
                        if let Some(existing) = bindings.get(&local_name) {
                            if matches!(existing.origin, HirBindingOrigin::Directive(_))
                                && existing.target == target
                            {
                                directives.push(HirResolvedDirective {
                                    source_module_id: module.module_id.clone(),
                                    local_name,
                                    kind: directive.kind,
                                    target,
                                    alias: directive.alias.clone(),
                                    span: directive.span,
                                });
                                continue;
                            }
                            errors.push(HirResolutionError {
                                package_name: package.summary.package_name.clone(),
                                source_module_id: module.module_id.clone(),
                                span: directive.span,
                                message: format!(
                                    "duplicate binding `{}` in module `{}`; first bound at {}:{}",
                                    local_name,
                                    module.module_id,
                                    existing.span.line,
                                    existing.span.column
                                ),
                            });
                            continue;
                        }
                        bindings.insert(
                            local_name.clone(),
                            HirResolvedBinding {
                                local_name: local_name.clone(),
                                origin: HirBindingOrigin::Directive(directive.kind),
                                target: target.clone(),
                                span: directive.span,
                            },
                        );
                        directives.push(HirResolvedDirective {
                            source_module_id: module.module_id.clone(),
                            local_name,
                            kind: directive.kind,
                            target,
                            alias: directive.alias.clone(),
                            span: directive.span,
                        });
                    }
                    Err(message) => errors.push(HirResolutionError {
                        package_name: package.summary.package_name.clone(),
                        source_module_id: module.module_id.clone(),
                        span: directive.span,
                        message,
                    }),
                }
            }

            modules.insert(
                module.module_id.clone(),
                HirResolvedModule {
                    module_id: module.module_id.clone(),
                    bindings,
                    directives,
                },
            );
        }

        packages.insert(
            package.summary.package_name.clone(),
            HirResolvedPackage {
                package_name: package.summary.package_name.clone(),
                modules,
            },
        );
    }

    if errors.is_empty() {
        Ok(HirResolvedWorkspace { packages })
    } else {
        Err(errors)
    }
}

pub fn derive_source_module_path(
    package_name: &str,
    root_file_name: &str,
    src_dir: &Path,
    module_path: &Path,
) -> Result<SourceModulePath, String> {
    let relative = module_path
        .strip_prefix(src_dir)
        .map_err(|err| format!("failed to relativize `{}`: {err}", module_path.display()))?;
    let mut components = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if components.is_empty() {
        return Err(format!("empty module path for `{}`", module_path.display()));
    }

    let file_name = components
        .pop()
        .ok_or_else(|| format!("empty module path for `{}`", module_path.display()))?;
    let stem = file_name
        .strip_suffix(".arc")
        .ok_or_else(|| format!("non-Arcana file `{}`", module_path.display()))?;
    if stem == "book" || stem == "shelf" {
        if file_name != root_file_name && !components.is_empty() {
            components.push(stem.to_string());
        }
    } else {
        components.push(stem.to_string());
    }

    let mut module_segments = vec![package_name.to_string()];
    module_segments.extend(components.iter().cloned());
    Ok(SourceModulePath {
        relative_segments: components,
        module_id: module_segments.join("."),
    })
}

fn lower_directive_kind(kind: &ParsedDirectiveKind) -> HirDirectiveKind {
    match kind {
        ParsedDirectiveKind::Import => HirDirectiveKind::Import,
        ParsedDirectiveKind::Use => HirDirectiveKind::Use,
        ParsedDirectiveKind::Reexport => HirDirectiveKind::Reexport,
    }
}

fn lower_symbol_kind(kind: &ParsedSymbolKind) -> HirSymbolKind {
    match kind {
        ParsedSymbolKind::Fn => HirSymbolKind::Fn,
        ParsedSymbolKind::System => HirSymbolKind::System,
        ParsedSymbolKind::Record => HirSymbolKind::Record,
        ParsedSymbolKind::Enum => HirSymbolKind::Enum,
        ParsedSymbolKind::OpaqueType => HirSymbolKind::OpaqueType,
        ParsedSymbolKind::Trait => HirSymbolKind::Trait,
        ParsedSymbolKind::Behavior => HirSymbolKind::Behavior,
        ParsedSymbolKind::Const => HirSymbolKind::Const,
    }
}

fn lower_opaque_policy(policy: &ParsedOpaqueTypePolicy) -> HirOpaqueTypePolicy {
    HirOpaqueTypePolicy {
        ownership: match policy.ownership {
            ParsedOpaqueOwnershipPolicy::Copy => HirOpaqueOwnershipPolicy::Copy,
            ParsedOpaqueOwnershipPolicy::Move => HirOpaqueOwnershipPolicy::Move,
        },
        boundary: match policy.boundary {
            ParsedOpaqueBoundaryPolicy::Safe => HirOpaqueBoundaryPolicy::Safe,
            ParsedOpaqueBoundaryPolicy::Unsafe => HirOpaqueBoundaryPolicy::Unsafe,
        },
    }
}

fn lower_param_mode(mode: &ParsedParamMode) -> HirParamMode {
    match mode {
        ParsedParamMode::Read => HirParamMode::Read,
        ParsedParamMode::Edit => HirParamMode::Edit,
        ParsedParamMode::Take => HirParamMode::Take,
    }
}

fn lower_symbol_body(body: &arcana_syntax::SymbolBody) -> HirSymbolBody {
    match body {
        arcana_syntax::SymbolBody::None => HirSymbolBody::None,
        arcana_syntax::SymbolBody::Record { fields } => HirSymbolBody::Record {
            fields: fields
                .iter()
                .map(|field| HirField {
                    name: field.name.clone(),
                    ty: field.ty.clone(),
                    span: field.span,
                })
                .collect(),
        },
        arcana_syntax::SymbolBody::Enum { variants } => HirSymbolBody::Enum {
            variants: variants
                .iter()
                .map(|variant| HirEnumVariant {
                    name: variant.name.clone(),
                    payload: variant.payload.clone(),
                    span: variant.span,
                })
                .collect(),
        },
        arcana_syntax::SymbolBody::Trait {
            assoc_types,
            methods,
        } => HirSymbolBody::Trait {
            assoc_types: assoc_types
                .iter()
                .map(|assoc_type| HirTraitAssocType {
                    name: assoc_type.name.clone(),
                    default_ty: assoc_type.default_ty.clone(),
                    span: assoc_type.span,
                })
                .collect(),
            methods: methods.iter().map(lower_trait_or_impl_method).collect(),
        },
    }
}

fn lower_trait_or_impl_method(method: &arcana_syntax::SymbolDecl) -> HirSymbol {
    HirSymbol {
        kind: lower_symbol_kind(&method.kind),
        name: method.name.clone(),
        exported: method.exported,
        is_async: method.is_async,
        type_params: method.type_params.clone(),
        where_clause: method.where_clause.clone(),
        params: method
            .params
            .iter()
            .map(|param| HirParam {
                mode: param.mode.as_ref().map(lower_param_mode),
                name: param.name.clone(),
                ty: param.ty.clone(),
            })
            .collect(),
        return_type: method.return_type.clone(),
        behavior_attrs: method
            .behavior_attrs
            .iter()
            .map(|attr| HirBehaviorAttr {
                name: attr.name.clone(),
                value: attr.value.clone(),
            })
            .collect(),
        opaque_policy: method.opaque_policy.as_ref().map(lower_opaque_policy),
        forewords: lower_forewords(&method.forewords),
        intrinsic_impl: method.intrinsic_impl.clone(),
        body: lower_symbol_body(&method.body),
        statements: lower_statements(&method.statements),
        rollups: lower_rollups(&method.rollups),
        surface_text: method.surface_text.clone(),
        span: method.span,
    }
}

fn lower_forewords(forewords: &[arcana_syntax::ForewordApp]) -> Vec<HirForewordApp> {
    forewords
        .iter()
        .map(|foreword| HirForewordApp {
            name: foreword.name.clone(),
            args: foreword
                .args
                .iter()
                .map(|arg| HirForewordArg {
                    name: arg.name.clone(),
                    value: arg.value.clone(),
                })
                .collect(),
            span: foreword.span,
        })
        .collect()
}

fn lower_rollups(rollups: &[arcana_syntax::PageRollup]) -> Vec<HirPageRollup> {
    rollups
        .iter()
        .map(|rollup| HirPageRollup {
            kind: match rollup.kind {
                arcana_syntax::PageRollupKind::Cleanup => HirPageRollupKind::Cleanup,
            },
            subject: rollup.subject.clone(),
            handler_path: rollup.handler_path.clone(),
            span: rollup.span,
        })
        .collect()
}

fn lower_assign_op(op: &ParsedAssignOp) -> HirAssignOp {
    match op {
        ParsedAssignOp::Assign => HirAssignOp::Assign,
        ParsedAssignOp::AddAssign => HirAssignOp::AddAssign,
        ParsedAssignOp::SubAssign => HirAssignOp::SubAssign,
        ParsedAssignOp::MulAssign => HirAssignOp::MulAssign,
        ParsedAssignOp::DivAssign => HirAssignOp::DivAssign,
        ParsedAssignOp::ModAssign => HirAssignOp::ModAssign,
        ParsedAssignOp::BitAndAssign => HirAssignOp::BitAndAssign,
        ParsedAssignOp::BitOrAssign => HirAssignOp::BitOrAssign,
        ParsedAssignOp::BitXorAssign => HirAssignOp::BitXorAssign,
        ParsedAssignOp::ShlAssign => HirAssignOp::ShlAssign,
        ParsedAssignOp::ShrAssign => HirAssignOp::ShrAssign,
    }
}

fn lower_assign_target(target: &arcana_syntax::AssignTarget) -> HirAssignTarget {
    match target {
        arcana_syntax::AssignTarget::Name { text } => HirAssignTarget::Name { text: text.clone() },
        arcana_syntax::AssignTarget::MemberAccess { target, member } => {
            HirAssignTarget::MemberAccess {
                target: Box::new(lower_assign_target(target)),
                member: member.clone(),
            }
        }
        arcana_syntax::AssignTarget::Index { target, index } => HirAssignTarget::Index {
            target: Box::new(lower_assign_target(target)),
            index: lower_expr(index),
        },
    }
}

fn lower_header_attachments(
    attachments: &[arcana_syntax::HeaderAttachment],
) -> Vec<HirHeaderAttachment> {
    attachments
        .iter()
        .map(|attachment| match attachment {
            arcana_syntax::HeaderAttachment::Named {
                name,
                value,
                forewords,
                span,
            } => HirHeaderAttachment::Named {
                name: name.clone(),
                value: lower_expr(value),
                forewords: lower_forewords(forewords),
                span: *span,
            },
            arcana_syntax::HeaderAttachment::Chain {
                expr,
                forewords,
                span,
            } => HirHeaderAttachment::Chain {
                expr: lower_expr(expr),
                forewords: lower_forewords(forewords),
                span: *span,
            },
        })
        .collect()
}

fn lower_expr(expr: &ParsedExpr) -> HirExpr {
    match expr {
        ParsedExpr::Path { segments } => HirExpr::Path {
            segments: segments.clone(),
        },
        ParsedExpr::BoolLiteral { value } => HirExpr::BoolLiteral { value: *value },
        ParsedExpr::IntLiteral { text } => HirExpr::IntLiteral { text: text.clone() },
        ParsedExpr::StrLiteral { text } => HirExpr::StrLiteral { text: text.clone() },
        ParsedExpr::Pair { left, right } => HirExpr::Pair {
            left: Box::new(lower_expr(left)),
            right: Box::new(lower_expr(right)),
        },
        ParsedExpr::CollectionLiteral { items } => HirExpr::CollectionLiteral {
            items: items.iter().map(lower_expr).collect(),
        },
        ParsedExpr::Match { subject, arms } => HirExpr::Match {
            subject: Box::new(lower_expr(subject)),
            arms: arms
                .iter()
                .map(|arm| HirMatchArm {
                    patterns: arm.patterns.iter().map(lower_match_pattern).collect(),
                    value: lower_expr(&arm.value),
                    span: arm.span,
                })
                .collect(),
        },
        ParsedExpr::Chain {
            style,
            introducer,
            steps,
        } => HirExpr::Chain {
            style: style.clone(),
            introducer: lower_chain_introducer(*introducer),
            steps: steps.iter().map(lower_chain_step).collect(),
        },
        ParsedExpr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
            attached,
        } => HirExpr::MemoryPhrase {
            family: family.clone(),
            arena: Box::new(lower_expr(arena)),
            init_args: init_args.iter().map(lower_phrase_arg).collect(),
            constructor: Box::new(lower_expr(constructor)),
            attached: lower_header_attachments(attached),
        },
        ParsedExpr::GenericApply { expr, type_args } => HirExpr::GenericApply {
            expr: Box::new(lower_expr(expr)),
            type_args: type_args.clone(),
        },
        ParsedExpr::QualifiedPhrase {
            subject,
            args,
            qualifier,
            attached,
        } => HirExpr::QualifiedPhrase {
            subject: Box::new(lower_expr(subject)),
            args: args.iter().map(lower_phrase_arg).collect(),
            qualifier: qualifier.clone(),
            attached: lower_header_attachments(attached),
        },
        ParsedExpr::Await { expr } => HirExpr::Await {
            expr: Box::new(lower_expr(expr)),
        },
        ParsedExpr::Unary { op, expr } => HirExpr::Unary {
            op: lower_unary_op(op),
            expr: Box::new(lower_expr(expr)),
        },
        ParsedExpr::Binary { left, op, right } => HirExpr::Binary {
            left: Box::new(lower_expr(left)),
            op: lower_binary_op(op),
            right: Box::new(lower_expr(right)),
        },
        ParsedExpr::MemberAccess { expr, member } => HirExpr::MemberAccess {
            expr: Box::new(lower_expr(expr)),
            member: member.clone(),
        },
        ParsedExpr::Index { expr, index } => HirExpr::Index {
            expr: Box::new(lower_expr(expr)),
            index: Box::new(lower_expr(index)),
        },
        ParsedExpr::Slice {
            expr,
            start,
            end,
            inclusive_end,
        } => HirExpr::Slice {
            expr: Box::new(lower_expr(expr)),
            start: start.as_ref().map(|expr| Box::new(lower_expr(expr))),
            end: end.as_ref().map(|expr| Box::new(lower_expr(expr))),
            inclusive_end: *inclusive_end,
        },
        ParsedExpr::Range {
            start,
            end,
            inclusive_end,
        } => HirExpr::Range {
            start: start.as_ref().map(|expr| Box::new(lower_expr(expr))),
            end: end.as_ref().map(|expr| Box::new(lower_expr(expr))),
            inclusive_end: *inclusive_end,
        },
    }
}

fn lower_match_pattern(pattern: &arcana_syntax::MatchPattern) -> HirMatchPattern {
    match pattern {
        arcana_syntax::MatchPattern::Wildcard => HirMatchPattern::Wildcard,
        arcana_syntax::MatchPattern::Literal { text } => {
            HirMatchPattern::Literal { text: text.clone() }
        }
        arcana_syntax::MatchPattern::Name { text } => HirMatchPattern::Name { text: text.clone() },
        arcana_syntax::MatchPattern::Variant { path, args } => HirMatchPattern::Variant {
            path: path.clone(),
            args: args.iter().map(lower_match_pattern).collect(),
        },
    }
}

fn lower_phrase_arg(arg: &arcana_syntax::PhraseArg) -> HirPhraseArg {
    match arg {
        arcana_syntax::PhraseArg::Positional(expr) => HirPhraseArg::Positional(lower_expr(expr)),
        arcana_syntax::PhraseArg::Named { name, value } => HirPhraseArg::Named {
            name: name.clone(),
            value: lower_expr(value),
        },
    }
}

fn lower_chain_step(step: &arcana_syntax::ChainStep) -> HirChainStep {
    HirChainStep {
        incoming: step.incoming.map(lower_chain_connector),
        stage: lower_expr(&step.stage),
        bind_args: step.bind_args.iter().map(lower_expr).collect(),
        text: step.text.clone(),
    }
}

fn lower_chain_connector(connector: arcana_syntax::ChainConnector) -> HirChainConnector {
    match connector {
        arcana_syntax::ChainConnector::Forward => HirChainConnector::Forward,
        arcana_syntax::ChainConnector::Reverse => HirChainConnector::Reverse,
    }
}

fn lower_chain_introducer(introducer: arcana_syntax::ChainIntroducer) -> HirChainIntroducer {
    match introducer {
        arcana_syntax::ChainIntroducer::Forward => HirChainIntroducer::Forward,
        arcana_syntax::ChainIntroducer::Reverse => HirChainIntroducer::Reverse,
    }
}

fn lower_unary_op(op: &arcana_syntax::UnaryOp) -> HirUnaryOp {
    match op {
        arcana_syntax::UnaryOp::Neg => HirUnaryOp::Neg,
        arcana_syntax::UnaryOp::Not => HirUnaryOp::Not,
        arcana_syntax::UnaryOp::BitNot => HirUnaryOp::BitNot,
        arcana_syntax::UnaryOp::BorrowRead => HirUnaryOp::BorrowRead,
        arcana_syntax::UnaryOp::BorrowMut => HirUnaryOp::BorrowMut,
        arcana_syntax::UnaryOp::Deref => HirUnaryOp::Deref,
        arcana_syntax::UnaryOp::Weave => HirUnaryOp::Weave,
        arcana_syntax::UnaryOp::Split => HirUnaryOp::Split,
    }
}

fn lower_binary_op(op: &arcana_syntax::BinaryOp) -> HirBinaryOp {
    match op {
        arcana_syntax::BinaryOp::Or => HirBinaryOp::Or,
        arcana_syntax::BinaryOp::And => HirBinaryOp::And,
        arcana_syntax::BinaryOp::EqEq => HirBinaryOp::EqEq,
        arcana_syntax::BinaryOp::NotEq => HirBinaryOp::NotEq,
        arcana_syntax::BinaryOp::Lt => HirBinaryOp::Lt,
        arcana_syntax::BinaryOp::LtEq => HirBinaryOp::LtEq,
        arcana_syntax::BinaryOp::Gt => HirBinaryOp::Gt,
        arcana_syntax::BinaryOp::GtEq => HirBinaryOp::GtEq,
        arcana_syntax::BinaryOp::BitOr => HirBinaryOp::BitOr,
        arcana_syntax::BinaryOp::BitXor => HirBinaryOp::BitXor,
        arcana_syntax::BinaryOp::BitAnd => HirBinaryOp::BitAnd,
        arcana_syntax::BinaryOp::Shl => HirBinaryOp::Shl,
        arcana_syntax::BinaryOp::Shr => HirBinaryOp::Shr,
        arcana_syntax::BinaryOp::Add => HirBinaryOp::Add,
        arcana_syntax::BinaryOp::Sub => HirBinaryOp::Sub,
        arcana_syntax::BinaryOp::Mul => HirBinaryOp::Mul,
        arcana_syntax::BinaryOp::Div => HirBinaryOp::Div,
        arcana_syntax::BinaryOp::Mod => HirBinaryOp::Mod,
    }
}

fn lower_statements(statements: &[arcana_syntax::Statement]) -> Vec<HirStatement> {
    statements.iter().map(lower_statement).collect()
}

fn lower_statement(statement: &arcana_syntax::Statement) -> HirStatement {
    HirStatement {
        kind: match &statement.kind {
            ParsedStatementKind::Let {
                mutable,
                name,
                value,
            } => HirStatementKind::Let {
                mutable: *mutable,
                name: name.clone(),
                value: lower_expr(value),
            },
            ParsedStatementKind::Return { value } => HirStatementKind::Return {
                value: value.as_ref().map(lower_expr),
            },
            ParsedStatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => HirStatementKind::If {
                condition: lower_expr(condition),
                then_branch: lower_statements(then_branch),
                else_branch: else_branch.as_ref().map(|branch| lower_statements(branch)),
            },
            ParsedStatementKind::While { condition, body } => HirStatementKind::While {
                condition: lower_expr(condition),
                body: lower_statements(body),
            },
            ParsedStatementKind::For {
                binding,
                iterable,
                body,
            } => HirStatementKind::For {
                binding: binding.clone(),
                iterable: lower_expr(iterable),
                body: lower_statements(body),
            },
            ParsedStatementKind::Defer { expr } => HirStatementKind::Defer {
                expr: lower_expr(expr),
            },
            ParsedStatementKind::Break => HirStatementKind::Break,
            ParsedStatementKind::Continue => HirStatementKind::Continue,
            ParsedStatementKind::Assign { target, op, value } => HirStatementKind::Assign {
                target: lower_assign_target(target),
                op: lower_assign_op(op),
                value: lower_expr(value),
            },
            ParsedStatementKind::Expr { expr } => HirStatementKind::Expr {
                expr: lower_expr(expr),
            },
        },
        forewords: lower_forewords(&statement.forewords),
        rollups: lower_rollups(&statement.rollups),
        span: statement.span,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    use super::freeze::FROZEN_HIR_NODE_KINDS;
    use super::{
        HirAssignOp, HirAssignTarget, HirBinaryOp, HirChainConnector, HirChainIntroducer,
        HirChainStep, HirDirectiveKind, HirExpr, HirForewordApp, HirForewordArg,
        HirHeaderAttachment, HirMatchPattern, HirPhraseArg, HirStatement, HirStatementKind,
        HirSymbolBody, HirSymbolKind, HirUnaryOp, build_package_layout, build_package_summary,
        build_workspace_package, build_workspace_summary, derive_source_module_path,
        lookup_method_candidates_for_type, lookup_symbol_path, lower_module_text,
        resolve_workspace,
    };

    fn expr_is_path(expr: &HirExpr, name: &str) -> bool {
        matches!(expr, HirExpr::Path { segments } if segments == &vec![name.to_string()])
    }

    fn expr_is_int_literal(expr: &HirExpr, text: &str) -> bool {
        matches!(expr, HirExpr::IntLiteral { text: value } if value == text)
    }

    fn expr_is_str_literal(expr: &HirExpr, text: &str) -> bool {
        matches!(expr, HirExpr::StrLiteral { text: value } if value == text)
    }

    #[test]
    fn lower_module_handles_opaque_type_declarations() {
        let module = lower_module_text(
            "pkg.types",
            "export opaque type Window as move, boundary_unsafe\nopaque type Token[T] as move, boundary_safe\n",
        )
        .expect("opaque types should lower");

        assert_eq!(module.symbols.len(), 2);
        assert_eq!(module.symbols[0].kind, HirSymbolKind::OpaqueType);
        assert_eq!(module.symbols[0].name, "Window");
        let policy = module.symbols[0].opaque_policy.expect("opaque policy");
        assert_eq!(policy.ownership, super::HirOpaqueOwnershipPolicy::Move);
        assert_eq!(policy.boundary, super::HirOpaqueBoundaryPolicy::Unsafe);

        assert_eq!(module.symbols[1].kind, HirSymbolKind::OpaqueType);
        assert_eq!(module.symbols[1].type_params, vec!["T".to_string()]);
        let policy = module.symbols[1].opaque_policy.expect("opaque policy");
        assert_eq!(policy.ownership, super::HirOpaqueOwnershipPolicy::Move);
        assert_eq!(policy.boundary, super::HirOpaqueBoundaryPolicy::Safe);
    }

    fn chain_step_texts(steps: &[HirChainStep]) -> Vec<String> {
        steps.iter().map(|step| step.text.clone()).collect()
    }

    #[test]
    fn frozen_hir_list_is_unique() {
        let mut kinds = FROZEN_HIR_NODE_KINDS.to_vec();
        kinds.sort_unstable();
        kinds.dedup();
        assert_eq!(kinds.len(), FROZEN_HIR_NODE_KINDS.len());
    }

    #[test]
    fn lower_module_text_preserves_public_surface() {
        let module = lower_module_text(
            "std.io",
            "import std.result\nreexport std.result\nexport fn print() -> Int:\n    return 0\nfn helper() -> Int:\n    return 1\n",
        )
        .expect("lowering should pass");

        assert_eq!(module.module_id, "std.io");
        assert_eq!(module.directives[0].kind, HirDirectiveKind::Import);
        assert_eq!(module.directives[1].kind, HirDirectiveKind::Reexport);
        assert!(module.has_symbol("print"));
        assert!(module.has_symbol("helper"));
        assert_eq!(module.impls.len(), 0);
        assert!(matches!(
            module.symbols[0].statements[0].kind,
            HirStatementKind::Return { .. }
        ));
        assert_eq!(
            module.summary_surface_rows(),
            vec![
                "export:fn:fn print() -> Int:".to_string(),
                "reexport:std.result".to_string(),
            ]
        );
    }

    #[test]
    fn lower_module_text_captures_async_functions_and_impls() {
        let module = lower_module_text(
            "async_demo",
            "export async fn worker[T, where std.iter.Iterator[T]](read it: T, count: Int) -> Int:\n    return count\nbehavior[phase=update, affinity=worker] fn tick():\n    return 0\nimpl[T] std.iter.Iterator[T] for RangeIter:\n    fn next(edit self: RangeIter) -> (Bool, Int):\n        return (false, 0)\n",
        )
        .expect("lowering should pass");

        assert_eq!(module.symbols.len(), 2);
        let worker = &module.symbols[0];
        assert!(worker.is_async);
        assert_eq!(worker.type_params, vec!["T".to_string()]);
        assert_eq!(
            worker.where_clause,
            Some("std.iter.Iterator[T]".to_string())
        );
        assert_eq!(worker.params.len(), 2);
        assert_eq!(worker.return_type, Some("Int".to_string()));
        let tick = &module.symbols[1];
        assert_eq!(tick.kind, super::HirSymbolKind::Behavior);
        assert_eq!(tick.behavior_attrs.len(), 2);
        assert_eq!(tick.behavior_attrs[0].name, "phase");
        assert_eq!(tick.behavior_attrs[0].value, "update");
        assert_eq!(
            module.summary_surface_rows(),
            vec!["export:fn:async fn worker[T, where std.iter.Iterator[T]](read it: T, count: Int) -> Int:".to_string()]
        );
        assert_eq!(module.impls.len(), 1);
        assert_eq!(module.impls[0].type_params, vec!["T".to_string()]);
        assert_eq!(
            module.impls[0].trait_path,
            Some("std.iter.Iterator[T]".to_string())
        );
        assert_eq!(module.impls[0].target_type, "RangeIter");
        assert_eq!(module.impls[0].methods.len(), 1);
        assert_eq!(module.impls[0].methods[0].name, "next");
    }

    #[test]
    fn lower_module_text_captures_structured_statements() {
        let module = lower_module_text(
            "flow_demo",
            "fn main() -> Int:\n    let mut frames = 0\n    while frames < 10:\n        if frames % 2 == 0:\n            frames += 1\n        else:\n            continue\n    return match frames:\n        10 => 1\n        _ => 0\n",
        )
        .expect("lowering should pass");

        let statements = &module.symbols[0].statements;
        assert_eq!(statements.len(), 3);
        match &statements[0].kind {
            HirStatementKind::Let {
                mutable,
                name,
                value,
            } => {
                assert!(*mutable);
                assert_eq!(name, "frames");
                assert!(expr_is_int_literal(value, "0"));
            }
            other => panic!("expected let statement, got {other:?}"),
        }
        match &statements[1].kind {
            HirStatementKind::While { condition, body } => {
                match condition {
                    HirExpr::Binary { left, op, right } => {
                        assert_eq!(*op, HirBinaryOp::Lt);
                        assert!(matches!(
                            left.as_ref(),
                            expr if expr_is_path(expr, "frames")
                        ));
                        assert!(matches!(right.as_ref(), expr if expr_is_int_literal(expr, "10")));
                    }
                    other => panic!("expected binary while condition, got {other:?}"),
                }
                assert_eq!(body.len(), 1);
                match &body[0].kind {
                    HirStatementKind::If {
                        condition,
                        then_branch,
                        else_branch,
                    } => {
                        match condition {
                            HirExpr::Binary { left, op, right } => {
                                assert_eq!(*op, HirBinaryOp::EqEq);
                                match left.as_ref() {
                                    HirExpr::Binary { left, op, right } => {
                                        assert_eq!(*op, HirBinaryOp::Mod);
                                        assert!(matches!(
                                            left.as_ref(),
                                            expr if expr_is_path(expr, "frames")
                                        ));
                                        assert!(matches!(
                                            right.as_ref(),
                                            expr if expr_is_int_literal(expr, "2")
                                        ));
                                    }
                                    other => panic!(
                                        "expected modulo expression in if condition, got {other:?}"
                                    ),
                                }
                                assert!(matches!(
                                    right.as_ref(),
                                    expr if expr_is_int_literal(expr, "0")
                                ));
                            }
                            other => panic!("expected equality if condition, got {other:?}"),
                        }
                        assert_eq!(then_branch.len(), 1);
                        match &then_branch[0].kind {
                            HirStatementKind::Assign { target, op, value } => {
                                assert!(matches!(
                                    target,
                                    HirAssignTarget::Name { text } if text == "frames"
                                ));
                                assert_eq!(*op, HirAssignOp::AddAssign);
                                assert!(matches!(
                                    value,
                                    expr if expr_is_int_literal(expr, "1")
                                ));
                            }
                            other => panic!("expected assignment, got {other:?}"),
                        }
                        let else_branch = else_branch.as_ref().expect("else branch should exist");
                        assert!(matches!(else_branch[0].kind, HirStatementKind::Continue));
                    }
                    other => panic!("expected nested if statement, got {other:?}"),
                }
            }
            other => panic!("expected while statement, got {other:?}"),
        }
        match &statements[2].kind {
            HirStatementKind::Return { value } => {
                match value.as_ref().expect("return should carry a value") {
                    HirExpr::Match { subject, arms } => {
                        assert!(matches!(
                            subject.as_ref(),
                            expr if expr_is_path(expr, "frames")
                        ));
                        assert_eq!(arms.len(), 2);
                        assert_eq!(
                            arms[0].patterns,
                            vec![HirMatchPattern::Literal {
                                text: "10".to_string()
                            }]
                        );
                        assert!(matches!(
                            arms[0].value,
                            ref expr if expr_is_int_literal(expr, "1")
                        ));
                        assert_eq!(arms[1].patterns, vec![HirMatchPattern::Wildcard]);
                    }
                    other => panic!("expected match expression, got {other:?}"),
                }
            }
            other => panic!("expected return statement, got {other:?}"),
        }
    }

    #[test]
    fn lower_module_text_captures_page_rollups() {
        let module = lower_module_text(
            "rollups",
            "fn cleanup(value: Int):\n    return\nfn run(seed: Int) -> Int:\n    let local = seed\n    while local > 0:\n        let scratch = local\n        local -= 1\n    [scratch, cleanup]#cleanup\n    return local\n[seed, cleanup]#cleanup\n",
        )
        .expect("rollups should lower");

        let run = module
            .symbols
            .iter()
            .find(|symbol| symbol.name == "run")
            .expect("run symbol should exist");
        assert_eq!(run.rollups.len(), 1);
        assert_eq!(run.rollups[0].subject, "seed");
        assert_eq!(run.rollups[0].handler_path, vec!["cleanup".to_string()]);
        match &run.statements[1] {
            HirStatement {
                kind: HirStatementKind::While { .. },
                rollups,
                ..
            } => {
                assert_eq!(rollups.len(), 1);
                assert_eq!(rollups[0].subject, "scratch");
                assert_eq!(rollups[0].kind.as_str(), "cleanup");
            }
            other => panic!("expected while statement with rollup, got {other:?}"),
        }
    }

    #[test]
    fn lower_module_text_captures_forewords_lang_items_intrinsics_and_systems() {
        let module = lower_module_text(
            "surface",
            "use std.result as result\n#test\n#inline\nfn smoke() -> Int:\n    return 0\n#stage[thread=worker, deterministic=true]\nsystem[phase=startup, affinity=main] fn boot():\n    #chain[phase=startup, deterministic=true]\n    forward :=> seed => step\nlang result = smoke\nintrinsic fn host_len(read text: Str) -> Int = HostTextLenBytes\nfn seed() -> Int:\n    return 1\nfn step(v: Int) -> Int:\n    return v\n",
        )
        .expect("lowering should pass");

        assert_eq!(module.lang_items.len(), 1);
        assert_eq!(module.lang_items[0].name, "result");
        assert_eq!(module.symbols[0].forewords.len(), 2);
        assert_eq!(module.symbols[1].kind, HirSymbolKind::System);
        assert_eq!(module.symbols[1].forewords[0].name, "stage");
        assert_eq!(module.symbols[1].statements[0].forewords[0].name, "chain");
        assert_eq!(
            module.symbols[2].intrinsic_impl.as_deref(),
            Some("HostTextLenBytes")
        );
    }

    #[test]
    fn lower_module_text_captures_collection_chain_and_memory_expressions() {
        let module = lower_module_text(
            "expr_demo",
            "fn main() -> Int:\n    let xs = [1, 2, 3]\n    let id = arena: store :> value = 1 <: Item\n    forward :=> seed => step\n    return xs[0]\n",
        )
        .expect("lowering should pass");

        match &module.symbols[0].statements[0].kind {
            HirStatementKind::Let {
                value: HirExpr::CollectionLiteral { items },
                ..
            } => {
                assert_eq!(items.len(), 3);
            }
            other => panic!("expected collection literal, got {other:?}"),
        }
        match &module.symbols[0].statements[1].kind {
            HirStatementKind::Let {
                value:
                    HirExpr::MemoryPhrase {
                        family,
                        constructor,
                        ..
                    },
                ..
            } => {
                assert_eq!(family, "arena");
                match constructor.as_ref() {
                    HirExpr::Path { segments } => assert_eq!(segments, &vec!["Item".to_string()]),
                    other => panic!("expected constructor path, got {other:?}"),
                }
            }
            other => panic!("expected memory phrase, got {other:?}"),
        }
        match &module.symbols[0].statements[2].kind {
            HirStatementKind::Expr {
                expr:
                    HirExpr::Chain {
                        style,
                        introducer,
                        steps,
                    },
            } => {
                assert_eq!(style, "forward");
                assert_eq!(*introducer, HirChainIntroducer::Forward);
                assert_eq!(chain_step_texts(steps), vec!["seed", "step"]);
                assert!(steps[0].incoming.is_none());
                assert_eq!(steps[1].incoming, Some(HirChainConnector::Forward));
            }
            other => panic!("expected chain expression, got {other:?}"),
        }
    }

    #[test]
    fn lower_module_text_captures_mixed_and_bound_chain_steps() {
        let module = lower_module_text(
            "chain_demo",
            "fn main(seed: Int) -> Int:\n    let score = forward :=> stage.seed with (seed) => stage.inc <= stage.dec <= stage.emit\n    return score\n",
        )
        .expect("lowering should pass");

        match &module.symbols[0].statements[0].kind {
            HirStatementKind::Let {
                value:
                    HirExpr::Chain {
                        style,
                        introducer,
                        steps,
                    },
                ..
            } => {
                assert_eq!(style, "forward");
                assert_eq!(*introducer, HirChainIntroducer::Forward);
                assert_eq!(
                    steps.iter().map(|step| step.incoming).collect::<Vec<_>>(),
                    vec![
                        None,
                        Some(HirChainConnector::Forward),
                        Some(HirChainConnector::Reverse),
                        Some(HirChainConnector::Reverse)
                    ]
                );
                assert_eq!(
                    chain_step_texts(steps),
                    vec![
                        "stage.seed with (seed)",
                        "stage.inc",
                        "stage.dec",
                        "stage.emit"
                    ]
                );
                assert!(matches!(&steps[0].stage, HirExpr::MemberAccess { .. }));
                assert_eq!(steps[0].bind_args.len(), 1);
                assert!(expr_is_path(&steps[0].bind_args[0], "seed"));
            }
            other => panic!("expected bound mixed chain expression, got {other:?}"),
        }
    }

    #[test]
    fn lower_module_text_captures_match_expressions() {
        let module = lower_module_text(
            "match_demo",
            "fn score(t: Token) -> Int:\n    return match t:\n        Token.Plus | Token.Minus => 1\n        Token.IntLit(v) => v\nfn main() -> Int:\n    let out = score :: Token.Minus :: call\n    let v = match out:\n        0 => 0\n        _ => 1\n    return v\n",
        )
        .expect("lowering should pass");

        match &module.symbols[0].statements[0].kind {
            HirStatementKind::Return { value } => {
                match value.as_ref().expect("match return expected") {
                    HirExpr::Match { subject, arms } => {
                        assert!(matches!(
                            subject.as_ref(),
                            expr if expr_is_path(expr, "t")
                        ));
                        assert_eq!(
                            arms[0].patterns,
                            vec![
                                HirMatchPattern::Name {
                                    text: "Token.Plus".to_string()
                                },
                                HirMatchPattern::Name {
                                    text: "Token.Minus".to_string()
                                }
                            ]
                        );
                        assert_eq!(
                            arms[1].patterns,
                            vec![HirMatchPattern::Variant {
                                path: "Token.IntLit".to_string(),
                                args: vec![HirMatchPattern::Name {
                                    text: "v".to_string()
                                }]
                            }]
                        );
                    }
                    other => panic!("expected match expression, got {other:?}"),
                }
            }
            other => panic!("expected return statement, got {other:?}"),
        }

        match &module.symbols[1].statements[1].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "v");
                match value {
                    HirExpr::Match { subject, arms } => {
                        assert!(matches!(
                            subject.as_ref(),
                            expr if expr_is_path(expr, "out")
                        ));
                        assert_eq!(arms.len(), 2);
                        assert_eq!(
                            arms[0].patterns,
                            vec![HirMatchPattern::Literal {
                                text: "0".to_string()
                            }]
                        );
                        assert_eq!(arms[1].patterns, vec![HirMatchPattern::Wildcard]);
                    }
                    other => panic!("expected match expression, got {other:?}"),
                }
            }
            other => panic!("expected let statement, got {other:?}"),
        }
    }

    #[test]
    fn lower_module_text_captures_record_enum_and_trait_members() {
        let module = lower_module_text(
            "types",
            "export record Counter:\n    value: Int\nexport enum Result[T]:\n    Ok(Int)\n    Err(Str)\nexport trait CounterOps[T]:\n    type Output\n    fn tick(edit self: T) -> Int:\n        return 0\n",
        )
        .expect("lowering should pass");

        assert_eq!(module.symbols.len(), 3);
        match &module.symbols[0].body {
            HirSymbolBody::Record { fields } => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].name, "value");
            }
            other => panic!("expected record body, got {other:?}"),
        }
        match &module.symbols[1].body {
            HirSymbolBody::Enum { variants } => {
                assert_eq!(variants.len(), 2);
                assert_eq!(variants[0].name, "Ok");
            }
            other => panic!("expected enum body, got {other:?}"),
        }
        match &module.symbols[2].body {
            HirSymbolBody::Trait {
                assoc_types,
                methods,
            } => {
                assert_eq!(assoc_types.len(), 1);
                assert_eq!(assoc_types[0].name, "Output");
                assert_eq!(methods.len(), 1);
                assert_eq!(methods[0].name, "tick");
            }
            other => panic!("expected trait body, got {other:?}"),
        }
        assert_eq!(
            module.summary_surface_rows(),
            vec![
                "export:enum:enum Result[T]:\\nOk(Int)\\nErr(Str)".to_string(),
                "export:record:record Counter:\\nvalue: Int".to_string(),
                "export:trait:trait CounterOps[T]:\\ntype Output\\nfn tick(edit self: T) -> Int:"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn lower_module_text_captures_structured_phrases_and_operators() {
        let module = lower_module_text(
            "expr_demo",
            "fn main() -> Int:\n    defer io.print[Str] :: \"bye\" :: call\n    let task = weave worker :: 41 :: call\n    let ready = task >> await\n    let ok = not false and ((1 + 2) << 3) >= 8\n    let cfg = winspell.loop.FrameConfig :: clear = 0 :: call\n    let printed = io.print[Int] :: ready, ok :: call\n    return printed\n",
        )
        .expect("lowering should pass");

        let statements = &module.symbols[0].statements;
        assert_eq!(statements.len(), 7);

        match &statements[0].kind {
            HirStatementKind::Defer { expr } => match expr {
                HirExpr::QualifiedPhrase {
                    subject,
                    args,
                    qualifier,
                    attached,
                } => {
                    assert_eq!(qualifier, "call");
                    assert!(attached.is_empty());
                    assert!(matches!(
                        subject.as_ref(),
                        HirExpr::GenericApply { expr, type_args }
                            if type_args == &vec!["Str".to_string()]
                                && matches!(
                                    expr.as_ref(),
                                    HirExpr::MemberAccess { member, .. } if member == "print"
                                )
                    ));
                    assert_eq!(args.len(), 1);
                    assert!(matches!(
                        &args[0],
                        HirPhraseArg::Positional(expr) if expr_is_str_literal(expr, "\"bye\"")
                    ));
                }
                other => panic!("expected defer phrase expression, got {other:?}"),
            },
            other => panic!("expected defer statement, got {other:?}"),
        }

        match &statements[1].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "task");
                match value {
                    HirExpr::Unary { op, expr } => {
                        assert_eq!(*op, HirUnaryOp::Weave);
                        assert!(matches!(
                            expr.as_ref(),
                            HirExpr::QualifiedPhrase { qualifier, .. } if qualifier == "call"
                        ));
                    }
                    other => panic!("expected weave unary expression, got {other:?}"),
                }
            }
            other => panic!("expected let task statement, got {other:?}"),
        }

        match &statements[2].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "ready");
                match value {
                    HirExpr::Await { expr } => {
                        assert!(matches!(expr.as_ref(), expr if expr_is_path(expr, "task")));
                    }
                    other => panic!("expected await expression, got {other:?}"),
                }
            }
            other => panic!("expected let ready statement, got {other:?}"),
        }

        match &statements[3].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "ok");
                match value {
                    HirExpr::Binary { left, op, right } => {
                        assert_eq!(*op, HirBinaryOp::And);
                        assert!(matches!(
                            left.as_ref(),
                            HirExpr::Unary {
                                op: HirUnaryOp::Not,
                                ..
                            }
                        ));
                        match right.as_ref() {
                            HirExpr::Binary { left, op, right } => {
                                assert_eq!(*op, HirBinaryOp::GtEq);
                                match left.as_ref() {
                                    HirExpr::Binary { left, op, right } => {
                                        assert_eq!(*op, HirBinaryOp::Shl);
                                        match left.as_ref() {
                                            HirExpr::Binary { op, .. } => {
                                                assert_eq!(*op, HirBinaryOp::Add);
                                            }
                                            other => panic!(
                                                "expected additive lhs in shift expression, got {other:?}"
                                            ),
                                        }
                                        assert!(matches!(
                                            right.as_ref(),
                                            expr if expr_is_int_literal(expr, "3")
                                        ));
                                    }
                                    other => panic!(
                                        "expected shift expression in comparison lhs, got {other:?}"
                                    ),
                                }
                                assert!(matches!(
                                    right.as_ref(),
                                    expr if expr_is_int_literal(expr, "8")
                                ));
                            }
                            other => panic!(
                                "expected comparison expression on rhs of logical and, got {other:?}"
                            ),
                        }
                    }
                    other => panic!("expected structured boolean expression, got {other:?}"),
                }
            }
            other => panic!("expected let ok statement, got {other:?}"),
        }

        match &statements[4].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "cfg");
                match value {
                    HirExpr::QualifiedPhrase {
                        subject,
                        args,
                        qualifier,
                        attached,
                    } => {
                        assert_eq!(qualifier, "call");
                        assert!(attached.is_empty());
                        assert!(matches!(
                            subject.as_ref(),
                            HirExpr::MemberAccess { member, .. } if member == "FrameConfig"
                        ));
                        assert_eq!(args.len(), 1);
                        assert!(matches!(
                            &args[0],
                            HirPhraseArg::Named { name, value }
                                if name == "clear" && expr_is_int_literal(value, "0")
                        ));
                    }
                    other => panic!("expected named-arg phrase, got {other:?}"),
                }
            }
            other => panic!("expected let cfg statement, got {other:?}"),
        }

        match &statements[5].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "printed");
                match value {
                    HirExpr::QualifiedPhrase {
                        subject,
                        args,
                        qualifier,
                        attached,
                    } => {
                        assert_eq!(qualifier, "call");
                        assert!(attached.is_empty());
                        assert!(matches!(
                            subject.as_ref(),
                            HirExpr::GenericApply { expr, type_args }
                                if type_args == &vec!["Int".to_string()]
                                    && matches!(
                                        expr.as_ref(),
                                        HirExpr::MemberAccess { member, .. } if member == "print"
                                    )
                        ));
                        assert_eq!(args.len(), 2);
                    }
                    other => panic!("expected print phrase, got {other:?}"),
                }
            }
            other => panic!("expected let printed statement, got {other:?}"),
        }

        match &statements[6].kind {
            HirStatementKind::Return { value } => {
                assert!(matches!(
                    value.as_ref().expect("return should have value"),
                    expr if expr_is_path(expr, "printed")
                ));
            }
            other => panic!("expected return statement, got {other:?}"),
        }
    }

    #[test]
    fn lower_module_text_captures_access_and_range_expressions() {
        let module = lower_module_text(
            "access_demo",
            "fn main() -> Int:\n    let tuple_head = pair.0\n    let color = spec.color\n    let xs = [1, 2, 3, 4]\n    let first = xs[0]\n    let tail = xs[1..]\n    let mid = xs[1..=2]\n    let whole = xs[..]\n    let r1 = 0..3\n    let r2 = ..=3\n    return first\n",
        )
        .expect("lowering should pass");

        let statements = &module.symbols[0].statements;
        match &statements[0].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "tuple_head");
                assert!(matches!(
                    value,
                    HirExpr::MemberAccess { member, .. } if member == "0"
                ));
            }
            other => panic!("expected tuple_head let, got {other:?}"),
        }
        match &statements[1].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "color");
                assert!(matches!(
                    value,
                    HirExpr::MemberAccess { member, .. } if member == "color"
                ));
            }
            other => panic!("expected color let, got {other:?}"),
        }
        match &statements[3].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "first");
                match value {
                    HirExpr::Index { expr, index } => {
                        assert!(matches!(expr.as_ref(), expr if expr_is_path(expr, "xs")));
                        assert!(matches!(index.as_ref(), expr if expr_is_int_literal(expr, "0")));
                    }
                    other => panic!("expected index expression, got {other:?}"),
                }
            }
            other => panic!("expected first let, got {other:?}"),
        }
        match &statements[4].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "tail");
                match value {
                    HirExpr::Slice {
                        start,
                        end,
                        inclusive_end,
                        ..
                    } => {
                        assert!(!inclusive_end);
                        assert!(matches!(
                            start.as_deref(),
                            Some(expr) if expr_is_int_literal(expr, "1")
                        ));
                        assert!(end.is_none());
                    }
                    other => panic!("expected tail slice, got {other:?}"),
                }
            }
            other => panic!("expected tail let, got {other:?}"),
        }
        match &statements[5].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "mid");
                match value {
                    HirExpr::Slice {
                        start,
                        end,
                        inclusive_end,
                        ..
                    } => {
                        assert!(*inclusive_end);
                        assert!(matches!(
                            start.as_deref(),
                            Some(expr) if expr_is_int_literal(expr, "1")
                        ));
                        assert!(matches!(
                            end.as_deref(),
                            Some(expr) if expr_is_int_literal(expr, "2")
                        ));
                    }
                    other => panic!("expected mid slice, got {other:?}"),
                }
            }
            other => panic!("expected mid let, got {other:?}"),
        }
        match &statements[6].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "whole");
                assert!(matches!(
                    value,
                    HirExpr::Slice {
                        start: None,
                        end: None,
                        inclusive_end: false,
                        ..
                    }
                ));
            }
            other => panic!("expected whole let, got {other:?}"),
        }
        match &statements[7].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "r1");
                assert!(matches!(
                    value,
                    HirExpr::Range {
                        start: Some(_),
                        end: Some(_),
                        inclusive_end: false
                    }
                ));
            }
            other => panic!("expected r1 let, got {other:?}"),
        }
        match &statements[8].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "r2");
                assert!(matches!(
                    value,
                    HirExpr::Range {
                        start: None,
                        end: Some(_),
                        inclusive_end: true
                    }
                ));
            }
            other => panic!("expected r2 let, got {other:?}"),
        }
    }

    #[test]
    fn lower_module_text_captures_pair_tuple_expressions() {
        let module = lower_module_text(
            "pair_expr",
            "fn main() -> Int:\n    let pair = (left, right)\n    return pair.0\n",
        )
        .expect("lowering should pass");

        match &module.symbols[0].statements[0].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "pair");
                assert!(matches!(
                    value,
                    HirExpr::Pair {
                        left,
                        right,
                    } if matches!(
                        left.as_ref(),
                        expr if expr_is_path(expr, "left")
                    ) && matches!(
                        right.as_ref(),
                        expr if expr_is_path(expr, "right")
                    )
                ));
            }
            other => panic!("expected pair let, got {other:?}"),
        }
    }

    #[test]
    fn lower_module_text_distinguishes_generic_apply_from_indexing() {
        let module = lower_module_text(
            "generic_apply",
            "fn main() -> Int:\n    let out = std.collections.list.new[(K, V)] :: :: call\n    return 0\n",
        )
        .expect("lowering should pass");

        match &module.symbols[0].statements[0].kind {
            HirStatementKind::Let {
                value:
                    HirExpr::QualifiedPhrase {
                        subject, qualifier, ..
                    },
                ..
            } => {
                assert_eq!(qualifier, "call");
                assert!(matches!(
                    subject.as_ref(),
                    HirExpr::GenericApply { type_args, .. }
                        if type_args == &vec!["(K, V)".to_string()]
                ));
            }
            other => panic!("expected generic qualified phrase, got {other:?}"),
        }
    }

    #[test]
    fn lower_module_text_captures_structured_header_attachments() {
        let module = lower_module_text(
            "header_attach",
            "fn main() -> Int:\n    Node :: :: call\n        value = 10\n        #chain[phase=update]\n        forward :=> show_node => bump_node => show_node\n    arena: arena_nodes :> 21 <: make_node\n        #chain[phase=plan]\n        plan :=> touch_id\n        forward :=> touch_id\n    return 0\n",
        )
        .expect("lowering should pass");

        match &module.symbols[0].statements[0].kind {
            HirStatementKind::Expr {
                expr:
                    HirExpr::QualifiedPhrase {
                        qualifier,
                        attached,
                        ..
                    },
            } => {
                assert_eq!(qualifier, "call");
                assert_eq!(attached.len(), 2);
                assert!(matches!(
                    &attached[0],
                    HirHeaderAttachment::Named {
                        name,
                        value,
                        forewords,
                        ..
                    } if name == "value" && expr_is_int_literal(value, "10")
                        && forewords.is_empty()
                ));
                assert!(matches!(
                    &attached[1],
                    HirHeaderAttachment::Chain {
                        expr:
                            HirExpr::Chain {
                                style,
                                introducer,
                                steps,
                            },
                        forewords,
                        ..
                    } if style == "forward"
                        && *introducer == HirChainIntroducer::Forward
                        && chain_step_texts(steps)
                            == vec!["show_node", "bump_node", "show_node"]
                        && matches!(
                            forewords.as_slice(),
                            [HirForewordApp { name, args, .. }]
                                if name == "chain"
                                    && matches!(
                                        args.as_slice(),
                                        [HirForewordArg { name: Some(arg_name), value }]
                                            if arg_name == "phase" && value == "update"
                                    )
                        )
                ));
            }
            other => panic!("expected qualified phrase statement, got {other:?}"),
        }

        match &module.symbols[0].statements[1].kind {
            HirStatementKind::Expr {
                expr:
                    HirExpr::MemoryPhrase {
                        family,
                        constructor,
                        attached,
                        ..
                    },
            } => {
                assert_eq!(family, "arena");
                match constructor.as_ref() {
                    HirExpr::Path { segments } => {
                        assert_eq!(segments, &vec!["make_node".to_string()])
                    }
                    other => panic!("expected constructor path, got {other:?}"),
                }
                assert_eq!(attached.len(), 2);
                assert!(matches!(
                    &attached[0],
                    HirHeaderAttachment::Chain {
                        expr:
                            HirExpr::Chain {
                                style,
                                introducer,
                                steps,
                            },
                        forewords,
                        ..
                    } if style == "plan"
                        && *introducer == HirChainIntroducer::Forward
                        && chain_step_texts(steps) == vec!["touch_id"]
                        && matches!(
                            forewords.as_slice(),
                            [HirForewordApp { name, args, .. }]
                                if name == "chain"
                                    && matches!(
                                        args.as_slice(),
                                        [HirForewordArg { name: Some(arg_name), value }]
                                            if arg_name == "phase" && value == "plan"
                                    )
                        )
                ));
                assert!(matches!(
                    &attached[1],
                    HirHeaderAttachment::Chain {
                        expr:
                            HirExpr::Chain {
                                style,
                                introducer,
                                steps,
                            },
                        forewords,
                        ..
                    } if style == "forward"
                        && *introducer == HirChainIntroducer::Forward
                        && chain_step_texts(steps) == vec!["touch_id"]
                        && forewords.is_empty()
                ));
            }
            other => panic!("expected memory phrase statement, got {other:?}"),
        }
    }

    #[test]
    fn lower_module_text_captures_assignment_targets() {
        let module = lower_module_text(
            "assign_demo",
            "fn main() -> Int:\n    self.tick.value = self.tick.value + 1\n    xs[1] = 9\n    xs[i] += 3\n    return 0\n",
        )
        .expect("lowering should pass");

        let statements = &module.symbols[0].statements;
        match &statements[0].kind {
            HirStatementKind::Assign { target, .. } => match target {
                HirAssignTarget::MemberAccess { target, member } => {
                    assert_eq!(member, "value");
                    assert!(matches!(
                        target.as_ref(),
                        HirAssignTarget::MemberAccess { member, .. } if member == "tick"
                    ));
                }
                other => panic!("expected member assignment target, got {other:?}"),
            },
            other => panic!("expected first assignment statement, got {other:?}"),
        }
        match &statements[1].kind {
            HirStatementKind::Assign { target, .. } => match target {
                HirAssignTarget::Index { target, index } => {
                    assert!(matches!(
                        target.as_ref(),
                        HirAssignTarget::Name { text } if text == "xs"
                    ));
                    assert!(expr_is_int_literal(index, "1"));
                }
                other => panic!("expected indexed assignment target, got {other:?}"),
            },
            other => panic!("expected second assignment statement, got {other:?}"),
        }
        match &statements[2].kind {
            HirStatementKind::Assign { target, .. } => match target {
                HirAssignTarget::Index { target, index } => {
                    assert!(matches!(
                        target.as_ref(),
                        HirAssignTarget::Name { text } if text == "xs"
                    ));
                    assert!(expr_is_path(index, "i"));
                }
                other => panic!("expected indexed compound-assignment target, got {other:?}"),
            },
            other => panic!("expected third assignment statement, got {other:?}"),
        }
    }

    #[test]
    fn lower_module_text_captures_borrow_and_deref_expressions() {
        let module = lower_module_text(
            "borrow_demo",
            "fn main() -> Int:\n    let local_x = 1\n    let mut local_y = 2\n    let x_ref = &local_x\n    let y_mut = &mut local_y\n    let sum = *x_ref + *y_mut\n    return sum\n",
        )
        .expect("lowering should pass");

        let statements = &module.symbols[0].statements;
        match &statements[2].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "x_ref");
                assert!(matches!(
                    value,
                    HirExpr::Unary {
                        op: HirUnaryOp::BorrowRead,
                        expr
                    } if matches!(
                        expr.as_ref(),
                        expr if expr_is_path(expr, "local_x")
                    )
                ));
            }
            other => panic!("expected x_ref let, got {other:?}"),
        }

        match &statements[3].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "y_mut");
                assert!(matches!(
                    value,
                    HirExpr::Unary {
                        op: HirUnaryOp::BorrowMut,
                        expr
                    } if matches!(
                        expr.as_ref(),
                        expr if expr_is_path(expr, "local_y")
                    )
                ));
            }
            other => panic!("expected y_mut let, got {other:?}"),
        }

        match &statements[4].kind {
            HirStatementKind::Let { name, value, .. } => {
                assert_eq!(name, "sum");
                assert!(matches!(
                    value,
                    HirExpr::Binary {
                        left,
                        op: HirBinaryOp::Add,
                        right
                    } if matches!(
                        left.as_ref(),
                        HirExpr::Unary {
                            op: HirUnaryOp::Deref,
                            expr
                        } if matches!(
                            expr.as_ref(),
                            expr if expr_is_path(expr, "x_ref")
                        )
                    ) && matches!(
                        right.as_ref(),
                        HirExpr::Unary {
                            op: HirUnaryOp::Deref,
                            expr
                        } if matches!(
                            expr.as_ref(),
                            expr if expr_is_path(expr, "y_mut")
                        )
                    )
                ));
            }
            other => panic!("expected sum let, got {other:?}"),
        }
    }

    #[test]
    fn build_package_summary_collects_dependency_edges() {
        let book = lower_module_text(
            "winspell",
            "reexport winspell.window\nexport fn open() -> Int:\n    return 0\n",
        )
        .expect("lowering should pass");
        let window = lower_module_text(
            "winspell.window",
            "import std.canvas\nuse std.result.Result\nfn helper() -> Int:\n    return 0\n",
        )
        .expect("lowering should pass");

        let package = build_package_summary("winspell", vec![window, book]);
        assert_eq!(package.package_name, "winspell");
        assert_eq!(package.module_count(), 2);
        assert!(package.module("winspell.window").is_some());
        assert_eq!(package.dependency_edges.len(), 3);
        assert_eq!(package.dependency_edges[0].source_module_id, "winspell");
        assert_eq!(package.dependency_edges[0].kind, HirDirectiveKind::Reexport);
        assert_eq!(
            package.dependency_edges[1].source_module_id,
            "winspell.window"
        );
        assert_eq!(
            package.summary_surface_rows(),
            vec![
                "module=winspell:export:fn:fn open() -> Int:".to_string(),
                "module=winspell:reexport:winspell.window".to_string(),
            ]
        );
    }

    #[test]
    fn build_workspace_summary_indexes_module_paths() {
        let book = lower_module_text(
            "winspell",
            "reexport winspell.window\nexport fn open() -> Int:\n    return 0\n",
        )
        .expect("lowering should pass");
        let window = lower_module_text(
            "winspell.window",
            "import std.canvas\nfn helper() -> Int:\n    return 0\n",
        )
        .expect("lowering should pass");
        let summary = build_package_summary("winspell", vec![book, window]);
        let layout = build_package_layout(
            &summary,
            BTreeMap::from([
                (
                    "winspell".to_string(),
                    Path::new("C:/repo/winspell/src/book.arc").to_path_buf(),
                ),
                (
                    "winspell.window".to_string(),
                    Path::new("C:/repo/winspell/src/window.arc").to_path_buf(),
                ),
            ]),
            BTreeMap::from([("window".to_string(), "winspell.window".to_string())]),
        )
        .expect("layout should build");
        let package = build_workspace_package(
            Path::new("C:/repo/winspell").to_path_buf(),
            BTreeSet::from(["std".to_string()]),
            summary,
            layout,
        )
        .expect("workspace package should build");
        let workspace =
            build_workspace_summary(vec![package]).expect("workspace summary should build");

        let winspell = workspace.package("winspell").expect("package should exist");
        assert_eq!(workspace.package_count(), 1);
        assert_eq!(workspace.module_count(), 2);
        assert!(winspell.module("winspell.window").is_some());
        assert!(
            winspell
                .module_path("winspell.window")
                .expect("module path should exist")
                .ends_with("window.arc")
        );
        assert!(
            winspell
                .resolve_relative_module(&["window".to_string()])
                .is_some()
        );
    }

    #[test]
    fn resolve_workspace_builds_module_and_symbol_bindings() {
        let std_summary = build_package_summary(
            "std",
            vec![
                lower_module_text("std.io", "export fn print() -> Int:\n    return 0\n")
                    .expect("std.io should lower"),
            ],
        );
        let std_layout = build_package_layout(
            &std_summary,
            BTreeMap::from([(
                "std.io".to_string(),
                Path::new("C:/repo/std/src/io.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("std layout should build");
        let std_package = build_workspace_package(
            Path::new("C:/repo/std").to_path_buf(),
            BTreeSet::new(),
            std_summary,
            std_layout,
        )
        .expect("std package should build");

        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    "import std.io\nuse std.io as io\nuse std.io.print\nexport fn main() -> Int:\n    return 0\n",
                )
                .expect("app should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_package = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let workspace =
            build_workspace_summary(vec![app_package, std_package]).expect("workspace builds");
        let resolved = resolve_workspace(&workspace).expect("resolution should succeed");
        let app = resolved.package("app").expect("app should resolve");
        let root = app.module("app").expect("root module should resolve");
        assert_eq!(root.directives.len(), 3);
        assert_eq!(
            root.bindings
                .get("io")
                .expect("alias should resolve")
                .target,
            super::HirResolvedTarget::Module {
                package_name: "std".to_string(),
                module_id: "std.io".to_string(),
            }
        );
        assert_eq!(
            root.bindings
                .get("print")
                .expect("symbol should resolve")
                .target,
            super::HirResolvedTarget::Symbol {
                package_name: "std".to_string(),
                module_id: "std.io".to_string(),
                symbol_name: "print".to_string(),
            }
        );
    }

    #[test]
    fn resolve_workspace_supports_dependency_alias_bindings() {
        let core_summary = build_package_summary(
            "core",
            vec![
                lower_module_text("core", "export fn value() -> Int:\n    return 7\n")
                    .expect("core should lower"),
            ],
        );
        let core_layout = build_package_layout(
            &core_summary,
            BTreeMap::from([(
                "core".to_string(),
                Path::new("C:/repo/core/src/book.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("core layout should build");
        let core_package = build_workspace_package(
            Path::new("C:/repo/core").to_path_buf(),
            BTreeSet::new(),
            core_summary,
            core_layout,
        )
        .expect("core package should build");

        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    "import util\nuse util.value\nfn main() -> Int:\n    return value :: :: call\n",
                )
                .expect("app should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_package = super::build_workspace_package_with_dep_packages(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeMap::from([("util".to_string(), "core".to_string())]),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let workspace =
            build_workspace_summary(vec![app_package, core_package]).expect("workspace builds");
        let resolved = resolve_workspace(&workspace).expect("resolution should succeed");
        let root = resolved
            .package("app")
            .and_then(|package| package.module("app"))
            .expect("app module should resolve");
        assert_eq!(root.directives.len(), 2);
        assert_eq!(
            root.bindings
                .get("value")
                .expect("alias symbol should resolve")
                .target,
            super::HirResolvedTarget::Symbol {
                package_name: "core".to_string(),
                module_id: "core".to_string(),
                symbol_name: "value".to_string(),
            }
        );
    }

    #[test]
    fn resolve_workspace_reports_invalid_dependencies() {
        let core_summary = build_package_summary(
            "core",
            vec![
                lower_module_text("core", "export fn value() -> Int:\n    return 0\n")
                    .expect("core should lower"),
            ],
        );
        let core_layout = build_package_layout(
            &core_summary,
            BTreeMap::from([(
                "core".to_string(),
                Path::new("C:/repo/core/src/book.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("core layout should build");
        let core_package = build_workspace_package(
            Path::new("C:/repo/core").to_path_buf(),
            BTreeSet::new(),
            core_summary,
            core_layout,
        )
        .expect("core package should build");

        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text("app", "import core\nfn main() -> Int:\n    return 0\n")
                    .expect("app should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_package = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let workspace =
            build_workspace_summary(vec![app_package, core_package]).expect("workspace builds");
        let errors = resolve_workspace(&workspace).expect_err("resolution should fail");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("not a direct dependency"));
        assert_eq!(errors[0].source_module_id, "app");
    }

    #[test]
    fn resolve_workspace_rejects_duplicate_top_level_symbols() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    "export fn mouse_in_window(read win: Window) -> Bool:\n    return false\nexport fn mouse_in_window(read win: Window) -> Bool:\n    return true\n",
                )
                .expect("app should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_package = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let workspace = build_workspace_summary(vec![app_package]).expect("workspace builds");
        let errors = resolve_workspace(&workspace).expect_err("resolution should fail");
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .message
                .contains("duplicate symbol `mouse_in_window`")
        );
        assert_eq!(errors[0].source_module_id, "app");
    }

    #[test]
    fn resolve_workspace_rejects_duplicate_directive_bindings() {
        let std_summary = build_package_summary(
            "std",
            vec![
                lower_module_text("std.io", "export fn print() -> Int:\n    return 0\n")
                    .expect("std.io should lower"),
                lower_module_text("std.text", "export fn len() -> Int:\n    return 0\n")
                    .expect("std.text should lower"),
            ],
        );
        let std_layout = build_package_layout(
            &std_summary,
            BTreeMap::from([
                (
                    "std.io".to_string(),
                    Path::new("C:/repo/std/src/io.arc").to_path_buf(),
                ),
                (
                    "std.text".to_string(),
                    Path::new("C:/repo/std/src/text.arc").to_path_buf(),
                ),
            ]),
            BTreeMap::new(),
        )
        .expect("std layout should build");
        let std_package = build_workspace_package(
            Path::new("C:/repo/std").to_path_buf(),
            BTreeSet::new(),
            std_summary,
            std_layout,
        )
        .expect("std package should build");

        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    "use std.io as io\nuse std.text as io\nfn main() -> Int:\n    return 0\n",
                )
                .expect("app should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_package = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::from(["std".to_string()]),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let workspace =
            build_workspace_summary(vec![app_package, std_package]).expect("workspace builds");
        let errors = resolve_workspace(&workspace).expect_err("resolution should fail");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("duplicate binding `io`"));
        assert_eq!(errors[0].source_module_id, "app");
    }

    #[test]
    fn resolve_workspace_allows_duplicate_directives_when_target_matches() {
        let std_summary = build_package_summary(
            "std",
            vec![
                lower_module_text("std.io", "export fn print() -> Int:\n    return 0\n")
                    .expect("std.io should lower"),
            ],
        );
        let std_layout = build_package_layout(
            &std_summary,
            BTreeMap::from([(
                "std.io".to_string(),
                Path::new("C:/repo/std/src/io.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("std layout should build");
        let std_package = build_workspace_package(
            Path::new("C:/repo/std").to_path_buf(),
            BTreeSet::new(),
            std_summary,
            std_layout,
        )
        .expect("std package should build");

        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    "import std.io\nuse std.io as io\nfn main() -> Int:\n    return 0\n",
                )
                .expect("app should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_package = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::from(["std".to_string()]),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let workspace =
            build_workspace_summary(vec![app_package, std_package]).expect("workspace builds");
        let resolved = resolve_workspace(&workspace).expect("resolution should succeed");
        let app_module = resolved
            .package("app")
            .and_then(|pkg| pkg.module("app"))
            .expect("app module should resolve");
        assert_eq!(
            app_module
                .bindings
                .get("io")
                .expect("alias should resolve")
                .local_name,
            "io"
        );
    }

    #[test]
    fn lookup_method_candidates_ignore_receiver_shaped_free_functions() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    "import app.types\nuse app.types.Counter\nfn main() -> Int:\n    let counter = Counter :: value = 1 :: call\n    return counter :: :: tap\n",
                )
                .expect("app should lower"),
                lower_module_text("app.types", "export record Counter:\n    value: Int\n")
                    .expect("types should lower"),
                lower_module_text(
                    "app.helpers",
                    "import app.types\nuse app.types.Counter\nfn tap(read self: Counter) -> Int:\n    return self.value + 1\n",
                )
                .expect("helpers should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([
                (
                    "app".to_string(),
                    Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
                ),
                (
                    "app.types".to_string(),
                    Path::new("C:/repo/app/src/types.arc").to_path_buf(),
                ),
                (
                    "app.helpers".to_string(),
                    Path::new("C:/repo/app/src/helpers.arc").to_path_buf(),
                ),
            ]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_package = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let workspace = build_workspace_summary(vec![app_package]).expect("workspace builds");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let resolved_module = resolved
            .package("app")
            .and_then(|package| package.module("app"))
            .expect("resolved app module should exist");

        let candidates =
            lookup_method_candidates_for_type(&workspace, resolved_module, "Counter", "tap");
        assert!(
            candidates.is_empty(),
            "receiver-shaped free function should not appear as method candidate"
        );
    }

    #[test]
    fn lookup_symbol_path_hides_private_dependency_symbols() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    "import core\nfn main() -> Int:\n    return core.shared :: :: call\n",
                )
                .expect("app should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_package = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::from(["core".to_string()]),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let core_summary =
            build_package_summary(
                "core",
                vec![lower_module_text(
                "core",
                "export fn shared() -> Int:\n    return 1\nfn hidden() -> Int:\n    return 0\n",
            )
            .expect("core should lower")],
            );
        let core_layout = build_package_layout(
            &core_summary,
            BTreeMap::from([(
                "core".to_string(),
                Path::new("C:/repo/core/src/book.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("core layout should build");
        let core_package = build_workspace_package(
            Path::new("C:/repo/core").to_path_buf(),
            BTreeSet::new(),
            core_summary,
            core_layout,
        )
        .expect("core package should build");

        let workspace =
            build_workspace_summary(vec![app_package, core_package]).expect("workspace builds");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let resolved_module = resolved
            .package("app")
            .and_then(|package| package.module("app"))
            .expect("resolved app module should exist");

        assert!(
            lookup_symbol_path(
                &workspace,
                resolved_module,
                &["core".to_string(), "shared".to_string()]
            )
            .is_some()
        );
        assert!(
            lookup_symbol_path(
                &workspace,
                resolved_module,
                &["core".to_string(), "hidden".to_string()]
            )
            .is_none()
        );
    }

    #[test]
    fn resolve_workspace_rejects_private_dependency_symbol_use() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    "use core.hidden\nfn main() -> Int:\n    return hidden :: :: call\n",
                )
                .expect("app should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_package = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::from(["core".to_string()]),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let core_summary =
            build_package_summary(
                "core",
                vec![lower_module_text(
                "core",
                "export fn shared() -> Int:\n    return 1\nfn hidden() -> Int:\n    return 0\n",
            )
            .expect("core should lower")],
            );
        let core_layout = build_package_layout(
            &core_summary,
            BTreeMap::from([(
                "core".to_string(),
                Path::new("C:/repo/core/src/book.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("core layout should build");
        let core_package = build_workspace_package(
            Path::new("C:/repo/core").to_path_buf(),
            BTreeSet::new(),
            core_summary,
            core_layout,
        )
        .expect("core package should build");

        let workspace =
            build_workspace_summary(vec![app_package, core_package]).expect("workspace builds");
        let errors = resolve_workspace(&workspace).expect_err("resolution should fail");
        assert!(
            errors[0]
                .message
                .contains("unresolved symbol `hidden` in module `core`"),
            "unexpected message: {}",
            errors[0].message
        );
    }

    #[test]
    fn lookup_method_candidates_allow_public_dependency_impl_methods_without_export_keyword() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    "import core.types\nuse core.types.Counter\nfn main() -> Int:\n    let counter = Counter :: value = 1 :: call\n    return counter :: :: tap\n",
                )
                .expect("app should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_package = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::from(["core".to_string()]),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let core_summary = build_package_summary(
            "core",
            vec![
                lower_module_text("core", "").expect("core root should lower"),
                lower_module_text("core.types", "export record Counter:\n    value: Int\n")
                    .expect("types should lower"),
                lower_module_text(
                    "core.helpers",
                    "import core.types\nuse core.types.Counter\nimpl Counter:\n    fn tap(read self: Counter) -> Int:\n        return self.value + 1\n",
                )
                .expect("helpers should lower"),
            ],
        );
        let core_layout = build_package_layout(
            &core_summary,
            BTreeMap::from([
                (
                    "core".to_string(),
                    Path::new("C:/repo/core/src/book.arc").to_path_buf(),
                ),
                (
                    "core.types".to_string(),
                    Path::new("C:/repo/core/src/types.arc").to_path_buf(),
                ),
                (
                    "core.helpers".to_string(),
                    Path::new("C:/repo/core/src/helpers.arc").to_path_buf(),
                ),
            ]),
            BTreeMap::new(),
        )
        .expect("core layout should build");
        let core_package = build_workspace_package(
            Path::new("C:/repo/core").to_path_buf(),
            BTreeSet::new(),
            core_summary,
            core_layout,
        )
        .expect("core package should build");

        let workspace =
            build_workspace_summary(vec![app_package, core_package]).expect("workspace builds");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let resolved_module = resolved
            .package("app")
            .and_then(|package| package.module("app"))
            .expect("resolved app module should exist");

        let candidates =
            lookup_method_candidates_for_type(&workspace, resolved_module, "Counter", "tap");
        assert_eq!(candidates.len(), 1);
    }

    #[test]
    fn lookup_method_candidates_allow_dependency_alias_impl_methods() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    "import util.types\nuse util.types.Counter\nfn main() -> Int:\n    let counter = Counter :: value = 1 :: call\n    return counter :: :: tap\n",
                )
                .expect("app should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_package = super::build_workspace_package_with_dep_packages(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeMap::from([("util".to_string(), "core".to_string())]),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let core_summary = build_package_summary(
            "core",
            vec![
                lower_module_text("core", "").expect("core root should lower"),
                lower_module_text("core.types", "export record Counter:\n    value: Int\n")
                    .expect("types should lower"),
                lower_module_text(
                    "core.helpers",
                    "import core.types\nuse core.types.Counter\nimpl Counter:\n    fn tap(read self: Counter) -> Int:\n        return self.value + 1\n",
                )
                .expect("helpers should lower"),
            ],
        );
        let core_layout = build_package_layout(
            &core_summary,
            BTreeMap::from([
                (
                    "core".to_string(),
                    Path::new("C:/repo/core/src/book.arc").to_path_buf(),
                ),
                (
                    "core.types".to_string(),
                    Path::new("C:/repo/core/src/types.arc").to_path_buf(),
                ),
                (
                    "core.helpers".to_string(),
                    Path::new("C:/repo/core/src/helpers.arc").to_path_buf(),
                ),
            ]),
            BTreeMap::new(),
        )
        .expect("core layout should build");
        let core_package = build_workspace_package(
            Path::new("C:/repo/core").to_path_buf(),
            BTreeSet::new(),
            core_summary,
            core_layout,
        )
        .expect("core package should build");

        let workspace =
            build_workspace_summary(vec![app_package, core_package]).expect("workspace builds");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let resolved_module = resolved
            .package("app")
            .and_then(|package| package.module("app"))
            .expect("resolved app module should exist");

        let candidates =
            lookup_method_candidates_for_type(&workspace, resolved_module, "Counter", "tap");
        assert_eq!(candidates.len(), 1);
    }

    #[test]
    fn lookup_method_candidates_ignore_private_dependency_receiver_types() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text("app", "fn main() -> Int:\n    return 0\n")
                    .expect("app should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_package = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::from(["core".to_string()]),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let core_summary = build_package_summary(
            "core",
            vec![lower_module_text(
                "core",
                "record Hidden:\n    value: Int\nimpl Hidden:\n    fn tap(read self: Hidden) -> Int:\n        return self.value + 1\n",
            )
            .expect("core should lower")],
        );
        let core_layout = build_package_layout(
            &core_summary,
            BTreeMap::from([(
                "core".to_string(),
                Path::new("C:/repo/core/src/book.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("core layout should build");
        let core_package = build_workspace_package(
            Path::new("C:/repo/core").to_path_buf(),
            BTreeSet::new(),
            core_summary,
            core_layout,
        )
        .expect("core package should build");

        let workspace =
            build_workspace_summary(vec![app_package, core_package]).expect("workspace builds");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let resolved_module = resolved
            .package("app")
            .and_then(|package| package.module("app"))
            .expect("resolved app module should exist");

        let candidates =
            lookup_method_candidates_for_type(&workspace, resolved_module, "core.Hidden", "tap");
        assert!(
            candidates.is_empty(),
            "private dependency receiver type should not contribute method candidates"
        );
    }

    #[test]
    fn derive_source_module_path_handles_root_and_nested_modules() {
        let src_dir = Path::new("C:/repo/winspell/src");
        let root = derive_source_module_path(
            "winspell",
            "book.arc",
            src_dir,
            Path::new("C:/repo/winspell/src/book.arc"),
        )
        .expect("root module should resolve");
        assert_eq!(root.relative_segments, Vec::<String>::new());
        assert_eq!(root.module_id, "winspell");

        let nested = derive_source_module_path(
            "winspell",
            "book.arc",
            src_dir,
            Path::new("C:/repo/winspell/src/render/window.arc"),
        )
        .expect("nested module should resolve");
        assert_eq!(
            nested.relative_segments,
            vec!["render".to_string(), "window".to_string()]
        );
        assert_eq!(nested.module_id, "winspell.render.window");
    }
}
