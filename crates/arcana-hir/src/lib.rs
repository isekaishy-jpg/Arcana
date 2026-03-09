pub mod freeze;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use arcana_syntax::{
    AssignOp as ParsedAssignOp, DirectiveKind as ParsedDirectiveKind, Expr as ParsedExpr,
    ParamMode as ParsedParamMode, ParsedModule, Span, StatementKind as ParsedStatementKind,
    SymbolKind as ParsedSymbolKind, parse_module,
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
            Self::Trait => "trait",
            Self::Behavior => "behavior",
            Self::Const => "const",
        }
    }
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
pub struct HirRawBlockEntry {
    pub text: String,
    pub span: Span,
    pub children: Vec<HirRawBlockEntry>,
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
    Opaque {
        text: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirPhraseArg {
    Positional(HirExpr),
    Named { name: String, value: HirExpr },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HirUnaryOp {
    Neg,
    Not,
    BitNot,
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
    Opaque {
        text: String,
        attached: Vec<HirRawBlockEntry>,
    },
    CollectionLiteral {
        items: Vec<HirExpr>,
    },
    Match {
        subject: Box<HirExpr>,
        arms: Vec<HirMatchArm>,
    },
    Chain {
        mode: String,
        reverse: bool,
        steps: Vec<String>,
    },
    MemoryPhrase {
        family: String,
        arena: Box<HirExpr>,
        init_args: Vec<HirPhraseArg>,
        constructor: String,
    },
    QualifiedPhrase {
        subject: Box<HirExpr>,
        args: Vec<HirPhraseArg>,
        qualifier: String,
        attached: Vec<HirRawBlockEntry>,
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
    Opaque {
        text: String,
    },
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

    pub fn exported_surface_rows(&self) -> Vec<String> {
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
}

impl HirSymbol {
    fn api_signature_text(&self) -> String {
        match self.kind {
            HirSymbolKind::Fn | HirSymbolKind::System => render_function_signature(self),
            HirSymbolKind::Record => render_record_signature(self),
            HirSymbolKind::Enum => render_enum_signature(self),
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

    pub fn exported_surface_rows(&self) -> Vec<String> {
        let mut rows = Vec::new();
        for module in &self.modules {
            for row in module.exported_surface_rows() {
                rows.push(format!("module={}:{}", module.module_id, row));
            }
        }
        rows.sort();
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

    if package.direct_deps.contains(first) {
        return workspace
            .package(first)
            .ok_or_else(|| {
                format!(
                    "dependency `{first}` is not loaded for `{}`",
                    package.summary.package_name
                )
            })
            .and_then(|dependency| {
                dependency
                    .module(&key)
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
        let resolved_package = workspace
            .package(&package_name)
            .ok_or_else(|| format!("resolved package `{package_name}` is not loaded"))?;
        let resolved_module = resolved_package
            .module(&module_id)
            .ok_or_else(|| format!("resolved module `{module_id}` is not loaded"))?;
        if resolved_module.has_symbol(symbol_name) {
            return Ok(ResolvedUseTarget::Symbol {
                package_name,
                module_id,
                symbol_name: symbol_name.clone(),
            });
        }
        return Err(format!(
            "unresolved symbol `{symbol_name}` in module `{module_id}`"
        ));
    }

    if let Some(first) = path.first() {
        if workspace.package(first).is_some()
            && first != &package.summary.package_name
            && first != "std"
            && !package.direct_deps.contains(first)
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
    if summary.modules.len() != layout.absolute_modules.len() {
        return Err(format!(
            "package `{}` has {} modules but layout indexes {}",
            summary.package_name,
            summary.modules.len(),
            layout.absolute_modules.len()
        ));
    }

    Ok(HirWorkspacePackage {
        root_dir,
        direct_deps,
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
            let mut bindings = BTreeMap::new();
            for symbol in &module.symbols {
                bindings
                    .entry(symbol.name.clone())
                    .or_insert(HirResolvedBinding {
                        local_name: symbol.name.clone(),
                        origin: HirBindingOrigin::LocalSymbol,
                        target: HirResolvedTarget::Symbol {
                            package_name: package.summary.package_name.clone(),
                            module_id: module.module_id.clone(),
                            symbol_name: symbol.name.clone(),
                        },
                        span: symbol.span,
                    });
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
                        bindings
                            .entry(local_name.clone())
                            .or_insert(HirResolvedBinding {
                                local_name: local_name.clone(),
                                origin: HirBindingOrigin::Directive(directive.kind),
                                target: target.clone(),
                                span: directive.span,
                            });
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
        ParsedSymbolKind::Trait => HirSymbolKind::Trait,
        ParsedSymbolKind::Behavior => HirSymbolKind::Behavior,
        ParsedSymbolKind::Const => HirSymbolKind::Const,
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
        arcana_syntax::AssignTarget::Opaque { text } => {
            HirAssignTarget::Opaque { text: text.clone() }
        }
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

fn lower_raw_block_entries(entries: &[arcana_syntax::RawBlockEntry]) -> Vec<HirRawBlockEntry> {
    entries
        .iter()
        .map(|entry| HirRawBlockEntry {
            text: entry.text.clone(),
            span: entry.span,
            children: lower_raw_block_entries(&entry.children),
        })
        .collect()
}

fn lower_expr(expr: &ParsedExpr) -> HirExpr {
    match expr {
        ParsedExpr::Opaque { text, attached } => HirExpr::Opaque {
            text: text.clone(),
            attached: lower_raw_block_entries(attached),
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
            mode,
            reverse,
            steps,
        } => HirExpr::Chain {
            mode: mode.clone(),
            reverse: *reverse,
            steps: steps.clone(),
        },
        ParsedExpr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
        } => HirExpr::MemoryPhrase {
            family: family.clone(),
            arena: Box::new(lower_expr(arena)),
            init_args: init_args.iter().map(lower_phrase_arg).collect(),
            constructor: constructor.clone(),
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
            attached: lower_raw_block_entries(attached),
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
        arcana_syntax::MatchPattern::Opaque { text } => {
            HirMatchPattern::Opaque { text: text.clone() }
        }
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

fn lower_unary_op(op: &arcana_syntax::UnaryOp) -> HirUnaryOp {
    match op {
        arcana_syntax::UnaryOp::Neg => HirUnaryOp::Neg,
        arcana_syntax::UnaryOp::Not => HirUnaryOp::Not,
        arcana_syntax::UnaryOp::BitNot => HirUnaryOp::BitNot,
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
        HirAssignOp, HirAssignTarget, HirBinaryOp, HirDirectiveKind, HirExpr, HirMatchPattern,
        HirPhraseArg, HirStatement, HirStatementKind, HirSymbolBody, HirSymbolKind, HirUnaryOp,
        build_package_layout, build_package_summary, build_workspace_package,
        build_workspace_summary, derive_source_module_path, lower_module_text, resolve_workspace,
    };

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
            module.exported_surface_rows(),
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
            module.exported_surface_rows(),
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
                assert!(matches!(
                    value,
                    HirExpr::Opaque { text, attached } if text == "0" && attached.is_empty()
                ));
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
                            HirExpr::Opaque { text, attached }
                                if text == "frames" && attached.is_empty()
                        ));
                        assert!(matches!(
                            right.as_ref(),
                            HirExpr::Opaque { text, attached } if text == "10" && attached.is_empty()
                        ));
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
                                            HirExpr::Opaque { text, attached }
                                                if text == "frames" && attached.is_empty()
                                        ));
                                        assert!(matches!(
                                            right.as_ref(),
                                            HirExpr::Opaque { text, attached }
                                                if text == "2" && attached.is_empty()
                                        ));
                                    }
                                    other => panic!(
                                        "expected modulo expression in if condition, got {other:?}"
                                    ),
                                }
                                assert!(matches!(
                                    right.as_ref(),
                                    HirExpr::Opaque { text, attached }
                                        if text == "0" && attached.is_empty()
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
                                    HirExpr::Opaque { text, attached }
                                        if text == "1" && attached.is_empty()
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
                            HirExpr::Opaque { text, attached }
                                if text == "frames" && attached.is_empty()
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
                            HirExpr::Opaque { ref text, ref attached }
                                if text == "1" && attached.is_empty()
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
            "use std.result as result\n#test\n#inline\nfn smoke() -> Int:\n    return 0\n#stage[phase=update, deterministic=true]\nsystem[phase=startup, affinity=main] fn boot():\n    #chain[phase=startup, deterministic=true]\n    forward :=> seed => step\nlang result = smoke\nintrinsic fn host_len(read text: Str) -> Int = HostTextLenBytes\nfn seed() -> Int:\n    return 1\nfn step(v: Int) -> Int:\n    return v\n",
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
                assert_eq!(constructor, "Item");
            }
            other => panic!("expected memory phrase, got {other:?}"),
        }
        match &module.symbols[0].statements[2].kind {
            HirStatementKind::Expr {
                expr:
                    HirExpr::Chain {
                        mode,
                        reverse,
                        steps,
                    },
            } => {
                assert_eq!(mode, "forward");
                assert!(!reverse);
                assert_eq!(steps, &vec!["seed".to_string(), "step".to_string()]);
            }
            other => panic!("expected chain expression, got {other:?}"),
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
                            HirExpr::Opaque { text, attached } if text == "t" && attached.is_empty()
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
                            HirExpr::Opaque { text, attached } if text == "out" && attached.is_empty()
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
            module.exported_surface_rows(),
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
                        HirExpr::MemberAccess { member, .. } if member == "print[Str]"
                    ));
                    assert_eq!(args.len(), 1);
                    assert!(matches!(
                        &args[0],
                        HirPhraseArg::Positional(HirExpr::Opaque { text, attached })
                            if text == "\"bye\"" && attached.is_empty()
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
                        assert!(matches!(
                            expr.as_ref(),
                            HirExpr::Opaque { text, attached } if text == "task" && attached.is_empty()
                        ));
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
                                            HirExpr::Opaque { text, attached }
                                                if text == "3" && attached.is_empty()
                                        ));
                                    }
                                    other => panic!(
                                        "expected shift expression in comparison lhs, got {other:?}"
                                    ),
                                }
                                assert!(matches!(
                                    right.as_ref(),
                                    HirExpr::Opaque { text, attached }
                                        if text == "8" && attached.is_empty()
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
                            HirPhraseArg::Named { name, value: HirExpr::Opaque { text, attached } }
                                if name == "clear" && text == "0" && attached.is_empty()
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
                            HirExpr::MemberAccess { member, .. } if member == "print[Int]"
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
                    HirExpr::Opaque { text, attached }
                        if text == "printed" && attached.is_empty()
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
                        assert!(matches!(
                            expr.as_ref(),
                            HirExpr::Opaque { text, attached } if text == "xs" && attached.is_empty()
                        ));
                        assert!(matches!(
                            index.as_ref(),
                            HirExpr::Opaque { text, attached } if text == "0" && attached.is_empty()
                        ));
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
                            Some(HirExpr::Opaque { text, attached })
                                if text == "1" && attached.is_empty()
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
                            Some(HirExpr::Opaque { text, attached })
                                if text == "1" && attached.is_empty()
                        ));
                        assert!(matches!(
                            end.as_deref(),
                            Some(HirExpr::Opaque { text, attached })
                                if text == "2" && attached.is_empty()
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
                    assert!(matches!(
                        index,
                        HirExpr::Opaque { text, attached } if text == "1" && attached.is_empty()
                    ));
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
                    assert!(matches!(
                        index,
                        HirExpr::Opaque { text, attached } if text == "i" && attached.is_empty()
                    ));
                }
                other => panic!("expected indexed compound-assignment target, got {other:?}"),
            },
            other => panic!("expected third assignment statement, got {other:?}"),
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
            package.exported_surface_rows(),
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
