pub mod freeze;
mod lookup;
mod render;
mod signature;
pub mod type_surface;

pub use lookup::{
    current_workspace_package_for_module, impl_target_is_public_from_package,
    lookup_method_candidates_for_hir_type, lookup_symbol_path,
    visible_method_package_names_for_module, visible_package_root_for_module,
};
pub(crate) use lookup::{
    lookup_symbol_path_in_module_context, visible_symbol_refs_in_module_for_package,
};
pub use render::{render_expr_fingerprint, render_symbol_fingerprint};
pub use signature::render_symbol_signature;
pub use type_surface::{
    HirLifetime, HirPath, HirPredicate, HirProjection, HirSurfaceRefs, HirTraitRef, HirType,
    HirTypeBindingId, HirTypeBindingScope, HirTypeKind, HirTypeSubstitutions, HirWhereClause,
    collect_hir_type_refs, collect_hir_where_clause_refs, hir_strip_reference_type,
    hir_type_app_args, hir_type_base_path, hir_type_is_boundary_safe, hir_type_matches,
    parse_hir_type, render_hir_trait_ref, render_hir_type, render_hir_where_clause,
    substitute_hir_type, validate_hir_tuple_contract,
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
    Object,
    Owner,
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
            Self::Object => "obj",
            Self::Owner => "create",
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
    pub where_clause: Option<HirWhereClause>,
    pub params: Vec<HirParam>,
    pub return_type: Option<HirType>,
    pub behavior_attrs: Vec<HirBehaviorAttr>,
    pub opaque_policy: Option<HirOpaqueTypePolicy>,
    pub availability: Vec<HirAvailabilityAttachment>,
    pub forewords: Vec<HirForewordApp>,
    pub intrinsic_impl: Option<String>,
    pub body: HirSymbolBody,
    pub statements: Vec<HirStatement>,
    pub cleanup_footers: Vec<HirCleanupFooter>,
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
    pub ty: HirType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirField {
    pub name: String,
    pub ty: HirType,
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
pub enum HirCleanupFooterKind {
    Cleanup,
}

impl HirCleanupFooterKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Cleanup => "cleanup",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirCleanupFooter {
    pub kind: HirCleanupFooterKind,
    pub subject: String,
    pub handler_path: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirAvailabilityAttachment {
    pub path: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirOwnerObject {
    pub type_path: Vec<String>,
    pub local_name: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirOwnerExit {
    pub name: String,
    pub condition: HirExpr,
    pub holds: Vec<String>,
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
        type_args: Vec<HirType>,
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
    pub availability: Vec<HirAvailabilityAttachment>,
    pub forewords: Vec<HirForewordApp>,
    pub cleanup_footers: Vec<HirCleanupFooter>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirEnumVariant {
    pub name: String,
    pub payload: Option<HirType>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirTraitAssocType {
    pub name: String,
    pub default_ty: Option<HirType>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirSymbolBody {
    None,
    Record {
        fields: Vec<HirField>,
    },
    Object {
        fields: Vec<HirField>,
        methods: Vec<HirSymbol>,
    },
    Enum {
        variants: Vec<HirEnumVariant>,
    },
    Owner {
        objects: Vec<HirOwnerObject>,
        exits: Vec<HirOwnerExit>,
    },
    Trait {
        assoc_types: Vec<HirTraitAssocType>,
        methods: Vec<HirSymbol>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirImplDecl {
    pub type_params: Vec<String>,
    pub trait_path: Option<HirTraitRef>,
    pub target_type: HirType,
    pub assoc_types: Vec<HirImplAssocTypeBinding>,
    pub methods: Vec<HirSymbol>,
    pub body_entries: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirImplAssocTypeBinding {
    pub name: String,
    pub value_ty: Option<HirType>,
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
                        render::encode_surface_text(&symbol.api_signature_text())
                    )
                }),
        );
        rows.sort();
        rows
    }

    pub fn hir_fingerprint_rows(&self) -> Vec<String> {
        let mut rows = Vec::new();
        rows.extend(
            self.directives
                .iter()
                .map(render::render_directive_fingerprint),
        );
        rows.extend(
            self.lang_items
                .iter()
                .map(render::render_lang_item_fingerprint),
        );
        rows.extend(self.symbols.iter().map(render::render_symbol_fingerprint));
        rows.extend(self.impls.iter().map(render::render_impl_fingerprint));
        rows
    }
}

impl HirSymbol {
    fn api_signature_text(&self) -> String {
        signature::render_symbol_signature(self)
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
            render::quote_fingerprint_text(&self.package_name)
        )];
        rows.extend(
            self.dependency_edges
                .iter()
                .map(render::render_dependency_edge_fingerprint),
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
    pub package_id: String,
    pub root_dir: PathBuf,
    pub direct_deps: BTreeSet<String>,
    pub direct_dep_packages: BTreeMap<String, String>,
    pub direct_dep_ids: BTreeMap<String, String>,
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

    pub fn dependency_package_id(&self, visible_name: &str) -> Option<&str> {
        self.direct_dep_ids.get(visible_name).map(String::as_str)
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
    package_names: BTreeMap<String, Vec<String>>,
    module_packages: BTreeMap<String, Vec<String>>,
}

impl HirWorkspaceSummary {
    pub fn package(&self, name: &str) -> Option<&HirWorkspacePackage> {
        let ids = self.package_names.get(name)?;
        let [package_id] = ids.as_slice() else {
            return None;
        };
        self.packages.get(package_id)
    }

    pub fn package_by_id(&self, package_id: &str) -> Option<&HirWorkspacePackage> {
        self.packages.get(package_id)
    }

    pub fn package_id_for_module(&self, module_id: &str) -> Option<&str> {
        let package_ids = self.module_packages.get(module_id)?;
        let [package_id] = package_ids.as_slice() else {
            return None;
        };
        Some(package_id.as_str())
    }

    pub fn package_for_module(&self, module_id: &str) -> Option<&HirWorkspacePackage> {
        self.package_id_for_module(module_id)
            .and_then(|package_id| self.package_by_id(package_id))
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
        package_id: String,
        package_name: String,
        module_id: String,
    },
    Symbol {
        package_id: String,
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
    pub package_id: String,
    pub module_id: String,
    pub bindings: BTreeMap<String, HirResolvedBinding>,
    pub directives: Vec<HirResolvedDirective>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirResolvedPackage {
    pub package_id: String,
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
    package_names: BTreeMap<String, Vec<String>>,
}

impl HirResolvedWorkspace {
    pub fn package(&self, package_name: &str) -> Option<&HirResolvedPackage> {
        let ids = self.package_names.get(package_name)?;
        let [package_id] = ids.as_slice() else {
            return None;
        };
        self.packages.get(package_id)
    }

    pub fn package_by_id(&self, package_id: &str) -> Option<&HirResolvedPackage> {
        self.packages.get(package_id)
    }
}

#[derive(Clone, Debug)]
pub struct HirMethodCandidate<'a> {
    pub package_id: &'a str,
    pub package_name: &'a str,
    pub module_id: &'a str,
    pub symbol: &'a HirSymbol,
    pub declared_receiver_hir: HirType,
    pub routine_key: String,
    pub trait_path: Option<Vec<String>>,
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

pub fn routine_key_for_object_method(
    module_id: &str,
    symbol_index: usize,
    method_index: usize,
) -> String {
    format!("{module_id}#obj-{symbol_index}-method-{method_index}")
}

fn canonical_ambient_type_root(path: &[String]) -> Option<&'static str> {
    match path {
        [name] => match name.as_str() {
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
        },
        _ => None,
    }
}

fn canonicalize_method_lookup_base_in_module(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    path: &[String],
) -> Vec<String> {
    lookup_symbol_path_in_module_context(workspace, package, module, path)
        .map(|symbol_ref| {
            let mut canonical = symbol_ref
                .module_id
                .split('.')
                .map(str::to_string)
                .collect::<Vec<_>>();
            canonical.push(symbol_ref.symbol.name.clone());
            canonical
        })
        .or_else(|| {
            canonical_ambient_type_root(path)
                .map(|canonical| canonical.split('.').map(str::to_string).collect::<Vec<_>>())
        })
        .unwrap_or_else(|| path.to_vec())
}

fn canonicalize_hir_path_in_module(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    path: &HirPath,
) -> HirPath {
    HirPath {
        segments: canonicalize_method_lookup_base_in_module(
            workspace,
            package,
            module,
            &path.segments,
        ),
        span: path.span,
    }
}

fn canonicalize_hir_trait_ref_in_module(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    trait_ref: &HirTraitRef,
) -> HirTraitRef {
    HirTraitRef {
        path: canonicalize_hir_path_in_module(workspace, package, module, &trait_ref.path),
        args: trait_ref
            .args
            .iter()
            .map(|arg| canonicalize_hir_type_in_module(workspace, package, module, arg))
            .collect(),
        span: trait_ref.span,
    }
}

pub fn canonicalize_hir_type_in_module(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    ty: &HirType,
) -> HirType {
    HirType {
        kind: match &ty.kind {
            HirTypeKind::Path(path) => HirTypeKind::Path(canonicalize_hir_path_in_module(
                workspace, package, module, path,
            )),
            HirTypeKind::Apply { base, args } => HirTypeKind::Apply {
                base: canonicalize_hir_path_in_module(workspace, package, module, base),
                args: args
                    .iter()
                    .map(|arg| canonicalize_hir_type_in_module(workspace, package, module, arg))
                    .collect(),
            },
            HirTypeKind::Ref {
                lifetime,
                mutable,
                inner,
            } => HirTypeKind::Ref {
                lifetime: lifetime.clone(),
                mutable: *mutable,
                inner: Box::new(canonicalize_hir_type_in_module(
                    workspace, package, module, inner,
                )),
            },
            HirTypeKind::Tuple(items) => HirTypeKind::Tuple(
                items
                    .iter()
                    .map(|item| canonicalize_hir_type_in_module(workspace, package, module, item))
                    .collect(),
            ),
            HirTypeKind::Projection(projection) => HirTypeKind::Projection(HirProjection {
                trait_ref: canonicalize_hir_trait_ref_in_module(
                    workspace,
                    package,
                    module,
                    &projection.trait_ref,
                ),
                assoc: projection.assoc.clone(),
                span: projection.span,
            }),
        },
        span: ty.span,
    }
}

pub trait HirLocalTypeLookup {
    fn contains_local(&self, name: &str) -> bool;
    fn type_of(&self, _name: &str) -> Option<&HirType> {
        None
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HirResolvedSymbolRef<'a> {
    pub package_id: &'a str,
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

fn hir_path_matches(path: &HirPath, expected: &[&str]) -> bool {
    path.segments
        .iter()
        .map(String::as_str)
        .eq(expected.iter().copied())
}

fn hir_path_matches_any(path: &HirPath, expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|candidate| hir_path_matches(path, candidate))
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

fn placeholder_binding_scope_for_type(ty: &HirType) -> HirTypeBindingScope {
    let mut scope = HirTypeBindingScope::default();
    for path in collect_hir_type_refs(ty).paths {
        if path.len() == 1 {
            let name = &path[0];
            if builtin_type_info(name).is_none() {
                scope.insert(name.clone());
            }
        }
    }
    scope
}

fn substitute_type_params_hir(
    ty: &HirType,
    bindings: &HirTypeBindingScope,
    substitutions: &HirTypeSubstitutions,
) -> HirType {
    substitute_hir_type(ty, bindings, substitutions)
}

fn builtin_hir_type(name: &str) -> HirType {
    HirType {
        kind: HirTypeKind::Path(HirPath {
            segments: vec![name.to_string()],
            span: Span::default(),
        }),
        span: Span::default(),
    }
}

fn ambient_apply_hir_type(base: &[&str], args: Vec<HirType>) -> HirType {
    let base = HirPath {
        segments: base.iter().map(|segment| segment.to_string()).collect(),
        span: Span::default(),
    };
    HirType {
        kind: HirTypeKind::Apply { base, args },
        span: Span::default(),
    }
}

fn build_symbol_result_type(
    module_id: &str,
    symbol: &HirSymbol,
    explicit_args: &[HirType],
) -> HirType {
    let base = HirPath {
        segments: module_id
            .split('.')
            .map(str::to_string)
            .chain(std::iter::once(symbol.name.clone()))
            .collect(),
        span: symbol.span,
    };
    let args = if explicit_args.is_empty() {
        symbol
            .type_params
            .iter()
            .map(|param| HirType {
                kind: HirTypeKind::Path(HirPath {
                    segments: vec![param.clone()],
                    span: symbol.span,
                }),
                span: symbol.span,
            })
            .collect::<Vec<_>>()
    } else {
        explicit_args.to_vec()
    };
    if args.is_empty() {
        HirType {
            kind: HirTypeKind::Path(base),
            span: symbol.span,
        }
    } else {
        HirType {
            kind: HirTypeKind::Apply { base, args },
            span: symbol.span,
        }
    }
}

fn extract_expr_generic_hir_type_args(expr: &HirExpr) -> Vec<HirType> {
    match expr {
        HirExpr::GenericApply { expr, type_args } => {
            let mut inherited = extract_expr_generic_hir_type_args(expr);
            inherited.extend(type_args.iter().cloned());
            inherited
        }
        _ => Vec::new(),
    }
}

fn symbol_return_type(
    workspace: &HirWorkspaceSummary,
    symbol_ref: HirResolvedSymbolRef<'_>,
) -> Option<HirType> {
    let package = workspace.package_by_id(symbol_ref.package_id)?;
    let module = package.module(symbol_ref.module_id)?;
    if let Some(return_type) = &symbol_ref.symbol.return_type {
        return Some(canonicalize_hir_type_in_module(
            workspace,
            package,
            module,
            return_type,
        ));
    }
    matches!(
        symbol_ref.symbol.kind,
        HirSymbolKind::Record
            | HirSymbolKind::Object
            | HirSymbolKind::Enum
            | HirSymbolKind::OpaqueType
    )
    .then(|| build_symbol_result_type(symbol_ref.module_id, symbol_ref.symbol, &[]))
}

fn symbol_call_return_type(
    workspace: &HirWorkspaceSummary,
    symbol_ref: HirResolvedSymbolRef<'_>,
    explicit_args: &[HirType],
) -> Option<HirType> {
    if matches!(
        symbol_ref.symbol.kind,
        HirSymbolKind::Record
            | HirSymbolKind::Object
            | HirSymbolKind::Enum
            | HirSymbolKind::OpaqueType
    ) {
        return Some(build_symbol_result_type(
            symbol_ref.module_id,
            symbol_ref.symbol,
            explicit_args,
        ));
    }
    let package = workspace.package_by_id(symbol_ref.package_id)?;
    let module = package.module(symbol_ref.module_id)?;
    let return_type = symbol_ref.symbol.return_type.as_ref()?;
    let canonical_return = canonicalize_hir_type_in_module(workspace, package, module, return_type);
    if explicit_args.is_empty() || symbol_ref.symbol.type_params.is_empty() {
        return Some(canonical_return);
    }
    let bindings = HirTypeBindingScope::from_names(symbol_ref.symbol.type_params.clone());
    let substitutions = symbol_ref
        .symbol
        .type_params
        .iter()
        .zip(explicit_args.iter())
        .filter_map(|(param, actual)| bindings.binding_id(param).map(|id| (id, actual.clone())))
        .collect::<HirTypeSubstitutions>();
    Some(substitute_type_params_hir(
        &canonical_return,
        &bindings,
        &substitutions,
    ))
}

fn infer_call_target_return_hir_type<L: HirLocalTypeLookup>(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    locals: &L,
    subject: &HirExpr,
) -> Option<HirType> {
    let path = flatten_callable_expr_path(subject)?;
    let generic_args = extract_expr_generic_hir_type_args(subject)
        .into_iter()
        .map(|arg| {
            let package = current_workspace_package_for_module(workspace, resolved_module)?;
            let module = package.module(&resolved_module.module_id)?;
            Some(canonicalize_hir_type_in_module(
                workspace, package, module, &arg,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    if let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, &path) {
        return symbol_call_return_type(workspace, symbol_ref, &generic_args);
    }
    if path.len() >= 2 {
        let enum_path = path[..path.len() - 1].to_vec();
        if let Some(enum_ref) = lookup_symbol_path(workspace, resolved_module, &enum_path)
            && matches!(enum_ref.symbol.kind, HirSymbolKind::Enum)
        {
            return symbol_call_return_type(workspace, enum_ref, &generic_args);
        }
    }
    let _ = locals;
    None
}

fn infer_member_access_hir_type<L: HirLocalTypeLookup>(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    locals: &L,
    expr: &HirExpr,
    member: &str,
) -> Option<HirType> {
    let base_ty = infer_receiver_expr_type(workspace, resolved_module, locals, expr)?;
    let base_path = hir_type_base_path(hir_strip_reference_type(&base_ty))?;
    let symbol_ref = lookup_symbol_path(workspace, resolved_module, &base_path)?;
    let field = match &symbol_ref.symbol.body {
        HirSymbolBody::Record { fields } | HirSymbolBody::Object { fields, .. } => {
            fields.iter().find(|field| field.name == member)?
        }
        _ => return None,
    };
    let declared_receiver = build_symbol_result_type(symbol_ref.module_id, symbol_ref.symbol, &[]);
    let package = workspace.package_by_id(symbol_ref.package_id)?;
    let module = package.module(symbol_ref.module_id)?;
    let canonical_declared =
        canonicalize_hir_type_in_module(workspace, package, module, &declared_receiver);
    let canonical_actual = canonicalize_hir_type_in_module(
        workspace,
        package,
        module,
        hir_strip_reference_type(&base_ty),
    );
    let bindings = HirTypeBindingScope::from_names(symbol_ref.symbol.type_params.clone());
    let mut substitutions = HirTypeSubstitutions::new();
    if !hir_type_matches(
        &canonical_declared,
        &canonical_actual,
        &bindings,
        &mut substitutions,
    ) {
        return Some(field.ty.clone());
    }
    Some(substitute_type_params_hir(
        &field.ty,
        &bindings,
        &substitutions,
    ))
}

fn infer_index_hir_type<L: HirLocalTypeLookup>(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    locals: &L,
    expr: &HirExpr,
) -> Option<HirType> {
    let base_ty = infer_receiver_expr_type(workspace, resolved_module, locals, expr)?;
    match &hir_strip_reference_type(&base_ty).kind {
        HirTypeKind::Apply { base, args }
            if hir_path_matches_any(
                base,
                &[
                    &["List"],
                    &["Array"],
                    &["std", "collections", "list", "List"],
                    &["std", "collections", "array", "Array"],
                ],
            ) =>
        {
            args.first().cloned()
        }
        HirTypeKind::Apply { base, args }
            if hir_path_matches_any(base, &[&["Map"], &["std", "collections", "map", "Map"]]) =>
        {
            args.get(1).cloned()
        }
        _ => None,
    }
}

fn infer_slice_hir_type<L: HirLocalTypeLookup>(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    locals: &L,
    expr: &HirExpr,
) -> Option<HirType> {
    let base_ty = infer_receiver_expr_type(workspace, resolved_module, locals, expr)?;
    match &hir_strip_reference_type(&base_ty).kind {
        HirTypeKind::Apply { base, args }
            if hir_path_matches_any(
                base,
                &[&["List"], &["std", "collections", "list", "List"]],
            ) =>
        {
            Some(ambient_apply_hir_type(
                &["List"],
                vec![
                    args.first()
                        .cloned()
                        .unwrap_or_else(|| builtin_hir_type("_")),
                ],
            ))
        }
        HirTypeKind::Apply { base, args }
            if hir_path_matches_any(
                base,
                &[&["Array"], &["std", "collections", "array", "Array"]],
            ) =>
        {
            Some(ambient_apply_hir_type(
                &["Array"],
                vec![
                    args.first()
                        .cloned()
                        .unwrap_or_else(|| builtin_hir_type("_")),
                ],
            ))
        }
        HirTypeKind::Apply { .. } => Some(base_ty),
        _ => Some(base_ty),
    }
}

pub fn infer_receiver_expr_type<L: HirLocalTypeLookup>(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    locals: &L,
    expr: &HirExpr,
) -> Option<HirType> {
    match expr {
        HirExpr::BoolLiteral { .. } => Some(builtin_hir_type("Bool")),
        HirExpr::IntLiteral { .. } => Some(builtin_hir_type("Int")),
        HirExpr::StrLiteral { .. } => Some(builtin_hir_type("Str")),
        HirExpr::CollectionLiteral { .. } => Some(ambient_apply_hir_type(
            &["List"],
            vec![builtin_hir_type("_")],
        )),
        HirExpr::Range { .. } => Some(builtin_hir_type("RangeInt")),
        HirExpr::Path { segments }
            if segments.len() == 1 && locals.contains_local(&segments[0]) =>
        {
            locals.type_of(&segments[0]).cloned()
        }
        HirExpr::Path { segments } => {
            let symbol_ref = lookup_symbol_path(workspace, resolved_module, segments)?;
            symbol_return_type(workspace, symbol_ref)
        }
        HirExpr::Unary { op, expr }
            if matches!(op, HirUnaryOp::BorrowRead | HirUnaryOp::BorrowMut) =>
        {
            infer_receiver_expr_type(workspace, resolved_module, locals, expr).map(|inner| {
                HirType {
                    kind: HirTypeKind::Ref {
                        lifetime: None,
                        mutable: matches!(op, HirUnaryOp::BorrowMut),
                        inner: Box::new(inner),
                    },
                    span: Span::default(),
                }
            })
        }
        HirExpr::Unary {
            op: HirUnaryOp::Weave,
            expr,
        } => infer_receiver_expr_type(workspace, resolved_module, locals, expr)
            .map(|inner| ambient_apply_hir_type(&["std", "concurrent", "Task"], vec![inner])),
        HirExpr::Unary {
            op: HirUnaryOp::Split,
            expr,
        } => infer_receiver_expr_type(workspace, resolved_module, locals, expr)
            .map(|inner| ambient_apply_hir_type(&["std", "concurrent", "Thread"], vec![inner])),
        HirExpr::Unary {
            op: HirUnaryOp::Deref,
            expr,
        } => infer_receiver_expr_type(workspace, resolved_module, locals, expr).map(|ty| {
            if let HirTypeKind::Ref { inner, .. } = ty.kind {
                *inner
            } else {
                ty
            }
        }),
        HirExpr::GenericApply { expr, .. } => {
            infer_receiver_expr_type(workspace, resolved_module, locals, expr)
        }
        HirExpr::QualifiedPhrase {
            subject, qualifier, ..
        } if qualifier == "call" => {
            infer_call_target_return_hir_type(workspace, resolved_module, locals, subject)
        }
        HirExpr::QualifiedPhrase { qualifier, .. } if qualifier.contains('.') => {
            let path = split_simple_path(qualifier)?;
            lookup_symbol_path(workspace, resolved_module, &path)
                .and_then(|symbol_ref| symbol_return_type(workspace, symbol_ref))
        }
        HirExpr::QualifiedPhrase {
            subject, qualifier, ..
        } if split_simple_path(qualifier).is_some() => {
            let subject_ty = infer_receiver_expr_type(workspace, resolved_module, locals, subject)?;
            let candidates = lookup_method_candidates_for_hir_type(
                workspace,
                resolved_module,
                &subject_ty,
                qualifier,
            );
            match candidates.as_slice() {
                [candidate] => candidate.symbol.return_type.as_ref().map(|return_type| {
                    let bindings =
                        placeholder_binding_scope_for_type(&candidate.declared_receiver_hir);
                    let mut substitutions = HirTypeSubstitutions::new();
                    let package = workspace
                        .package(candidate.package_name)
                        .expect("candidate package");
                    let module = package
                        .module(candidate.module_id)
                        .expect("candidate module");
                    let canonical_declared = canonicalize_hir_type_in_module(
                        workspace,
                        package,
                        module,
                        &candidate.declared_receiver_hir,
                    );
                    let canonical_actual = canonicalize_hir_type_in_module(
                        workspace,
                        package,
                        module,
                        hir_strip_reference_type(&subject_ty),
                    );
                    let _ = hir_type_matches(
                        &canonical_declared,
                        &canonical_actual,
                        &bindings,
                        &mut substitutions,
                    );
                    substitute_type_params_hir(return_type, &bindings, &substitutions)
                }),
                _ => None,
            }
        }
        HirExpr::MemberAccess { expr, member } => {
            infer_member_access_hir_type(workspace, resolved_module, locals, expr, member)
        }
        HirExpr::Index { expr, .. } => {
            infer_index_hir_type(workspace, resolved_module, locals, expr)
        }
        HirExpr::Slice { expr, .. } => {
            infer_slice_hir_type(workspace, resolved_module, locals, expr)
        }
        HirExpr::Match { arms, .. } => {
            let inferred = arms
                .iter()
                .filter_map(|arm| {
                    infer_receiver_expr_type(workspace, resolved_module, locals, &arm.value)
                })
                .collect::<Vec<_>>();
            let first = inferred.first()?.clone();
            inferred
                .iter()
                .all(|candidate| candidate == &first)
                .then_some(first)
        }
        HirExpr::Await { expr } => {
            let awaited = infer_receiver_expr_type(workspace, resolved_module, locals, expr)?;
            match &hir_strip_reference_type(&awaited).kind {
                HirTypeKind::Apply { base, args }
                    if hir_path_matches_any(
                        base,
                        &[
                            &["Task"],
                            &["Thread"],
                            &["std", "concurrent", "Task"],
                            &["std", "concurrent", "Thread"],
                        ],
                    ) =>
                {
                    args.first().cloned()
                }
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn infer_receiver_expr_type_text<L: HirLocalTypeLookup>(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    locals: &L,
    expr: &HirExpr,
) -> Option<String> {
    infer_receiver_expr_type(workspace, resolved_module, locals, expr).map(|ty| ty.render())
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
    let Some(subject_type) = infer_receiver_expr_type(workspace, resolved_module, locals, subject)
    else {
        return false;
    };
    let stripped = hir_strip_reference_type(&subject_type);
    let Some(path) = hir_type_base_path(stripped) else {
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
                where_clause: symbol
                    .where_clause
                    .as_ref()
                    .map(type_surface::lower_surface_where_clause),
                params: symbol
                    .params
                    .iter()
                    .map(|param| HirParam {
                        mode: param.mode.as_ref().map(lower_param_mode),
                        name: param.name.clone(),
                        ty: type_surface::lower_surface_type(&param.ty),
                    })
                    .collect(),
                return_type: symbol
                    .return_type
                    .as_ref()
                    .map(type_surface::lower_surface_type),
                behavior_attrs: symbol
                    .behavior_attrs
                    .iter()
                    .map(|attr| HirBehaviorAttr {
                        name: attr.name.clone(),
                        value: attr.value.clone(),
                    })
                    .collect(),
                opaque_policy: symbol.opaque_policy.as_ref().map(lower_opaque_policy),
                availability: lower_availability_attachments(&symbol.availability),
                forewords: lower_forewords(&symbol.forewords),
                intrinsic_impl: symbol.intrinsic_impl.clone(),
                body: lower_symbol_body(&symbol.body),
                statements: lower_statements(&symbol.statements),
                cleanup_footers: lower_cleanup_footers(&symbol.cleanup_footers),
                span: symbol.span,
            })
            .collect(),
        impls: parsed
            .impls
            .iter()
            .map(|impl_decl| HirImplDecl {
                type_params: impl_decl.type_params.clone(),
                trait_path: impl_decl
                    .trait_path
                    .as_ref()
                    .map(type_surface::lower_surface_trait_ref),
                target_type: type_surface::lower_surface_type(&impl_decl.target_type),
                assoc_types: impl_decl
                    .assoc_types
                    .iter()
                    .map(|assoc_type| HirImplAssocTypeBinding {
                        name: assoc_type.name.clone(),
                        value_ty: assoc_type
                            .value_ty
                            .as_ref()
                            .map(type_surface::lower_surface_type),
                        span: assoc_type.span,
                    })
                    .collect(),
                methods: impl_decl
                    .methods
                    .iter()
                    .map(lower_trait_or_impl_method)
                    .collect(),
                body_entries: impl_decl.body_entries.clone(),
                span: impl_decl.span,
            })
            .collect(),
    }
}

fn resolve_module_target(
    package: &HirWorkspacePackage,
    workspace: &HirWorkspaceSummary,
    path: &[String],
) -> Result<(String, String, String), String> {
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
                    package.package_id.clone(),
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
                            std_package.package_id.clone(),
                            std_package.summary.package_name.clone(),
                            module.module_id.clone(),
                        )
                    })
                    .ok_or_else(|| format!("unresolved module `{key}`"))
            });
    }

    if let Some(dependency_package_id) = package.dependency_package_id(first) {
        let dependency_module_id = package
            .dependency_module_id(path)
            .ok_or_else(|| format!("unresolved module `{key}`"))?;
        return workspace
            .package_by_id(dependency_package_id)
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
                            dependency.package_id.clone(),
                            dependency.summary.package_name.clone(),
                            module.module_id.clone(),
                        )
                    })
                    .ok_or_else(|| format!("unresolved module `{key}`"))
            });
    }

    if let Some(module) = package.resolve_relative_module(path) {
        return Ok((
            package.package_id.clone(),
            package.summary.package_name.clone(),
            module.module_id.clone(),
        ));
    }

    if workspace
        .packages
        .values()
        .any(|dependency| dependency.summary.package_name == *first)
    {
        return Err(format!(
            "package `{first}` is not a direct dependency of `{}`",
            package.summary.package_name
        ));
    }

    Err(format!("unresolved module `{key}`"))
}

enum ResolvedUseTarget {
    Module {
        package_id: String,
        package_name: String,
        module_id: String,
    },
    Symbol {
        package_id: String,
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
        let Ok((package_id, package_name, module_id)) =
            resolve_module_target(package, workspace, prefix)
        else {
            continue;
        };
        if prefix_len == path.len() {
            return Ok(ResolvedUseTarget::Module {
                package_id,
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
            &package_id,
            &module_id,
            symbol_name,
        );
        if visible_symbols.len() == 1 {
            return Ok(ResolvedUseTarget::Symbol {
                package_id,
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

    if let Some(first) = path.first()
        && workspace
            .packages
            .values()
            .any(|candidate| candidate.summary.package_name == *first)
        && first != &package.summary.package_name
        && first != "std"
        && package.dependency_package_name(first).is_none()
    {
        return Err(format!(
            "package `{first}` is not a direct dependency of `{}`",
            package.summary.package_name
        ));
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
    package_id: String,
    root_dir: PathBuf,
    direct_deps: BTreeSet<String>,
    summary: HirPackageSummary,
    layout: HirPackageLayout,
) -> Result<HirWorkspacePackage, String> {
    let direct_dep_packages = direct_deps
        .iter()
        .map(|name| (name.clone(), name.clone()))
        .collect();
    build_workspace_package_with_dep_packages(
        package_id,
        root_dir,
        direct_dep_packages,
        direct_deps
            .iter()
            .map(|name| (name.clone(), name.clone()))
            .collect(),
        summary,
        layout,
    )
}

pub fn build_workspace_package_with_dep_packages(
    package_id: String,
    root_dir: PathBuf,
    direct_dep_packages: BTreeMap<String, String>,
    direct_dep_ids: BTreeMap<String, String>,
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

    let direct_deps = direct_dep_ids.values().cloned().collect();
    Ok(HirWorkspacePackage {
        package_id,
        root_dir,
        direct_deps,
        direct_dep_packages,
        direct_dep_ids,
        summary,
        layout,
    })
}

pub fn build_workspace_summary(
    packages: Vec<HirWorkspacePackage>,
) -> Result<HirWorkspaceSummary, String> {
    let mut package_map = BTreeMap::new();
    let mut package_names = BTreeMap::<String, Vec<String>>::new();
    let mut module_packages = BTreeMap::<String, Vec<String>>::new();
    for package in packages {
        let package_id = package.package_id.clone();
        if package_map
            .insert(package_id.clone(), package.clone())
            .is_some()
        {
            return Err(format!(
                "duplicate package id `{package_id}` in workspace summary"
            ));
        }
        package_names
            .entry(package.summary.package_name.clone())
            .or_default()
            .push(package_id.clone());
        for module in &package.summary.modules {
            let owners = module_packages.entry(module.module_id.clone()).or_default();
            if !owners.iter().any(|existing| existing == &package_id) {
                owners.push(package_id.clone());
            }
        }
    }
    Ok(HirWorkspaceSummary {
        packages: package_map,
        package_names,
        module_packages,
    })
}

pub fn resolve_workspace(
    workspace: &HirWorkspaceSummary,
) -> Result<HirResolvedWorkspace, Vec<HirResolutionError>> {
    let mut packages = BTreeMap::new();
    let mut package_names = BTreeMap::<String, Vec<String>>::new();
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
                            package_id: package.package_id.clone(),
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
                            |(package_id, package_name, module_id)| HirResolvedTarget::Module {
                                package_id,
                                package_name,
                                module_id,
                            },
                        )
                    }
                    HirDirectiveKind::Use => {
                        resolve_use_target(package, workspace, &directive.path).map(
                            |resolved_target| match resolved_target {
                                ResolvedUseTarget::Module {
                                    package_id,
                                    package_name,
                                    module_id,
                                } => HirResolvedTarget::Module {
                                    package_id,
                                    package_name,
                                    module_id,
                                },
                                ResolvedUseTarget::Symbol {
                                    package_id,
                                    package_name,
                                    module_id,
                                    symbol_name,
                                } => HirResolvedTarget::Symbol {
                                    package_id,
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
                    package_id: package.package_id.clone(),
                    module_id: module.module_id.clone(),
                    bindings,
                    directives,
                },
            );
        }

        packages.insert(
            package.package_id.clone(),
            HirResolvedPackage {
                package_id: package.package_id.clone(),
                package_name: package.summary.package_name.clone(),
                modules,
            },
        );
        package_names
            .entry(package.summary.package_name.clone())
            .or_default()
            .push(package.package_id.clone());
    }

    if errors.is_empty() {
        Ok(HirResolvedWorkspace {
            packages,
            package_names,
        })
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
        ParsedSymbolKind::Object => HirSymbolKind::Object,
        ParsedSymbolKind::Owner => HirSymbolKind::Owner,
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
                    ty: type_surface::lower_surface_type(&field.ty),
                    span: field.span,
                })
                .collect(),
        },
        arcana_syntax::SymbolBody::Object { fields, methods } => HirSymbolBody::Object {
            fields: fields
                .iter()
                .map(|field| HirField {
                    name: field.name.clone(),
                    ty: type_surface::lower_surface_type(&field.ty),
                    span: field.span,
                })
                .collect(),
            methods: methods.iter().map(lower_trait_or_impl_method).collect(),
        },
        arcana_syntax::SymbolBody::Enum { variants } => HirSymbolBody::Enum {
            variants: variants
                .iter()
                .map(|variant| HirEnumVariant {
                    name: variant.name.clone(),
                    payload: variant
                        .payload
                        .as_ref()
                        .map(type_surface::lower_surface_type),
                    span: variant.span,
                })
                .collect(),
        },
        arcana_syntax::SymbolBody::Owner { objects, exits } => HirSymbolBody::Owner {
            objects: objects
                .iter()
                .map(|object| HirOwnerObject {
                    type_path: object.type_path.clone(),
                    local_name: object.local_name.clone(),
                    span: object.span,
                })
                .collect(),
            exits: exits
                .iter()
                .map(|owner_exit| HirOwnerExit {
                    name: owner_exit.name.clone(),
                    condition: lower_expr(&owner_exit.condition),
                    holds: owner_exit.holds.clone(),
                    span: owner_exit.span,
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
                    default_ty: assoc_type
                        .default_ty
                        .as_ref()
                        .map(type_surface::lower_surface_type),
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
        where_clause: method
            .where_clause
            .as_ref()
            .map(type_surface::lower_surface_where_clause),
        params: method
            .params
            .iter()
            .map(|param| HirParam {
                mode: param.mode.as_ref().map(lower_param_mode),
                name: param.name.clone(),
                ty: type_surface::lower_surface_type(&param.ty),
            })
            .collect(),
        return_type: method
            .return_type
            .as_ref()
            .map(type_surface::lower_surface_type),
        behavior_attrs: method
            .behavior_attrs
            .iter()
            .map(|attr| HirBehaviorAttr {
                name: attr.name.clone(),
                value: attr.value.clone(),
            })
            .collect(),
        opaque_policy: method.opaque_policy.as_ref().map(lower_opaque_policy),
        availability: lower_availability_attachments(&method.availability),
        forewords: lower_forewords(&method.forewords),
        intrinsic_impl: method.intrinsic_impl.clone(),
        body: lower_symbol_body(&method.body),
        statements: lower_statements(&method.statements),
        cleanup_footers: lower_cleanup_footers(&method.cleanup_footers),
        span: method.span,
    }
}

fn lower_availability_attachments(
    attachments: &[arcana_syntax::AvailabilityAttachment],
) -> Vec<HirAvailabilityAttachment> {
    attachments
        .iter()
        .map(|attachment| HirAvailabilityAttachment {
            path: attachment.path.clone(),
            span: attachment.span,
        })
        .collect()
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

fn lower_cleanup_footers(
    cleanup_footers: &[arcana_syntax::CleanupFooter],
) -> Vec<HirCleanupFooter> {
    cleanup_footers
        .iter()
        .map(|rollup| HirCleanupFooter {
            kind: match rollup.kind {
                arcana_syntax::CleanupFooterKind::Cleanup => HirCleanupFooterKind::Cleanup,
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
            type_args: type_args
                .iter()
                .map(type_surface::lower_surface_type)
                .collect(),
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
        availability: lower_availability_attachments(&statement.availability),
        forewords: lower_forewords(&statement.forewords),
        cleanup_footers: lower_cleanup_footers(&statement.cleanup_footers),
        span: statement.span,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    use super::freeze::FROZEN_HIR_NODE_KINDS;
    use super::{
        HirAssignOp, HirAssignTarget, HirBinaryOp, HirChainConnector, HirChainIntroducer,
        HirChainStep, HirDirectiveKind, HirExpr, HirForewordApp, HirForewordArg,
        HirHeaderAttachment, HirMatchPattern, HirPackageLayout, HirPackageSummary, HirPhraseArg,
        HirStatement, HirStatementKind, HirSymbolBody, HirSymbolKind, HirTraitRef, HirType,
        HirUnaryOp, HirWhereClause, HirWorkspacePackage, build_package_layout,
        build_package_summary, build_workspace_summary, derive_source_module_path,
        lookup_method_candidates_for_hir_type, lookup_symbol_path, lower_module_text,
        parse_hir_type, resolve_workspace,
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
            worker.where_clause.as_ref().map(HirWhereClause::render),
            Some("std.iter.Iterator[T]".to_string())
        );
        assert_eq!(worker.params.len(), 2);
        assert_eq!(
            worker.return_type.as_ref().map(HirType::render),
            Some("Int".to_string())
        );
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
            module.impls[0].trait_path.as_ref().map(HirTraitRef::render),
            Some("std.iter.Iterator[T]".to_string())
        );
        assert_eq!(module.impls[0].target_type.render(), "RangeIter");
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
    fn lower_module_text_captures_cleanup_footers() {
        let module = lower_module_text(
            "cleanup_footers",
            "fn cleanup(take value: Int) -> Result[Unit, Str]:\n    return Result.Ok[Unit, Str] :: :: call\nfn run(seed: Int) -> Int:\n    let local = seed\n    while local > 0:\n        let scratch = local\n        local -= 1\n    -cleanup[target = scratch, handler = cleanup]\n    return local\n-cleanup[target = seed, handler = cleanup]\n",
        )
        .expect("cleanup footers should lower");

        let run = module
            .symbols
            .iter()
            .find(|symbol| symbol.name == "run")
            .expect("run symbol should exist");
        assert_eq!(run.cleanup_footers.len(), 1);
        assert_eq!(run.cleanup_footers[0].subject, "seed");
        assert_eq!(
            run.cleanup_footers[0].handler_path,
            vec!["cleanup".to_string()]
        );
        match &run.statements[1] {
            HirStatement {
                kind: HirStatementKind::While { .. },
                cleanup_footers,
                ..
            } => {
                assert_eq!(cleanup_footers.len(), 1);
                assert_eq!(cleanup_footers[0].subject, "scratch");
                assert_eq!(cleanup_footers[0].kind.as_str(), "cleanup");
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
                            if type_args.iter().map(HirType::render).collect::<Vec<_>>()
                                == vec!["Str".to_string()]
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
                                if type_args.iter().map(HirType::render).collect::<Vec<_>>()
                                    == vec!["Int".to_string()]
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
                        if type_args.iter().map(HirType::render).collect::<Vec<_>>()
                            == vec!["(K, V)".to_string()]
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

    fn build_workspace_package(
        root_dir: PathBuf,
        direct_deps: BTreeSet<String>,
        summary: HirPackageSummary,
        layout: HirPackageLayout,
    ) -> Result<HirWorkspacePackage, String> {
        let package_id = summary.package_name.clone();
        super::build_workspace_package(package_id, root_dir, direct_deps, summary, layout)
    }

    fn build_workspace_package_with_dep_packages(
        root_dir: PathBuf,
        direct_dep_packages: BTreeMap<String, String>,
        summary: HirPackageSummary,
        layout: HirPackageLayout,
    ) -> Result<HirWorkspacePackage, String> {
        let package_id = summary.package_name.clone();
        let direct_dep_ids = direct_dep_packages
            .iter()
            .map(|(alias, package_id)| (alias.clone(), package_id.clone()))
            .collect();
        super::build_workspace_package_with_dep_packages(
            package_id,
            root_dir,
            direct_dep_packages,
            direct_dep_ids,
            summary,
            layout,
        )
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
                package_id: "std".to_string(),
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
                package_id: "std".to_string(),
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
        let app_package = build_workspace_package_with_dep_packages(
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
                package_id: "core".to_string(),
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

        let receiver = parse_hir_type("Counter").expect("receiver should parse");
        let candidates =
            lookup_method_candidates_for_hir_type(&workspace, resolved_module, &receiver, "tap");
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

        let receiver = parse_hir_type("Counter").expect("receiver should parse");
        let candidates =
            lookup_method_candidates_for_hir_type(&workspace, resolved_module, &receiver, "tap");
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
        let app_package = build_workspace_package_with_dep_packages(
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

        let receiver = parse_hir_type("Counter").expect("receiver should parse");
        let candidates =
            lookup_method_candidates_for_hir_type(&workspace, resolved_module, &receiver, "tap");
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

        let receiver = parse_hir_type("core.Hidden").expect("receiver should parse");
        let candidates =
            lookup_method_candidates_for_hir_type(&workspace, resolved_module, &receiver, "tap");
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
