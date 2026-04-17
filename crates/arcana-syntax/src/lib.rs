use std::collections::{BTreeMap, BTreeSet};

use arcana_cabi::{ArcanaCabiBindingRawType, ArcanaCabiBindingScalarType};
pub mod freeze;
mod language_contract;
pub mod surface_text;
pub mod type_surface;

pub use language_contract::{
    ConstructCompletionKind, HeadedModifierKeyword, HeadedRegionHead, MemoryDetailDescriptor,
    MemoryDetailKey, MemoryDetailValueKind, MemoryFamily, MemoryFamilyDescriptor,
    memory_detail_descriptor, memory_family_descriptor, memory_family_descriptors,
    memory_modifier_allowed,
};

pub use surface_text::{ParsedSurfaceText, SurfaceTextToken, parse_surface_text};
pub use type_surface::{
    SurfaceLifetime, SurfacePath, SurfacePredicate, SurfaceProjection, SurfaceRefs,
    SurfaceTraitRef, SurfaceType, SurfaceTypeKind, SurfaceWhereClause, collect_surface_type_refs,
    collect_surface_where_clause_refs, parse_surface_path, parse_surface_trait_ref,
    parse_surface_type, parse_surface_where_clause, surface_type_is_boundary_safe,
    validate_tuple_type_contract,
};

type ParsedFunctionTail = (
    Vec<String>,
    Option<SurfaceWhereClause>,
    Vec<ParamDecl>,
    Option<SurfaceType>,
);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectiveKind {
    Import,
    Use,
    Reexport,
}

impl DirectiveKind {
    pub const fn keyword(&self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Use => "use",
            Self::Reexport => "reexport",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleDirective {
    pub kind: DirectiveKind,
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub forewords: Vec<ForewordApp>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForewordTier {
    Basic,
    Executable,
}

impl ForewordTier {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Executable => "executable",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForewordVisibility {
    Package,
    Public,
}

impl ForewordVisibility {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Package => "package",
            Self::Public => "public",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForewordRetention {
    Compile,
    Tooling,
    Runtime,
}

impl ForewordRetention {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Compile => "compile",
            Self::Tooling => "tooling",
            Self::Runtime => "runtime",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForewordAction {
    Metadata,
    Transform,
}

impl ForewordAction {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Metadata => "metadata",
            Self::Transform => "transform",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForewordPhase {
    Frontend,
}

impl ForewordPhase {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Frontend => "frontend",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForewordPayloadType {
    Bool,
    Int,
    Str,
    Symbol,
    Path,
}

impl ForewordPayloadType {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Bool => "Bool",
            Self::Int => "Int",
            Self::Str => "Str",
            Self::Symbol => "Symbol",
            Self::Path => "Path",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ForewordDefinitionTarget {
    Import,
    Reexport,
    Use,
    Function,
    Record,
    Object,
    Owner,
    Enum,
    OpaqueType,
    Trait,
    Behavior,
    System,
    Const,
    TraitMethod,
    ImplMethod,
    Field,
    Param,
}

impl ForewordDefinitionTarget {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Reexport => "reexport",
            Self::Use => "use",
            Self::Function => "fn",
            Self::Record => "record",
            Self::Object => "obj",
            Self::Owner => "owner",
            Self::Enum => "enum",
            Self::OpaqueType => "opaque_type",
            Self::Trait => "trait",
            Self::Behavior => "behavior",
            Self::System => "system",
            Self::Const => "const",
            Self::TraitMethod => "trait_method",
            Self::ImplMethod => "impl_method",
            Self::Field => "field",
            Self::Param => "param",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForewordPayloadField {
    pub name: String,
    pub optional: bool,
    pub ty: ForewordPayloadType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForewordDefinitionDecl {
    pub qualified_name: Vec<String>,
    pub tier: ForewordTier,
    pub visibility: ForewordVisibility,
    pub phase: ForewordPhase,
    pub action: ForewordAction,
    pub targets: Vec<ForewordDefinitionTarget>,
    pub retention: ForewordRetention,
    pub payload: Vec<ForewordPayloadField>,
    pub repeatable: bool,
    pub conflicts: Vec<Vec<String>>,
    pub diagnostic_namespace: Option<String>,
    pub handler: Option<Vec<String>>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForewordHandlerDecl {
    pub qualified_name: Vec<String>,
    pub phase: ForewordPhase,
    pub protocol: String,
    pub product: String,
    pub entry: String,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForewordAliasKind {
    Alias,
    Reexport,
}

impl ForewordAliasKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Alias => "alias",
            Self::Reexport => "reexport",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForewordAliasDecl {
    pub kind: ForewordAliasKind,
    pub source_name: Vec<String>,
    pub alias_name: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Fn,
    System,
    Record,
    Struct,
    Union,
    Array,
    Object,
    Owner,
    Enum,
    OpaqueType,
    Trait,
    Behavior,
    Const,
}

impl SymbolKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Fn => "fn",
            Self::System => "system",
            Self::Record => "record",
            Self::Struct => "struct",
            Self::Union => "union",
            Self::Array => "array",
            Self::Object => "obj",
            Self::Owner => "create",
            Self::Enum => "enum",
            Self::OpaqueType => "opaque type",
            Self::Trait => "trait",
            Self::Behavior => "behavior",
            Self::Const => "const",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpaqueOwnershipPolicy {
    Copy,
    Move,
}

impl OpaqueOwnershipPolicy {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Move => "move",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpaqueBoundaryPolicy {
    Safe,
    Unsafe,
}

impl OpaqueBoundaryPolicy {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "boundary_safe",
            Self::Unsafe => "boundary_unsafe",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpaqueTypePolicy {
    pub ownership: OpaqueOwnershipPolicy,
    pub boundary: OpaqueBoundaryPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamMode {
    Read,
    Edit,
    Take,
    Hold,
}

impl ParamMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Edit => "edit",
            Self::Take => "take",
            Self::Hold => "hold",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamDecl {
    pub mode: Option<ParamMode>,
    pub name: String,
    pub ty: SurfaceType,
    pub forewords: Vec<ForewordApp>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDecl {
    pub name: String,
    pub ty: SurfaceType,
    pub bit_width: Option<u16>,
    pub forewords: Vec<ForewordApp>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumVariantDecl {
    pub name: String,
    pub payload: Option<SurfaceType>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraitAssocTypeDecl {
    pub name: String,
    pub default_ty: Option<SurfaceType>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BehaviorAttr {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForewordArgValue {
    Raw(String),
    Bool(bool),
    Int(i64),
    Str(String),
    Symbol(String),
    Path(Vec<String>),
}

impl ForewordArgValue {
    pub fn parse(source: &str) -> Self {
        let trimmed = source.trim();
        if let Some(unquoted) = unquote_double_quoted_literal(trimmed) {
            return Self::Str(unquoted.to_string());
        }
        if trimmed == "true" {
            return Self::Bool(true);
        }
        if trimmed == "false" {
            return Self::Bool(false);
        }
        if let Ok(value) = trimmed.parse::<i64>() {
            return Self::Int(value);
        }
        if is_path_like(trimmed) {
            let segments = trimmed
                .split('.')
                .map(|segment| segment.to_string())
                .collect();
            if !trimmed.contains('.') {
                return Self::Symbol(trimmed.to_string());
            }
            return Self::Path(segments);
        }
        Self::Raw(trimmed.to_string())
    }

    pub fn render(&self) -> String {
        match self {
            Self::Raw(value) => value.clone(),
            Self::Bool(value) => value.to_string(),
            Self::Int(value) => value.to_string(),
            Self::Str(value) => format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\"")),
            Self::Symbol(value) => value.clone(),
            Self::Path(segments) => segments.join("."),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForewordArg {
    pub name: Option<String>,
    pub value: String,
    pub typed_value: ForewordArgValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForewordApp {
    pub name: String,
    pub path: Vec<String>,
    pub args: Vec<ForewordArg>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LangItemDecl {
    pub name: String,
    pub target: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanupFooterKind {
    Cleanup,
}

impl CleanupFooterKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Cleanup => "cleanup",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanupFooter {
    pub kind: CleanupFooterKind,
    pub subject: String,
    pub handler_path: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AvailabilityAttachment {
    pub path: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerObjectDecl {
    pub type_path: Vec<String>,
    pub local_name: String,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualifiedPhraseQualifierKind {
    Call,
    Try,
    Apply,
    AwaitApply,
    Await,
    Weave,
    Split,
    Must,
    Fallback,
    BareMethod,
    NamedPath,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerExitDecl {
    pub name: String,
    pub condition: Expr,
    pub retains: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawBlockEntry {
    pub text: String,
    pub span: Span,
    pub children: Vec<RawBlockEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchArm {
    pub patterns: Vec<MatchPattern>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderAttachment {
    Named {
        name: String,
        value: Expr,
        forewords: Vec<ForewordApp>,
        span: Span,
    },
    Chain {
        expr: Expr,
        forewords: Vec<ForewordApp>,
        span: Span,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchPattern {
    Wildcard,
    Literal {
        text: String,
    },
    Name {
        text: String,
    },
    Variant {
        path: String,
        args: Vec<MatchPattern>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhraseArg {
    Positional(Expr),
    Named { name: String, value: Expr },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeadedModifierKind {
    Keyword(HeadedModifierKeyword),
    Name(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadedModifier {
    pub kind: HeadedModifierKind,
    pub payload: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecycleLineKind {
    Expr {
        gate: Expr,
    },
    Let {
        mutable: bool,
        name: String,
        gate: Expr,
    },
    Assign {
        name: String,
        gate: Expr,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecycleLine {
    pub kind: RecycleLineKind,
    pub modifier: Option<HeadedModifier>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindLineKind {
    Let {
        mutable: bool,
        name: String,
        gate: Expr,
    },
    Assign {
        name: String,
        gate: Expr,
    },
    Require {
        expr: Expr,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindLine {
    pub kind: BindLineKind,
    pub modifier: Option<HeadedModifier>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructLine {
    pub name: String,
    pub value: Expr,
    pub modifier: Option<HeadedModifier>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstructDestination {
    Deliver { name: String },
    Place { target: AssignTarget },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructRegion {
    pub completion: ConstructCompletionKind,
    pub target: Box<Expr>,
    pub destination: Option<ConstructDestination>,
    pub default_modifier: Option<HeadedModifier>,
    pub lines: Vec<ConstructLine>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NominalFieldRegionKind {
    Record,
    Struct,
    Union,
}

impl NominalFieldRegionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Record => "record",
            Self::Struct => "struct",
            Self::Union => "union",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordRegion {
    pub kind: NominalFieldRegionKind,
    pub completion: ConstructCompletionKind,
    pub target: Box<Expr>,
    pub base: Option<Box<Expr>>,
    pub destination: Option<ConstructDestination>,
    pub default_modifier: Option<HeadedModifier>,
    pub lines: Vec<ConstructLine>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrayLine {
    pub index: usize,
    pub value: Expr,
    pub modifier: Option<HeadedModifier>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrayRegion {
    pub completion: ConstructCompletionKind,
    pub target: Box<Expr>,
    pub base: Option<Box<Expr>>,
    pub destination: Option<ConstructDestination>,
    pub default_modifier: Option<HeadedModifier>,
    pub lines: Vec<ArrayLine>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryDetailLine {
    pub key: MemoryDetailKey,
    pub value: Expr,
    pub modifier: Option<HeadedModifier>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemorySpecDecl {
    pub family: MemoryFamily,
    pub name: String,
    pub default_modifier: Option<HeadedModifier>,
    pub details: Vec<MemoryDetailLine>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainConnector {
    Forward,
    Reverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainIntroducer {
    Forward,
    Reverse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainStep {
    pub incoming: Option<ChainConnector>,
    pub stage: Expr,
    pub bind_args: Vec<Expr>,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
    CapabilityRead,
    CapabilityEdit,
    CapabilityTake,
    CapabilityHold,
    Deref,
    Weave,
    Split,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionFamily {
    Inferred,
    Contiguous,
    Strided,
}

impl ProjectionFamily {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inferred => "inferred",
            Self::Contiguous => "contiguous",
            Self::Strided => "strided",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Path {
        segments: Vec<String>,
    },
    BoolLiteral {
        value: bool,
    },
    IntLiteral {
        text: String,
    },
    FloatLiteral {
        text: String,
        kind: FloatLiteralKind,
    },
    StrLiteral {
        text: String,
    },
    Pair {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    CollectionLiteral {
        items: Vec<Expr>,
    },
    Match {
        subject: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    ConstructRegion(Box<ConstructRegion>),
    RecordRegion(Box<RecordRegion>),
    ArrayRegion(Box<ArrayRegion>),
    Chain {
        style: String,
        introducer: ChainIntroducer,
        steps: Vec<ChainStep>,
    },
    MemoryPhrase {
        family: String,
        arena: Box<Expr>,
        init_args: Vec<PhraseArg>,
        constructor: Box<Expr>,
        attached: Vec<HeaderAttachment>,
    },
    GenericApply {
        expr: Box<Expr>,
        type_args: Vec<SurfaceType>,
    },
    QualifiedPhrase {
        subject: Box<Expr>,
        args: Vec<PhraseArg>,
        qualifier_kind: QualifiedPhraseQualifierKind,
        qualifier: String,
        qualifier_type_args: Vec<SurfaceType>,
        attached: Vec<HeaderAttachment>,
    },
    Await {
        expr: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    MemberAccess {
        expr: Box<Expr>,
        member: String,
    },
    Index {
        expr: Box<Expr>,
        index: Box<Expr>,
    },
    Slice {
        expr: Box<Expr>,
        family: ProjectionFamily,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        len: Option<Box<Expr>>,
        stride: Option<Box<Expr>>,
        inclusive_end: bool,
    },
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive_end: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatLiteralKind {
    F32,
    F64,
}

impl FloatLiteralKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::F32 => "F32",
            Self::F64 => "F64",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
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

impl AssignOp {
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
pub enum AssignTarget {
    Name {
        text: String,
    },
    Deref {
        expr: Expr,
    },
    MemberAccess {
        target: Box<AssignTarget>,
        member: String,
    },
    Index {
        target: Box<AssignTarget>,
        index: Expr,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeferAction {
    Expr { expr: Expr },
    Reclaim { expr: Expr },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementKind {
    Let {
        mutable: bool,
        name: String,
        value: Expr,
    },
    Return {
        value: Option<Expr>,
    },
    If {
        condition: Expr,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
    },
    While {
        condition: Expr,
        body: Vec<Statement>,
    },
    For {
        binding: String,
        iterable: Expr,
        body: Vec<Statement>,
    },
    Defer {
        action: DeferAction,
    },
    Reclaim {
        expr: Expr,
    },
    Break,
    Continue,
    Assign {
        target: AssignTarget,
        op: AssignOp,
        value: Expr,
    },
    Recycle {
        default_modifier: Option<HeadedModifier>,
        lines: Vec<RecycleLine>,
    },
    Bind {
        default_modifier: Option<HeadedModifier>,
        lines: Vec<BindLine>,
    },
    Record(RecordRegion),
    Array(ArrayRegion),
    Construct(ConstructRegion),
    MemorySpec(MemorySpecDecl),
    Expr {
        expr: Expr,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement {
    pub kind: StatementKind,
    pub availability: Vec<AvailabilityAttachment>,
    pub forewords: Vec<ForewordApp>,
    pub cleanup_footers: Vec<CleanupFooter>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolBody {
    None,
    Record {
        fields: Vec<FieldDecl>,
    },
    Struct {
        fields: Vec<FieldDecl>,
    },
    Union {
        fields: Vec<FieldDecl>,
    },
    Array {
        element_ty: SurfaceType,
        len: usize,
    },
    Object {
        fields: Vec<FieldDecl>,
        methods: Vec<SymbolDecl>,
    },
    Enum {
        variants: Vec<EnumVariantDecl>,
    },
    Owner {
        objects: Vec<OwnerObjectDecl>,
        context_type: Option<SurfaceType>,
        exits: Vec<OwnerExitDecl>,
    },
    Trait {
        assoc_types: Vec<TraitAssocTypeDecl>,
        methods: Vec<SymbolDecl>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolDecl {
    pub name: String,
    pub kind: SymbolKind,
    pub exported: bool,
    pub is_async: bool,
    pub type_params: Vec<String>,
    pub where_clause: Option<SurfaceWhereClause>,
    pub params: Vec<ParamDecl>,
    pub return_type: Option<SurfaceType>,
    pub behavior_attrs: Vec<BehaviorAttr>,
    pub opaque_policy: Option<OpaqueTypePolicy>,
    pub availability: Vec<AvailabilityAttachment>,
    pub forewords: Vec<ForewordApp>,
    pub intrinsic_impl: Option<String>,
    pub native_impl: Option<String>,
    pub body: SymbolBody,
    pub statements: Vec<Statement>,
    pub cleanup_footers: Vec<CleanupFooter>,
    pub surface_text: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeCallbackDecl {
    pub name: String,
    pub params: Vec<ParamDecl>,
    pub return_type: Option<SurfaceType>,
    pub callback_type: Option<SurfaceType>,
    pub target: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShackleDeclKind {
    Type,
    Struct,
    Union,
    Flags,
    Const,
    ImportFn,
    Callback,
    Fn,
    Thunk,
}

impl ShackleDeclKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Struct => "struct",
            Self::Union => "union",
            Self::Flags => "flags",
            Self::Const => "const",
            Self::ImportFn => "import_fn",
            Self::Callback => "callback",
            Self::Fn => "fn",
            Self::Thunk => "thunk",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShackleDecl {
    pub exported: bool,
    pub kind: ShackleDeclKind,
    pub name: String,
    pub params: Vec<ParamDecl>,
    pub return_type: Option<SurfaceType>,
    pub callback_type: Option<SurfaceType>,
    pub binding: Option<String>,
    pub body_entries: Vec<String>,
    pub raw_decl: Option<ShackleRawDecl>,
    pub import_target: Option<ShackleImportTarget>,
    pub thunk_target: Option<ShackleThunkTarget>,
    pub surface_text: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShackleImportTarget {
    pub library: String,
    pub symbol: String,
    pub abi: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShackleThunkTarget {
    pub target: String,
    pub abi: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShackleFieldSpec {
    pub name: String,
    pub ty: ArcanaCabiBindingRawType,
    pub bit_width: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShackleEnumVariantSpec {
    pub name: String,
    pub value: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShackleRawDecl {
    Alias {
        target: ArcanaCabiBindingRawType,
    },
    Array {
        element_type: ArcanaCabiBindingRawType,
        len: usize,
    },
    Enum {
        repr: ArcanaCabiBindingScalarType,
        variants: Vec<ShackleEnumVariantSpec>,
    },
    Struct {
        fields: Vec<ShackleFieldSpec>,
    },
    Union {
        fields: Vec<ShackleFieldSpec>,
    },
    Flags {
        repr: ArcanaCabiBindingScalarType,
    },
    Callback {
        abi: String,
        params: Vec<ArcanaCabiBindingRawType>,
        return_type: ArcanaCabiBindingRawType,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImplAssocTypeBinding {
    pub name: String,
    pub value_ty: Option<SurfaceType>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImplDecl {
    pub type_params: Vec<String>,
    pub trait_path: Option<SurfaceTraitRef>,
    pub target_type: SurfaceType,
    pub assoc_types: Vec<ImplAssocTypeBinding>,
    pub methods: Vec<SymbolDecl>,
    pub body_entries: Vec<String>,
    pub surface_text: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedModule {
    pub line_count: usize,
    pub non_empty_line_count: usize,
    pub directives: Vec<ModuleDirective>,
    pub lang_items: Vec<LangItemDecl>,
    pub memory_specs: Vec<MemorySpecDecl>,
    pub native_callbacks: Vec<NativeCallbackDecl>,
    pub shackle_decls: Vec<ShackleDecl>,
    pub foreword_definitions: Vec<ForewordDefinitionDecl>,
    pub foreword_handlers: Vec<ForewordHandlerDecl>,
    pub foreword_aliases: Vec<ForewordAliasDecl>,
    pub symbols: Vec<SymbolDecl>,
    pub impls: Vec<ImplDecl>,
}

pub fn parse_module(source: &str) -> Result<ParsedModule, String> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut line_count = 0usize;
    let mut non_empty = 0usize;
    let mut source_lines = Vec::with_capacity(lines.len());

    for (idx, line) in lines.iter().enumerate() {
        line_count = idx + 1;
        let analysis = analyze_line(line, idx)?;
        let counts_as_non_empty = analysis.counts_as_non_empty();
        if counts_as_non_empty {
            non_empty += 1;
        }
        source_lines.push(SourceLine {
            text: analysis.trimmed.to_string(),
            leading_spaces: analysis.leading_spaces,
            line: idx + 1,
            counts_as_non_empty,
        });
    }

    let (entries, _) = collect_block_entries(&source_lines, 0, 0)?;
    let mut directives = Vec::new();
    let mut lang_items = Vec::new();
    let mut memory_specs = Vec::new();
    let mut native_callbacks = Vec::new();
    let mut shackle_decls = Vec::new();
    let mut foreword_definitions = Vec::new();
    let mut foreword_handlers = Vec::new();
    let mut foreword_aliases = Vec::new();
    let mut symbols = Vec::new();
    let mut impls = Vec::new();
    let mut pending_forewords = Vec::new();
    let mut pending_availability = Vec::new();
    let mut index = 0usize;
    while index < entries.len() {
        let entry = &entries[index];
        if let Some(foreword) = parse_foreword_app(&entry.text, entry.span)? {
            pending_forewords.push(foreword);
            index += 1;
            continue;
        }
        if let Some(attachment) = parse_availability_attachment(entry)? {
            if !module_has_following_availability_target(&entries, index)? {
                // Fall through so ordinary path-like statements keep their existing meaning.
            } else {
                pending_availability.push(attachment);
                index += 1;
                continue;
            }
        }
        if parse_cleanup_footer_entry(entry)?.is_some() {
            return Err(format!(
                "{}:{}: cleanup footer without a valid owning header",
                entry.span.line, entry.span.column
            ));
        }
        if let Some(alias) = parse_foreword_alias_decl(&entry.text, entry.span)? {
            if !pending_availability.is_empty() {
                let span = pending_availability[0].span;
                return Err(format!(
                    "{}:{}: availability attachments cannot target foreword aliases",
                    span.line, span.column
                ));
            }
            if !pending_forewords.is_empty() {
                let foreword = &pending_forewords[0];
                return Err(format!(
                    "{}:{}: forewords cannot target foreword aliases",
                    foreword.span.line, foreword.span.column
                ));
            }
            foreword_aliases.push(alias);
            index += 1;
            continue;
        }
        if let Some(handler) = parse_foreword_handler_decl(entry)? {
            if !pending_availability.is_empty() {
                let span = pending_availability[0].span;
                return Err(format!(
                    "{}:{}: availability attachments cannot target foreword handlers",
                    span.line, span.column
                ));
            }
            if !pending_forewords.is_empty() {
                let foreword = &pending_forewords[0];
                return Err(format!(
                    "{}:{}: forewords cannot target foreword handlers",
                    foreword.span.line, foreword.span.column
                ));
            }
            foreword_handlers.push(handler);
            index += 1;
            continue;
        }
        if let Some(definition) = parse_foreword_definition_decl(entry)? {
            if !pending_availability.is_empty() {
                let span = pending_availability[0].span;
                return Err(format!(
                    "{}:{}: availability attachments cannot target foreword declarations",
                    span.line, span.column
                ));
            }
            if !pending_forewords.is_empty() {
                let foreword = &pending_forewords[0];
                return Err(format!(
                    "{}:{}: forewords cannot target foreword declarations",
                    foreword.span.line, foreword.span.column
                ));
            }
            foreword_definitions.push(definition);
            index += 1;
            continue;
        }
        if let Some(mut directive) = parse_directive(&entry.text, entry.span)? {
            directive.forewords = std::mem::take(&mut pending_forewords);
            directives.push(directive);
            index += 1;
            continue;
        }
        if let Some(lang_item) = parse_lang_item(&entry.text, entry.span)? {
            if !pending_forewords.is_empty() {
                let foreword = &pending_forewords[0];
                return Err(format!(
                    "{}:{}: forewords cannot target `lang` items in v1",
                    foreword.span.line, foreword.span.column
                ));
            }
            lang_items.push(lang_item);
            index += 1;
            continue;
        }

        if let Some(memory_spec) = parse_module_memory_spec(entry)? {
            if !pending_availability.is_empty() {
                let span = pending_availability[0].span;
                return Err(format!(
                    "{}:{}: availability attachments cannot target `Memory` module specs",
                    span.line, span.column
                ));
            }
            if !pending_forewords.is_empty() {
                let foreword = &pending_forewords[0];
                return Err(format!(
                    "{}:{}: forewords cannot target `Memory` module specs in v1",
                    foreword.span.line, foreword.span.column
                ));
            }
            memory_specs.push(memory_spec);
            index += 1;
            continue;
        }

        if let Some(callback) = parse_native_callback_decl(&entry.text, entry.span)? {
            if !pending_availability.is_empty() {
                let span = pending_availability[0].span;
                return Err(format!(
                    "{}:{}: availability attachments cannot target native callbacks",
                    span.line, span.column
                ));
            }
            if !pending_forewords.is_empty() {
                let foreword = &pending_forewords[0];
                return Err(format!(
                    "{}:{}: forewords cannot target native callbacks in v1",
                    foreword.span.line, foreword.span.column
                ));
            }
            native_callbacks.push(callback);
            index += 1;
            continue;
        }

        if let Some(shackle_decl) = parse_shackle_decl(entry)? {
            if !pending_availability.is_empty() {
                let span = pending_availability[0].span;
                return Err(format!(
                    "{}:{}: availability attachments cannot target shackle declarations",
                    span.line, span.column
                ));
            }
            if !pending_forewords.is_empty() {
                let foreword = &pending_forewords[0];
                return Err(format!(
                    "{}:{}: forewords cannot target shackle declarations in v1",
                    foreword.span.line, foreword.span.column
                ));
            }
            shackle_decls.push(shackle_decl);
            index += 1;
            continue;
        }

        if let Some(impl_decl) = parse_impl_decl(entry)? {
            if !pending_availability.is_empty() {
                let span = pending_availability[0].span;
                return Err(format!(
                    "{}:{}: availability attachments can only target block-owning headers",
                    span.line, span.column
                ));
            }
            if !pending_forewords.is_empty() {
                let foreword = &pending_forewords[0];
                return Err(format!(
                    "{}:{}: forewords cannot target `impl` blocks directly in v1",
                    foreword.span.line, foreword.span.column
                ));
            }
            impls.push(impl_decl);
            index += 1;
            continue;
        }

        if let Some(mut symbol) = parse_symbol_entry(entry)? {
            symbol.forewords = std::mem::take(&mut pending_forewords);
            symbol.availability = std::mem::take(&mut pending_availability);
            if !symbol.availability.is_empty() && !symbol_can_own_availability(&symbol) {
                let span = symbol.availability[0].span;
                return Err(format!(
                    "{}:{}: availability attachments can only target block-owning headers",
                    span.line, span.column
                ));
            }
            let (cleanup_footers, consumed) =
                collect_following_cleanup_footers(&entries, index + 1)?;
            if !cleanup_footers.is_empty() {
                if symbol_can_own_cleanup_footers(&symbol) {
                    symbol.cleanup_footers = cleanup_footers;
                } else {
                    let span = cleanup_footers[0].span;
                    return Err(format!(
                        "{}:{}: cleanup footers can only attach to owning function, behavior, or system headers",
                        span.line, span.column
                    ));
                }
            }
            symbols.push(symbol);
            index += 1 + consumed;
            continue;
        }
        if !pending_availability.is_empty() {
            let span = pending_availability[0].span;
            return Err(format!(
                "{}:{}: availability attachment without a valid target",
                span.line, span.column
            ));
        }

        return Err(format!(
            "{}:{}: unsupported top-level syntax: `{}`",
            entry.span.line, entry.span.column, entry.text
        ));
    }

    if let Some(foreword) = pending_forewords.first() {
        return Err(format!(
            "{}:{}: foreword without a valid target",
            foreword.span.line, foreword.span.column
        ));
    }
    if let Some(attachment) = pending_availability.first() {
        return Err(format!(
            "{}:{}: availability attachment without a valid target",
            attachment.span.line, attachment.span.column
        ));
    }

    let mut parsed = ParsedModule {
        line_count: line_count.max(1),
        non_empty_line_count: non_empty,
        directives,
        lang_items,
        memory_specs,
        native_callbacks,
        shackle_decls,
        foreword_definitions,
        foreword_handlers,
        foreword_aliases,
        symbols,
        impls,
    };
    validate_module_foreword_contract(&parsed)?;
    apply_only_foreword_filters(&mut parsed)?;
    validate_module_rollup_contract(&parsed)?;
    validate_module_tuple_contract(&parsed)?;
    validate_module_phrase_contract(&parsed)?;
    Ok(parsed)
}

struct AnalyzedLine<'a> {
    trimmed: &'a str,
    leading_spaces: usize,
}

impl AnalyzedLine<'_> {
    fn counts_as_non_empty(&self) -> bool {
        !self.trimmed.is_empty() && !self.trimmed.starts_with("//")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceLine {
    text: String,
    leading_spaces: usize,
    line: usize,
    counts_as_non_empty: bool,
}

fn analyze_line<'a>(line: &'a str, line_index: usize) -> Result<AnalyzedLine<'a>, String> {
    let mut leading_spaces = 0usize;
    for (column, ch) in line.chars().enumerate() {
        match ch {
            ' ' => {
                leading_spaces += 1;
                continue;
            }
            '\t' => {
                return Err(format!(
                    "{}:{}: tabs are not allowed in indentation",
                    line_index + 1,
                    column + 1
                ));
            }
            _ => break,
        }
    }

    Ok(AnalyzedLine {
        trimmed: line.trim(),
        leading_spaces,
    })
}

fn collect_block_entries(
    lines: &[SourceLine],
    start_index: usize,
    indent: usize,
) -> Result<(Vec<RawBlockEntry>, usize), String> {
    let mut index = start_index;
    let mut entries = Vec::new();

    while let Some(next_index) = next_non_empty_index(lines, index) {
        let line = &lines[next_index];
        if line.leading_spaces < indent {
            return Ok((entries, next_index));
        }
        if line.leading_spaces > indent {
            return Err(format!("{}:{}: unexpected indentation", line.line, 1));
        }

        let mut entry = RawBlockEntry {
            text: line.text.clone(),
            span: Span::new(line.line, 1),
            children: Vec::new(),
        };

        index = next_index + 1;
        if let Some(child_index) = next_non_empty_index(lines, index) {
            let child = &lines[child_index];
            if child.leading_spaces > indent {
                let (children, next_child_index) =
                    collect_block_entries(lines, child_index, child.leading_spaces)?;
                entry.children = children;
                index = next_child_index;
            }
        }

        entries.push(entry);
    }

    Ok((entries, lines.len()))
}

fn next_non_empty_index(lines: &[SourceLine], mut index: usize) -> Option<usize> {
    while index < lines.len() {
        if lines[index].counts_as_non_empty {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn parse_directive(trimmed: &str, span: Span) -> Result<Option<ModuleDirective>, String> {
    let (kind, rest) = if let Some(rest) = trimmed.strip_prefix("import ") {
        (DirectiveKind::Import, rest)
    } else if let Some(rest) = trimmed.strip_prefix("use ") {
        (DirectiveKind::Use, rest)
    } else if let Some(rest) = trimmed.strip_prefix("reexport ") {
        (DirectiveKind::Reexport, rest)
    } else {
        return Ok(None);
    };

    let (path_text, alias) = match rest.split_once(" as ") {
        Some((path, alias)) => (path, Some(alias)),
        None => (rest, None),
    };
    let path = parse_path(path_text).map_err(|detail| {
        format!(
            "{}:{}: malformed {} directive: {}",
            span.line,
            span.column,
            kind.keyword(),
            detail
        )
    })?;
    let alias = alias
        .map(str::trim)
        .filter(|alias| !alias.is_empty())
        .map(|alias| {
            if is_identifier(alias) {
                Ok(alias.to_string())
            } else {
                Err(format!(
                    "{}:{}: malformed {} directive: invalid alias `{}`",
                    span.line,
                    span.column,
                    kind.keyword(),
                    alias
                ))
            }
        })
        .transpose()?;

    Ok(Some(ModuleDirective {
        kind,
        path,
        alias,
        forewords: Vec::new(),
        span,
    }))
}

fn parse_lang_item(trimmed: &str, span: Span) -> Result<Option<LangItemDecl>, String> {
    let Some(rest) = trimmed.strip_prefix("lang ") else {
        return Ok(None);
    };
    let (name, target) = rest.split_once('=').ok_or_else(|| {
        format!(
            "{}:{}: malformed `lang` item declaration",
            span.line, span.column
        )
    })?;
    let name = name.trim();
    if !is_identifier(name) {
        return Err(format!(
            "{}:{}: malformed `lang` item declaration",
            span.line, span.column
        ));
    }
    let target = parse_path(target).map_err(|_| {
        format!(
            "{}:{}: malformed `lang` item declaration",
            span.line, span.column
        )
    })?;
    Ok(Some(LangItemDecl {
        name: name.to_string(),
        target,
        span,
    }))
}

fn parse_foreword_alias_decl(
    trimmed: &str,
    span: Span,
) -> Result<Option<ForewordAliasDecl>, String> {
    let (kind, rest) = if let Some(rest) = trimmed.strip_prefix("foreword alias ") {
        (ForewordAliasKind::Alias, rest)
    } else if let Some(rest) = trimmed.strip_prefix("foreword reexport ") {
        (ForewordAliasKind::Reexport, rest)
    } else {
        return Ok(None);
    };
    let (source_name, alias_name) = if let Some((alias_name, source_name)) = rest.split_once(" = ")
    {
        (source_name, alias_name)
    } else if let Some((source_name, alias_name)) = rest.split_once(" as ") {
        (source_name, alias_name)
    } else {
        return Err(format!(
            "{}:{}: malformed foreword {} declaration",
            span.line,
            span.column,
            kind.as_str()
        ));
    };
    let source_name = parse_path(source_name).map_err(|_| {
        format!(
            "{}:{}: malformed foreword {} declaration",
            span.line,
            span.column,
            kind.as_str()
        )
    })?;
    let alias_name = parse_path(alias_name).map_err(|_| {
        format!(
            "{}:{}: malformed foreword {} declaration",
            span.line,
            span.column,
            kind.as_str()
        )
    })?;
    Ok(Some(ForewordAliasDecl {
        kind,
        source_name,
        alias_name,
        span,
    }))
}

fn parse_foreword_field_entries(
    entries: &[RawBlockEntry],
    label: &str,
    span: Span,
) -> Result<BTreeMap<String, (String, Span)>, String> {
    if entries.is_empty() {
        return Err(format!(
            "{}:{}: `{label}` is missing its declaration body",
            span.line, span.column
        ));
    }
    let mut fields = BTreeMap::new();
    for entry in entries {
        if !entry.children.is_empty() {
            return Err(format!(
                "{}:{}: `{label}` fields cannot own nested blocks",
                entry.span.line, entry.span.column
            ));
        }
        let Some((key, value)) = entry.text.split_once('=') else {
            return Err(format!(
                "{}:{}: malformed `{label}` field `{}`",
                entry.span.line, entry.span.column, entry.text
            ));
        };
        let key = key.trim();
        let value = value.trim();
        if !is_identifier(key) || value.is_empty() {
            return Err(format!(
                "{}:{}: malformed `{label}` field `{}`",
                entry.span.line, entry.span.column, entry.text
            ));
        }
        if fields
            .insert(key.to_string(), (value.to_string(), entry.span))
            .is_some()
        {
            return Err(format!(
                "{}:{}: duplicate `{label}` field `{key}`",
                entry.span.line, entry.span.column
            ));
        }
    }
    Ok(fields)
}

fn parse_foreword_scalar_text(value: &str) -> Option<String> {
    if let Some(text) = unquote_double_quoted_literal(value) {
        return Some(text.to_string());
    }
    is_path_like(value).then(|| value.trim().to_string())
}

fn parse_foreword_bool_field(value: &str, span: Span, label: &str) -> Result<bool, String> {
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!(
            "{}:{}: `{label}` must be `true` or `false`",
            span.line, span.column
        )),
    }
}

fn parse_foreword_path_field(value: &str, span: Span, label: &str) -> Result<Vec<String>, String> {
    parse_path(value.trim()).map_err(|_| {
        format!(
            "{}:{}: `{label}` must be a qualified path",
            span.line, span.column
        )
    })
}

fn parse_foreword_path_array(
    value: &str,
    span: Span,
    label: &str,
) -> Result<Vec<Vec<String>>, String> {
    let inner = value
        .trim()
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
        .ok_or_else(|| {
            format!(
                "{}:{}: `{label}` must be a `[ ... ]` list",
                span.line, span.column
            )
        })?;
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    split_top_level(inner, ',')
        .into_iter()
        .map(|part| parse_foreword_path_field(part.trim(), span, label))
        .collect()
}

fn parse_foreword_target_name(name: &str) -> Option<ForewordDefinitionTarget> {
    match name.trim() {
        "import" => Some(ForewordDefinitionTarget::Import),
        "reexport" => Some(ForewordDefinitionTarget::Reexport),
        "use" => Some(ForewordDefinitionTarget::Use),
        "fn" => Some(ForewordDefinitionTarget::Function),
        "record" => Some(ForewordDefinitionTarget::Record),
        "obj" => Some(ForewordDefinitionTarget::Object),
        "owner" => Some(ForewordDefinitionTarget::Owner),
        "enum" => Some(ForewordDefinitionTarget::Enum),
        "opaque_type" => Some(ForewordDefinitionTarget::OpaqueType),
        "trait" => Some(ForewordDefinitionTarget::Trait),
        "behavior" => Some(ForewordDefinitionTarget::Behavior),
        "system" => Some(ForewordDefinitionTarget::System),
        "const" => Some(ForewordDefinitionTarget::Const),
        "trait_method" => Some(ForewordDefinitionTarget::TraitMethod),
        "impl_method" => Some(ForewordDefinitionTarget::ImplMethod),
        "field" => Some(ForewordDefinitionTarget::Field),
        "param" => Some(ForewordDefinitionTarget::Param),
        _ => None,
    }
}

fn parse_foreword_target_array(
    value: &str,
    span: Span,
) -> Result<Vec<ForewordDefinitionTarget>, String> {
    let inner = value
        .trim()
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
        .ok_or_else(|| {
            format!(
                "{}:{}: `targets` must be a `[ ... ]` list",
                span.line, span.column
            )
        })?;
    if inner.trim().is_empty() {
        return Err(format!(
            "{}:{}: `targets` must not be empty",
            span.line, span.column
        ));
    }
    let mut targets = Vec::new();
    for part in split_top_level(inner, ',') {
        let Some(target) = parse_foreword_target_name(part.trim()) else {
            return Err(format!(
                "{}:{}: unsupported foreword target `{}`",
                span.line,
                span.column,
                part.trim()
            ));
        };
        targets.push(target);
    }
    Ok(targets)
}

fn parse_foreword_payload_type(value: &str) -> Option<ForewordPayloadType> {
    match value.trim() {
        "Bool" => Some(ForewordPayloadType::Bool),
        "Int" => Some(ForewordPayloadType::Int),
        "Str" => Some(ForewordPayloadType::Str),
        "Symbol" => Some(ForewordPayloadType::Symbol),
        "Path" => Some(ForewordPayloadType::Path),
        _ => None,
    }
}

fn parse_foreword_payload_schema(
    value: &str,
    span: Span,
) -> Result<Vec<ForewordPayloadField>, String> {
    let inner = value
        .trim()
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
        .ok_or_else(|| {
            format!(
                "{}:{}: `payload` must be a `[ ... ]` list",
                span.line, span.column
            )
        })?;
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut fields = Vec::new();
    for part in split_top_level(inner, ',') {
        let Some((name, ty)) = part.trim().split_once(':') else {
            return Err(format!(
                "{}:{}: malformed payload field `{}`",
                span.line,
                span.column,
                part.trim()
            ));
        };
        let name = name.trim();
        let optional = name.ends_with('?');
        let name = name.trim_end_matches('?').trim();
        if !is_identifier(name) {
            return Err(format!(
                "{}:{}: malformed payload field `{}`",
                span.line,
                span.column,
                part.trim()
            ));
        }
        let Some(ty) = parse_foreword_payload_type(ty.trim()) else {
            return Err(format!(
                "{}:{}: unsupported payload field type `{}`",
                span.line,
                span.column,
                ty.trim()
            ));
        };
        fields.push(ForewordPayloadField {
            name: name.to_string(),
            optional,
            ty,
        });
    }
    Ok(fields)
}

fn parse_foreword_definition_decl(
    entry: &RawBlockEntry,
) -> Result<Option<ForewordDefinitionDecl>, String> {
    if entry.text.starts_with("foreword handler ")
        || entry.text.starts_with("foreword alias ")
        || entry.text.starts_with("foreword reexport ")
    {
        return Ok(None);
    }
    let Some(rest) = entry.text.strip_prefix("foreword ") else {
        return Ok(None);
    };
    let Some(name_text) = rest.trim().strip_suffix(':') else {
        return Err(format!(
            "{}:{}: malformed foreword declaration",
            entry.span.line, entry.span.column
        ));
    };
    let qualified_name = parse_path(name_text).map_err(|_| {
        format!(
            "{}:{}: malformed foreword declaration",
            entry.span.line, entry.span.column
        )
    })?;
    let fields = parse_foreword_field_entries(&entry.children, "foreword", entry.span)?;
    let (tier, tier_span) = fields.get("tier").ok_or_else(|| {
        format!(
            "{}:{}: `foreword` is missing required field `tier`",
            entry.span.line, entry.span.column
        )
    })?;
    let tier = match tier.trim() {
        "basic" => ForewordTier::Basic,
        "executable" => ForewordTier::Executable,
        _ => {
            return Err(format!(
                "{}:{}: `tier` must be `basic` or `executable`",
                tier_span.line, tier_span.column
            ));
        }
    };
    let visibility = match fields
        .get("visibility")
        .map(|(value, _)| value.trim())
        .unwrap_or("package")
    {
        "package" => ForewordVisibility::Package,
        "public" => ForewordVisibility::Public,
        _ => {
            let span = fields
                .get("visibility")
                .map(|(_, span)| *span)
                .unwrap_or(entry.span);
            return Err(format!(
                "{}:{}: `visibility` must be `package` or `public`",
                span.line, span.column
            ));
        }
    };
    let phase = match fields
        .get("phase")
        .map(|(value, _)| value.trim())
        .unwrap_or("frontend")
    {
        "frontend" => ForewordPhase::Frontend,
        _ => {
            let span = fields
                .get("phase")
                .map(|(_, span)| *span)
                .unwrap_or(entry.span);
            return Err(format!(
                "{}:{}: `phase` must be `frontend`",
                span.line, span.column
            ));
        }
    };
    let action = match fields
        .get("action")
        .map(|(value, _)| value.trim())
        .unwrap_or("metadata")
    {
        "metadata" => ForewordAction::Metadata,
        "transform" => ForewordAction::Transform,
        _ => {
            let span = fields
                .get("action")
                .map(|(_, span)| *span)
                .unwrap_or(entry.span);
            return Err(format!(
                "{}:{}: `action` must be `metadata` or `transform`",
                span.line, span.column
            ));
        }
    };
    let (targets, target_span) = fields.get("targets").ok_or_else(|| {
        format!(
            "{}:{}: `foreword` is missing required field `targets`",
            entry.span.line, entry.span.column
        )
    })?;
    let targets = parse_foreword_target_array(targets, *target_span)?;
    let (retention, retention_span) = fields.get("retention").ok_or_else(|| {
        format!(
            "{}:{}: `foreword` is missing required field `retention`",
            entry.span.line, entry.span.column
        )
    })?;
    let retention = match retention.trim() {
        "compile" => ForewordRetention::Compile,
        "tooling" => ForewordRetention::Tooling,
        "runtime" => ForewordRetention::Runtime,
        _ => {
            return Err(format!(
                "{}:{}: `retention` must be `compile`, `tooling`, or `runtime`",
                retention_span.line, retention_span.column
            ));
        }
    };
    let payload = fields
        .get("payload")
        .map(|(value, span)| parse_foreword_payload_schema(value, *span))
        .transpose()?
        .unwrap_or_default();
    let repeatable = fields
        .get("repeatable")
        .map(|(value, span)| parse_foreword_bool_field(value, *span, "repeatable"))
        .transpose()?
        .unwrap_or(false);
    let conflicts = fields
        .get("conflicts")
        .map(|(value, span)| parse_foreword_path_array(value, *span, "conflicts"))
        .transpose()?
        .unwrap_or_default();
    let diagnostic_namespace = fields
        .get("diagnostic_namespace")
        .map(|(value, span)| {
            parse_foreword_scalar_text(value).ok_or_else(|| {
                format!(
                    "{}:{}: `diagnostic_namespace` must be a string or symbol path",
                    span.line, span.column
                )
            })
        })
        .transpose()?;
    let handler = fields
        .get("handler")
        .map(|(value, span)| parse_foreword_path_field(value, *span, "handler"))
        .transpose()?;
    Ok(Some(ForewordDefinitionDecl {
        qualified_name,
        tier,
        visibility,
        phase,
        action,
        targets,
        retention,
        payload,
        repeatable,
        conflicts,
        diagnostic_namespace,
        handler,
        span: entry.span,
    }))
}

fn parse_foreword_handler_decl(
    entry: &RawBlockEntry,
) -> Result<Option<ForewordHandlerDecl>, String> {
    let Some(rest) = entry.text.strip_prefix("foreword handler ") else {
        return Ok(None);
    };
    let Some(name_text) = rest.trim().strip_suffix(':') else {
        return Err(format!(
            "{}:{}: malformed foreword handler declaration",
            entry.span.line, entry.span.column
        ));
    };
    let qualified_name = parse_path(name_text).map_err(|_| {
        format!(
            "{}:{}: malformed foreword handler declaration",
            entry.span.line, entry.span.column
        )
    })?;
    let fields = parse_foreword_field_entries(&entry.children, "foreword handler", entry.span)?;
    let phase = match fields
        .get("phase")
        .map(|(value, _)| value.trim())
        .unwrap_or("frontend")
    {
        "frontend" => ForewordPhase::Frontend,
        _ => {
            let span = fields
                .get("phase")
                .map(|(_, span)| *span)
                .unwrap_or(entry.span);
            return Err(format!(
                "{}:{}: `phase` must be `frontend`",
                span.line, span.column
            ));
        }
    };
    let (protocol, protocol_span) = fields.get("protocol").ok_or_else(|| {
        format!(
            "{}:{}: `foreword handler` is missing required field `protocol`",
            entry.span.line, entry.span.column
        )
    })?;
    let protocol = parse_foreword_scalar_text(protocol).ok_or_else(|| {
        format!(
            "{}:{}: `protocol` must be a string or symbol",
            protocol_span.line, protocol_span.column
        )
    })?;
    let (product, product_span) = fields.get("product").ok_or_else(|| {
        format!(
            "{}:{}: `foreword handler` is missing required field `product`",
            entry.span.line, entry.span.column
        )
    })?;
    let product = parse_foreword_scalar_text(product).ok_or_else(|| {
        format!(
            "{}:{}: `product` must be a string or symbol",
            product_span.line, product_span.column
        )
    })?;
    let (entry_name, entry_span) = fields.get("entry").ok_or_else(|| {
        format!(
            "{}:{}: `foreword handler` is missing required field `entry`",
            entry.span.line, entry.span.column
        )
    })?;
    let entry_name = parse_foreword_scalar_text(entry_name).ok_or_else(|| {
        format!(
            "{}:{}: `entry` must be a string or symbol",
            entry_span.line, entry_span.column
        )
    })?;
    Ok(Some(ForewordHandlerDecl {
        qualified_name,
        phase,
        protocol,
        product,
        entry: entry_name,
        span: entry.span,
    }))
}

fn parse_foreword_app(trimmed: &str, span: Span) -> Result<Option<ForewordApp>, String> {
    let Some(rest) = trimmed.strip_prefix('#') else {
        return Ok(None);
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return Err(format!("{}:{}: malformed foreword", span.line, span.column));
    }
    if let Some(open_idx) = rest.find('[') {
        let close_idx = find_matching_delim(rest, open_idx, '[', ']')
            .ok_or_else(|| format!("{}:{}: malformed foreword", span.line, span.column))?;
        if close_idx != rest.len() - 1 {
            return Err(format!("{}:{}: malformed foreword", span.line, span.column));
        }
        let name = rest[..open_idx].trim();
        let path = parse_path(name)
            .map_err(|_| format!("{}:{}: malformed foreword", span.line, span.column))?;
        let args = parse_foreword_args(&rest[open_idx + 1..close_idx], span)?;
        return Ok(Some(ForewordApp {
            name: path.join("."),
            path,
            args,
            span,
        }));
    }
    let path = parse_path(rest)
        .map_err(|_| format!("{}:{}: malformed foreword", span.line, span.column))?;
    Ok(Some(ForewordApp {
        name: path.join("."),
        path,
        args: Vec::new(),
        span,
    }))
}

fn split_leading_foreword_segment(source: &str) -> Option<(&str, &str)> {
    let trimmed = source.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    for (index, ch) in trimmed.char_indices() {
        if in_string {
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
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            c if c.is_whitespace() && depth == 0 => {
                return Some((&trimmed[..index], trimmed[index..].trim_start()));
            }
            _ => {}
        }
    }
    Some((trimmed, ""))
}

fn parse_leading_foreword_apps(
    source: &str,
    span: Span,
) -> Result<(Vec<ForewordApp>, String), String> {
    let mut rest = source.trim();
    let mut forewords = Vec::new();
    while let Some((token, next)) = split_leading_foreword_segment(rest) {
        let Some(foreword) = parse_foreword_app(token, span)? else {
            break;
        };
        forewords.push(foreword);
        rest = next;
    }
    Ok((forewords, rest.to_string()))
}

fn parse_foreword_args(source: &str, span: Span) -> Result<Vec<ForewordArg>, String> {
    let source = source.trim();
    if source.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = Vec::new();
    for part in split_top_level(source, ',') {
        let part = part.trim();
        if part.is_empty() {
            return Err(format!(
                "{}:{}: malformed foreword argument list",
                span.line, span.column
            ));
        }
        if let Some(index) = find_top_level_named_eq(part) {
            let name = part[..index].trim();
            let value = part[index + 1..].trim();
            if !is_identifier(name) || value.is_empty() {
                return Err(format!(
                    "{}:{}: malformed foreword argument `{part}`",
                    span.line, span.column
                ));
            }
            args.push(ForewordArg {
                name: Some(name.to_string()),
                value: value.to_string(),
                typed_value: ForewordArgValue::parse(value),
            });
        } else {
            args.push(ForewordArg {
                name: None,
                value: part.to_string(),
                typed_value: ForewordArgValue::parse(part),
            });
        }
    }
    Ok(args)
}

fn parse_path(path: &str) -> Result<Vec<String>, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("missing path".to_string());
    }

    let segments = trimmed
        .split('.')
        .map(str::trim)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if segments.iter().any(|segment| segment.is_empty()) {
        return Err(format!("invalid path `{trimmed}`"));
    }
    for segment in &segments {
        if !is_identifier(segment) {
            return Err(format!("invalid path segment `{segment}`"));
        }
    }
    Ok(segments)
}

fn parse_symbol_entry(entry: &RawBlockEntry) -> Result<Option<SymbolDecl>, String> {
    validate_raw_function_header_tuple_contract(&entry.text, entry.span)?;
    let Some(mut symbol) = parse_symbol_header(&entry.text, entry.span) else {
        let rest = entry
            .text
            .strip_prefix("export ")
            .unwrap_or(&entry.text)
            .trim();
        if rest.starts_with("intrinsic ") {
            return Err(format!(
                "{}:{}: malformed intrinsic function declaration",
                entry.span.line, entry.span.column
            ));
        }
        if rest.starts_with("native fn ") || rest.starts_with("native callback ") {
            return Err(format!(
                "{}:{}: malformed native binding declaration",
                entry.span.line, entry.span.column
            ));
        }
        if rest.starts_with("shackle ") {
            return Err(format!(
                "{}:{}: malformed shackle declaration",
                entry.span.line, entry.span.column
            ));
        }
        if rest.starts_with("opaque type ") {
            return Err(format!(
                "{}:{}: malformed opaque type declaration",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(None);
    };
    if (symbol.intrinsic_impl.is_some() || symbol.native_impl.is_some())
        && !entry.children.is_empty()
    {
        return Err(format!(
            "{}:{}: intrinsic/native functions cannot own nested blocks",
            entry.span.line, entry.span.column
        ));
    }
    symbol.surface_text = collect_symbol_surface(&entry.text, &symbol.kind, &entry.children);
    symbol.body = if symbol.kind == SymbolKind::Owner {
        let (objects, context_type) = match &symbol.body {
            SymbolBody::Owner {
                objects,
                context_type,
                ..
            } => (objects.clone(), context_type.clone()),
            _ => (Vec::new(), None),
        };
        parse_owner_body(&entry.children, objects, context_type)?
    } else if symbol.kind == SymbolKind::Array {
        if !entry.children.is_empty() {
            return Err(format!(
                "{}:{}: arrays cannot own nested blocks",
                entry.span.line, entry.span.column
            ));
        }
        symbol.body.clone()
    } else {
        parse_symbol_body(&symbol.kind, &entry.children)?
    };
    symbol.statements = parse_symbol_statements(&symbol.kind, &entry.children)?;
    Ok(Some(symbol))
}

fn parse_symbol_header(trimmed: &str, span: Span) -> Option<SymbolDecl> {
    let exported = trimmed.starts_with("export ");
    let rest = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    if rest.starts_with("native callback ") {
        return None;
    }
    if let Some(symbol) = parse_intrinsic_symbol(rest, exported, span) {
        return Some(symbol);
    }
    if let Some(symbol) = parse_native_symbol(rest, exported, span) {
        return Some(symbol);
    }
    if let Some(symbol) = parse_behavior_symbol(rest, exported, span) {
        return Some(symbol);
    }
    if let Some(symbol) = parse_system_symbol(rest, exported, span) {
        return Some(symbol);
    }
    if let Some(symbol) = parse_opaque_symbol(rest, exported, span) {
        return Some(symbol);
    }
    if let Some(symbol) = parse_owner_symbol(rest, exported, span) {
        return Some(symbol);
    }
    if let Some(symbol) = parse_array_symbol(rest, exported, span) {
        return Some(symbol);
    }
    let (is_async, rest) = if let Some(rest) = rest.strip_prefix("async ") {
        (true, rest)
    } else {
        (false, rest)
    };
    for (keyword, kind) in [
        ("fn", SymbolKind::Fn),
        ("system", SymbolKind::System),
        ("record", SymbolKind::Record),
        ("struct", SymbolKind::Struct),
        ("union", SymbolKind::Union),
        ("obj", SymbolKind::Object),
        ("enum", SymbolKind::Enum),
        ("trait", SymbolKind::Trait),
        ("behavior", SymbolKind::Behavior),
        ("const", SymbolKind::Const),
    ] {
        let Some(rest) = rest.strip_prefix(keyword) else {
            continue;
        };
        let Some(rest) = rest.strip_prefix(' ') else {
            continue;
        };
        let signature = parse_symbol_signature(kind, rest, span)?;
        return Some(SymbolDecl {
            name: signature.name,
            kind,
            exported,
            is_async,
            type_params: signature.type_params,
            where_clause: signature.where_clause,
            params: signature.params,
            return_type: signature.return_type,
            behavior_attrs: Vec::new(),
            opaque_policy: None,
            availability: Vec::new(),
            forewords: Vec::new(),
            intrinsic_impl: None,
            native_impl: None,
            body: SymbolBody::None,
            statements: Vec::new(),
            cleanup_footers: Vec::new(),
            surface_text: trimmed.to_string(),
            span,
        });
    }
    None
}

fn parse_array_symbol(rest: &str, exported: bool, span: Span) -> Option<SymbolDecl> {
    let rest = rest.strip_prefix("array ")?;
    let (name, element_ty, len) = parse_array_signature(rest, span)?;
    Some(SymbolDecl {
        name,
        kind: SymbolKind::Array,
        exported,
        is_async: false,
        type_params: Vec::new(),
        where_clause: None,
        params: Vec::new(),
        return_type: None,
        behavior_attrs: Vec::new(),
        opaque_policy: None,
        availability: Vec::new(),
        forewords: Vec::new(),
        intrinsic_impl: None,
        native_impl: None,
        body: SymbolBody::Array { element_ty, len },
        statements: Vec::new(),
        cleanup_footers: Vec::new(),
        surface_text: format!("array {}", rest.trim()),
        span,
    })
}

fn parse_owner_symbol(rest: &str, exported: bool, span: Span) -> Option<SymbolDecl> {
    let rest = rest.strip_prefix("create ")?;
    let (name, objects, context_type) = parse_owner_signature(rest)?;
    Some(SymbolDecl {
        name,
        kind: SymbolKind::Owner,
        exported,
        is_async: false,
        type_params: Vec::new(),
        where_clause: None,
        params: Vec::new(),
        return_type: None,
        behavior_attrs: Vec::new(),
        opaque_policy: None,
        availability: Vec::new(),
        forewords: Vec::new(),
        intrinsic_impl: None,
        native_impl: None,
        body: SymbolBody::Owner {
            objects,
            context_type,
            exits: Vec::new(),
        },
        statements: Vec::new(),
        cleanup_footers: Vec::new(),
        surface_text: format!("create {}", rest.trim()),
        span,
    })
}

fn parse_behavior_symbol(rest: &str, exported: bool, span: Span) -> Option<SymbolDecl> {
    let open_idx = rest.find('[')?;
    if !rest[..open_idx].trim().eq("behavior") {
        return None;
    }
    let close_idx = find_matching_delim(rest, open_idx, '[', ']')?;
    let attrs = parse_behavior_attrs(&rest[open_idx + 1..close_idx]).ok()?;
    let after_attrs = rest[close_idx + 1..].trim();
    let fn_rest = after_attrs.strip_prefix("fn ")?;
    let signature = parse_symbol_signature(SymbolKind::Fn, fn_rest, span)?;
    Some(SymbolDecl {
        name: signature.name,
        kind: SymbolKind::Behavior,
        exported,
        is_async: false,
        type_params: signature.type_params,
        where_clause: signature.where_clause,
        params: signature.params,
        return_type: signature.return_type,
        behavior_attrs: attrs,
        opaque_policy: None,
        availability: Vec::new(),
        forewords: Vec::new(),
        intrinsic_impl: None,
        native_impl: None,
        body: SymbolBody::None,
        statements: Vec::new(),
        cleanup_footers: Vec::new(),
        surface_text: format!(
            "behavior[{}] fn {}",
            &rest[open_idx + 1..close_idx],
            fn_rest
        ),
        span,
    })
}

fn parse_system_symbol(rest: &str, exported: bool, span: Span) -> Option<SymbolDecl> {
    let open_idx = rest.find('[')?;
    if !rest[..open_idx].trim().eq("system") {
        return None;
    }
    let close_idx = find_matching_delim(rest, open_idx, '[', ']')?;
    let attrs = parse_behavior_attrs(&rest[open_idx + 1..close_idx]).ok()?;
    let after_attrs = rest[close_idx + 1..].trim();
    let fn_rest = after_attrs.strip_prefix("fn ")?;
    let signature = parse_symbol_signature(SymbolKind::Fn, fn_rest, span)?;
    Some(SymbolDecl {
        name: signature.name,
        kind: SymbolKind::System,
        exported,
        is_async: false,
        type_params: signature.type_params,
        where_clause: signature.where_clause,
        params: signature.params,
        return_type: signature.return_type,
        behavior_attrs: attrs,
        opaque_policy: None,
        availability: Vec::new(),
        forewords: Vec::new(),
        intrinsic_impl: None,
        native_impl: None,
        body: SymbolBody::None,
        statements: Vec::new(),
        cleanup_footers: Vec::new(),
        surface_text: format!("system[{}] fn {}", &rest[open_idx + 1..close_idx], fn_rest),
        span,
    })
}

fn parse_intrinsic_symbol(rest: &str, exported: bool, span: Span) -> Option<SymbolDecl> {
    let rest = rest.strip_prefix("intrinsic fn ")?;
    let (signature_text, binding_text) = rest.split_once('=')?;
    let binding = binding_text.trim();
    if binding.is_empty() || !is_path_like(binding) {
        return None;
    }
    let signature = parse_symbol_signature(SymbolKind::Fn, signature_text.trim(), span)?;
    Some(SymbolDecl {
        name: signature.name,
        kind: SymbolKind::Fn,
        exported,
        is_async: false,
        type_params: signature.type_params,
        where_clause: signature.where_clause,
        params: signature.params,
        return_type: signature.return_type,
        behavior_attrs: Vec::new(),
        opaque_policy: None,
        availability: Vec::new(),
        forewords: Vec::new(),
        intrinsic_impl: Some(binding.to_string()),
        native_impl: None,
        body: SymbolBody::None,
        statements: Vec::new(),
        cleanup_footers: Vec::new(),
        surface_text: format!("intrinsic fn {} = {}", signature_text.trim(), binding),
        span,
    })
}

fn parse_native_symbol(rest: &str, exported: bool, span: Span) -> Option<SymbolDecl> {
    let rest = rest.strip_prefix("native fn ")?;
    let (signature_text, binding_text) = rest.split_once('=')?;
    let binding = binding_text.trim();
    if binding.is_empty() || !is_path_like(binding) {
        return None;
    }
    let signature = parse_symbol_signature(SymbolKind::Fn, signature_text.trim(), span)?;
    Some(SymbolDecl {
        name: signature.name,
        kind: SymbolKind::Fn,
        exported,
        is_async: false,
        type_params: signature.type_params,
        where_clause: signature.where_clause,
        params: signature.params,
        return_type: signature.return_type,
        behavior_attrs: Vec::new(),
        opaque_policy: None,
        availability: Vec::new(),
        forewords: Vec::new(),
        intrinsic_impl: None,
        native_impl: Some(binding.to_string()),
        body: SymbolBody::None,
        statements: Vec::new(),
        cleanup_footers: Vec::new(),
        surface_text: format!("native fn {} = {}", signature_text.trim(), binding),
        span,
    })
}

fn parse_native_callback_decl(
    trimmed: &str,
    span: Span,
) -> Result<Option<NativeCallbackDecl>, String> {
    let Some(rest) = trimmed.strip_prefix("native callback ").map(str::trim) else {
        return Ok(None);
    };
    let (signature_text, target_text) = rest.split_once('=').ok_or_else(|| {
        format!(
            "{}:{}: malformed native callback declaration",
            span.line, span.column
        )
    })?;
    if let Some((name, callback_type)) =
        parse_native_callback_type_ref(signature_text.trim(), span)?
    {
        return Ok(Some(NativeCallbackDecl {
            name,
            params: Vec::new(),
            return_type: None,
            callback_type: Some(callback_type),
            target: parse_path(target_text.trim()).map_err(|_| {
                format!(
                    "{}:{}: malformed native callback target",
                    span.line, span.column
                )
            })?,
            span,
        }));
    }
    let signature = parse_symbol_signature(SymbolKind::Fn, signature_text.trim(), span)
        .ok_or_else(|| {
            format!(
                "{}:{}: malformed native callback declaration",
                span.line, span.column
            )
        })?;
    if !signature.type_params.is_empty() {
        return Err(format!(
            "{}:{}: native callbacks do not support type parameters",
            span.line, span.column
        ));
    }
    if signature.where_clause.is_some() {
        return Err(format!(
            "{}:{}: native callbacks do not support where clauses",
            span.line, span.column
        ));
    }
    Ok(Some(NativeCallbackDecl {
        name: signature.name,
        params: signature.params,
        return_type: signature.return_type,
        callback_type: None,
        target: parse_path(target_text.trim()).map_err(|_| {
            format!(
                "{}:{}: malformed native callback target",
                span.line, span.column
            )
        })?,
        span,
    }))
}

fn parse_native_callback_type_ref(
    header: &str,
    span: Span,
) -> Result<Option<(String, SurfaceType)>, String> {
    let Some((name_text, ty_text)) = header.split_once(':') else {
        return Ok(None);
    };
    let name = parse_symbol_name(name_text.trim()).ok_or_else(|| {
        format!(
            "{}:{}: malformed native callback declaration",
            span.line, span.column
        )
    })?;
    if name_text.trim() != name {
        return Ok(None);
    }
    let callback_type = parse_surface_type(ty_text.trim()).map_err(|_| {
        format!(
            "{}:{}: malformed native callback type reference",
            span.line, span.column
        )
    })?;
    Ok(Some((name, callback_type)))
}

fn parse_shackle_decl(entry: &RawBlockEntry) -> Result<Option<ShackleDecl>, String> {
    let trimmed = entry.text.trim();
    let exported = trimmed.starts_with("export ");
    let rest = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let Some(rest) = rest.strip_prefix("shackle ").map(str::trim) else {
        return Ok(None);
    };

    let body_entries = collect_raw_block_body_entries(&entry.children);
    let surface_text = collect_shackle_surface(trimmed, &entry.children);

    let parse_name_only = |kind: ShackleDeclKind, rest: &str| -> Result<ShackleDecl, String> {
        let (header_text, binding) = split_optional_binding(rest);
        let name = parse_symbol_name(header_text.trim()).ok_or_else(|| {
            format!(
                "{}:{}: malformed shackle declaration",
                entry.span.line, entry.span.column
            )
        })?;
        let raw_decl = shackle_raw_decl_from_surface(
            kind,
            binding.as_deref(),
            &body_entries,
            &[],
            None,
            entry.span,
        )?;
        Ok(ShackleDecl {
            exported,
            kind,
            name,
            params: Vec::new(),
            return_type: None,
            callback_type: None,
            binding,
            body_entries: body_entries.clone(),
            raw_decl,
            import_target: None,
            thunk_target: None,
            surface_text: surface_text.clone(),
            span: entry.span,
        })
    };

    if let Some(rest) = rest.strip_prefix("import fn ") {
        return parse_shackle_function_decl(
            exported,
            ShackleDeclKind::ImportFn,
            rest,
            entry.span,
            body_entries,
            surface_text,
            true,
        )
        .map(Some);
    }
    if let Some(rest) = rest.strip_prefix("fn ") {
        return parse_shackle_function_decl(
            exported,
            ShackleDeclKind::Fn,
            rest,
            entry.span,
            body_entries,
            surface_text,
            false,
        )
        .map(Some);
    }
    if let Some(rest) = rest.strip_prefix("thunk ") {
        return parse_shackle_function_decl(
            exported,
            ShackleDeclKind::Thunk,
            rest,
            entry.span,
            body_entries,
            surface_text,
            true,
        )
        .map(Some);
    }
    if let Some(rest) = rest.strip_prefix("callback ") {
        let signature = parse_symbol_signature(SymbolKind::Fn, rest.trim(), entry.span)
            .ok_or_else(|| {
                format!(
                    "{}:{}: malformed shackle callback declaration",
                    entry.span.line, entry.span.column
                )
            })?;
        if !signature.type_params.is_empty() || signature.where_clause.is_some() {
            return Err(format!(
                "{}:{}: shackle callbacks do not support type parameters or where clauses",
                entry.span.line, entry.span.column
            ));
        }
        let raw_decl = shackle_raw_decl_from_surface(
            ShackleDeclKind::Callback,
            None,
            &body_entries,
            &signature.params,
            signature.return_type.as_ref(),
            entry.span,
        )?;
        return Ok(Some(ShackleDecl {
            exported,
            kind: ShackleDeclKind::Callback,
            name: signature.name,
            params: signature.params,
            return_type: signature.return_type,
            callback_type: None,
            binding: None,
            body_entries,
            raw_decl,
            import_target: None,
            thunk_target: None,
            surface_text,
            span: entry.span,
        }));
    }
    if let Some(rest) = rest.strip_prefix("const ") {
        let (signature_text, binding) = split_required_binding(rest, entry.span)?;
        let signature =
            parse_symbol_signature(SymbolKind::Const, signature_text.trim(), entry.span)
                .ok_or_else(|| {
                    format!(
                        "{}:{}: malformed shackle const declaration",
                        entry.span.line, entry.span.column
                    )
                })?;
        return Ok(Some(ShackleDecl {
            exported,
            kind: ShackleDeclKind::Const,
            name: signature.name,
            params: Vec::new(),
            return_type: signature.return_type,
            callback_type: None,
            binding: Some(binding),
            body_entries,
            raw_decl: None,
            import_target: None,
            thunk_target: None,
            surface_text,
            span: entry.span,
        }));
    }
    if let Some(rest) = rest.strip_prefix("type ") {
        return parse_name_only(ShackleDeclKind::Type, rest).map(Some);
    }
    if let Some(rest) = rest.strip_prefix("struct ") {
        return parse_name_only(ShackleDeclKind::Struct, rest).map(Some);
    }
    if let Some(rest) = rest.strip_prefix("union ") {
        return parse_name_only(ShackleDeclKind::Union, rest).map(Some);
    }
    if let Some(rest) = rest.strip_prefix("flags ") {
        return parse_name_only(ShackleDeclKind::Flags, rest).map(Some);
    }

    Err(format!(
        "{}:{}: malformed shackle declaration",
        entry.span.line, entry.span.column
    ))
}

fn shackle_raw_decl_from_surface(
    kind: ShackleDeclKind,
    binding: Option<&str>,
    body_entries: &[String],
    params: &[ParamDecl],
    return_type: Option<&SurfaceType>,
    span: Span,
) -> Result<Option<ShackleRawDecl>, String> {
    match kind {
        ShackleDeclKind::Type => {
            let Some(binding) = binding else {
                return Ok(None);
            };
            if let Some(repr) = ArcanaCabiBindingScalarType::parse(binding)
                && !body_entries.is_empty()
            {
                return Ok(Some(ShackleRawDecl::Enum {
                    repr,
                    variants: parse_shackle_enum_variants(body_entries, span)?,
                }));
            }
            if let Some((element_type, len)) = parse_shackle_fixed_array_type_expr(binding)? {
                return Ok(Some(ShackleRawDecl::Array { element_type, len }));
            }
            Ok(Some(ShackleRawDecl::Alias {
                target: parse_shackle_raw_type_expr(binding)?,
            }))
        }
        ShackleDeclKind::Struct => Ok(Some(ShackleRawDecl::Struct {
            fields: body_entries
                .iter()
                .map(|line| parse_shackle_field_spec(line))
                .collect::<Result<Vec<_>, _>>()?,
        })),
        ShackleDeclKind::Union => Ok(Some(ShackleRawDecl::Union {
            fields: body_entries
                .iter()
                .map(|line| parse_shackle_field_spec(line))
                .collect::<Result<Vec<_>, _>>()?,
        })),
        ShackleDeclKind::Flags => {
            let Some(binding) = binding else {
                return Ok(None);
            };
            let Some(repr) = ArcanaCabiBindingScalarType::parse(binding) else {
                return Err(format!(
                    "{}:{}: shackle flags repr `{binding}` must be a scalar integer type",
                    span.line, span.column
                ));
            };
            Ok(Some(ShackleRawDecl::Flags { repr }))
        }
        ShackleDeclKind::Callback => Ok(Some(ShackleRawDecl::Callback {
            abi: "system".to_string(),
            params: params
                .iter()
                .map(|param| parse_shackle_surface_type_raw_type(&param.ty))
                .collect::<Result<Vec<_>, _>>()?,
            return_type: return_type
                .map(parse_shackle_surface_type_raw_type)
                .transpose()?
                .unwrap_or(ArcanaCabiBindingRawType::Void),
        })),
        ShackleDeclKind::Const
        | ShackleDeclKind::ImportFn
        | ShackleDeclKind::Fn
        | ShackleDeclKind::Thunk => Ok(None),
    }
}

fn parse_shackle_import_target(binding: &str) -> Result<ShackleImportTarget, String> {
    let (library, symbol) = binding.split_once('.').ok_or_else(|| {
        format!("shackle import binding `{binding}` must be `<library>.<symbol>`")
    })?;
    if library.trim().is_empty() || symbol.trim().is_empty() {
        return Err(format!(
            "shackle import binding `{binding}` must use non-empty library and symbol names"
        ));
    }
    Ok(ShackleImportTarget {
        library: library.trim().to_string(),
        symbol: symbol.trim().to_string(),
        abi: "system".to_string(),
    })
}

fn parse_shackle_enum_variants(
    body_entries: &[String],
    span: Span,
) -> Result<Vec<ShackleEnumVariantSpec>, String> {
    body_entries
        .iter()
        .map(|line| {
            let trimmed = line.trim().trim_end_matches(',');
            let (name, value_text) = trimmed.split_once('=').ok_or_else(|| {
                format!(
                    "{}:{}: malformed shackle enum variant `{trimmed}`",
                    span.line, span.column
                )
            })?;
            let name = parse_symbol_name(name.trim()).ok_or_else(|| {
                format!(
                    "{}:{}: malformed shackle enum variant name `{}`",
                    span.line,
                    span.column,
                    name.trim()
                )
            })?;
            let value = value_text.trim().parse::<i64>().map_err(|err| {
                format!(
                    "{}:{}: malformed shackle enum value `{}`: {err}",
                    span.line,
                    span.column,
                    value_text.trim()
                )
            })?;
            Ok(ShackleEnumVariantSpec { name, value })
        })
        .collect()
}

fn parse_shackle_field_spec(line: &str) -> Result<ShackleFieldSpec, String> {
    let trimmed = line.trim().trim_end_matches(',');
    let (name, ty_text) = trimmed
        .split_once(':')
        .ok_or_else(|| format!("malformed shackle field `{trimmed}`"))?;
    let name = sanitize_name(name.trim());
    let ty_text = ty_text.trim();
    if let Some((base_text, width_text)) = ty_text.rsplit_once(" bits ") {
        let bit_width = width_text
            .trim()
            .parse::<u16>()
            .map_err(|err| format!("invalid shackle bitfield width `{width_text}`: {err}"))?;
        return Ok(ShackleFieldSpec {
            name,
            ty: parse_shackle_raw_type_expr(base_text.trim())?,
            bit_width: Some(bit_width),
        });
    }
    Ok(ShackleFieldSpec {
        name,
        ty: parse_shackle_raw_type_expr(ty_text)?,
        bit_width: None,
    })
}

fn parse_shackle_fixed_array_type_expr(
    text: &str,
) -> Result<Option<(ArcanaCabiBindingRawType, usize)>, String> {
    let trimmed = text.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Ok(None);
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let Some((element_text, len_text)) = inner.rsplit_once(';') else {
        return Ok(None);
    };
    let len = len_text
        .trim()
        .parse::<usize>()
        .map_err(|err| format!("invalid fixed array length `{}`: {err}", len_text.trim()))?;
    Ok(Some((
        parse_shackle_raw_type_expr(element_text.trim())?,
        len,
    )))
}

fn parse_shackle_surface_type_raw_type(
    ty: &SurfaceType,
) -> Result<ArcanaCabiBindingRawType, String> {
    match &ty.kind {
        SurfaceTypeKind::Path(path) => {
            let rendered = ty.render();
            if let Some(scalar) = ArcanaCabiBindingScalarType::parse(
                path.segments
                    .first()
                    .map(String::as_str)
                    .unwrap_or(&rendered),
            ) {
                return Ok(ArcanaCabiBindingRawType::Scalar(scalar));
            }
            Ok(ArcanaCabiBindingRawType::Named(rendered))
        }
        SurfaceTypeKind::Apply { .. }
        | SurfaceTypeKind::Tuple(_)
        | SurfaceTypeKind::Ref { .. }
        | SurfaceTypeKind::Projection(_) => Err(format!(
            "unsupported raw shackle signature type `{}`",
            ty.render()
        )),
    }
}

fn parse_shackle_raw_type_expr(text: &str) -> Result<ArcanaCabiBindingRawType, String> {
    let trimmed = text.trim();
    if trimmed == "c_void" || trimmed == "()" {
        return Ok(ArcanaCabiBindingRawType::Void);
    }
    if let Some(scalar) = ArcanaCabiBindingScalarType::parse(trimmed) {
        return Ok(ArcanaCabiBindingRawType::Scalar(scalar));
    }
    if let Some(rest) = trimmed.strip_prefix("*mut ") {
        return Ok(ArcanaCabiBindingRawType::Pointer {
            mutable: true,
            inner: Box::new(parse_shackle_raw_type_expr(rest.trim())?),
        });
    }
    if let Some(rest) = trimmed.strip_prefix("*const ") {
        return Ok(ArcanaCabiBindingRawType::Pointer {
            mutable: false,
            inner: Box::new(parse_shackle_raw_type_expr(rest.trim())?),
        });
    }
    if let Some(function_pointer) = parse_shackle_function_pointer_type(trimmed)? {
        return Ok(function_pointer);
    }
    Ok(ArcanaCabiBindingRawType::Named(trimmed.to_string()))
}

fn parse_shackle_function_pointer_type(
    text: &str,
) -> Result<Option<ArcanaCabiBindingRawType>, String> {
    let (nullable, inner) = if text.starts_with("Option<") && text.ends_with('>') {
        (true, &text["Option<".len()..text.len() - 1])
    } else {
        (false, text)
    };
    let inner = inner.trim();
    let inner = inner.strip_prefix("unsafe ").unwrap_or(inner).trim();
    let Some(after_extern) = inner.strip_prefix("extern ") else {
        return Ok(None);
    };
    let Some((abi_text, after_abi)) = after_extern.split_once(" fn(") else {
        return Ok(None);
    };
    let Some((params_text, return_text)) = split_signature_param_section(after_abi) else {
        return Err(format!("malformed shackle function pointer `{text}`"));
    };
    let params = split_signature_params(&params_text)
        .into_iter()
        .map(|param_text| parse_shackle_raw_type_expr(&param_text))
        .collect::<Result<Vec<_>, _>>()?;
    let return_type = if return_text.trim().is_empty() {
        ArcanaCabiBindingRawType::Void
    } else {
        let return_text = return_text
            .trim()
            .strip_prefix("->")
            .map(str::trim)
            .unwrap_or("");
        if return_text.is_empty() {
            ArcanaCabiBindingRawType::Void
        } else {
            parse_shackle_raw_type_expr(return_text)?
        }
    };
    Ok(Some(ArcanaCabiBindingRawType::FunctionPointer {
        abi: abi_text.trim().trim_matches('"').to_string(),
        nullable,
        params,
        return_type: Box::new(return_type),
    }))
}

fn split_signature_params(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
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
            ',' if square_depth == 0 && paren_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        parts.push(trimmed.to_string());
    }
    parts
}

fn split_signature_param_section(text: &str) -> Option<(String, String)> {
    let mut nested_paren_depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => nested_paren_depth += 1,
            ')' => {
                if nested_paren_depth == 0 {
                    return Some((text[..index].to_string(), text[index + 1..].to_string()));
                }
                nested_paren_depth -= 1;
            }
            _ => {}
        }
    }
    None
}

fn sanitize_name(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "_".to_string()
    } else if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        format!("_{out}")
    } else {
        out
    }
}

fn split_optional_binding(source: &str) -> (&str, Option<String>) {
    match source.split_once('=') {
        Some((head, tail)) => (
            head.trim(),
            Some(tail.trim().trim_end_matches(':').trim().to_string()),
        ),
        None => (source.trim(), None),
    }
}

fn split_required_binding(source: &str, span: Span) -> Result<(&str, String), String> {
    source
        .split_once('=')
        .map(|(head, tail)| {
            (
                head.trim(),
                tail.trim().trim_end_matches(':').trim().to_string(),
            )
        })
        .ok_or_else(|| {
            format!(
                "{}:{}: shackle declaration is missing a required binding target",
                span.line, span.column
            )
        })
}

fn parse_shackle_function_decl(
    exported: bool,
    kind: ShackleDeclKind,
    source: &str,
    span: Span,
    body_entries: Vec<String>,
    surface_text: String,
    requires_binding: bool,
) -> Result<ShackleDecl, String> {
    let (signature_text, binding) = if requires_binding {
        let (head, target) = split_required_binding(source, span)?;
        (head, Some(target))
    } else {
        let (head, target) = split_optional_binding(source);
        (head, target)
    };
    let signature = parse_symbol_signature(SymbolKind::Fn, signature_text.trim(), span)
        .ok_or_else(|| {
            format!(
                "{}:{}: malformed shackle function declaration",
                span.line, span.column
            )
        })?;
    if !signature.type_params.is_empty() || signature.where_clause.is_some() {
        return Err(format!(
            "{}:{}: shackle functions do not support type parameters or where clauses",
            span.line, span.column
        ));
    }
    let import_target = if kind == ShackleDeclKind::ImportFn {
        binding
            .as_deref()
            .map(parse_shackle_import_target)
            .transpose()?
    } else {
        None
    };
    let thunk_target = if kind == ShackleDeclKind::Thunk {
        binding.as_ref().map(|target| ShackleThunkTarget {
            target: target.clone(),
            abi: "system".to_string(),
        })
    } else {
        None
    };
    Ok(ShackleDecl {
        exported,
        kind,
        name: signature.name,
        params: signature.params,
        return_type: signature.return_type,
        callback_type: None,
        binding,
        body_entries,
        raw_decl: None,
        import_target,
        thunk_target,
        surface_text,
        span,
    })
}

fn collect_raw_block_body_entries(entries: &[RawBlockEntry]) -> Vec<String> {
    let mut out = Vec::new();
    for entry in entries {
        out.push(entry.text.clone());
        out.extend(collect_raw_block_body_entries(&entry.children));
    }
    out
}

fn collect_shackle_surface(trimmed: &str, entries: &[RawBlockEntry]) -> String {
    let mut surface_lines = vec![
        trimmed
            .strip_prefix("export ")
            .unwrap_or(trimmed)
            .to_string(),
    ];
    surface_lines.extend(collect_raw_block_body_entries(entries));
    surface_lines.join("\n")
}

fn parse_opaque_symbol(rest: &str, exported: bool, span: Span) -> Option<SymbolDecl> {
    let header = rest.strip_prefix("opaque type ")?.trim();
    let name = parse_symbol_name(header)?;
    let after_name = &header[name.len()..];
    let (type_params, where_clause, remainder) = parse_type_params_and_where(after_name.trim())?;
    let remainder = remainder.trim();
    let policy_text = remainder.strip_prefix("as ")?;
    let policy = parse_opaque_type_policy(policy_text)?;
    Some(SymbolDecl {
        name,
        kind: SymbolKind::OpaqueType,
        exported,
        is_async: false,
        type_params,
        where_clause,
        params: Vec::new(),
        return_type: None,
        behavior_attrs: Vec::new(),
        opaque_policy: Some(policy),
        availability: Vec::new(),
        forewords: Vec::new(),
        intrinsic_impl: None,
        native_impl: None,
        body: SymbolBody::None,
        statements: Vec::new(),
        cleanup_footers: Vec::new(),
        surface_text: format!("opaque type {header}"),
        span,
    })
}

fn parse_opaque_type_policy(text: &str) -> Option<OpaqueTypePolicy> {
    let mut ownership = None;
    let mut boundary = None;
    for atom in split_top_level(text.trim(), ',') {
        let atom = atom.trim();
        if atom.is_empty() {
            continue;
        }
        match atom {
            "copy" => {
                if ownership.replace(OpaqueOwnershipPolicy::Copy).is_some() {
                    return None;
                }
            }
            "move" => {
                if ownership.replace(OpaqueOwnershipPolicy::Move).is_some() {
                    return None;
                }
            }
            "boundary_safe" => {
                if boundary.replace(OpaqueBoundaryPolicy::Safe).is_some() {
                    return None;
                }
            }
            "boundary_unsafe" => {
                if boundary.replace(OpaqueBoundaryPolicy::Unsafe).is_some() {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(OpaqueTypePolicy {
        ownership: ownership?,
        boundary: boundary?,
    })
}

fn collect_symbol_surface(trimmed: &str, kind: &SymbolKind, entries: &[RawBlockEntry]) -> String {
    let mut surface_lines = vec![
        trimmed
            .strip_prefix("export ")
            .unwrap_or(trimmed)
            .to_string(),
    ];

    if matches!(
        kind,
        SymbolKind::Fn
            | SymbolKind::Behavior
            | SymbolKind::System
            | SymbolKind::Const
            | SymbolKind::Owner
            | SymbolKind::OpaqueType
    ) {
        return surface_lines.join("\n");
    }

    surface_lines.extend(entries.iter().map(|entry| entry.text.clone()));
    surface_lines.join("\n")
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedSymbolSignature {
    name: String,
    type_params: Vec<String>,
    where_clause: Option<SurfaceWhereClause>,
    params: Vec<ParamDecl>,
    return_type: Option<SurfaceType>,
}

fn parse_array_signature(rest: &str, _span: Span) -> Option<(String, SurfaceType, usize)> {
    let header = rest.trim().strip_suffix(':').unwrap_or(rest.trim()).trim();
    let open_idx = header.find('[')?;
    let close_idx = find_matching_delim(header, open_idx, '[', ']')?;
    if close_idx != header.len().saturating_sub(1) {
        return None;
    }
    let name = parse_symbol_name(header[..open_idx].trim())?;
    if header[..open_idx].trim() != name {
        return None;
    }
    let inside = &header[open_idx + 1..close_idx];
    let mut parts = split_top_level(inside, ',').into_iter().map(str::trim);
    let element_ty = parse_surface_type(parts.next()?).ok()?;
    let len_text = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let len = len_text.parse::<usize>().ok()?;
    if len == 0 {
        return None;
    }
    Some((name, element_ty, len))
}

fn parse_symbol_signature(
    kind: SymbolKind,
    rest: &str,
    span: Span,
) -> Option<ParsedSymbolSignature> {
    let rest = rest.trim();
    let header = rest.strip_suffix(':').unwrap_or(rest).trim();
    let name = parse_symbol_name(header)?;
    let after_name = &header[name.len()..];
    let (type_params, where_clause, params, return_type) = match kind {
        SymbolKind::Fn | SymbolKind::System => parse_function_signature_tail(after_name, span)?,
        SymbolKind::Record
        | SymbolKind::Struct
        | SymbolKind::Union
        | SymbolKind::Object
        | SymbolKind::Enum
        | SymbolKind::Trait
        | SymbolKind::Behavior
        | SymbolKind::OpaqueType => parse_named_type_tail(after_name)?,
        SymbolKind::Array => return None,
        SymbolKind::Owner => return None,
        SymbolKind::Const => parse_const_signature_tail(after_name),
    };

    Some(ParsedSymbolSignature {
        name,
        type_params,
        where_clause,
        params,
        return_type,
    })
}

fn parse_function_signature_tail(tail: &str, span: Span) -> Option<ParsedFunctionTail> {
    let tail = tail.trim();
    let (type_params, where_clause, remainder) = parse_type_params_and_where(tail)?;
    let remainder = remainder.trim();
    let open_idx = remainder.find('(')?;
    let close_idx = find_matching_delim(remainder, open_idx, '(', ')')?;
    let params = parse_param_list(&remainder[open_idx + 1..close_idx], span).ok()?;
    let after_params = remainder[close_idx + 1..].trim();
    let return_type = after_params.strip_prefix("->").and_then(|ty| {
        let ty = ty.trim();
        (!ty.is_empty())
            .then(|| parse_surface_type(ty))
            .transpose()
            .ok()
            .flatten()
    });
    Some((type_params, where_clause, params, return_type))
}

fn parse_named_type_tail(tail: &str) -> Option<ParsedFunctionTail> {
    let (type_params, where_clause, remainder) = parse_type_params_and_where(tail.trim())?;
    if !remainder.trim().is_empty() {
        return None;
    }
    Some((type_params, where_clause, Vec::new(), None))
}

fn parse_const_signature_tail(
    tail: &str,
) -> (
    Vec<String>,
    Option<SurfaceWhereClause>,
    Vec<ParamDecl>,
    Option<SurfaceType>,
) {
    let return_type = tail.trim().strip_prefix(':').and_then(|ty| {
        let ty = ty.trim();
        (!ty.is_empty())
            .then(|| parse_surface_type(ty))
            .transpose()
            .ok()
            .flatten()
    });
    (Vec::new(), None, Vec::new(), return_type)
}

fn parse_owner_signature(
    rest: &str,
) -> Option<(String, Vec<OwnerObjectDecl>, Option<SurfaceType>)> {
    let header = rest.trim().strip_suffix(':').unwrap_or(rest.trim()).trim();
    let name = parse_symbol_name(header)?;
    let after_name = header[name.len()..].trim();
    if !after_name.starts_with('[') {
        return None;
    }
    let close_idx = find_matching_delim(after_name, 0, '[', ']')?;
    let objects_text = &after_name[1..close_idx];
    let mut remainder = after_name[close_idx + 1..].trim();
    let mut objects = Vec::new();
    for item in split_top_level(objects_text, ',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let path = parse_path(item).ok()?;
        let local_name = path.last()?.clone();
        objects.push(OwnerObjectDecl {
            type_path: path,
            local_name,
            span: Span::default(),
        });
    }
    let context_type = if let Some(context_rest) = remainder.strip_prefix("context:") {
        let (context_text, next) = context_rest.rsplit_once(" scope-exit")?;
        let context_text = context_text.trim();
        if context_text.is_empty() || !next.trim().is_empty() {
            return None;
        }
        remainder = "scope-exit";
        Some(parse_surface_type(context_text).ok()?)
    } else {
        None
    };
    if remainder != "scope-exit" {
        return None;
    }
    Some((name, objects, context_type))
}

fn parse_type_params_and_where(
    tail: &str,
) -> Option<(Vec<String>, Option<SurfaceWhereClause>, &str)> {
    let tail = tail.trim();
    let Some('[') = tail.chars().next() else {
        return Some((Vec::new(), None, tail));
    };
    let close_idx = find_matching_delim(tail, 0, '[', ']')?;
    let inside = &tail[1..close_idx];
    let mut type_params = Vec::new();
    let parts = split_top_level(inside, ',')
        .into_iter()
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let mut where_clause = None;
    let mut index = 0usize;
    while index < parts.len() {
        let part = parts[index].trim();
        if part.is_empty() {
            index += 1;
            continue;
        }
        if let Some(clause) = part.strip_prefix("where ") {
            let mut clause_parts = vec![clause.trim().to_string()];
            clause_parts.extend(parts[index + 1..].iter().cloned());
            let combined = clause_parts.join(", ");
            where_clause = Some(parse_surface_where_clause(&combined).ok()?);
            break;
        } else {
            type_params.push(part.to_string());
        }
        index += 1;
    }
    Some((type_params, where_clause, &tail[close_idx + 1..]))
}

fn parse_param_list(source: &str, span: Span) -> Result<Vec<ParamDecl>, String> {
    let source = source.trim();
    if source.is_empty() {
        return Ok(Vec::new());
    }

    let mut params = Vec::new();
    for part in split_top_level(source, ',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let (forewords, part) = parse_leading_foreword_apps(part, span)?;
        let part = part.trim();
        let (mode, rest) = if let Some(rest) = part.strip_prefix("read ") {
            (Some(ParamMode::Read), rest)
        } else if let Some(rest) = part.strip_prefix("edit ") {
            (Some(ParamMode::Edit), rest)
        } else if let Some(rest) = part.strip_prefix("take ") {
            (Some(ParamMode::Take), rest)
        } else if let Some(rest) = part.strip_prefix("hold ") {
            (Some(ParamMode::Hold), rest)
        } else {
            (None, part)
        };

        let (name, ty) = rest
            .split_once(':')
            .ok_or_else(|| format!("malformed parameter `{part}`"))?;
        let name = name.trim();
        let ty = ty.trim();
        if !is_identifier(name) || ty.is_empty() {
            return Err(format!("malformed parameter `{part}`"));
        }
        params.push(ParamDecl {
            mode,
            name: name.to_string(),
            ty: parse_surface_type(ty)
                .map_err(|message| format!("malformed parameter `{part}`: {message}"))?,
            forewords,
            span,
        });
    }

    Ok(params)
}

fn parse_behavior_attrs(source: &str) -> Result<Vec<BehaviorAttr>, String> {
    let source = source.trim();
    if source.is_empty() {
        return Ok(Vec::new());
    }

    let mut attrs = Vec::new();
    for part in split_top_level(source, ',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (name, value) = part
            .split_once('=')
            .ok_or_else(|| format!("malformed behavior attribute `{part}`"))?;
        let name = name.trim();
        let value = value.trim();
        if !is_identifier(name) || value.is_empty() {
            return Err(format!("malformed behavior attribute `{part}`"));
        }
        attrs.push(BehaviorAttr {
            name: name.to_string(),
            value: value.to_string(),
        });
    }
    Ok(attrs)
}

fn parse_impl_decl(entry: &RawBlockEntry) -> Result<Option<ImplDecl>, String> {
    let Some(mut rest) = entry.text.strip_prefix("impl") else {
        return Ok(None);
    };
    rest = rest.trim_start();
    let mut type_params = Vec::new();
    if rest.starts_with('[') {
        let close_idx = find_matching_delim(rest, 0, '[', ']').ok_or_else(|| {
            format!(
                "{}:{}: malformed impl declaration",
                entry.span.line, entry.span.column
            )
        })?;
        type_params.extend(
            split_top_level(&rest[1..close_idx], ',')
                .into_iter()
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(str::to_string),
        );
        rest = rest[close_idx + 1..].trim_start();
    }
    if rest.is_empty() {
        return Err(format!(
            "{}:{}: malformed impl declaration",
            entry.span.line, entry.span.column
        ));
    }
    let header = rest.strip_suffix(':').unwrap_or(rest).trim();
    let (trait_path, target_type) = match header.rsplit_once(" for ") {
        Some((trait_path, target_type)) => (
            parse_surface_trait_ref(trait_path.trim()).ok(),
            parse_surface_type(target_type.trim()).ok(),
        ),
        None => (None, parse_surface_type(header).ok()),
    };
    let Some(target_type) = target_type else {
        return Err(format!(
            "{}:{}: malformed impl declaration",
            entry.span.line, entry.span.column
        ));
    };
    let body_entries = entry
        .children
        .iter()
        .map(|entry| entry.text.clone())
        .collect::<Vec<_>>();
    let mut assoc_types = Vec::new();
    let mut methods = Vec::new();
    let mut pending_forewords = Vec::new();
    let mut index = 0usize;
    while index < entry.children.len() {
        let child = &entry.children[index];
        if let Some(foreword) = parse_foreword_app(&child.text, child.span)? {
            pending_forewords.push(foreword);
            index += 1;
            continue;
        }
        if parse_cleanup_footer_entry(child)?.is_some() {
            return Err(format!(
                "{}:{}: cleanup footer without a valid owning header",
                child.span.line, child.span.column
            ));
        }
        if let Some(assoc_type) = parse_impl_assoc_type_binding(&child.text, child.span) {
            if !pending_forewords.is_empty() {
                let foreword = &pending_forewords[0];
                return Err(format!(
                    "{}:{}: forewords cannot target impl assoc type bindings in v1",
                    foreword.span.line, foreword.span.column
                ));
            }
            assoc_types.push(assoc_type);
            index += 1;
            continue;
        }
        if let Some(mut method) = parse_symbol_entry(child)? {
            method.forewords = std::mem::take(&mut pending_forewords);
            let (cleanup_footers, consumed) =
                collect_following_cleanup_footers(&entry.children, index + 1)?;
            if !cleanup_footers.is_empty() {
                if symbol_can_own_cleanup_footers(&method) {
                    method.cleanup_footers = cleanup_footers;
                } else {
                    let span = method.span;
                    return Err(format!(
                        "{}:{}: cleanup footers can only attach to owning function, behavior, or system headers",
                        span.line, span.column
                    ));
                }
            }
            methods.push(method);
            index += 1 + consumed;
            continue;
        }
        return Err(format!(
            "{}:{}: unsupported `impl` item syntax: `{}`",
            child.span.line, child.span.column, child.text
        ));
    }
    if let Some(foreword) = pending_forewords.first() {
        return Err(format!(
            "{}:{}: foreword without a valid target",
            foreword.span.line, foreword.span.column
        ));
    }
    let mut surface_lines = vec![entry.text.clone()];
    surface_lines.extend(body_entries.iter().cloned());
    Ok(Some(ImplDecl {
        type_params,
        trait_path,
        target_type,
        assoc_types,
        methods,
        body_entries,
        surface_text: surface_lines.join("\n"),
        span: entry.span,
    }))
}

fn parse_symbol_body(kind: &SymbolKind, entries: &[RawBlockEntry]) -> Result<SymbolBody, String> {
    match kind {
        SymbolKind::Fn
        | SymbolKind::Const
        | SymbolKind::Behavior
        | SymbolKind::System
        | SymbolKind::Owner
        | SymbolKind::OpaqueType
        | SymbolKind::Array => {
            if matches!(kind, SymbolKind::OpaqueType) && !entries.is_empty() {
                return Err("opaque types cannot own nested blocks".to_string());
            }
            if matches!(kind, SymbolKind::Array) && !entries.is_empty() {
                return Err("arrays cannot own nested blocks".to_string());
            }
            if matches!(kind, SymbolKind::Owner) {
                let exits = entries
                    .iter()
                    .map(parse_owner_exit_decl)
                    .collect::<Result<Vec<_>, _>>()?;
                let objects = Vec::new();
                return Ok(SymbolBody::Owner {
                    objects,
                    context_type: None,
                    exits,
                });
            }
            Ok(SymbolBody::None)
        }
        SymbolKind::Record | SymbolKind::Struct | SymbolKind::Union => {
            let mut fields = Vec::new();
            let mut pending_forewords = Vec::new();
            for entry in entries {
                if let Some(foreword) = parse_foreword_app(&entry.text, entry.span)? {
                    pending_forewords.push(foreword);
                    continue;
                }
                let Some(field) = parse_field_decl(
                    &entry.text,
                    entry.span,
                    std::mem::take(&mut pending_forewords),
                ) else {
                    return Err(format!(
                        "{}:{}: unsupported `{}` item syntax: `{}`",
                        entry.span.line,
                        entry.span.column,
                        kind.as_str(),
                        entry.text
                    ));
                };
                fields.push(field);
            }
            if let Some(foreword) = pending_forewords.first() {
                return Err(format!(
                    "{}:{}: foreword without a valid target",
                    foreword.span.line, foreword.span.column
                ));
            }
            Ok(match kind {
                SymbolKind::Record => SymbolBody::Record { fields },
                SymbolKind::Struct => SymbolBody::Struct { fields },
                SymbolKind::Union => SymbolBody::Union { fields },
                _ => unreachable!(),
            })
        }
        SymbolKind::Object => {
            let mut fields = Vec::new();
            let mut methods = Vec::new();
            let mut pending_forewords = Vec::new();
            let mut index = 0usize;
            while index < entries.len() {
                let entry = &entries[index];
                if let Some(foreword) = parse_foreword_app(&entry.text, entry.span)? {
                    pending_forewords.push(foreword);
                    index += 1;
                    continue;
                }
                if parse_cleanup_footer_entry(entry)?.is_some() {
                    return Err(format!(
                        "{}:{}: cleanup footer without a valid owning header",
                        entry.span.line, entry.span.column
                    ));
                }
                if let Some(field) = parse_field_decl(
                    &entry.text,
                    entry.span,
                    std::mem::take(&mut pending_forewords),
                ) {
                    fields.push(field);
                    index += 1;
                    continue;
                }
                if let Some(mut method) = parse_symbol_entry(entry)? {
                    method.forewords = std::mem::take(&mut pending_forewords);
                    let (cleanup_footers, consumed) =
                        collect_following_cleanup_footers(entries, index + 1)?;
                    if !cleanup_footers.is_empty() {
                        if symbol_can_own_cleanup_footers(&method) {
                            method.cleanup_footers = cleanup_footers;
                        } else {
                            let span = method.span;
                            return Err(format!(
                                "{}:{}: cleanup footers can only attach to owning function, behavior, or system headers",
                                span.line, span.column
                            ));
                        }
                    }
                    methods.push(method);
                    index += 1 + consumed;
                    continue;
                }
                return Err(format!(
                    "{}:{}: unsupported `obj` item syntax: `{}`",
                    entry.span.line, entry.span.column, entry.text
                ));
            }
            if let Some(foreword) = pending_forewords.first() {
                return Err(format!(
                    "{}:{}: foreword without a valid target",
                    foreword.span.line, foreword.span.column
                ));
            }
            Ok(SymbolBody::Object { fields, methods })
        }
        SymbolKind::Enum => Ok(SymbolBody::Enum {
            variants: entries
                .iter()
                .filter_map(|entry| parse_enum_variant_decl(&entry.text, entry.span))
                .collect(),
        }),
        SymbolKind::Trait => {
            let mut assoc_types = Vec::new();
            let mut methods = Vec::new();
            let mut pending_forewords = Vec::new();
            let mut index = 0usize;
            while index < entries.len() {
                let entry = &entries[index];
                if let Some(foreword) = parse_foreword_app(&entry.text, entry.span)? {
                    pending_forewords.push(foreword);
                    index += 1;
                    continue;
                }
                if parse_cleanup_footer_entry(entry)?.is_some() {
                    return Err(format!(
                        "{}:{}: cleanup footer without a valid owning header",
                        entry.span.line, entry.span.column
                    ));
                }
                if let Some(assoc_type) = parse_trait_assoc_type_decl(&entry.text, entry.span) {
                    if !pending_forewords.is_empty() {
                        let foreword = &pending_forewords[0];
                        return Err(format!(
                            "{}:{}: forewords cannot target trait assoc type declarations in v1",
                            foreword.span.line, foreword.span.column
                        ));
                    }
                    assoc_types.push(assoc_type);
                    index += 1;
                    continue;
                }
                if let Some(mut method) = parse_symbol_entry(entry)? {
                    method.forewords = std::mem::take(&mut pending_forewords);
                    let (cleanup_footers, consumed) =
                        collect_following_cleanup_footers(entries, index + 1)?;
                    if !cleanup_footers.is_empty() {
                        if symbol_can_own_cleanup_footers(&method) {
                            method.cleanup_footers = cleanup_footers;
                        } else {
                            let span = method.span;
                            return Err(format!(
                                "{}:{}: cleanup footers can only attach to owning function, behavior, or system headers",
                                span.line, span.column
                            ));
                        }
                    }
                    methods.push(method);
                    index += 1 + consumed;
                    continue;
                }
                return Err(format!(
                    "{}:{}: unsupported `trait` item syntax: `{}`",
                    entry.span.line, entry.span.column, entry.text
                ));
            }
            if let Some(foreword) = pending_forewords.first() {
                return Err(format!(
                    "{}:{}: foreword without a valid target",
                    foreword.span.line, foreword.span.column
                ));
            }
            Ok(SymbolBody::Trait {
                assoc_types,
                methods,
            })
        }
    }
}

fn parse_symbol_statements(
    kind: &SymbolKind,
    entries: &[RawBlockEntry],
) -> Result<Vec<Statement>, String> {
    match kind {
        SymbolKind::Fn | SymbolKind::Behavior | SymbolKind::System => {
            parse_statement_block(entries, 0)
        }
        SymbolKind::Trait
        | SymbolKind::Object
        | SymbolKind::Record
        | SymbolKind::Struct
        | SymbolKind::Union
        | SymbolKind::Array
        | SymbolKind::Enum
        | SymbolKind::Const
        | SymbolKind::Owner
        | SymbolKind::OpaqueType => Ok(Vec::new()),
    }
}

fn parse_owner_body(
    entries: &[RawBlockEntry],
    objects: Vec<OwnerObjectDecl>,
    context_type: Option<SurfaceType>,
) -> Result<SymbolBody, String> {
    let exits = entries
        .iter()
        .map(parse_owner_exit_decl)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SymbolBody::Owner {
        objects,
        context_type,
        exits,
    })
}

fn parse_owner_exit_decl(entry: &RawBlockEntry) -> Result<OwnerExitDecl, String> {
    let text = entry.text.trim();
    let (name, rest) = if let Some(rest) = text.strip_prefix("exit when ") {
        ("exit".to_string(), rest)
    } else if let Some((name, rest)) = text.split_once(": when ") {
        let name = name.trim();
        if !is_identifier(name) {
            return Err(format!(
                "{}:{}: malformed owner exit clause `{}`",
                entry.span.line, entry.span.column, entry.text
            ));
        }
        (name.to_string(), rest)
    } else {
        return Err(format!(
            "{}:{}: malformed owner exit clause `{}`",
            entry.span.line, entry.span.column, entry.text
        ));
    };
    if !entry.children.is_empty() {
        return Err(format!(
            "{}:{}: owner exit clauses cannot own nested blocks",
            entry.span.line, entry.span.column
        ));
    }
    let (condition_text, retains) = match rest.split_once(" retain ") {
        Some((condition, hold_text)) => (
            condition.trim(),
            parse_owner_retain_list(hold_text.trim(), entry.span)?,
        ),
        None => (rest.trim(), Vec::new()),
    };
    if condition_text.is_empty() {
        return Err(format!(
            "{}:{}: owner exit clause is missing a condition",
            entry.span.line, entry.span.column
        ));
    }
    Ok(OwnerExitDecl {
        name,
        condition: parse_expression(condition_text, &[], entry.span)?,
        retains,
        span: entry.span,
    })
}

fn parse_owner_retain_list(text: &str, span: Span) -> Result<Vec<String>, String> {
    let open = text
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .ok_or_else(|| format!("{}:{}: malformed owner retain list", span.line, span.column))?;
    let mut retains = Vec::new();
    for item in split_top_level(open, ',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if !is_identifier(item) {
            return Err(format!(
                "{}:{}: malformed owner retain target `{item}`",
                span.line, span.column
            ));
        }
        retains.push(item.to_string());
    }
    Ok(retains)
}

fn parse_statement_block(
    entries: &[RawBlockEntry],
    loop_depth: usize,
) -> Result<Vec<Statement>, String> {
    let mut statements = Vec::new();
    let mut pending_forewords = Vec::new();
    let mut pending_availability = Vec::new();
    let mut index = 0usize;
    while index < entries.len() {
        let entry = &entries[index];
        if let Some(foreword) = parse_foreword_app(&entry.text, entry.span)? {
            pending_forewords.push(foreword);
            index += 1;
            continue;
        }
        if let Some(attachment) = parse_availability_attachment(entry)? {
            if !statement_has_following_availability_target(entries, index, loop_depth)? {
                // Fall through so standalone statements like `break` are not consumed as attachments.
            } else {
                pending_availability.push(attachment);
                index += 1;
                continue;
            }
        }
        if entry.text == "else:" {
            return Err(format!(
                "{}:{}: `else` without a preceding `if`",
                entry.span.line, entry.span.column
            ));
        }
        if entry.text.starts_with("else ") {
            return Err(format!(
                "{}:{}: malformed `else` clause",
                entry.span.line, entry.span.column
            ));
        }
        if parse_cleanup_footer_entry(entry)?.is_some() {
            return Err(format!(
                "{}:{}: cleanup footer without a valid owning header",
                entry.span.line, entry.span.column
            ));
        }

        let mut statement = parse_statement(entry, loop_depth)?;
        statement.availability = std::mem::take(&mut pending_availability);
        if !statement.availability.is_empty() && !statement_can_own_availability(&statement) {
            let span = statement.availability[0].span;
            return Err(format!(
                "{}:{}: availability attachments can only target block-owning headers",
                span.line, span.column
            ));
        }
        statement.forewords = std::mem::take(&mut pending_forewords);
        let mut next_index = index + 1;
        if let StatementKind::If { else_branch, .. } = &mut statement.kind
            && let Some(next) = entries.get(next_index)
        {
            if next.text == "else:" {
                *else_branch = Some(parse_statement_block(&next.children, loop_depth)?);
                next_index += 1;
            } else if next.text.starts_with("else ") {
                return Err(format!(
                    "{}:{}: malformed `else` clause",
                    next.span.line, next.span.column
                ));
            }
        }

        let (cleanup_footers, consumed) = collect_following_cleanup_footers(entries, next_index)?;
        if !cleanup_footers.is_empty() {
            if statement_can_own_cleanup_footers(&statement) {
                statement.cleanup_footers = cleanup_footers;
            } else {
                let span = entries[next_index].span;
                return Err(format!(
                    "{}:{}: cleanup footers can only attach to block-owning headers",
                    span.line, span.column
                ));
            }
        }

        statements.push(statement);
        index = next_index + consumed;
    }

    if let Some(foreword) = pending_forewords.first() {
        return Err(format!(
            "{}:{}: foreword without a valid target",
            foreword.span.line, foreword.span.column
        ));
    }
    if let Some(attachment) = pending_availability.first() {
        return Err(format!(
            "{}:{}: availability attachment without a valid target",
            attachment.span.line, attachment.span.column
        ));
    }

    Ok(statements)
}

fn parse_statement(entry: &RawBlockEntry, loop_depth: usize) -> Result<Statement, String> {
    if let Some(statement) = parse_headed_region_statement(entry, loop_depth)? {
        return Ok(statement);
    }

    if let Some(rest) = entry.text.strip_prefix("if ") {
        let condition = parse_expression(
            &parse_block_header(rest, "if", entry.span)?,
            &[],
            entry.span,
        )?;
        return Ok(Statement {
            kind: StatementKind::If {
                condition,
                then_branch: parse_statement_block(&entry.children, loop_depth)?,
                else_branch: None,
            },
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        });
    }

    if let Some(rest) = entry.text.strip_prefix("while ") {
        let condition = parse_expression(
            &parse_block_header(rest, "while", entry.span)?,
            &[],
            entry.span,
        )?;
        return Ok(Statement {
            kind: StatementKind::While {
                condition,
                body: parse_statement_block(&entry.children, loop_depth + 1)?,
            },
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        });
    }

    if let Some(rest) = entry.text.strip_prefix("for ") {
        let header = parse_block_header(rest, "for", entry.span)?;
        let (binding, iterable) = header.split_once(" in ").ok_or_else(|| {
            format!(
                "{}:{}: malformed `for` statement",
                entry.span.line, entry.span.column
            )
        })?;
        let binding = binding.trim();
        let iterable = iterable.trim();
        if !is_valid_tuple_binding_pattern(binding) || iterable.is_empty() {
            return Err(format!(
                "{}:{}: malformed `for` statement",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(Statement {
            kind: StatementKind::For {
                binding: binding.to_string(),
                iterable: parse_expression(iterable, &[], entry.span)?,
                body: parse_statement_block(&entry.children, loop_depth + 1)?,
            },
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        });
    }

    if let Some(rest) = entry.text.strip_prefix("let ") {
        let (mutable, rest) = if let Some(rest) = rest.strip_prefix("mut ") {
            (true, rest)
        } else {
            (false, rest)
        };
        let (name, value) = rest.split_once('=').ok_or_else(|| {
            format!(
                "{}:{}: malformed `let` statement",
                entry.span.line, entry.span.column
            )
        })?;
        let name = name.trim();
        let value = value.trim();
        if !is_valid_tuple_binding_pattern(name) || value.is_empty() {
            return Err(format!(
                "{}:{}: malformed `let` statement",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(Statement {
            kind: StatementKind::Let {
                mutable,
                name: name.to_string(),
                value: parse_expression(value, &entry.children, entry.span)?,
            },
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        });
    }

    if let Some(rest) = entry.text.strip_prefix("return") {
        let value = match rest.trim() {
            "" if entry.children.is_empty() => None,
            "" => {
                return Err(format!(
                    "{}:{}: malformed `return` statement",
                    entry.span.line, entry.span.column
                ));
            }
            value => Some(parse_expression(value, &entry.children, entry.span)?),
        };
        return Ok(Statement {
            kind: StatementKind::Return { value },
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        });
    }

    if let Some(rest) = entry.text.strip_prefix("defer ") {
        let rest = rest.trim();
        let action = if let Some(reclaim_rest) = rest.strip_prefix("reclaim ") {
            let reclaim_rest = reclaim_rest.trim();
            if reclaim_rest.is_empty() {
                return Err(format!(
                    "{}:{}: malformed `defer reclaim` statement",
                    entry.span.line, entry.span.column
                ));
            }
            DeferAction::Reclaim {
                expr: parse_expression(reclaim_rest, &entry.children, entry.span)?,
            }
        } else {
            DeferAction::Expr {
                expr: parse_expression(rest, &entry.children, entry.span)?,
            }
        };
        return Ok(Statement {
            kind: StatementKind::Defer { action },
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        });
    }

    if let Some(rest) = entry.text.strip_prefix("reclaim ") {
        let rest = rest.trim();
        if rest.is_empty() {
            return Err(format!(
                "{}:{}: malformed `reclaim` statement",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(Statement {
            kind: StatementKind::Reclaim {
                expr: parse_expression(rest, &entry.children, entry.span)?,
            },
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        });
    }

    if entry.text == "break" {
        if loop_depth == 0 {
            return Err(format!(
                "{}:{}: `break` is only valid inside loops",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(Statement {
            kind: StatementKind::Break,
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        });
    }

    if entry.text == "continue" {
        if loop_depth == 0 {
            return Err(format!(
                "{}:{}: `continue` is only valid inside loops",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(Statement {
            kind: StatementKind::Continue,
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        });
    }

    if let Some((target, op, value)) = parse_assignment_statement(&entry.text)? {
        return Ok(Statement {
            kind: StatementKind::Assign {
                target,
                op,
                value: parse_expression(&value, &entry.children, entry.span)?,
            },
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        });
    }

    if let Some(message) = unknown_headed_region_error(entry)? {
        return Err(message);
    }

    Ok(Statement {
        kind: StatementKind::Expr {
            expr: parse_expression(&entry.text, &entry.children, entry.span)?,
        },
        availability: Vec::new(),
        forewords: Vec::new(),
        cleanup_footers: Vec::new(),
        span: entry.span,
    })
}

#[derive(Clone, Copy)]
enum HeadedModifierMode {
    Recycle,
    Bind,
    Construct,
    Memory,
}

fn unknown_headed_region_error(entry: &RawBlockEntry) -> Result<Option<String>, String> {
    if entry.children.is_empty() {
        return Ok(None);
    }
    let trimmed = entry.text.trim();
    let Some((head, _)) = split_first_token(trimmed) else {
        return Ok(None);
    };
    if !is_identifier(head)
        || is_non_headed_reserved_statement_head(head)
        || HeadedRegionHead::parse(head).is_some()
    {
        return Ok(None);
    }
    let (_, modifier) =
        parse_headed_modifier_suffix(trimmed, HeadedModifierMode::Recycle, entry.span)?;
    if modifier.is_none() {
        return Ok(None);
    }
    Ok(Some(format!(
        "{}:{}: unknown headed region head `{head}`",
        entry.span.line, entry.span.column
    )))
}

fn parse_module_memory_spec(entry: &RawBlockEntry) -> Result<Option<MemorySpecDecl>, String> {
    if parse_headed_region_head(&entry.text) != Some(HeadedRegionHead::Memory) {
        return Ok(None);
    }
    Ok(Some(parse_memory_spec_decl(entry, true)?))
}

fn parse_headed_region_statement(
    entry: &RawBlockEntry,
    loop_depth: usize,
) -> Result<Option<Statement>, String> {
    let Some(head) = parse_headed_region_head(&entry.text) else {
        return Ok(None);
    };

    if head == HeadedRegionHead::Recycle {
        let (header, default_modifier) =
            parse_headed_modifier_suffix(&entry.text, HeadedModifierMode::Recycle, entry.span)?;
        if header != head.as_str() {
            return Err(format!(
                "{}:{}: malformed `recycle` header",
                entry.span.line, entry.span.column
            ));
        }
        if entry.children.is_empty() {
            return Err(format!(
                "{}:{}: `recycle` requires an indented region body",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(Some(Statement {
            kind: StatementKind::Recycle {
                default_modifier,
                lines: entry
                    .children
                    .iter()
                    .map(|line| parse_recycle_line(line, loop_depth))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        }));
    }

    if head == HeadedRegionHead::Bind {
        let (header, default_modifier) =
            parse_headed_modifier_suffix(&entry.text, HeadedModifierMode::Bind, entry.span)?;
        if header != head.as_str() {
            return Err(format!(
                "{}:{}: malformed `bind` header",
                entry.span.line, entry.span.column
            ));
        }
        if entry.children.is_empty() {
            return Err(format!(
                "{}:{}: `bind` requires an indented region body",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(Some(Statement {
            kind: StatementKind::Bind {
                default_modifier,
                lines: entry
                    .children
                    .iter()
                    .map(parse_bind_line)
                    .collect::<Result<Vec<_>, _>>()?,
            },
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        }));
    }

    if head.is_record_like() {
        let region = parse_record_region(&entry.text, &entry.children, entry.span)?;
        if matches!(region.completion, ConstructCompletionKind::Yield) {
            return Err(format!(
                "{}:{}: `{} yield` is expression-form only",
                entry.span.line,
                entry.span.column,
                region.kind.as_str()
            ));
        }
        return Ok(Some(Statement {
            kind: StatementKind::Record(region),
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        }));
    }

    if head == HeadedRegionHead::Array {
        let region = parse_array_region(&entry.text, &entry.children, entry.span)?;
        if matches!(region.completion, ConstructCompletionKind::Yield) {
            return Err(format!(
                "{}:{}: `array yield` is expression-form only",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(Some(Statement {
            kind: StatementKind::Array(region),
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        }));
    }

    if head == HeadedRegionHead::Construct {
        let region = parse_construct_region(&entry.text, &entry.children, entry.span)?;
        if matches!(region.completion, ConstructCompletionKind::Yield) {
            return Err(format!(
                "{}:{}: `construct yield` is expression-form only",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(Some(Statement {
            kind: StatementKind::Construct(region),
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        }));
    }

    if head == HeadedRegionHead::Memory {
        return Ok(Some(Statement {
            kind: StatementKind::MemorySpec(parse_memory_spec_decl(entry, false)?),
            availability: Vec::new(),
            forewords: Vec::new(),
            cleanup_footers: Vec::new(),
            span: entry.span,
        }));
    }

    Ok(None)
}

fn parse_construct_yield_expression(
    text: &str,
    attached: &[RawBlockEntry],
    span: Span,
) -> Result<Option<Expr>, String> {
    if parse_headed_region_head(text) != Some(HeadedRegionHead::Construct) {
        return Ok(None);
    }
    if attached.is_empty() {
        return Ok(None);
    }
    let region = parse_construct_region(text, attached, span)?;
    if !matches!(region.completion, ConstructCompletionKind::Yield) {
        return Err(format!(
            "{}:{}: only `construct yield` is valid in expression position",
            span.line, span.column
        ));
    }
    Ok(Some(Expr::ConstructRegion(Box::new(region))))
}

fn parse_record_yield_expression(
    text: &str,
    attached: &[RawBlockEntry],
    span: Span,
) -> Result<Option<Expr>, String> {
    let Some(head) = parse_headed_region_head(text) else {
        return Ok(None);
    };
    if !head.is_record_like() {
        return Ok(None);
    }
    if attached.is_empty() {
        return Ok(None);
    }
    let region = parse_record_region(text, attached, span)?;
    if !matches!(region.completion, ConstructCompletionKind::Yield) {
        return Err(format!(
            "{}:{}: only `{} yield` is valid in expression position",
            span.line,
            span.column,
            region.kind.as_str()
        ));
    }
    Ok(Some(Expr::RecordRegion(Box::new(region))))
}

fn parse_array_yield_expression(
    text: &str,
    attached: &[RawBlockEntry],
    span: Span,
) -> Result<Option<Expr>, String> {
    if parse_headed_region_head(text) != Some(HeadedRegionHead::Array) || attached.is_empty() {
        return Ok(None);
    }
    let region = parse_array_region(text, attached, span)?;
    if !matches!(region.completion, ConstructCompletionKind::Yield) {
        return Err(format!(
            "{}:{}: only `array yield` is valid in expression position",
            span.line, span.column
        ));
    }
    Ok(Some(Expr::ArrayRegion(Box::new(region))))
}

fn parse_recycle_line(entry: &RawBlockEntry, loop_depth: usize) -> Result<RecycleLine, String> {
    let (text, modifier) =
        parse_headed_modifier_suffix(&entry.text, HeadedModifierMode::Recycle, entry.span)?;
    if let Some(rest) = text.strip_prefix("let ") {
        let (mutable, rest) = if let Some(rest) = rest.strip_prefix("mut ") {
            (true, rest)
        } else {
            (false, rest)
        };
        let (name, value) = rest.split_once('=').ok_or_else(|| {
            format!(
                "{}:{}: malformed `recycle` binding line",
                entry.span.line, entry.span.column
            )
        })?;
        let name = name.trim();
        let value = value.trim();
        if looks_like_tuple_binding(name) || !is_identifier(name) || value.is_empty() {
            return Err(format!(
                "{}:{}: malformed `recycle` binding line",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(RecycleLine {
            kind: RecycleLineKind::Let {
                mutable,
                name: name.to_string(),
                gate: parse_expression(value, &entry.children, entry.span)?,
            },
            modifier,
            span: entry.span,
        });
    }
    if let Some((target, op, value)) = parse_assignment_statement(&text)? {
        let AssignTarget::Name { text: name } = target else {
            return Err(format!(
                "{}:{}: `recycle` assignment lines only allow plain local names",
                entry.span.line, entry.span.column
            ));
        };
        if !matches!(op, AssignOp::Assign) {
            return Err(format!(
                "{}:{}: `recycle` assignment lines only allow `=`",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(RecycleLine {
            kind: RecycleLineKind::Assign {
                name,
                gate: parse_expression(&value, &entry.children, entry.span)?,
            },
            modifier,
            span: entry.span,
        });
    }
    if modifier.as_ref().is_some_and(|modifier| {
        matches!(
            modifier.kind,
            HeadedModifierKind::Keyword(HeadedModifierKeyword::Break)
                | HeadedModifierKind::Keyword(HeadedModifierKeyword::Continue)
        ) && loop_depth == 0
    }) {
        return Err(format!(
            "{}:{}: `break` and `continue` recycle exits are only valid inside loops",
            entry.span.line, entry.span.column
        ));
    }
    Ok(RecycleLine {
        kind: RecycleLineKind::Expr {
            gate: parse_expression(&text, &entry.children, entry.span)?,
        },
        modifier,
        span: entry.span,
    })
}

fn parse_bind_line(entry: &RawBlockEntry) -> Result<BindLine, String> {
    let (text, modifier) =
        parse_headed_modifier_suffix(&entry.text, HeadedModifierMode::Bind, entry.span)?;
    if let Some(rest) = text.strip_prefix("require ") {
        let expr = rest.trim();
        if expr.is_empty() {
            return Err(format!(
                "{}:{}: malformed `bind require` line",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(BindLine {
            kind: BindLineKind::Require {
                expr: parse_expression(expr, &entry.children, entry.span)?,
            },
            modifier,
            span: entry.span,
        });
    }
    if let Some(rest) = text.strip_prefix("let ") {
        let (mutable, rest) = if let Some(rest) = rest.strip_prefix("mut ") {
            (true, rest)
        } else {
            (false, rest)
        };
        let (name, value) = rest.split_once('=').ok_or_else(|| {
            format!(
                "{}:{}: malformed `bind` binding line",
                entry.span.line, entry.span.column
            )
        })?;
        let name = name.trim();
        let value = value.trim();
        if looks_like_tuple_binding(name) || !is_identifier(name) || value.is_empty() {
            return Err(format!(
                "{}:{}: malformed `bind` binding line",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(BindLine {
            kind: BindLineKind::Let {
                mutable,
                name: name.to_string(),
                gate: parse_expression(value, &entry.children, entry.span)?,
            },
            modifier,
            span: entry.span,
        });
    }
    if let Some((target, op, value)) = parse_assignment_statement(&text)? {
        let AssignTarget::Name { text: name } = target else {
            return Err(format!(
                "{}:{}: `bind` refinement lines only allow plain local names",
                entry.span.line, entry.span.column
            ));
        };
        if !matches!(op, AssignOp::Assign) {
            return Err(format!(
                "{}:{}: `bind` refinement lines only allow `=`",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(BindLine {
            kind: BindLineKind::Assign {
                name,
                gate: parse_expression(&value, &entry.children, entry.span)?,
            },
            modifier,
            span: entry.span,
        });
    }
    Err(format!(
        "{}:{}: invalid `bind` line",
        entry.span.line, entry.span.column
    ))
}

fn parse_construct_region(
    text: &str,
    children: &[RawBlockEntry],
    span: Span,
) -> Result<ConstructRegion, String> {
    if children.is_empty() {
        return Err(format!(
            "{}:{}: `construct` requires an indented region body",
            span.line, span.column
        ));
    }
    let Some(rest) = text.strip_prefix("construct ") else {
        return Err(format!(
            "{}:{}: malformed `construct` header",
            span.line, span.column
        ));
    };
    let (header, default_modifier) =
        parse_headed_modifier_suffix(rest, HeadedModifierMode::Construct, span)?;
    let Some((completion_text, remainder)) = split_first_token(&header) else {
        return Err(format!(
            "{}:{}: malformed `construct` header",
            span.line, span.column
        ));
    };
    let completion = match completion_text {
        "yield" => ConstructCompletionKind::Yield,
        "deliver" => ConstructCompletionKind::Deliver,
        "place" => ConstructCompletionKind::Place,
        _ => {
            return Err(format!(
                "{}:{}: malformed `construct` completion clause",
                span.line, span.column
            ));
        }
    };
    let remainder = remainder.trim();
    let (target_text, destination) = match completion {
        ConstructCompletionKind::Yield => {
            if remainder.is_empty() {
                return Err(format!(
                    "{}:{}: `construct yield` requires a constructor target",
                    span.line, span.column
                ));
            }
            (remainder, None)
        }
        ConstructCompletionKind::Deliver => {
            let arrow = find_top_level_token(remainder, "->").ok_or_else(|| {
                format!(
                    "{}:{}: `construct deliver` requires `-> <name>`",
                    span.line, span.column
                )
            })?;
            let target = remainder[..arrow].trim();
            let destination = remainder[arrow + 2..].trim();
            if target.is_empty() || !is_identifier(destination) {
                return Err(format!(
                    "{}:{}: malformed `construct deliver` clause",
                    span.line, span.column
                ));
            }
            (
                target,
                Some(ConstructDestination::Deliver {
                    name: destination.to_string(),
                }),
            )
        }
        ConstructCompletionKind::Place => {
            let arrow = find_top_level_token(remainder, "->").ok_or_else(|| {
                format!(
                    "{}:{}: `construct place` requires `-> <target>`",
                    span.line, span.column
                )
            })?;
            let target = remainder[..arrow].trim();
            let destination = remainder[arrow + 2..].trim();
            if target.is_empty() || destination.is_empty() {
                return Err(format!(
                    "{}:{}: malformed `construct place` clause",
                    span.line, span.column
                ));
            }
            (
                target,
                Some(ConstructDestination::Place {
                    target: parse_assign_target(destination)?,
                }),
            )
        }
    };
    Ok(ConstructRegion {
        completion,
        target: Box::new(parse_expression(target_text, &[], span)?),
        destination,
        default_modifier,
        lines: children
            .iter()
            .map(parse_construct_line)
            .collect::<Result<Vec<_>, _>>()?,
        span,
    })
}

fn parse_record_region(
    text: &str,
    children: &[RawBlockEntry],
    span: Span,
) -> Result<RecordRegion, String> {
    if children.is_empty() {
        return Err(format!(
            "{}:{}: `record` requires an indented region body",
            span.line, span.column
        ));
    }
    let (kind, rest) = if let Some(rest) = text.strip_prefix("record ") {
        (NominalFieldRegionKind::Record, rest)
    } else if let Some(rest) = text.strip_prefix("struct ") {
        (NominalFieldRegionKind::Struct, rest)
    } else if let Some(rest) = text.strip_prefix("union ") {
        (NominalFieldRegionKind::Union, rest)
    } else {
        return Err(format!(
            "{}:{}: malformed nominal field region header",
            span.line, span.column
        ));
    };
    let (header, default_modifier) =
        parse_headed_modifier_suffix(rest, HeadedModifierMode::Construct, span)?;
    let Some((completion_text, remainder)) = split_first_token(&header) else {
        return Err(format!(
            "{}:{}: malformed `record` header",
            span.line, span.column
        ));
    };
    let completion = match completion_text {
        "yield" => ConstructCompletionKind::Yield,
        "deliver" => ConstructCompletionKind::Deliver,
        "place" => ConstructCompletionKind::Place,
        _ => {
            return Err(format!(
                "{}:{}: malformed `{}` completion clause",
                span.line,
                span.column,
                kind.as_str()
            ));
        }
    };
    let remainder = remainder.trim();
    let (target_clause, destination) = match completion {
        ConstructCompletionKind::Yield => {
            if remainder.is_empty() {
                return Err(format!(
                    "{}:{}: `{} yield` requires a target",
                    span.line,
                    span.column,
                    kind.as_str()
                ));
            }
            (remainder, None)
        }
        ConstructCompletionKind::Deliver => {
            let arrow = find_top_level_token(remainder, "->").ok_or_else(|| {
                format!(
                    "{}:{}: `{} deliver` requires `-> <name>`",
                    span.line,
                    span.column,
                    kind.as_str()
                )
            })?;
            let target = remainder[..arrow].trim();
            let destination = remainder[arrow + 2..].trim();
            if target.is_empty() || !is_identifier(destination) {
                return Err(format!(
                    "{}:{}: malformed `{}` deliver clause",
                    span.line,
                    span.column,
                    kind.as_str()
                ));
            }
            (
                target,
                Some(ConstructDestination::Deliver {
                    name: destination.to_string(),
                }),
            )
        }
        ConstructCompletionKind::Place => {
            let arrow = find_top_level_token(remainder, "->").ok_or_else(|| {
                format!(
                    "{}:{}: `{} place` requires `-> <target>`",
                    span.line,
                    span.column,
                    kind.as_str()
                )
            })?;
            let target = remainder[..arrow].trim();
            let destination = remainder[arrow + 2..].trim();
            if target.is_empty() || destination.is_empty() {
                return Err(format!(
                    "{}:{}: malformed `{}` place clause",
                    span.line,
                    span.column,
                    kind.as_str()
                ));
            }
            (
                target,
                Some(ConstructDestination::Place {
                    target: parse_assign_target(destination)?,
                }),
            )
        }
    };
    let (target_text, base_text) = split_record_target_base_clause(target_clause);
    if target_text.is_empty() {
        return Err(format!(
            "{}:{}: malformed `{}` target clause",
            span.line,
            span.column,
            kind.as_str()
        ));
    }
    Ok(RecordRegion {
        kind,
        completion,
        target: Box::new(parse_expression(target_text, &[], span)?),
        base: base_text
            .map(|base| parse_expression(base, &[], span))
            .transpose()?
            .map(Box::new),
        destination,
        default_modifier,
        lines: children
            .iter()
            .map(|line| parse_record_line(line, kind))
            .collect::<Result<Vec<_>, _>>()?,
        span,
    })
}

fn split_record_target_base_clause(text: &str) -> (&str, Option<&str>) {
    if let Some(index) = find_top_level_keyword(text, "from") {
        let target = text[..index].trim();
        let base = text[index + "from".len()..].trim();
        if !target.is_empty() && !base.is_empty() {
            return (target, Some(base));
        }
    }
    (text.trim(), None)
}

fn parse_construct_line(entry: &RawBlockEntry) -> Result<ConstructLine, String> {
    let (text, modifier) =
        parse_headed_modifier_suffix(&entry.text, HeadedModifierMode::Construct, entry.span)?;
    let index = find_top_level_named_eq(&text).ok_or_else(|| {
        format!(
            "{}:{}: malformed `construct` contribution line",
            entry.span.line, entry.span.column
        )
    })?;
    let name = text[..index].trim();
    let value = text[index + 1..].trim();
    if !is_identifier(name) || value.is_empty() {
        return Err(format!(
            "{}:{}: malformed `construct` contribution line",
            entry.span.line, entry.span.column
        ));
    }
    Ok(ConstructLine {
        name: name.to_string(),
        value: parse_expression(value, &entry.children, entry.span)?,
        modifier,
        span: entry.span,
    })
}

fn parse_record_line(
    entry: &RawBlockEntry,
    kind: NominalFieldRegionKind,
) -> Result<ConstructLine, String> {
    let line = parse_construct_line(entry)?;
    if matches!(
        kind,
        NominalFieldRegionKind::Record | NominalFieldRegionKind::Struct
    ) && line.name == "payload"
    {
        return Err(format!(
            "{}:{}: `{}` does not accept `payload = ...` contributions",
            entry.span.line,
            entry.span.column,
            kind.as_str()
        ));
    }
    Ok(line)
}

fn parse_array_region(
    text: &str,
    children: &[RawBlockEntry],
    span: Span,
) -> Result<ArrayRegion, String> {
    if children.is_empty() {
        return Err(format!(
            "{}:{}: `array` requires an indented region body",
            span.line, span.column
        ));
    }
    let Some(rest) = text.strip_prefix("array ") else {
        return Err(format!(
            "{}:{}: malformed `array` header",
            span.line, span.column
        ));
    };
    let (header, default_modifier) =
        parse_headed_modifier_suffix(rest, HeadedModifierMode::Construct, span)?;
    let Some((completion_text, remainder)) = split_first_token(&header) else {
        return Err(format!(
            "{}:{}: malformed `array` header",
            span.line, span.column
        ));
    };
    let completion = match completion_text {
        "yield" => ConstructCompletionKind::Yield,
        "deliver" => ConstructCompletionKind::Deliver,
        "place" => ConstructCompletionKind::Place,
        _ => {
            return Err(format!(
                "{}:{}: malformed `array` completion clause",
                span.line, span.column
            ));
        }
    };
    let remainder = remainder.trim();
    let (target_clause, destination) = match completion {
        ConstructCompletionKind::Yield => {
            if remainder.is_empty() {
                return Err(format!(
                    "{}:{}: `array yield` requires an array target",
                    span.line, span.column
                ));
            }
            (remainder, None)
        }
        ConstructCompletionKind::Deliver => {
            let arrow = find_top_level_token(remainder, "->").ok_or_else(|| {
                format!(
                    "{}:{}: `array deliver` requires `-> <name>`",
                    span.line, span.column
                )
            })?;
            let target = remainder[..arrow].trim();
            let destination = remainder[arrow + 2..].trim();
            if target.is_empty() || !is_identifier(destination) {
                return Err(format!(
                    "{}:{}: malformed `array deliver` clause",
                    span.line, span.column
                ));
            }
            (
                target,
                Some(ConstructDestination::Deliver {
                    name: destination.to_string(),
                }),
            )
        }
        ConstructCompletionKind::Place => {
            let arrow = find_top_level_token(remainder, "->").ok_or_else(|| {
                format!(
                    "{}:{}: `array place` requires `-> <target>`",
                    span.line, span.column
                )
            })?;
            let target = remainder[..arrow].trim();
            let destination = remainder[arrow + 2..].trim();
            if target.is_empty() || destination.is_empty() {
                return Err(format!(
                    "{}:{}: malformed `array place` clause",
                    span.line, span.column
                ));
            }
            (
                target,
                Some(ConstructDestination::Place {
                    target: parse_assign_target(destination)?,
                }),
            )
        }
    };
    let (target_text, base_text) = split_record_target_base_clause(target_clause);
    if target_text.is_empty() {
        return Err(format!(
            "{}:{}: malformed `array` target clause",
            span.line, span.column
        ));
    }
    Ok(ArrayRegion {
        completion,
        target: Box::new(parse_expression(target_text, &[], span)?),
        base: base_text
            .map(|base| parse_expression(base, &[], span))
            .transpose()?
            .map(Box::new),
        destination,
        default_modifier,
        lines: children
            .iter()
            .map(parse_array_line)
            .collect::<Result<Vec<_>, _>>()?,
        span,
    })
}

fn parse_array_line(entry: &RawBlockEntry) -> Result<ArrayLine, String> {
    let (text, modifier) =
        parse_headed_modifier_suffix(&entry.text, HeadedModifierMode::Construct, entry.span)?;
    let Some(index_text) = text.strip_prefix('[').and_then(|rest| {
        rest.split_once(']')
            .map(|(head, tail)| (head.trim(), tail.trim()))
    }) else {
        return Err(format!(
            "{}:{}: malformed `array` contribution line",
            entry.span.line, entry.span.column
        ));
    };
    let (index_text, remainder) = index_text;
    let Some(value_text) = remainder.strip_prefix('=').map(str::trim) else {
        return Err(format!(
            "{}:{}: malformed `array` contribution line",
            entry.span.line, entry.span.column
        ));
    };
    let index = index_text.parse::<usize>().map_err(|_| {
        format!(
            "{}:{}: malformed `array` contribution line",
            entry.span.line, entry.span.column
        )
    })?;
    if value_text.is_empty() {
        return Err(format!(
            "{}:{}: malformed `array` contribution line",
            entry.span.line, entry.span.column
        ));
    }
    Ok(ArrayLine {
        index,
        value: parse_expression(value_text, &entry.children, entry.span)?,
        modifier,
        span: entry.span,
    })
}

fn parse_memory_spec_decl(
    entry: &RawBlockEntry,
    module_scope: bool,
) -> Result<MemorySpecDecl, String> {
    let Some(rest) = entry.text.strip_prefix("Memory ") else {
        return Err(format!(
            "{}:{}: malformed `Memory` header",
            entry.span.line, entry.span.column
        ));
    };
    if entry.children.is_empty() {
        return Err(format!(
            "{}:{}: `Memory` requires an indented region body",
            entry.span.line, entry.span.column
        ));
    }
    let (header, default_modifier) =
        parse_headed_modifier_suffix(rest, HeadedModifierMode::Memory, entry.span)?;
    let Some((family_text, name_text)) = split_top_level_single_colon(&header) else {
        return Err(format!(
            "{}:{}: malformed `Memory` target slot; expected `<family>:<name>`",
            entry.span.line, entry.span.column
        ));
    };
    let family = MemoryFamily::parse(family_text.trim()).ok_or_else(|| {
        format!(
            "{}:{}: invalid `Memory` family `{}`",
            entry.span.line,
            entry.span.column,
            family_text.trim()
        )
    })?;
    let name = name_text.trim();
    if !is_identifier(name) {
        return Err(format!(
            "{}:{}: malformed `Memory` target slot; expected `<family>:<name>`",
            entry.span.line, entry.span.column
        ));
    }
    let _ = module_scope;
    Ok(MemorySpecDecl {
        family,
        name: name.to_string(),
        default_modifier,
        details: entry
            .children
            .iter()
            .map(parse_memory_detail_line)
            .collect::<Result<Vec<_>, _>>()?,
        span: entry.span,
    })
}

fn parse_memory_detail_line(entry: &RawBlockEntry) -> Result<MemoryDetailLine, String> {
    let (text, modifier) =
        parse_headed_modifier_suffix(&entry.text, HeadedModifierMode::Memory, entry.span)?;
    let index = find_top_level_named_eq(&text).ok_or_else(|| {
        format!(
            "{}:{}: malformed `Memory` detail line",
            entry.span.line, entry.span.column
        )
    })?;
    let key = text[..index].trim();
    let value = text[index + 1..].trim();
    let key = MemoryDetailKey::parse(key).ok_or_else(|| {
        format!(
            "{}:{}: invalid `Memory` detail key `{}`",
            entry.span.line, entry.span.column, key
        )
    })?;
    if value.is_empty() {
        return Err(format!(
            "{}:{}: malformed `Memory` detail line",
            entry.span.line, entry.span.column
        ));
    }
    Ok(MemoryDetailLine {
        key,
        value: parse_expression(value, &entry.children, entry.span)?,
        modifier,
        span: entry.span,
    })
}

fn parse_headed_modifier_suffix(
    text: &str,
    mode: HeadedModifierMode,
    span: Span,
) -> Result<(String, Option<HeadedModifier>), String> {
    for index in find_top_level_token_positions(text, " -").into_iter().rev() {
        let head = text[..index].trim_end();
        let tail = text[index + 2..].trim();
        let Some((name, payload_text)) = split_first_token(tail) else {
            continue;
        };
        if !is_identifier(name) {
            continue;
        }
        let keyword = HeadedModifierKeyword::parse(name);
        let known = !matches!(keyword, HeadedModifierKeyword::NamedExit);
        if !known && !modifier_allows_named(mode) {
            continue;
        }
        let kind = if known {
            HeadedModifierKind::Keyword(keyword)
        } else {
            HeadedModifierKind::Name(name.to_string())
        };
        let payload = if payload_text.trim().is_empty() {
            None
        } else {
            Some(parse_expression(payload_text.trim(), &[], span)?)
        };
        return Ok((
            head.trim().to_string(),
            Some(HeadedModifier {
                kind,
                payload,
                span,
            }),
        ));
    }
    Ok((text.trim().to_string(), None))
}

fn modifier_allows_named(mode: HeadedModifierMode) -> bool {
    matches!(
        mode,
        HeadedModifierMode::Recycle | HeadedModifierMode::Memory
    )
}

fn split_first_token(text: &str) -> Option<(&str, &str)> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    for (index, ch) in trimmed.char_indices() {
        if ch.is_whitespace() {
            let name = &trimmed[..index];
            let rest = trimmed[index..].trim_start();
            return Some((name, rest));
        }
    }
    Some((trimmed, ""))
}

fn parse_headed_region_head(text: &str) -> Option<HeadedRegionHead> {
    let (head, _) = split_first_token(text.trim())?;
    HeadedRegionHead::parse(head)
}

fn is_non_headed_reserved_statement_head(head: &str) -> bool {
    matches!(
        head,
        "if" | "while" | "for" | "let" | "return" | "defer" | "break" | "continue"
    )
}

fn parse_expression(text: &str, attached: &[RawBlockEntry], span: Span) -> Result<Expr, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "{}:{}: malformed expression",
            span.line, span.column
        ));
    }
    if let Some(expr) = parse_construct_yield_expression(trimmed, attached, span)? {
        return Ok(expr);
    }
    if let Some(expr) = parse_record_yield_expression(trimmed, attached, span)? {
        return Ok(expr);
    }
    if let Some(expr) = parse_array_yield_expression(trimmed, attached, span)? {
        return Ok(expr);
    }
    if let Some(rest) = trimmed.strip_prefix("match ") {
        return parse_match_expression(rest, attached, span);
    }

    let expr = parse_expression_core(trimmed)?;
    if attached.is_empty() {
        return Ok(expr);
    }

    match expr {
        Expr::QualifiedPhrase {
            subject,
            args,
            qualifier_kind,
            qualifier,
            qualifier_type_args,
            ..
        } => Ok(Expr::QualifiedPhrase {
            subject,
            args,
            qualifier_kind,
            qualifier,
            qualifier_type_args,
            attached: parse_header_attachments(attached)?,
        }),
        Expr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
            ..
        } => Ok(Expr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
            attached: parse_header_attachments(attached)?,
        }),
        _ => Err(format!(
            "{}:{}: attached blocks are only valid on standalone qualified/memory phrase statements",
            span.line, span.column
        )),
    }
}

fn parse_expression_core(text: &str) -> Result<Expr, String> {
    let trimmed = text.trim();
    if let Some(expr) = parse_grouped_non_tuple_expression(trimmed)? {
        return Ok(expr);
    }
    if let Some(inner) = strip_group_parens(trimmed) {
        return parse_expression_core(inner);
    }
    if let Some(expr) = parse_chain_expression(trimmed)? {
        return Ok(expr);
    }
    if let Some(expr) = parse_memory_phrase(trimmed)? {
        return Ok(expr);
    }
    if let Some(expr) = parse_qualified_phrase(trimmed)? {
        return Ok(expr);
    }
    if let Some(expr) = parse_pair_expression(trimmed)? {
        return Ok(expr);
    }
    if let Some(expr) = parse_collection_literal(trimmed)? {
        return Ok(expr);
    }
    parse_range_expression(trimmed)
}

fn parse_grouped_non_tuple_expression(text: &str) -> Result<Option<Expr>, String> {
    if !text.starts_with('(') || !text.ends_with(')') {
        return Ok(None);
    }
    let Some(close_idx) = find_matching_delim(text, 0, '(', ')') else {
        return Ok(None);
    };
    if close_idx != text.len() - 1 {
        return Ok(None);
    }
    let inner = text[1..close_idx].trim();
    if inner.is_empty() || !contains_top_level_char(inner, ',') {
        return Ok(None);
    }
    parse_non_tuple_comma_expression(inner)
}

fn parse_non_tuple_comma_expression(text: &str) -> Result<Option<Expr>, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() || !contains_top_level_char(trimmed, ',') {
        return Ok(None);
    }
    let qualified_before_comma = top_level_token_precedes_first_comma(trimmed, "::");
    let memory_before_comma = top_level_token_precedes_first_comma(trimmed, ":>");
    if let Some(expr) = parse_chain_expression(trimmed)? {
        return Ok(Some(expr));
    }
    if memory_before_comma
        && let Some(expr) = parse_memory_phrase(trimmed)?
        && matches!(
            &expr,
            Expr::MemoryPhrase { constructor, .. } if is_memory_constructor_like(constructor)
        )
    {
        return Ok(Some(expr));
    }
    if qualified_before_comma
        && matches!(
            parse_qualified_phrase(trimmed)?,
            Some(Expr::QualifiedPhrase { ref qualifier, .. })
                if classify_qualified_phrase_qualifier(qualifier).is_some()
        )
    {
        return parse_qualified_phrase(trimmed);
    }
    Ok(None)
}

#[derive(Clone, Copy)]
struct BinaryOpSpec {
    token: &'static str,
    op: BinaryOp,
    keyword: bool,
}

impl BinaryOpSpec {
    const fn keyword(token: &'static str, op: BinaryOp) -> Self {
        Self {
            token,
            op,
            keyword: true,
        }
    }

    const fn symbol(token: &'static str, op: BinaryOp) -> Self {
        Self {
            token,
            op,
            keyword: false,
        }
    }
}

fn parse_range_expression(text: &str) -> Result<Expr, String> {
    if let Some(expr) = parse_range(text)? {
        return Ok(expr);
    }
    parse_logical_or_expression(text)
}

fn parse_logical_or_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_logical_and_expression,
        &[BinaryOpSpec::keyword("or", BinaryOp::Or)],
    )
}

fn parse_logical_and_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_equality_expression,
        &[BinaryOpSpec::keyword("and", BinaryOp::And)],
    )
}

fn parse_equality_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_comparison_expression,
        &[
            BinaryOpSpec::symbol("==", BinaryOp::EqEq),
            BinaryOpSpec::symbol("!=", BinaryOp::NotEq),
        ],
    )
}

fn parse_comparison_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_bit_or_expression,
        &[
            BinaryOpSpec::symbol("<=", BinaryOp::LtEq),
            BinaryOpSpec::symbol(">=", BinaryOp::GtEq),
            BinaryOpSpec::symbol("<", BinaryOp::Lt),
            BinaryOpSpec::symbol(">", BinaryOp::Gt),
        ],
    )
}

fn parse_bit_or_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_bit_xor_expression,
        &[BinaryOpSpec::symbol("|", BinaryOp::BitOr)],
    )
}

fn parse_bit_xor_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_bit_and_expression,
        &[BinaryOpSpec::symbol("^", BinaryOp::BitXor)],
    )
}

fn parse_bit_and_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_shift_expression,
        &[BinaryOpSpec::symbol("&", BinaryOp::BitAnd)],
    )
}

fn parse_shift_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_additive_expression,
        &[
            BinaryOpSpec::symbol("<<", BinaryOp::Shl),
            BinaryOpSpec::keyword("shr", BinaryOp::Shr),
        ],
    )
}

fn parse_additive_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_multiplicative_expression,
        &[
            BinaryOpSpec::symbol("+", BinaryOp::Add),
            BinaryOpSpec::symbol("-", BinaryOp::Sub),
        ],
    )
}

fn parse_multiplicative_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_unary_expression,
        &[
            BinaryOpSpec::symbol("*", BinaryOp::Mul),
            BinaryOpSpec::symbol("/", BinaryOp::Div),
            BinaryOpSpec::symbol("%", BinaryOp::Mod),
        ],
    )
}

fn parse_binary_layer(
    text: &str,
    lower: fn(&str) -> Result<Expr, String>,
    ops: &[BinaryOpSpec],
) -> Result<Expr, String> {
    if let Some((index, op, token_len)) = find_top_level_binary_op(text, ops) {
        let left = text[..index].trim();
        let right = text[index + token_len..].trim();
        if !left.is_empty() && !right.is_empty() {
            return Ok(Expr::Binary {
                left: Box::new(parse_binary_layer(left, lower, ops)?),
                op,
                right: Box::new(lower(right)?),
            });
        }
    }
    lower(text)
}

fn parse_unary_expression(text: &str) -> Result<Expr, String> {
    if let Some(expr) = parse_grouped_non_tuple_expression(text)? {
        return Ok(expr);
    }
    if let Some(inner) = strip_group_parens(text) {
        return parse_expression_core(inner);
    }
    if let Some(rest) = text.strip_prefix('&') {
        let rest = rest.trim_start();
        if let Some((op, rest)) = parse_capability_unary(rest) {
            return Ok(Expr::Unary {
                op,
                expr: Box::new(parse_unary_expression(rest)?),
            });
        }
    }
    if let Some(rest) = strip_keyword_prefix(text, "weave") {
        return Ok(Expr::Unary {
            op: UnaryOp::Weave,
            expr: Box::new(parse_unary_expression(rest)?),
        });
    }
    if let Some(rest) = strip_keyword_prefix(text, "split") {
        return Ok(Expr::Unary {
            op: UnaryOp::Split,
            expr: Box::new(parse_unary_expression(rest)?),
        });
    }
    if let Some(rest) = strip_keyword_prefix(text, "not") {
        return Ok(Expr::Unary {
            op: UnaryOp::Not,
            expr: Box::new(parse_unary_expression(rest)?),
        });
    }
    if let Some(rest) = text.strip_prefix('-') {
        let rest = rest.trim();
        if !rest.is_empty() {
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(parse_unary_expression(rest)?),
            });
        }
    }
    if let Some(rest) = text.strip_prefix('~') {
        let rest = rest.trim();
        if !rest.is_empty() {
            return Ok(Expr::Unary {
                op: UnaryOp::BitNot,
                expr: Box::new(parse_unary_expression(rest)?),
            });
        }
    }
    if let Some(rest) = text.strip_prefix('*') {
        let rest = rest.trim();
        if !rest.is_empty() {
            return Ok(Expr::Unary {
                op: UnaryOp::Deref,
                expr: Box::new(parse_unary_expression(rest)?),
            });
        }
    }
    parse_postfix_expression(text)
}

fn parse_postfix_expression(text: &str) -> Result<Expr, String> {
    if let Some(expr) = parse_chain_expression(text)? {
        return Ok(expr);
    }
    if let Some(expr) = parse_memory_phrase(text)? {
        return Ok(expr);
    }
    if let Some(expr) = parse_qualified_phrase(text)? {
        return Ok(expr);
    }
    if let Some(expr) = parse_await_expression(text)? {
        return Ok(expr);
    }
    if let Some(expr) = parse_access_expression(text)? {
        return Ok(expr);
    }
    if let Some(expr) = parse_collection_literal(text)? {
        return Ok(expr);
    }
    if let Some(expr) = parse_literal_expression(text) {
        return Ok(expr);
    }
    if let Some(expr) = parse_path_expression(text) {
        return Ok(expr);
    }
    Err(format!("unsupported expression syntax `{}`", text.trim()))
}

fn parse_literal_expression(text: &str) -> Option<Expr> {
    let trimmed = text.trim();
    match trimmed {
        "true" => Some(Expr::BoolLiteral { value: true }),
        "false" => Some(Expr::BoolLiteral { value: false }),
        _ if is_float_literal(trimmed) => Some(Expr::FloatLiteral {
            text: trimmed.to_string(),
            kind: parse_float_literal_kind(trimmed),
        }),
        _ if is_int_literal(trimmed) => Some(Expr::IntLiteral {
            text: trimmed.to_string(),
        }),
        _ if is_string_literal(trimmed) => Some(Expr::StrLiteral {
            text: trimmed.to_string(),
        }),
        _ => None,
    }
}

fn parse_path_expression(text: &str) -> Option<Expr> {
    let trimmed = text.trim();
    if is_identifier(trimmed) {
        return Some(Expr::Path {
            segments: vec![trimmed.to_string()],
        });
    }
    None
}

fn validate_chain_style(style: &str) -> Result<(), String> {
    match style {
        "forward" | "lazy" | "parallel" | "async" | "plan" | "broadcast" | "collect" => Ok(()),
        "reverse" => Err(
            "chain style `reverse` was removed; use `<style> :=<` with `<=` connectors".to_string(),
        ),
        _ => Err(format!(
            "unknown chain style `{style}`; supported: forward, lazy, parallel, async, plan, broadcast, collect"
        )),
    }
}

fn chain_style_supports_reverse_introducer(style: &str) -> bool {
    matches!(style, "forward" | "lazy" | "async" | "plan" | "collect")
}

fn chain_style_supports_reverse_connectors(style: &str) -> bool {
    matches!(style, "forward" | "lazy" | "async" | "plan" | "collect")
}

fn parse_chain_expression(text: &str) -> Result<Option<Expr>, String> {
    let (style_text, introducer, step_text) = if let Some(index) = find_top_level_token(text, ":=>")
    {
        (&text[..index], ChainIntroducer::Forward, &text[index + 3..])
    } else if let Some(index) = find_top_level_token(text, ":=<") {
        (&text[..index], ChainIntroducer::Reverse, &text[index + 3..])
    } else {
        return Ok(None);
    };
    let style = style_text.trim();
    if !is_identifier(style) {
        return Ok(None);
    }
    validate_chain_style(style)?;
    let Some(steps) = parse_chain_steps(step_text, style, introducer)? else {
        return Ok(None);
    };
    Ok(Some(Expr::Chain {
        style: style.to_string(),
        introducer,
        steps,
    }))
}

fn parse_chain_steps(
    text: &str,
    style: &str,
    introducer: ChainIntroducer,
) -> Result<Option<Vec<ChainStep>>, String> {
    let parts = tokenize_chain_steps(text)?;
    if parts.is_empty() {
        return Ok(None);
    }

    let connectors = parts
        .iter()
        .skip(1)
        .filter_map(|(incoming, _)| *incoming)
        .collect::<Vec<_>>();
    if matches!(introducer, ChainIntroducer::Reverse) {
        if !chain_style_supports_reverse_introducer(style) {
            return Err(format!(
                "chain style `{style}` does not support reverse-introduced chains"
            ));
        }
        if connectors
            .iter()
            .any(|connector| *connector != ChainConnector::Reverse)
        {
            return Err(
                "reverse-introduced chains only support reverse (`<=`) connectors in v1"
                    .to_string(),
            );
        }
    } else if let Some(first) = connectors.first() {
        if *first != ChainConnector::Forward {
            return Err(
                "forward-introduced chains must begin with a forward (`=>`) segment".to_string(),
            );
        }
        let mut changed = false;
        for connector in connectors.iter().copied() {
            match connector {
                ChainConnector::Forward if changed => {
                    return Err(
                        "chain expressions allow at most one direction change in v1".to_string()
                    );
                }
                ChainConnector::Forward => {}
                ChainConnector::Reverse => {
                    if !chain_style_supports_reverse_connectors(style) {
                        return Err(format!(
                            "chain style `{style}` does not support reverse connectors"
                        ));
                    }
                    changed = true;
                }
            }
        }
    }

    let mut steps = Vec::with_capacity(parts.len());
    for (incoming, raw_text) in parts {
        let trimmed = raw_text.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let (stage_text, bind_args) = match split_chain_with_args(trimmed)? {
            Some((stage_text, args_text)) => {
                let args = parse_chain_bind_args(args_text)?;
                (stage_text, args)
            }
            None => (trimmed, Vec::new()),
        };
        let stage = parse_expression_core(stage_text)?;
        if !is_chain_stage_expr(&stage) {
            return Err(format!("unsupported chain stage `{stage_text}`"));
        }
        steps.push(ChainStep {
            incoming,
            stage,
            bind_args,
            text: trimmed.to_string(),
        });
    }
    Ok(Some(steps))
}

fn tokenize_chain_steps(text: &str) -> Result<Vec<(Option<ChainConnector>, &str)>, String> {
    let mut parts = Vec::new();
    let mut pending = None;
    let mut start = 0usize;
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if depth_paren != 0 || depth_bracket != 0 || depth_brace != 0 {
            continue;
        }

        let incoming = if text[idx..].starts_with("=>") {
            Some(ChainConnector::Forward)
        } else if text[idx..].starts_with("<=") {
            Some(ChainConnector::Reverse)
        } else {
            None
        };
        let Some(incoming) = incoming else {
            continue;
        };
        parts.push((pending, &text[start..idx]));
        pending = Some(incoming);
        start = idx + 2;
    }

    parts.push((pending, &text[start..]));
    if parts.iter().any(|(_, part)| part.trim().is_empty()) {
        return Err("malformed chain expression".to_string());
    }
    Ok(parts)
}

fn split_chain_with_args(text: &str) -> Result<Option<(&str, &str)>, String> {
    let Some(index) = find_top_level_keyword(text, "with") else {
        return Ok(None);
    };
    let stage = text[..index].trim();
    let args = text[index + "with".len()..].trim();
    if stage.is_empty() || !args.starts_with('(') {
        return Err("malformed bound chain stage".to_string());
    }
    let Some(close_idx) = find_matching_delim(args, 0, '(', ')') else {
        return Err("malformed bound chain stage".to_string());
    };
    if close_idx != args.len() - 1 {
        return Err("malformed bound chain stage".to_string());
    }
    Ok(Some((stage, &args[1..close_idx])))
}

fn parse_chain_bind_args(text: &str) -> Result<Vec<Expr>, String> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    split_top_level(text, ',')
        .into_iter()
        .map(str::trim)
        .map(|part| {
            if part.is_empty() {
                Err("malformed bound chain stage".to_string())
            } else {
                parse_expression_core(part)
            }
        })
        .collect()
}

fn is_chain_stage_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Path { .. } => true,
        Expr::MemberAccess { expr, member } => is_identifier(member) && is_chain_stage_expr(expr),
        Expr::GenericApply { expr, .. } => is_chain_stage_expr(expr),
        _ => false,
    }
}

fn parse_memory_phrase(text: &str) -> Result<Option<Expr>, String> {
    let Some(alloc_index) = find_top_level_token(text, ":>") else {
        return Ok(None);
    };
    let Some(close_index) = find_top_level_token(text, "<:") else {
        return Ok(None);
    };
    if close_index <= alloc_index {
        return Ok(None);
    }

    let family_and_arena = text[..alloc_index].trim();
    let init_text = text[alloc_index + 2..close_index].trim();
    let constructor = text[close_index + 2..].trim();
    let Some((family, arena_text)) = split_top_level_single_colon(family_and_arena) else {
        return Ok(None);
    };
    let family = family.trim();
    let arena_text = arena_text.trim();
    if !is_identifier(family) || arena_text.is_empty() || constructor.is_empty() {
        return Ok(None);
    }
    if MemoryFamily::parse(family).is_none() {
        return Err(format!(
            "unknown memory type `{family}`; supported now: arena, frame, pool, temp, session, ring, slab"
        ));
    }
    let arena = parse_expression_core(arena_text)?;
    let init_args = match parse_phrase_args(init_text, PhraseArgContext::Memory)? {
        Some(args) => args,
        None => return Ok(None),
    };
    let constructor_expr = parse_expression_core(constructor).map_err(|_| {
        format!(
            "invalid memory phrase constructor `{constructor}`; expected path or path[type_args]"
        )
    })?;
    Ok(Some(Expr::MemoryPhrase {
        family: family.to_string(),
        arena: Box::new(arena),
        init_args,
        constructor: Box::new(constructor_expr),
        attached: Vec::new(),
    }))
}

fn parse_pair_expression(text: &str) -> Result<Option<Expr>, String> {
    let Some(parts) = tuple_parts_if_whole(text) else {
        return Ok(None);
    };
    if parts.len() != 2 || parts.iter().any(|part| part.is_empty()) {
        return Ok(None);
    }
    Ok(Some(Expr::Pair {
        left: Box::new(parse_expression_core(parts[0])?),
        right: Box::new(parse_expression_core(parts[1])?),
    }))
}

fn parse_collection_literal(text: &str) -> Result<Option<Expr>, String> {
    let trimmed = text.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Ok(None);
    }
    let Some(close_idx) = find_matching_delim(trimmed, 0, '[', ']') else {
        return Ok(None);
    };
    if close_idx != trimmed.len() - 1 {
        return Ok(None);
    }
    let inside = &trimmed[1..close_idx];
    let mut items = Vec::new();
    if !inside.trim().is_empty() {
        for part in split_top_level(inside, ',') {
            let part = part.trim();
            if part.is_empty() {
                return Ok(None);
            }
            items.push(parse_expression_core(part)?);
        }
    }
    Ok(Some(Expr::CollectionLiteral { items }))
}

fn parse_qualified_phrase(text: &str) -> Result<Option<Expr>, String> {
    let positions = find_top_level_token_positions(text, "::");
    if positions.len() < 2 {
        return Ok(None);
    }

    let first = positions[0];
    let last = *positions.last().expect("len checked above");
    let subject_text = text[..first].trim();
    let args_text = text[first + 2..last].trim();
    let qualifier = text[last + 2..].trim();
    if subject_text.is_empty() || qualifier.is_empty() || subject_text_defers_to_unary(subject_text)
    {
        return Ok(None);
    }

    let subject = parse_expression_core(subject_text)?;
    let Some((qualifier_kind, qualifier, qualifier_type_args)) =
        parse_qualified_phrase_qualifier(qualifier)
    else {
        return Ok(None);
    };
    let args = match parse_phrase_args(args_text, PhraseArgContext::Qualified)? {
        Some(args) => args,
        None => return Ok(None),
    };
    Ok(Some(Expr::QualifiedPhrase {
        subject: Box::new(subject),
        args,
        qualifier_kind,
        qualifier,
        qualifier_type_args,
        attached: Vec::new(),
    }))
}

fn parse_qualified_phrase_qualifier(
    text: &str,
) -> Option<(QualifiedPhraseQualifierKind, String, Vec<SurfaceType>)> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (base, type_args) = if let Some((base, inside)) = split_trailing_bracket_suffix(trimmed) {
        let type_args = parse_generic_arg_types(inside)?;
        (base.trim(), type_args)
    } else {
        (trimmed, Vec::new())
    };
    let kind = classify_qualified_phrase_qualifier(base)?;
    Some((kind, base.to_string(), type_args))
}

fn subject_text_defers_to_unary(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with('&')
        || trimmed.starts_with('-')
        || trimmed.starts_with('~')
        || trimmed.starts_with('*')
        || trimmed.starts_with("weave ")
        || trimmed.starts_with("split ")
        || trimmed.starts_with("not ")
}

fn parse_header_attachments(entries: &[RawBlockEntry]) -> Result<Vec<HeaderAttachment>, String> {
    let mut attachments = Vec::new();
    let mut pending_forewords = Vec::new();
    for entry in entries {
        if let Some(foreword) = parse_foreword_app(&entry.text, entry.span)? {
            pending_forewords.push(foreword);
            continue;
        }

        if let Some((name, value_text)) = split_header_attachment_named_entry(&entry.text) {
            attachments.push(HeaderAttachment::Named {
                name: name.to_string(),
                value: parse_expression(value_text, &entry.children, entry.span)?,
                forewords: std::mem::take(&mut pending_forewords),
                span: entry.span,
            });
            continue;
        }

        let expr = parse_expression(&entry.text, &entry.children, entry.span)?;
        if !matches!(expr, Expr::Chain { .. }) {
            return Err(format!(
                "{}:{}: header attached blocks only accept `name = expr` entries or chain lines",
                entry.span.line, entry.span.column
            ));
        }
        attachments.push(HeaderAttachment::Chain {
            expr,
            forewords: std::mem::take(&mut pending_forewords),
            span: entry.span,
        });
    }

    if let Some(foreword) = pending_forewords.first() {
        return Err(format!(
            "{}:{}: foreword without a valid target",
            foreword.span.line, foreword.span.column
        ));
    }

    Ok(attachments)
}

fn split_header_attachment_named_entry(text: &str) -> Option<(&str, &str)> {
    let index = find_top_level_named_eq(text)?;
    let name = text[..index].trim();
    let value = text[index + 1..].trim();
    if is_identifier(name) && !value.is_empty() {
        Some((name, value))
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PhraseArgContext {
    Qualified,
    Memory,
}

impl PhraseArgContext {
    fn arity_error(self) -> &'static str {
        match self {
            Self::Qualified => "qualified phrase allows at most 3 top-level arguments",
            Self::Memory => "memory phrase allows at most 3 top-level arguments",
        }
    }

    fn trailing_comma_error(self) -> &'static str {
        match self {
            Self::Qualified => "trailing comma is not allowed before phrase qualifier",
            Self::Memory => "trailing comma is not allowed before memory phrase qualifier",
        }
    }
}

fn parse_phrase_args(
    text: &str,
    context: PhraseArgContext,
) -> Result<Option<Vec<PhraseArg>>, String> {
    if text.is_empty() {
        return Ok(Some(Vec::new()));
    }

    if text.trim_end().ends_with(',') {
        return Err(context.trailing_comma_error().to_string());
    }

    let mut args = Vec::new();
    for part in split_top_level(text, ',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        if args.len() >= 3 {
            return Err(context.arity_error().to_string());
        }
        args.push(parse_phrase_arg(trimmed)?);
    }
    Ok(Some(args))
}

fn parse_phrase_arg(text: &str) -> Result<PhraseArg, String> {
    if let Some(index) = find_top_level_named_eq(text) {
        let name = text[..index].trim();
        let value = text[index + 1..].trim();
        if is_identifier(name) && !value.is_empty() {
            return Ok(PhraseArg::Named {
                name: name.to_string(),
                value: parse_expression_core(value)?,
            });
        }
    }

    Ok(PhraseArg::Positional(parse_expression_core(text)?))
}

fn parse_await_expression(text: &str) -> Result<Option<Expr>, String> {
    let Some(index) = find_top_level_token(text, ">>") else {
        return Ok(None);
    };
    let left = text[..index].trim();
    let right = text[index + 2..].trim();
    if left.is_empty() || right != "await" {
        return Ok(None);
    }
    Ok(Some(Expr::Await {
        expr: Box::new(parse_expression_core(left)?),
    }))
}

fn parse_access_expression(text: &str) -> Result<Option<Expr>, String> {
    if let Some((base, inside)) = split_trailing_bracket_suffix(text) {
        let base = base.trim();
        if base.is_empty() {
            return Ok(None);
        }
        if is_path_like(base)
            && let Some(type_args) = parse_generic_arg_types(inside)
        {
            return Ok(Some(Expr::GenericApply {
                expr: Box::new(parse_expression_core(base)?),
                type_args,
            }));
        }
        if let Some(projection) = parse_projection_spec(base, inside)? {
            return Ok(Some(projection));
        }
        if let Some((start, end, inclusive_end)) = parse_range_parts(inside) {
            return Ok(Some(Expr::Slice {
                expr: Box::new(parse_expression_core(base)?),
                family: ProjectionFamily::Inferred,
                start: parse_optional_range_bound(start)?,
                end: parse_optional_range_bound(end)?,
                len: None,
                stride: None,
                inclusive_end,
            }));
        }
        if should_parse_index_brackets(inside) {
            return Ok(Some(Expr::Index {
                expr: Box::new(parse_expression_core(base)?),
                index: Box::new(parse_expression_core(inside.trim())?),
            }));
        }
    }

    if let Some((base, member)) = split_member_access(text) {
        return Ok(Some(Expr::MemberAccess {
            expr: Box::new(parse_expression_core(base.trim())?),
            member: member.trim().to_string(),
        }));
    }

    Ok(None)
}

fn parse_range(text: &str) -> Result<Option<Expr>, String> {
    let Some((start, end, inclusive_end)) = parse_range_parts(text) else {
        return Ok(None);
    };
    if start.trim().is_empty() && end.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(Expr::Range {
        start: parse_optional_range_bound(start)?,
        end: parse_optional_range_bound(end)?,
        inclusive_end,
    }))
}

fn parse_optional_range_bound(text: &str) -> Result<Option<Box<Expr>>, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(Box::new(parse_logical_or_expression(trimmed)?)))
}

fn parse_range_parts(text: &str) -> Option<(&str, &str, bool)> {
    if let Some(index) = find_top_level_token(text, "..=") {
        return Some((&text[..index], &text[index + 3..], true));
    }
    find_top_level_token(text, "..").map(|index| (&text[..index], &text[index + 2..], false))
}

fn parse_projection_spec(base: &str, inside: &str) -> Result<Option<Expr>, String> {
    let trimmed = inside.trim();
    if let Some(fields) = trimmed.strip_prefix("contiguous ") {
        let named = parse_projection_named_fields(fields, "contiguous projection")?;
        let start = named
            .get("start")
            .ok_or_else(|| "contiguous projection is missing required field `start`".to_string())?;
        let end = named
            .get("end")
            .ok_or_else(|| "contiguous projection is missing required field `end`".to_string())?;
        return Ok(Some(Expr::Slice {
            expr: Box::new(parse_expression_core(base)?),
            family: ProjectionFamily::Contiguous,
            start: Some(Box::new(parse_expression_core(start)?)),
            end: Some(Box::new(parse_expression_core(end)?)),
            len: None,
            stride: None,
            inclusive_end: false,
        }));
    }
    if let Some(fields) = trimmed.strip_prefix("strided ") {
        let named = parse_projection_named_fields(fields, "strided projection")?;
        let start = named
            .get("start")
            .ok_or_else(|| "strided projection is missing required field `start`".to_string())?;
        let len = named
            .get("len")
            .ok_or_else(|| "strided projection is missing required field `len`".to_string())?;
        let stride = named
            .get("stride")
            .ok_or_else(|| "strided projection is missing required field `stride`".to_string())?;
        return Ok(Some(Expr::Slice {
            expr: Box::new(parse_expression_core(base)?),
            family: ProjectionFamily::Strided,
            start: Some(Box::new(parse_expression_core(start)?)),
            end: None,
            len: Some(Box::new(parse_expression_core(len)?)),
            stride: Some(Box::new(parse_expression_core(stride)?)),
            inclusive_end: false,
        }));
    }
    Ok(None)
}

fn parse_projection_named_fields<'a>(
    text: &'a str,
    context: &str,
) -> Result<BTreeMap<String, &'a str>, String> {
    let mut fields = BTreeMap::new();
    for field in split_top_level(text, ',') {
        let field = field.trim();
        if field.is_empty() {
            return Err(format!("{context} contains an empty field"));
        }
        let Some((name, value)) = split_top_level_named_field(field) else {
            return Err(format!("{context} field `{field}` must use `name: value`"));
        };
        if !is_identifier(name) {
            return Err(format!(
                "{context} field name `{name}` is not a valid identifier"
            ));
        }
        if value.trim().is_empty() {
            return Err(format!("{context} field `{name}` is missing a value"));
        }
        if fields.insert(name.to_string(), value.trim()).is_some() {
            return Err(format!("{context} field `{name}` is repeated"));
        }
    }
    Ok(fields)
}

fn split_top_level_named_field(text: &str) -> Option<(&str, &str)> {
    let mut square_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut string_delim = None::<char>;
    let mut escape = false;
    for (index, ch) in text.char_indices() {
        if let Some(delim) = string_delim {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
                continue;
            }
            if ch == delim {
                string_delim = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => string_delim = Some(ch),
            '[' => square_depth += 1,
            ']' => square_depth = square_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            ':' if square_depth == 0 && paren_depth == 0 && brace_depth == 0 => {
                return Some((text[..index].trim(), text[index + 1..].trim()));
            }
            _ => {}
        }
    }
    None
}

fn split_trailing_bracket_suffix(text: &str) -> Option<(&str, &str)> {
    if !text.ends_with(']') {
        return None;
    }

    let mut candidate = None;
    for (index, ch) in text.char_indices() {
        if ch != '[' {
            continue;
        }
        let Some(close) = find_matching_delim(text, index, '[', ']') else {
            continue;
        };
        if close == text.len() - 1 {
            candidate = Some(index);
        }
    }

    let open = candidate?;
    Some((&text[..open], &text[open + 1..text.len() - 1]))
}

fn should_parse_index_brackets(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    if parse_range_parts(trimmed).is_some() {
        return true;
    }
    if matches!(trimmed, "true" | "false")
        || trimmed.starts_with('"')
        || trimmed.parse::<i64>().is_ok()
    {
        return true;
    }
    if trimmed.starts_with('(')
        || trimmed.starts_with('[')
        || trimmed.starts_with('-')
        || trimmed.starts_with('~')
    {
        return true;
    }
    if trimmed.starts_with("not ")
        || trimmed.starts_with("weave ")
        || trimmed.starts_with("split ")
        || trimmed.contains("::")
        || trimmed.contains(">>")
    {
        return true;
    }
    if let Some(first) = trimmed.chars().next() {
        return first.is_ascii_lowercase() || first == '_';
    }
    false
}

fn parse_generic_arg_types(text: &str) -> Option<Vec<SurfaceType>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut args = Vec::new();
    for arg in split_top_level(trimmed, ',').into_iter().map(str::trim) {
        if arg.is_empty() {
            return None;
        }
        let parsed = parse_surface_type(arg).ok()?;
        args.push(parsed);
    }
    Some(args)
}

fn split_member_access(text: &str) -> Option<(&str, &str)> {
    if is_float_literal(text.trim()) {
        return None;
    }
    let positions = find_top_level_dot_positions(text);
    let index = *positions.last()?;
    let base = text[..index].trim();
    let member = text[index + 1..].trim();
    if base.is_empty() || member.is_empty() {
        return None;
    }
    Some((base, member))
}

fn assign_target_from_expr(expr: Expr) -> Result<AssignTarget, String> {
    match expr {
        Expr::Path { segments } if segments.len() == 1 => Ok(AssignTarget::Name {
            text: segments[0].clone(),
        }),
        Expr::Unary {
            op: UnaryOp::Deref,
            expr,
        } => Ok(AssignTarget::Deref { expr: *expr }),
        Expr::MemberAccess { expr, member } => Ok(AssignTarget::MemberAccess {
            target: Box::new(assign_target_from_expr(*expr)?),
            member,
        }),
        Expr::Index { expr, index } => Ok(AssignTarget::Index {
            target: Box::new(assign_target_from_expr(*expr)?),
            index: *index,
        }),
        Expr::GenericApply { expr, .. } => assign_target_from_expr(*expr),
        other => Err(format!("unsupported assignment target `{other:?}`")),
    }
}

fn parse_assign_target(text: &str) -> Result<AssignTarget, String> {
    let trimmed = text.trim();
    let expr = parse_expression_core(trimmed)?;
    assign_target_from_expr(expr).map_err(|_| format!("unsupported assignment target `{trimmed}`"))
}

fn find_top_level_binary_op(text: &str, ops: &[BinaryOpSpec]) -> Option<(usize, BinaryOp, usize)> {
    let mut candidate = None;
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if depth_paren != 0 || depth_bracket != 0 || depth_brace != 0 {
            continue;
        }

        for spec in ops {
            if operator_matches_at(text, idx, *spec) {
                candidate = Some((idx, spec.op, spec.token.len()));
                break;
            }
        }
    }

    candidate
}

fn operator_matches_at(text: &str, index: usize, spec: BinaryOpSpec) -> bool {
    if !text[index..].starts_with(spec.token) {
        return false;
    }
    if spec.keyword {
        return has_word_boundary_before(text, index)
            && has_word_boundary_after(text, index + spec.token.len());
    }

    match spec.token {
        "<" => {
            !matches!(text[index + 1..].chars().next(), Some('=' | '<'))
                && !matches!(text[..index].chars().next_back(), Some('<'))
        }
        ">" => {
            !matches!(text[index + 1..].chars().next(), Some('=' | '>'))
                && !matches!(text[..index].chars().next_back(), Some('>'))
        }
        "|" => !matches!(text[index + 1..].chars().next(), Some('=')),
        "&" => !matches!(text[index + 1..].chars().next(), Some('=')),
        "+" => !matches!(text[index + 1..].chars().next(), Some('=')),
        "-" => !matches!(text[index + 1..].chars().next(), Some('=' | '>')),
        "*" => !matches!(text[index + 1..].chars().next(), Some('=')),
        "/" => !matches!(text[index + 1..].chars().next(), Some('=')),
        "%" => !matches!(text[index + 1..].chars().next(), Some('=')),
        "^" => !matches!(text[index + 1..].chars().next(), Some('=')),
        _ => true,
    }
}

fn strip_keyword_prefix<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(keyword)?;
    if rest.is_empty() || !has_word_boundary_after(text, keyword.len()) {
        return None;
    }
    Some(rest.trim_start())
}

fn parse_capability_unary(text: &str) -> Option<(UnaryOp, &str)> {
    [
        ("read", UnaryOp::CapabilityRead),
        ("edit", UnaryOp::CapabilityEdit),
        ("take", UnaryOp::CapabilityTake),
        ("hold", UnaryOp::CapabilityHold),
    ]
    .into_iter()
    .find_map(|(keyword, op)| strip_keyword_prefix(text, keyword).map(|rest| (op, rest)))
}

fn strip_group_parens(text: &str) -> Option<&str> {
    if !text.starts_with('(') || !text.ends_with(')') {
        return None;
    }
    let close = find_matching_delim(text, 0, '(', ')')?;
    if close != text.len() - 1 {
        return None;
    }
    let inner = text[1..close].trim();
    if inner.is_empty() || contains_top_level_char(inner, ',') {
        return None;
    }
    Some(inner)
}

fn contains_top_level_char(text: &str, needle: char) -> bool {
    find_top_level_char_index(text, needle).is_some()
}

fn find_top_level_char_index(text: &str, needle: char) -> Option<usize> {
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 && ch == needle {
            return Some(idx);
        }
    }

    None
}

fn top_level_token_precedes_first_comma(text: &str, token: &str) -> bool {
    match (
        find_top_level_char_index(text, ','),
        find_top_level_token(text, token),
    ) {
        (Some(comma), Some(token_index)) => token_index < comma,
        _ => false,
    }
}

fn find_top_level_named_eq(text: &str) -> Option<usize> {
    let (index, op, len) = find_top_level_assignment_op(text)?;
    if op == AssignOp::Assign && len == 1 {
        return Some(index);
    }
    None
}

fn split_top_level_single_colon(text: &str) -> Option<(&str, &str)> {
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            ':' if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 => {
                let prev = text[..idx].chars().next_back();
                let next = text[idx + 1..].chars().next();
                if !matches!(prev, Some(':')) && !matches!(next, Some(':' | '>' | '<' | '=')) {
                    return Some((&text[..idx], &text[idx + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

fn has_word_boundary_before(text: &str, index: usize) -> bool {
    !matches!(text[..index].chars().next_back(), Some(ch) if is_identifier_continue(ch))
}

fn has_word_boundary_after(text: &str, index: usize) -> bool {
    !matches!(text[index..].chars().next(), Some(ch) if is_identifier_continue(ch))
}

fn find_top_level_token_positions(text: &str, token: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if depth_paren != 0 || depth_bracket != 0 || depth_brace != 0 {
            continue;
        }

        if text[idx..].starts_with(token) {
            positions.push(idx);
        }
    }

    positions
}

fn find_top_level_dot_positions(text: &str) -> Vec<usize> {
    find_top_level_token_positions(text, ".")
        .into_iter()
        .filter(|index| {
            !matches!(text[..*index].chars().next_back(), Some('.'))
                && !matches!(text[*index + 1..].chars().next(), Some('.'))
        })
        .collect()
}

fn parse_match_expression(
    rest: &str,
    attached: &[RawBlockEntry],
    span: Span,
) -> Result<Expr, String> {
    let Some(subject) = rest.strip_suffix(':') else {
        return Err(format!(
            "{}:{}: malformed `match` expression",
            span.line, span.column
        ));
    };
    let subject = subject.trim();
    if subject.is_empty() || attached.is_empty() {
        return Err(format!(
            "{}:{}: malformed `match` expression",
            span.line, span.column
        ));
    }

    let mut arms = Vec::new();
    for entry in attached {
        arms.push(parse_match_arm(entry)?);
    }

    Ok(Expr::Match {
        subject: Box::new(parse_expression_core(subject)?),
        arms,
    })
}

fn parse_match_arm(entry: &RawBlockEntry) -> Result<MatchArm, String> {
    let Some(index) = find_top_level_token(&entry.text, "=>") else {
        return Err(format!(
            "{}:{}: malformed `match` arm",
            entry.span.line, entry.span.column
        ));
    };
    let patterns_text = entry.text[..index].trim();
    let value_text = entry.text[index + 2..].trim();
    if patterns_text.is_empty() || value_text.is_empty() {
        return Err(format!(
            "{}:{}: malformed `match` arm",
            entry.span.line, entry.span.column
        ));
    }

    let patterns = split_top_level(patterns_text, '|')
        .into_iter()
        .map(str::trim)
        .filter(|pattern| !pattern.is_empty())
        .map(parse_match_pattern)
        .collect::<Result<Vec<_>, _>>()?;
    if patterns.is_empty() {
        return Err(format!(
            "{}:{}: malformed `match` arm",
            entry.span.line, entry.span.column
        ));
    }

    Ok(MatchArm {
        patterns,
        value: parse_expression(value_text, &entry.children, entry.span)?,
        span: entry.span,
    })
}

fn parse_match_pattern(text: &str) -> Result<MatchPattern, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("malformed `match` pattern".to_string());
    }
    if trimmed == "_" {
        return Ok(MatchPattern::Wildcard);
    }
    if is_match_literal(trimmed) {
        return Ok(MatchPattern::Literal {
            text: trimmed.to_string(),
        });
    }
    if let Some(variant) = parse_variant_pattern(trimmed)? {
        return Ok(variant);
    }
    if is_path_like(trimmed) {
        return Ok(MatchPattern::Name {
            text: trimmed.to_string(),
        });
    }
    Err(format!("unsupported `match` pattern `{trimmed}`"))
}

fn parse_variant_pattern(text: &str) -> Result<Option<MatchPattern>, String> {
    let Some(open_idx) = text.find('(') else {
        return Ok(None);
    };
    let Some(close_idx) = find_matching_delim(text, open_idx, '(', ')') else {
        return Ok(None);
    };
    if close_idx != text.len() - 1 {
        return Ok(None);
    }
    let path = text[..open_idx].trim();
    if !is_path_like(path) {
        return Ok(None);
    }
    let inside = text[open_idx + 1..close_idx].trim();
    let args = if inside.is_empty() {
        Vec::new()
    } else {
        split_top_level(inside, ',')
            .into_iter()
            .map(str::trim)
            .filter(|arg| !arg.is_empty())
            .map(parse_match_pattern)
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(Some(MatchPattern::Variant {
        path: path.to_string(),
        args,
    }))
}

fn is_match_literal(text: &str) -> bool {
    matches!(text, "true" | "false") || text.starts_with('"') || text.parse::<i64>().is_ok()
}

fn parse_block_header(rest: &str, keyword: &str, span: Span) -> Result<String, String> {
    let Some(header) = rest.strip_suffix(':') else {
        return Err(format!(
            "{}:{}: malformed `{keyword}` statement",
            span.line, span.column
        ));
    };
    let header = header.trim();
    if header.is_empty() {
        return Err(format!(
            "{}:{}: malformed `{keyword}` statement",
            span.line, span.column
        ));
    }
    Ok(header.to_string())
}

fn parse_assignment_statement(
    text: &str,
) -> Result<Option<(AssignTarget, AssignOp, String)>, String> {
    let Some((index, op, op_len)) = find_top_level_assignment_op(text) else {
        return Ok(None);
    };
    let target = text[..index].trim();
    let value = text[index + op_len..].trim();
    if target.is_empty() || value.is_empty() {
        return Ok(None);
    }
    Ok(Some((parse_assign_target(target)?, op, value.to_string())))
}

fn find_top_level_assignment_op(text: &str) -> Option<(usize, AssignOp, usize)> {
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if depth_paren != 0 || depth_bracket != 0 || depth_brace != 0 {
            continue;
        }

        for (token, op) in [
            ("<<=", AssignOp::ShlAssign),
            ("shr=", AssignOp::ShrAssign),
            ("+=", AssignOp::AddAssign),
            ("-=", AssignOp::SubAssign),
            ("*=", AssignOp::MulAssign),
            ("/=", AssignOp::DivAssign),
            ("%=", AssignOp::ModAssign),
            ("&=", AssignOp::BitAndAssign),
            ("|=", AssignOp::BitOrAssign),
            ("^=", AssignOp::BitXorAssign),
            ("=", AssignOp::Assign),
        ] {
            if !text[idx..].starts_with(token) {
                continue;
            }
            if token == "=" {
                let prev = text[..idx].chars().next_back();
                let next = text[idx + 1..].chars().next();
                if matches!(prev, Some('<' | '>' | '!' | '=' | ':'))
                    || matches!(next, Some('=' | '>' | '<'))
                {
                    continue;
                }
            }
            return Some((idx, op, token.len()));
        }
    }

    None
}

fn find_top_level_token(text: &str, token: &str) -> Option<usize> {
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if depth_paren == 0
            && depth_bracket == 0
            && depth_brace == 0
            && text[idx..].starts_with(token)
        {
            return Some(idx);
        }
    }

    None
}

fn find_top_level_keyword(text: &str, keyword: &str) -> Option<usize> {
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if depth_paren != 0 || depth_bracket != 0 || depth_brace != 0 {
            continue;
        }

        if text[idx..].starts_with(keyword)
            && has_word_boundary_before(text, idx)
            && has_word_boundary_after(text, idx + keyword.len())
        {
            return Some(idx);
        }
    }

    None
}

fn parse_field_decl(trimmed: &str, span: Span, forewords: Vec<ForewordApp>) -> Option<FieldDecl> {
    let (name, ty_text) = trimmed.split_once(':')?;
    let name = name.trim();
    let ty_text = ty_text.trim();
    if !is_identifier(name) || ty_text.is_empty() {
        return None;
    }
    let (ty_text, bit_width) = if let Some((ty, width_text)) = ty_text.rsplit_once(" bits ") {
        let width = width_text.trim().parse::<u16>().ok()?;
        (ty.trim(), Some(width))
    } else {
        (ty_text, None)
    };
    Some(FieldDecl {
        name: name.to_string(),
        ty: parse_surface_type(ty_text).ok()?,
        bit_width,
        forewords,
        span,
    })
}

fn parse_enum_variant_decl(trimmed: &str, span: Span) -> Option<EnumVariantDecl> {
    let name = parse_symbol_name(trimmed)?;
    let tail = trimmed[name.len()..].trim();
    let payload = if tail.is_empty() {
        None
    } else if tail.starts_with('(') && tail.ends_with(')') {
        parse_surface_type(tail[1..tail.len() - 1].trim()).ok()
    } else {
        None
    };
    Some(EnumVariantDecl {
        name,
        payload,
        span,
    })
}

fn parse_trait_assoc_type_decl(trimmed: &str, span: Span) -> Option<TraitAssocTypeDecl> {
    let rest = trimmed.strip_prefix("type ")?;
    let (name, default_ty) = match rest.split_once('=') {
        Some((name, value)) => (name.trim(), parse_surface_type(value.trim()).ok()),
        None => (rest.trim(), None),
    };
    if !is_identifier(name) {
        return None;
    }
    Some(TraitAssocTypeDecl {
        name: name.to_string(),
        default_ty,
        span,
    })
}

fn parse_impl_assoc_type_binding(trimmed: &str, span: Span) -> Option<ImplAssocTypeBinding> {
    let rest = trimmed.strip_prefix("type ")?;
    let (name, value_ty) = match rest.split_once('=') {
        Some((name, value)) => (name.trim(), parse_surface_type(value.trim()).ok()),
        None => (rest.trim(), None),
    };
    if !is_identifier(name) {
        return None;
    }
    Some(ImplAssocTypeBinding {
        name: name.to_string(),
        value_ty,
        span,
    })
}

fn parse_cleanup_footer_entry(entry: &RawBlockEntry) -> Result<Option<CleanupFooter>, String> {
    let text = entry.text.trim();
    if text.starts_with('[') {
        let Some(close_idx) = find_matching_delim(text, 0, '[', ']') else {
            return Ok(None);
        };
        let suffix = text[close_idx + 1..].trim();
        if suffix == "#cleanup" {
            return Err(format!(
                "{}:{}: `#cleanup` has been replaced by `-cleanup[target = name, handler = path]`",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(None);
    }
    if !text.starts_with('-') {
        return Ok(None);
    }
    if !entry.children.is_empty() {
        return Err(format!(
            "{}:{}: cleanup footer lines cannot own nested blocks",
            entry.span.line, entry.span.column
        ));
    }
    let Some(rest) = text.strip_prefix("-cleanup") else {
        let name = text
            .strip_prefix('-')
            .and_then(|value| value.split_once('[').map(|(head, _)| head).or(Some(value)))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("?");
        return Err(format!(
            "{}:{}: attached `-{name}` is unsupported here; this footer position currently accepts only `-cleanup`",
            entry.span.line, entry.span.column
        ));
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return Ok(Some(CleanupFooter {
            kind: CleanupFooterKind::Cleanup,
            subject: String::new(),
            handler_path: Vec::new(),
            span: entry.span,
        }));
    }
    let payload = rest
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'));
    let Some(payload) = payload else {
        return Err(format!(
            "{}:{}: malformed `-cleanup` footer payload",
            entry.span.line, entry.span.column
        ));
    };
    let mut subject = None::<String>;
    let mut handler_path = None::<Vec<String>>;
    for part in split_top_level(payload, ',') {
        let Some((name, value)) = part.split_once('=') else {
            return Err(format!(
                "{}:{}: cleanup footer entries must use named fields",
                entry.span.line, entry.span.column
            ));
        };
        let field = name.trim();
        let value = value.trim();
        match field {
            "target" => {
                if subject.is_some() {
                    return Err(format!(
                        "{}:{}: cleanup footer field `target` is duplicated",
                        entry.span.line, entry.span.column
                    ));
                }
                if !is_identifier(value) {
                    return Err(format!(
                        "{}:{}: cleanup footer target must be a binding name",
                        entry.span.line, entry.span.column
                    ));
                }
                subject = Some(value.to_string());
            }
            "handler" => {
                if handler_path.is_some() {
                    return Err(format!(
                        "{}:{}: cleanup footer field `handler` is duplicated",
                        entry.span.line, entry.span.column
                    ));
                }
                handler_path = Some(parse_path(value).map_err(|detail| {
                    format!(
                        "{}:{}: cleanup footer handler must be a named callable path: {}",
                        entry.span.line, entry.span.column, detail
                    )
                })?);
            }
            other => {
                return Err(format!(
                    "{}:{}: unknown cleanup footer field `{other}`",
                    entry.span.line, entry.span.column
                ));
            }
        }
    }
    if handler_path.is_some() && subject.is_none() {
        return Err(format!(
            "{}:{}: cleanup footer `handler` requires `target`",
            entry.span.line, entry.span.column
        ));
    }
    Ok(Some(CleanupFooter {
        kind: CleanupFooterKind::Cleanup,
        subject: subject.unwrap_or_default(),
        handler_path: handler_path.unwrap_or_default(),
        span: entry.span,
    }))
}

fn collect_following_cleanup_footers(
    entries: &[RawBlockEntry],
    start_index: usize,
) -> Result<(Vec<CleanupFooter>, usize), String> {
    let mut index = start_index;
    let mut cleanup_footers = Vec::new();
    let mut saw_bare_cleanup = false;
    let mut seen_targets = BTreeSet::new();
    while index < entries.len() {
        let Some(rollup) = parse_cleanup_footer_entry(&entries[index])? else {
            break;
        };
        if rollup.subject.is_empty() {
            if saw_bare_cleanup {
                return Err(format!(
                    "{}:{}: duplicate bare `-cleanup` footer",
                    rollup.span.line, rollup.span.column
                ));
            }
            saw_bare_cleanup = true;
        } else {
            if !seen_targets.insert(rollup.subject.clone()) {
                return Err(format!(
                    "{}:{}: duplicate cleanup footer target `{}`",
                    rollup.span.line, rollup.span.column, rollup.subject
                ));
            }
            if saw_bare_cleanup && rollup.handler_path.is_empty() {
                return Err(format!(
                    "{}:{}: `-cleanup[target = {}]` is redundant when bare `-cleanup` is already present",
                    rollup.span.line, rollup.span.column, rollup.subject
                ));
            }
        }
        cleanup_footers.push(rollup);
        index += 1;
    }
    Ok((cleanup_footers, index - start_index))
}

fn parse_availability_attachment(
    entry: &RawBlockEntry,
) -> Result<Option<AvailabilityAttachment>, String> {
    if !entry.children.is_empty() {
        return Ok(None);
    }
    let Some(path) = parse_path(&entry.text).ok() else {
        return Ok(None);
    };
    Ok(Some(AvailabilityAttachment {
        path,
        span: entry.span,
    }))
}

fn module_has_following_availability_target(
    entries: &[RawBlockEntry],
    start_index: usize,
) -> Result<bool, String> {
    let mut index = start_index + 1;
    while let Some(entry) = entries.get(index) {
        if parse_availability_attachment(entry)?.is_some() {
            index += 1;
            continue;
        }
        if let Some(symbol) = parse_symbol_entry(entry)? {
            return Ok(symbol_can_own_availability(&symbol));
        }
        return Ok(false);
    }
    Ok(false)
}

fn statement_has_following_availability_target(
    entries: &[RawBlockEntry],
    start_index: usize,
    loop_depth: usize,
) -> Result<bool, String> {
    let mut index = start_index + 1;
    while let Some(entry) = entries.get(index) {
        if parse_availability_attachment(entry)?.is_some() {
            index += 1;
            continue;
        }
        let statement = parse_statement(entry, loop_depth)?;
        return Ok(statement_can_own_availability(&statement));
    }
    Ok(false)
}

fn symbol_can_own_cleanup_footers(symbol: &SymbolDecl) -> bool {
    matches!(
        symbol.kind,
        SymbolKind::Fn | SymbolKind::Behavior | SymbolKind::System
    )
}

fn statement_can_own_cleanup_footers(statement: &Statement) -> bool {
    match &statement.kind {
        StatementKind::If { .. } | StatementKind::While { .. } | StatementKind::For { .. } => true,
        StatementKind::Expr { expr } => expr_has_attached_block(expr),
        _ => false,
    }
}

fn symbol_can_own_availability(symbol: &SymbolDecl) -> bool {
    matches!(
        symbol.kind,
        SymbolKind::Fn | SymbolKind::Behavior | SymbolKind::System
    )
}

fn statement_can_own_availability(statement: &Statement) -> bool {
    matches!(
        statement.kind,
        StatementKind::If { .. } | StatementKind::While { .. } | StatementKind::For { .. }
    ) || matches!(&statement.kind, StatementKind::Expr { expr } if expr_has_attached_block(expr))
}

fn expr_has_attached_block(expr: &Expr) -> bool {
    match expr {
        Expr::QualifiedPhrase { attached, .. } | Expr::MemoryPhrase { attached, .. } => {
            !attached.is_empty()
        }
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ForewordTarget {
    Import,
    Reexport,
    Use,
    Function,
    Record,
    Object,
    Owner,
    Enum,
    OpaqueType,
    Trait,
    TraitMethod,
    ImplMethod,
    Behavior,
    System,
    Const,
    Field,
    Param,
}

fn is_builtin_foreword_name(name: &str) -> bool {
    matches!(
        name,
        "deprecated"
            | "only"
            | "test"
            | "allow"
            | "deny"
            | "inline"
            | "cold"
            | "boundary"
            | "stage"
            | "chain"
            | "unsafe"
    )
}

fn validate_module_foreword_contract(parsed: &ParsedModule) -> Result<(), String> {
    for directive in &parsed.directives {
        validate_foreword_list(
            &directive.forewords,
            match directive.kind {
                DirectiveKind::Import => ForewordTarget::Import,
                DirectiveKind::Use => ForewordTarget::Use,
                DirectiveKind::Reexport => ForewordTarget::Reexport,
            },
            None,
        )?;
    }
    for symbol in &parsed.symbols {
        validate_symbol_foreword_contract(symbol, symbol_foreword_target(symbol.kind), None)?;
    }
    for impl_decl in &parsed.impls {
        for method in &impl_decl.methods {
            validate_symbol_foreword_contract(method, ForewordTarget::ImplMethod, None)?;
        }
    }
    Ok(())
}

fn symbol_foreword_target(kind: SymbolKind) -> ForewordTarget {
    match kind {
        SymbolKind::Fn => ForewordTarget::Function,
        SymbolKind::Record => ForewordTarget::Record,
        SymbolKind::Struct => ForewordTarget::Record,
        SymbolKind::Union => ForewordTarget::Record,
        SymbolKind::Array => ForewordTarget::Record,
        SymbolKind::Object => ForewordTarget::Object,
        SymbolKind::Owner => ForewordTarget::Owner,
        SymbolKind::Enum => ForewordTarget::Enum,
        SymbolKind::OpaqueType => ForewordTarget::OpaqueType,
        SymbolKind::Trait => ForewordTarget::Trait,
        SymbolKind::Behavior => ForewordTarget::Behavior,
        SymbolKind::System => ForewordTarget::System,
        SymbolKind::Const => ForewordTarget::Const,
    }
}

fn foreword_target_allows(target: ForewordTarget, foreword_name: &str) -> bool {
    if !is_builtin_foreword_name(foreword_name) {
        return false;
    }
    match foreword_name {
        "deprecated" => matches!(
            target,
            ForewordTarget::Function
                | ForewordTarget::Record
                | ForewordTarget::Object
                | ForewordTarget::Owner
                | ForewordTarget::Enum
                | ForewordTarget::OpaqueType
                | ForewordTarget::Trait
                | ForewordTarget::TraitMethod
                | ForewordTarget::ImplMethod
                | ForewordTarget::Const
                | ForewordTarget::Field
                | ForewordTarget::Param
        ),
        "only" => true,
        "test" => matches!(target, ForewordTarget::Function),
        "allow" | "deny" => matches!(
            target,
            ForewordTarget::Import
                | ForewordTarget::Reexport
                | ForewordTarget::Use
                | ForewordTarget::Trait
                | ForewordTarget::Behavior
                | ForewordTarget::System
                | ForewordTarget::Function
                | ForewordTarget::Record
                | ForewordTarget::Object
                | ForewordTarget::Owner
                | ForewordTarget::Enum
                | ForewordTarget::OpaqueType
                | ForewordTarget::TraitMethod
                | ForewordTarget::ImplMethod
                | ForewordTarget::Const
                | ForewordTarget::Field
                | ForewordTarget::Param
        ),
        "inline" | "cold" => matches!(
            target,
            ForewordTarget::Function | ForewordTarget::TraitMethod | ForewordTarget::ImplMethod
        ),
        "boundary" => matches!(
            target,
            ForewordTarget::Function | ForewordTarget::ImplMethod
        ),
        "stage" => matches!(
            target,
            ForewordTarget::Function
                | ForewordTarget::TraitMethod
                | ForewordTarget::ImplMethod
                | ForewordTarget::Behavior
                | ForewordTarget::System
        ),
        "unsafe" => matches!(
            target,
            ForewordTarget::Function
                | ForewordTarget::TraitMethod
                | ForewordTarget::ImplMethod
                | ForewordTarget::Behavior
                | ForewordTarget::System
        ),
        _ => false,
    }
}

fn validate_symbol_foreword_contract(
    symbol: &SymbolDecl,
    target: ForewordTarget,
    inherited_boundary_target: Option<&str>,
) -> Result<(), String> {
    let boundary_target = validate_foreword_list(&symbol.forewords, target, Some(symbol))?;
    for param in &symbol.params {
        validate_foreword_list(&param.forewords, ForewordTarget::Param, None)?;
    }
    let active_boundary_target = boundary_target.as_deref().or(inherited_boundary_target);
    validate_statement_foreword_contract(
        &symbol.statements,
        matches!(symbol.kind, SymbolKind::Behavior | SymbolKind::System),
    )?;
    if let Some(target) = active_boundary_target {
        validate_boundary_signature(symbol, target)?;
    }
    if let SymbolBody::Trait { methods, .. } = &symbol.body {
        for method in methods {
            validate_symbol_foreword_contract(
                method,
                ForewordTarget::TraitMethod,
                active_boundary_target,
            )?;
        }
    }
    match &symbol.body {
        SymbolBody::Record { fields } | SymbolBody::Object { fields, .. } => {
            for field in fields {
                validate_foreword_list(&field.forewords, ForewordTarget::Field, None)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_foreword_list(
    forewords: &[ForewordApp],
    target: ForewordTarget,
    symbol: Option<&SymbolDecl>,
) -> Result<Option<String>, String> {
    let mut boundary_target = None;
    let mut saw_stage = false;
    let mut saw_inline = false;
    let mut saw_cold = false;
    for foreword in forewords {
        if foreword.path.len() == 1 && !is_builtin_foreword_name(&foreword.name) {
            return Err(format!(
                "{}:{}: user-defined forewords must use qualified names",
                foreword.span.line, foreword.span.column
            ));
        }
        if foreword.path.len() == 1 && !foreword_target_allows(target, foreword.name.as_str()) {
            return Err(format!(
                "{}:{}: `#{}` is not a valid foreword for this target",
                foreword.span.line, foreword.span.column, foreword.name
            ));
        }
        if foreword.path.len() > 1 {
            continue;
        }
        match foreword.name.as_str() {
            "deprecated" => validate_deprecated_payload(foreword)?,
            "only" => {
                let _ = evaluate_only_foreword(foreword)?;
            }
            "test" => {
                validate_empty_foreword_payload(foreword)?;
                if let Some(symbol) = symbol {
                    validate_test_contract(symbol, foreword)?;
                }
            }
            "allow" | "deny" => validate_lint_payload(foreword)?,
            "inline" => {
                validate_empty_foreword_payload(foreword)?;
                if saw_cold {
                    return Err(format!(
                        "{}:{}: `#inline` conflicts with `#cold` on the same target",
                        foreword.span.line, foreword.span.column
                    ));
                }
                saw_inline = true;
            }
            "cold" => {
                validate_empty_foreword_payload(foreword)?;
                if saw_inline {
                    return Err(format!(
                        "{}:{}: `#cold` conflicts with `#inline` on the same target",
                        foreword.span.line, foreword.span.column
                    ));
                }
                saw_cold = true;
            }
            "boundary" => {
                let target_name = parse_boundary_payload(foreword)?;
                if boundary_target.replace(target_name).is_some() {
                    return Err(format!(
                        "{}:{}: duplicate foreword `boundary` on declaration",
                        foreword.span.line, foreword.span.column
                    ));
                }
            }
            "stage" => {
                validate_stage_payload(foreword)?;
                if saw_stage {
                    return Err(format!(
                        "{}:{}: duplicate foreword `stage` on declaration",
                        foreword.span.line, foreword.span.column
                    ));
                }
                saw_stage = true;
            }
            "unsafe" => validate_unsafe_payload(foreword)?,
            _ => {}
        }
    }
    Ok(boundary_target)
}

fn validate_empty_foreword_payload(foreword: &ForewordApp) -> Result<(), String> {
    if foreword.args.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{}:{}: invalid payload for foreword `#{}`: expected no payload",
        foreword.span.line, foreword.span.column, foreword.name
    ))
}

fn validate_deprecated_payload(foreword: &ForewordApp) -> Result<(), String> {
    if foreword.args.len() != 1 {
        return Err(format!(
            "{}:{}: invalid payload for foreword `#deprecated`: expected one string argument",
            foreword.span.line, foreword.span.column
        ));
    }
    match &foreword.args[0] {
        ForewordArg {
            name: None,
            typed_value: ForewordArgValue::Str(_),
            ..
        } => Ok(()),
        _ => Err(format!(
            "{}:{}: invalid payload for foreword `#deprecated`: expected one string argument",
            foreword.span.line, foreword.span.column
        )),
    }
}

fn validate_lint_payload(foreword: &ForewordApp) -> Result<(), String> {
    if foreword.args.is_empty() {
        return Err(format!(
            "{}:{}: invalid payload for foreword: expected one or more lint names",
            foreword.span.line, foreword.span.column
        ));
    }
    for arg in &foreword.args {
        let Some(value) = arg.name.as_ref().is_none().then_some(&arg.typed_value) else {
            return Err(format!(
                "{}:{}: invalid payload for foreword: lint names must be positional symbols",
                foreword.span.line, foreword.span.column
            ));
        };
        if !matches!(
            value,
            ForewordArgValue::Symbol(_) | ForewordArgValue::Path(_)
        ) {
            return Err(format!(
                "{}:{}: invalid payload for foreword: lint names must be positional symbols",
                foreword.span.line, foreword.span.column
            ));
        }
    }
    Ok(())
}

fn foreword_arg_symbol_or_string(value: &ForewordArgValue) -> Option<String> {
    match value {
        ForewordArgValue::Str(value) | ForewordArgValue::Symbol(value) => Some(value.clone()),
        _ => None,
    }
}

fn parse_foreword_path_or_string(value: &str) -> Option<String> {
    if let Some(unquoted) = unquote_double_quoted_literal(value) {
        return Some(unquoted.to_string());
    }
    is_path_like(value).then(|| value.trim().to_string())
}

fn parse_boundary_payload(foreword: &ForewordApp) -> Result<String, String> {
    if foreword.args.len() != 1 {
        return Err(format!(
            "{}:{}: invalid payload for foreword `#boundary`: expected one named field `target`",
            foreword.span.line, foreword.span.column
        ));
    }
    let arg = &foreword.args[0];
    if arg.name.as_deref() != Some("target") {
        return Err(format!(
            "{}:{}: invalid payload for foreword `#boundary`: expected `target = \"lua\"|\"sql\"`",
            foreword.span.line, foreword.span.column
        ));
    }
    let Some(target) = foreword_arg_symbol_or_string(&arg.typed_value) else {
        return Err(format!(
            "{}:{}: invalid payload for foreword `#boundary`: `target` must be a string or symbol",
            foreword.span.line, foreword.span.column
        ));
    };
    if !matches!(target.as_str(), "lua" | "sql") {
        return Err(format!(
            "{}:{}: invalid payload for foreword `#boundary`: unsupported target `{target}`",
            foreword.span.line, foreword.span.column
        ));
    }
    Ok(target)
}

fn evaluate_only_foreword(foreword: &ForewordApp) -> Result<bool, String> {
    if foreword.args.is_empty() {
        return Err(format!(
            "{}:{}: invalid payload for foreword `#only`: expected named fields like os=..., arch=...",
            foreword.span.line, foreword.span.column
        ));
    }
    let mut include = true;
    for arg in &foreword.args {
        let Some(key) = arg.name.as_deref() else {
            return Err(format!(
                "{}:{}: invalid payload for foreword `#only`: expected named fields",
                foreword.span.line, foreword.span.column
            ));
        };
        let Some(value) = foreword_arg_symbol_or_string(&arg.typed_value) else {
            return Err(format!(
                "{}:{}: invalid payload for foreword `#only`: `os`/`arch` require string or symbol values",
                foreword.span.line, foreword.span.column
            ));
        };
        match key {
            "os" => include &= value == std::env::consts::OS,
            "arch" => include &= value == std::env::consts::ARCH,
            _ => {
                return Err(format!(
                    "{}:{}: invalid payload for foreword `#only`: only `os` and `arch` keys are supported",
                    foreword.span.line, foreword.span.column
                ));
            }
        }
    }
    Ok(include)
}

fn apply_only_foreword_filters(parsed: &mut ParsedModule) -> Result<(), String> {
    parsed.directives = filter_only_vec(std::mem::take(&mut parsed.directives), |directive| {
        include_for_only_forewords(&directive.forewords)
    })?;
    parsed.symbols = filter_only_vec(std::mem::take(&mut parsed.symbols), |symbol| {
        include_for_only_forewords(&symbol.forewords)
    })?
    .into_iter()
    .map(filter_symbol_only_forewords)
    .collect::<Result<Vec<_>, _>>()?;
    parsed.impls = std::mem::take(&mut parsed.impls)
        .into_iter()
        .map(filter_impl_only_forewords)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(())
}

fn include_for_only_forewords(forewords: &[ForewordApp]) -> Result<bool, String> {
    let mut include = true;
    for foreword in forewords {
        if foreword.name == "only" {
            include &= evaluate_only_foreword(foreword)?;
        }
    }
    Ok(include)
}

fn validate_stage_payload(foreword: &ForewordApp) -> Result<(), String> {
    for arg in &foreword.args {
        let Some(key) = arg.name.as_deref() else {
            return Err(format!(
                "{}:{}: invalid payload for foreword `#stage`: expected named key/value pairs",
                foreword.span.line, foreword.span.column
            ));
        };
        match key {
            "pure" => parse_contract_bool(arg.value.as_str(), foreword, "stage.pure")?,
            "deterministic" => {
                parse_contract_bool(arg.value.as_str(), foreword, "stage.deterministic")?
            }
            "rollback_safe" => {
                parse_contract_bool(arg.value.as_str(), foreword, "stage.rollback_safe")?
            }
            "effect" => parse_contract_effect(arg.value.as_str(), foreword)?,
            "thread" => parse_contract_thread(arg.value.as_str(), foreword)?,
            "authority" => parse_contract_authority(arg.value.as_str(), foreword)?,
            "reads" | "writes" | "excludes" => {
                parse_contract_resource(arg.value.as_str(), foreword)?
            }
            other => {
                return Err(format!(
                    "{}:{}: invalid #stage contract key '{other}'",
                    foreword.span.line, foreword.span.column
                ));
            }
        }
    }
    Ok(())
}

fn validate_unsafe_payload(foreword: &ForewordApp) -> Result<(), String> {
    match foreword.args.as_slice() {
        [
            ForewordArg {
                name: None,
                typed_value: ForewordArgValue::Str(value),
                ..
            },
        ] if !value.is_empty() => Ok(()),
        _ => Err(format!(
            "{}:{}: invalid payload for foreword `#unsafe`: expected exactly one non-empty string trace id",
            foreword.span.line, foreword.span.column
        )),
    }
}

fn validate_chain_payload(foreword: &ForewordApp) -> Result<(), String> {
    for arg in &foreword.args {
        let Some(key) = arg.name.as_deref() else {
            return Err(format!(
                "{}:{}: invalid payload for foreword `#chain`: expected named key/value pairs",
                foreword.span.line, foreword.span.column
            ));
        };
        match key {
            "phase" => parse_contract_phase(arg.value.as_str(), foreword)?,
            "deterministic" => {
                parse_contract_bool(arg.value.as_str(), foreword, "chain.deterministic")?
            }
            "thread" => parse_contract_thread(arg.value.as_str(), foreword)?,
            "authority" => parse_contract_authority(arg.value.as_str(), foreword)?,
            "rollback_safe" => {
                parse_contract_bool(arg.value.as_str(), foreword, "chain.rollback_safe")?
            }
            other => {
                return Err(format!(
                    "{}:{}: invalid #chain contract key '{other}'",
                    foreword.span.line, foreword.span.column
                ));
            }
        }
    }
    Ok(())
}

fn parse_contract_bool(value: &str, foreword: &ForewordApp, label: &str) -> Result<(), String> {
    match value.trim() {
        "true" | "false" => Ok(()),
        _ => Err(format!(
            "{}:{}: invalid payload for `{label}`: expected bool",
            foreword.span.line, foreword.span.column
        )),
    }
}

fn parse_contract_effect(value: &str, foreword: &ForewordApp) -> Result<(), String> {
    match parse_foreword_path_or_string(value).as_deref() {
        Some("read" | "write" | "exclusive_write" | "emit" | "render") => Ok(()),
        _ => Err(format!(
            "{}:{}: invalid payload for `stage.effect`",
            foreword.span.line, foreword.span.column
        )),
    }
}

fn parse_contract_thread(value: &str, foreword: &ForewordApp) -> Result<(), String> {
    match parse_foreword_path_or_string(value).as_deref() {
        Some("main" | "worker" | "any") => Ok(()),
        _ => Err(format!(
            "{}:{}: invalid payload for `thread`",
            foreword.span.line, foreword.span.column
        )),
    }
}

fn parse_contract_authority(value: &str, foreword: &ForewordApp) -> Result<(), String> {
    match parse_foreword_path_or_string(value).as_deref() {
        Some("local" | "client" | "server" | "any") => Ok(()),
        _ => Err(format!(
            "{}:{}: invalid payload for `authority`",
            foreword.span.line, foreword.span.column
        )),
    }
}

fn parse_contract_phase(value: &str, foreword: &ForewordApp) -> Result<(), String> {
    match parse_foreword_path_or_string(value).as_deref() {
        Some("startup" | "fixed" | "fixed_update" | "update" | "render" | "net" | "event") => {
            Ok(())
        }
        _ => Err(format!(
            "{}:{}: invalid payload for `phase`",
            foreword.span.line, foreword.span.column
        )),
    }
}

fn parse_contract_resource(value: &str, foreword: &ForewordApp) -> Result<(), String> {
    if parse_foreword_path_or_string(value).is_some() {
        return Ok(());
    }
    Err(format!(
        "{}:{}: invalid payload for stage resource set; expected type path",
        foreword.span.line, foreword.span.column
    ))
}

fn filter_only_vec<T, F>(items: Vec<T>, include: F) -> Result<Vec<T>, String>
where
    F: Fn(&T) -> Result<bool, String>,
{
    let mut filtered = Vec::new();
    for item in items {
        if include(&item)? {
            filtered.push(item);
        }
    }
    Ok(filtered)
}

fn filter_symbol_only_forewords(mut symbol: SymbolDecl) -> Result<SymbolDecl, String> {
    if let SymbolBody::Trait { methods, .. } = &mut symbol.body {
        *methods = filter_only_vec(std::mem::take(methods), |method| {
            include_for_only_forewords(&method.forewords)
        })?;
    }
    Ok(symbol)
}

fn filter_impl_only_forewords(mut impl_decl: ImplDecl) -> Result<ImplDecl, String> {
    impl_decl.methods = filter_only_vec(std::mem::take(&mut impl_decl.methods), |method| {
        include_for_only_forewords(&method.forewords)
    })?;
    Ok(impl_decl)
}

fn validate_test_contract(symbol: &SymbolDecl, foreword: &ForewordApp) -> Result<(), String> {
    if symbol.exported {
        return Err(format!(
            "{}:{}: `#test` functions must not be exported in v1",
            foreword.span.line, foreword.span.column
        ));
    }
    if !symbol.params.is_empty() {
        return Err(format!(
            "{}:{}: `#test` functions must have zero parameters",
            foreword.span.line, foreword.span.column
        ));
    }
    if let Some(return_type) = &symbol.return_type {
        let rendered = return_type.render();
        let trimmed = rendered.trim();
        if trimmed != "Unit" && trimmed != "Int" {
            return Err(format!(
                "{}:{}: `#test` functions must return Unit or Int",
                foreword.span.line, foreword.span.column
            ));
        }
    }
    Ok(())
}

fn validate_boundary_signature(symbol: &SymbolDecl, target: &str) -> Result<(), String> {
    for param in &symbol.params {
        if matches!(param.mode, Some(ParamMode::Edit | ParamMode::Hold)) || param.ty.is_mut_ref() {
            return Err(format!(
                "{}:{}: `#boundary` target `{target}` does not allow mutable borrows",
                symbol.span.line, symbol.span.column
            ));
        }
        if !surface_type_is_boundary_safe(&param.ty) {
            return Err(format!(
                "{}:{}: type `{}` is not boundary-safe for target `{target}`",
                symbol.span.line,
                symbol.span.column,
                param.ty.render()
            ));
        }
    }
    if let Some(return_type) = &symbol.return_type {
        if return_type.is_ref() {
            return Err(format!(
                "{}:{}: `#boundary` target `{target}` requires owned return type (no references)",
                symbol.span.line, symbol.span.column
            ));
        }
        if !surface_type_is_boundary_safe(return_type) {
            return Err(format!(
                "{}:{}: type `{}` is not boundary-safe for target `{target}`",
                symbol.span.line,
                symbol.span.column,
                return_type.render()
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinOwnershipClass {
    Copy,
    Move,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuiltinTypeInfo {
    pub name: &'static str,
    pub ownership: BuiltinOwnershipClass,
    pub boundary_unsafe: bool,
}

// Single source of truth for the remaining language-reserved builtin types.
// Runtime/resource handles are now source-declared opaque types in `std.*`.
const BUILTIN_TYPE_INFOS: &[BuiltinTypeInfo] = &[
    BuiltinTypeInfo {
        name: "Int",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "I8",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "U8",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "I16",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "U16",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "I32",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "U32",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "I64",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "U64",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "ISize",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "USize",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "F32",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "F64",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "Unit",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "Str",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "Bytes",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "ByteBuffer",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "Utf16",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "Utf16Buffer",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "View",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: true,
    },
    BuiltinTypeInfo {
        name: "Contiguous",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "Strided",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "Mapped",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: true,
    },
    BuiltinTypeInfo {
        name: "Bool",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "RangeInt",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: true,
    },
    BuiltinTypeInfo {
        name: "List",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "Array",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "Map",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: false,
    },
    BuiltinTypeInfo {
        name: "Arena",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: true,
    },
    BuiltinTypeInfo {
        name: "ArenaId",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: true,
    },
    BuiltinTypeInfo {
        name: "FrameArena",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: true,
    },
    BuiltinTypeInfo {
        name: "FrameId",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: true,
    },
    BuiltinTypeInfo {
        name: "PoolArena",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: true,
    },
    BuiltinTypeInfo {
        name: "PoolId",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: true,
    },
    BuiltinTypeInfo {
        name: "Task",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: true,
    },
    BuiltinTypeInfo {
        name: "Thread",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: true,
    },
    BuiltinTypeInfo {
        name: "Channel",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: true,
    },
    BuiltinTypeInfo {
        name: "Mutex",
        ownership: BuiltinOwnershipClass::Move,
        boundary_unsafe: true,
    },
    BuiltinTypeInfo {
        name: "AtomicInt",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: true,
    },
    BuiltinTypeInfo {
        name: "AtomicBool",
        ownership: BuiltinOwnershipClass::Copy,
        boundary_unsafe: true,
    },
];

pub fn builtin_type_info(name: &str) -> Option<BuiltinTypeInfo> {
    BUILTIN_TYPE_INFOS
        .iter()
        .find(|info| info.name == name)
        .copied()
}

pub fn is_builtin_type_name(name: &str) -> bool {
    builtin_type_info(name).is_some()
}

pub fn builtin_ownership_class(name: &str) -> Option<BuiltinOwnershipClass> {
    builtin_type_info(name).map(|info| info.ownership)
}

pub fn is_builtin_boundary_unsafe_type_name(name: &str) -> bool {
    builtin_type_info(name).is_some_and(|info| info.boundary_unsafe)
}

fn unquote_double_quoted_literal(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        Some(&trimmed[1..trimmed.len() - 1])
    } else {
        None
    }
}

fn validate_statement_foreword_contract(
    statements: &[Statement],
    require_chain_contracts: bool,
) -> Result<(), String> {
    for statement in statements {
        for foreword in &statement.forewords {
            match (foreword.path.len(), foreword.name.as_str()) {
                (1, "chain") => validate_chain_payload(foreword)?,
                (1, "unsafe") => validate_unsafe_payload(foreword)?,
                _ => {
                    return Err(format!(
                        "{}:{}: `#{}` is not a valid statement-level contract",
                        foreword.span.line, foreword.span.column, foreword.name
                    ));
                }
            }
        }
        if let Some(foreword) = statement
            .forewords
            .iter()
            .find(|foreword| foreword.name == "chain")
            && !statement_is_chain_target(statement)
        {
            return Err(format!(
                "{}:{}: `#chain` can only target chain statements",
                foreword.span.line, foreword.span.column
            ));
        }
        if require_chain_contracts
            && statement_is_chain_target(statement)
            && statement.forewords.is_empty()
        {
            return Err(format!(
                "{}:{}: system boundary chain must declare explicit #chain[...] contract",
                statement.span.line, statement.span.column
            ));
        }
        match &statement.kind {
            StatementKind::If {
                then_branch,
                else_branch,
                ..
            } => {
                validate_statement_foreword_contract(then_branch, require_chain_contracts)?;
                if let Some(else_branch) = else_branch {
                    validate_statement_foreword_contract(else_branch, require_chain_contracts)?;
                }
            }
            StatementKind::While { body, .. } | StatementKind::For { body, .. } => {
                validate_statement_foreword_contract(body, require_chain_contracts)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_module_phrase_contract(parsed: &ParsedModule) -> Result<(), String> {
    for symbol in &parsed.symbols {
        validate_statement_phrase_contract(&symbol.statements)?;
    }
    for impl_decl in &parsed.impls {
        for method in &impl_decl.methods {
            validate_statement_phrase_contract(&method.statements)?;
        }
    }
    Ok(())
}

fn validate_statement_phrase_contract(statements: &[Statement]) -> Result<(), String> {
    for statement in statements {
        match &statement.kind {
            StatementKind::Let { value, .. } => {
                validate_expr_phrase_contract(value, statement.span, false)?;
            }
            StatementKind::Return { value } => {
                if let Some(value) = value {
                    validate_expr_phrase_contract(value, statement.span, false)?;
                }
            }
            StatementKind::Reclaim { expr } => {
                validate_expr_phrase_contract(expr, statement.span, false)?;
            }
            StatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                validate_expr_phrase_contract(condition, statement.span, false)?;
                validate_statement_phrase_contract(then_branch)?;
                if let Some(else_branch) = else_branch {
                    validate_statement_phrase_contract(else_branch)?;
                }
            }
            StatementKind::While { condition, body } => {
                validate_expr_phrase_contract(condition, statement.span, false)?;
                validate_statement_phrase_contract(body)?;
            }
            StatementKind::For { iterable, body, .. } => {
                validate_expr_phrase_contract(iterable, statement.span, false)?;
                validate_statement_phrase_contract(body)?;
            }
            StatementKind::Array(region) => {
                validate_expr_phrase_contract(&region.target, statement.span, false)?;
                if let Some(base) = &region.base {
                    validate_expr_phrase_contract(base, statement.span, false)?;
                }
                validate_headed_modifier_phrase_contract(
                    region.default_modifier.as_ref(),
                    statement.span,
                )?;
                for line in &region.lines {
                    validate_expr_phrase_contract(&line.value, line.span, false)?;
                    validate_headed_modifier_phrase_contract(line.modifier.as_ref(), line.span)?;
                }
            }
            StatementKind::Defer { action } => match action {
                DeferAction::Expr { expr } | DeferAction::Reclaim { expr } => {
                    validate_expr_phrase_contract(expr, statement.span, false)?;
                }
            },
            StatementKind::Assign { value, .. } => {
                validate_expr_phrase_contract(value, statement.span, false)?;
            }
            StatementKind::Recycle {
                default_modifier,
                lines,
            } => {
                validate_headed_modifier_phrase_contract(
                    default_modifier.as_ref(),
                    statement.span,
                )?;
                for line in lines {
                    match &line.kind {
                        RecycleLineKind::Expr { gate }
                        | RecycleLineKind::Let { gate, .. }
                        | RecycleLineKind::Assign { gate, .. } => {
                            validate_expr_phrase_contract(gate, line.span, false)?;
                        }
                    }
                    validate_headed_modifier_phrase_contract(line.modifier.as_ref(), line.span)?;
                }
            }
            StatementKind::Bind {
                default_modifier,
                lines,
            } => {
                validate_headed_modifier_phrase_contract(
                    default_modifier.as_ref(),
                    statement.span,
                )?;
                for line in lines {
                    match &line.kind {
                        BindLineKind::Let { gate, .. } | BindLineKind::Assign { gate, .. } => {
                            validate_expr_phrase_contract(gate, line.span, false)?;
                        }
                        BindLineKind::Require { expr } => {
                            validate_expr_phrase_contract(expr, line.span, false)?;
                        }
                    }
                    validate_headed_modifier_phrase_contract(line.modifier.as_ref(), line.span)?;
                }
            }
            StatementKind::Record(region) => {
                validate_expr_phrase_contract(&region.target, region.span, false)?;
                if let Some(base) = &region.base {
                    validate_expr_phrase_contract(base, region.span, false)?;
                }
                validate_headed_modifier_phrase_contract(
                    region.default_modifier.as_ref(),
                    region.span,
                )?;
                if let Some(ConstructDestination::Place { target }) = &region.destination {
                    validate_assign_target_tuple_contract(target, region.span)?;
                }
                for line in &region.lines {
                    validate_expr_phrase_contract(&line.value, line.span, false)?;
                    validate_headed_modifier_phrase_contract(line.modifier.as_ref(), line.span)?;
                }
            }
            StatementKind::Construct(region) => {
                validate_expr_phrase_contract(&region.target, region.span, false)?;
                validate_headed_modifier_phrase_contract(
                    region.default_modifier.as_ref(),
                    region.span,
                )?;
                if let Some(ConstructDestination::Place { target }) = &region.destination {
                    validate_assign_target_tuple_contract(target, region.span)?;
                }
                for line in &region.lines {
                    validate_expr_phrase_contract(&line.value, line.span, false)?;
                    validate_headed_modifier_phrase_contract(line.modifier.as_ref(), line.span)?;
                }
            }
            StatementKind::MemorySpec(spec) => {
                validate_headed_modifier_phrase_contract(
                    spec.default_modifier.as_ref(),
                    spec.span,
                )?;
                for detail in &spec.details {
                    validate_expr_phrase_contract(&detail.value, detail.span, false)?;
                    validate_headed_modifier_phrase_contract(
                        detail.modifier.as_ref(),
                        detail.span,
                    )?;
                }
            }
            StatementKind::Expr { expr } => {
                validate_expr_phrase_contract(expr, statement.span, true)?;
            }
            StatementKind::Break | StatementKind::Continue => {}
        }
    }
    Ok(())
}

fn validate_expr_phrase_contract(
    expr: &Expr,
    span: Span,
    allow_header_attachments: bool,
) -> Result<(), String> {
    match expr {
        Expr::Path { .. }
        | Expr::BoolLiteral { .. }
        | Expr::IntLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StrLiteral { .. } => {}
        Expr::Pair { left, right } => {
            validate_expr_phrase_contract(left, span, false)?;
            validate_expr_phrase_contract(right, span, false)?;
        }
        Expr::CollectionLiteral { items } => {
            for item in items {
                validate_expr_phrase_contract(item, span, false)?;
            }
        }
        Expr::Match { subject, arms } => {
            validate_expr_phrase_contract(subject, span, false)?;
            for arm in arms {
                validate_expr_phrase_contract(&arm.value, arm.span, false)?;
            }
        }
        Expr::RecordRegion(region) => {
            validate_expr_phrase_contract(&region.target, region.span, false)?;
            if let Some(base) = &region.base {
                validate_expr_phrase_contract(base, region.span, false)?;
            }
            validate_headed_modifier_phrase_contract(
                region.default_modifier.as_ref(),
                region.span,
            )?;
            for line in &region.lines {
                validate_expr_phrase_contract(&line.value, line.span, false)?;
                validate_headed_modifier_phrase_contract(line.modifier.as_ref(), line.span)?;
            }
        }
        Expr::ArrayRegion(region) => {
            validate_expr_phrase_contract(&region.target, region.span, false)?;
            if let Some(base) = &region.base {
                validate_expr_phrase_contract(base, region.span, false)?;
            }
            validate_headed_modifier_phrase_contract(
                region.default_modifier.as_ref(),
                region.span,
            )?;
            for line in &region.lines {
                validate_expr_phrase_contract(&line.value, line.span, false)?;
                validate_headed_modifier_phrase_contract(line.modifier.as_ref(), line.span)?;
            }
        }
        Expr::ConstructRegion(region) => {
            validate_expr_phrase_contract(&region.target, region.span, false)?;
            validate_headed_modifier_phrase_contract(
                region.default_modifier.as_ref(),
                region.span,
            )?;
            for line in &region.lines {
                validate_expr_phrase_contract(&line.value, line.span, false)?;
                validate_headed_modifier_phrase_contract(line.modifier.as_ref(), line.span)?;
            }
        }
        Expr::Chain { steps, .. } => {
            for step in steps {
                validate_expr_phrase_contract(&step.stage, span, false)?;
                for arg in &step.bind_args {
                    validate_expr_phrase_contract(arg, span, false)?;
                }
            }
        }
        Expr::MemoryPhrase {
            arena,
            init_args,
            constructor,
            attached,
            ..
        } => {
            validate_expr_phrase_contract(arena, span, false)?;
            for arg in init_args {
                validate_phrase_arg_contract(arg, span)?;
            }
            if !is_memory_constructor_like(constructor) {
                return Err(format!(
                    "{}:{}: invalid memory phrase constructor; expected path or path[type_args]",
                    span.line, span.column
                ));
            }
            if !attached.is_empty() {
                if !allow_header_attachments {
                    return Err(format!(
                        "{}:{}: attached blocks are only valid on standalone qualified/memory phrase statements",
                        span.line, span.column
                    ));
                }
                validate_header_attachment_phrase_contract(attached)?;
            }
        }
        Expr::GenericApply { expr, .. } => {
            validate_expr_phrase_contract(expr, span, false)?;
        }
        Expr::QualifiedPhrase {
            subject,
            args,
            qualifier_kind,
            qualifier,
            qualifier_type_args,
            attached,
        } => {
            validate_expr_phrase_contract(subject, span, false)?;
            for arg in args {
                validate_phrase_arg_contract(arg, span)?;
            }
            if !qualifier_type_args.is_empty()
                && !matches!(
                    qualifier_kind,
                    QualifiedPhraseQualifierKind::Call
                        | QualifiedPhraseQualifierKind::BareMethod
                        | QualifiedPhraseQualifierKind::NamedPath
                )
            {
                return Err(format!(
                    "{}:{}: qualifier `{}` does not support generic arguments",
                    span.line, span.column, qualifier
                ));
            }
            if !attached.is_empty() {
                if !allow_header_attachments {
                    return Err(format!(
                        "{}:{}: attached blocks are only valid on standalone qualified/memory phrase statements",
                        span.line, span.column
                    ));
                }
                validate_qualified_phrase_attachment_contract(*qualifier_kind, attached)?;
            }
        }
        Expr::Await { expr } | Expr::Unary { expr, .. } => {
            validate_expr_phrase_contract(expr, span, false)?;
        }
        Expr::Binary { left, right, .. } => {
            validate_expr_phrase_contract(left, span, false)?;
            validate_expr_phrase_contract(right, span, false)?;
        }
        Expr::MemberAccess { expr, .. } => {
            validate_expr_phrase_contract(expr, span, false)?;
        }
        Expr::Index { expr, index } => {
            validate_expr_phrase_contract(expr, span, false)?;
            validate_expr_phrase_contract(index, span, false)?;
        }
        Expr::Slice {
            expr,
            start,
            end,
            len,
            stride,
            ..
        } => {
            validate_expr_phrase_contract(expr, span, false)?;
            if let Some(start) = start {
                validate_expr_phrase_contract(start, span, false)?;
            }
            if let Some(end) = end {
                validate_expr_phrase_contract(end, span, false)?;
            }
            if let Some(len) = len {
                validate_expr_phrase_contract(len, span, false)?;
            }
            if let Some(stride) = stride {
                validate_expr_phrase_contract(stride, span, false)?;
            }
        }
        Expr::Range { start, end, .. } => {
            if let Some(start) = start {
                validate_expr_phrase_contract(start, span, false)?;
            }
            if let Some(end) = end {
                validate_expr_phrase_contract(end, span, false)?;
            }
        }
    }
    Ok(())
}

fn validate_phrase_arg_contract(arg: &PhraseArg, span: Span) -> Result<(), String> {
    match arg {
        PhraseArg::Positional(expr) | PhraseArg::Named { value: expr, .. } => {
            validate_expr_phrase_contract(expr, span, false)
        }
    }
}

fn validate_headed_modifier_phrase_contract(
    modifier: Option<&HeadedModifier>,
    span: Span,
) -> Result<(), String> {
    if let Some(modifier) = modifier
        && let Some(payload) = &modifier.payload
    {
        validate_expr_phrase_contract(payload, span, false)?;
    }
    Ok(())
}

fn classify_qualified_phrase_qualifier(qualifier: &str) -> Option<QualifiedPhraseQualifierKind> {
    match qualifier.trim() {
        "call" => Some(QualifiedPhraseQualifierKind::Call),
        "?" => Some(QualifiedPhraseQualifierKind::Try),
        ">" => Some(QualifiedPhraseQualifierKind::Apply),
        ">>" => Some(QualifiedPhraseQualifierKind::AwaitApply),
        "await" => Some(QualifiedPhraseQualifierKind::Await),
        "weave" => Some(QualifiedPhraseQualifierKind::Weave),
        "split" => Some(QualifiedPhraseQualifierKind::Split),
        "must" => Some(QualifiedPhraseQualifierKind::Must),
        "fallback" => Some(QualifiedPhraseQualifierKind::Fallback),
        value if is_path_like(value) => {
            if value.contains('.') {
                Some(QualifiedPhraseQualifierKind::NamedPath)
            } else {
                Some(QualifiedPhraseQualifierKind::BareMethod)
            }
        }
        _ => None,
    }
}

fn is_memory_constructor_path_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Path { .. } => true,
        Expr::MemberAccess { expr, .. } => is_memory_constructor_path_expr(expr),
        _ => false,
    }
}

fn is_memory_constructor_like(expr: &Expr) -> bool {
    match expr {
        Expr::GenericApply { expr, .. } => is_memory_constructor_path_expr(expr),
        _ => is_memory_constructor_path_expr(expr),
    }
}

fn validate_header_attachment_phrase_contract(
    attachments: &[HeaderAttachment],
) -> Result<(), String> {
    for attachment in attachments {
        match attachment {
            HeaderAttachment::Named { value, span, .. } => {
                validate_expr_phrase_contract(value, *span, false)?;
            }
            HeaderAttachment::Chain { expr, span, .. } => {
                validate_expr_phrase_contract(expr, *span, false)?;
            }
        }
    }
    Ok(())
}

fn validate_qualified_phrase_attachment_contract(
    qualifier_kind: QualifiedPhraseQualifierKind,
    attachments: &[HeaderAttachment],
) -> Result<(), String> {
    validate_header_attachment_phrase_contract(attachments)?;
    let allow_named = matches!(
        qualifier_kind,
        QualifiedPhraseQualifierKind::Call
            | QualifiedPhraseQualifierKind::Apply
            | QualifiedPhraseQualifierKind::BareMethod
    );
    for attachment in attachments {
        let HeaderAttachment::Named { span, .. } = attachment else {
            continue;
        };
        if !allow_named {
            let qualifier = match qualifier_kind {
                QualifiedPhraseQualifierKind::Call => "call",
                QualifiedPhraseQualifierKind::Try => "?",
                QualifiedPhraseQualifierKind::Apply => ">",
                QualifiedPhraseQualifierKind::AwaitApply => ">>",
                QualifiedPhraseQualifierKind::Await => "await",
                QualifiedPhraseQualifierKind::Weave => "weave",
                QualifiedPhraseQualifierKind::Split => "split",
                QualifiedPhraseQualifierKind::Must => "must",
                QualifiedPhraseQualifierKind::Fallback => "fallback",
                QualifiedPhraseQualifierKind::BareMethod => "method",
                QualifiedPhraseQualifierKind::NamedPath => "path qualifier",
            };
            return Err(format!(
                "{}:{}: qualifier `{}` does not support named header entries",
                span.line, span.column, qualifier
            ));
        }
    }
    Ok(())
}

fn statement_is_chain_target(statement: &Statement) -> bool {
    matches!(&statement.kind, StatementKind::Expr { expr } if matches!(expr, Expr::Chain { .. }))
}

fn validate_module_rollup_contract(parsed: &ParsedModule) -> Result<(), String> {
    for symbol in &parsed.symbols {
        validate_symbol_rollup_contract(symbol)?;
    }
    for impl_decl in &parsed.impls {
        for method in &impl_decl.methods {
            validate_symbol_rollup_contract(method)?;
        }
    }
    Ok(())
}

fn validate_symbol_rollup_contract(symbol: &SymbolDecl) -> Result<(), String> {
    validate_statement_rollup_contract(&symbol.statements)?;
    if let SymbolBody::Trait { methods, .. } = &symbol.body {
        for method in methods {
            validate_symbol_rollup_contract(method)?;
        }
    }
    Ok(())
}

fn validate_statement_rollup_contract(statements: &[Statement]) -> Result<(), String> {
    for statement in statements {
        match &statement.kind {
            StatementKind::If {
                then_branch,
                else_branch,
                ..
            } => {
                validate_statement_rollup_contract(then_branch)?;
                if let Some(else_branch) = else_branch {
                    validate_statement_rollup_contract(else_branch)?;
                }
            }
            StatementKind::While { body, .. } | StatementKind::For { body, .. } => {
                validate_statement_rollup_contract(body)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_module_tuple_contract(parsed: &ParsedModule) -> Result<(), String> {
    for symbol in &parsed.symbols {
        validate_symbol_tuple_contract(symbol)?;
    }
    for impl_decl in &parsed.impls {
        validate_impl_tuple_contract(impl_decl)?;
    }
    Ok(())
}

fn validate_symbol_tuple_contract(symbol: &SymbolDecl) -> Result<(), String> {
    for param in &symbol.params {
        validate_tuple_type_contract(&param.ty, symbol.span, "parameter type")?;
    }
    if let Some(return_type) = &symbol.return_type {
        validate_tuple_type_contract(return_type, symbol.span, "return type")?;
    }

    match &symbol.body {
        SymbolBody::None => {}
        SymbolBody::Record { fields } => {
            for field in fields {
                validate_tuple_type_contract(&field.ty, field.span, "field type")?;
            }
        }
        SymbolBody::Struct { fields } | SymbolBody::Union { fields } => {
            for field in fields {
                validate_tuple_type_contract(&field.ty, field.span, "field type")?;
            }
        }
        SymbolBody::Array { element_ty, .. } => {
            validate_tuple_type_contract(element_ty, symbol.span, "array element type")?;
        }
        SymbolBody::Object { fields, methods } => {
            for field in fields {
                validate_tuple_type_contract(&field.ty, field.span, "field type")?;
            }
            for method in methods {
                validate_symbol_tuple_contract(method)?;
            }
        }
        SymbolBody::Enum { variants } => {
            for variant in variants {
                if let Some(payload) = &variant.payload {
                    validate_tuple_type_contract(payload, variant.span, "enum variant payload")?;
                }
            }
        }
        SymbolBody::Owner { exits, .. } => {
            for owner_exit in exits {
                validate_expr_tuple_contract(&owner_exit.condition, owner_exit.span)?;
            }
        }
        SymbolBody::Trait {
            assoc_types,
            methods,
        } => {
            for assoc_type in assoc_types {
                if let Some(default_ty) = &assoc_type.default_ty {
                    validate_tuple_type_contract(
                        default_ty,
                        assoc_type.span,
                        "associated type default",
                    )?;
                }
            }
            for method in methods {
                validate_symbol_tuple_contract(method)?;
            }
        }
    }

    validate_statement_block_tuple_contract(&symbol.statements)
}

fn validate_impl_tuple_contract(impl_decl: &ImplDecl) -> Result<(), String> {
    if matches!(impl_decl.target_type.kind, SurfaceTypeKind::Tuple(_)) {
        return Err(format!(
            "{}:{}: tuple impl targets are not part of v1",
            impl_decl.span.line, impl_decl.span.column
        ));
    }
    validate_tuple_type_contract(&impl_decl.target_type, impl_decl.span, "impl target type")?;
    for assoc_type in &impl_decl.assoc_types {
        if let Some(value_ty) = &assoc_type.value_ty {
            validate_tuple_type_contract(value_ty, assoc_type.span, "associated type binding")?;
        }
    }
    for method in &impl_decl.methods {
        validate_symbol_tuple_contract(method)?;
    }
    Ok(())
}

fn validate_statement_block_tuple_contract(statements: &[Statement]) -> Result<(), String> {
    for statement in statements {
        match &statement.kind {
            StatementKind::Let { value, .. } => {
                validate_expr_tuple_contract(value, statement.span)?;
            }
            StatementKind::Return { value } => {
                if let Some(value) = value {
                    validate_expr_tuple_contract(value, statement.span)?;
                }
            }
            StatementKind::Reclaim { expr } => {
                validate_expr_tuple_contract(expr, statement.span)?;
            }
            StatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                validate_expr_tuple_contract(condition, statement.span)?;
                validate_statement_block_tuple_contract(then_branch)?;
                if let Some(else_branch) = else_branch {
                    validate_statement_block_tuple_contract(else_branch)?;
                }
            }
            StatementKind::While { condition, body } => {
                validate_expr_tuple_contract(condition, statement.span)?;
                validate_statement_block_tuple_contract(body)?;
            }
            StatementKind::For { iterable, body, .. } => {
                validate_expr_tuple_contract(iterable, statement.span)?;
                validate_statement_block_tuple_contract(body)?;
            }
            StatementKind::Defer {
                action: DeferAction::Expr { expr } | DeferAction::Reclaim { expr },
            }
            | StatementKind::Expr { expr } => {
                validate_expr_tuple_contract(expr, statement.span)?;
            }
            StatementKind::Assign { target, value, .. } => {
                validate_assign_target_tuple_contract(target, statement.span)?;
                validate_expr_tuple_contract(value, statement.span)?;
            }
            StatementKind::Recycle {
                default_modifier,
                lines,
            } => {
                validate_headed_modifier_tuple_contract(default_modifier.as_ref(), statement.span)?;
                for line in lines {
                    match &line.kind {
                        RecycleLineKind::Expr { gate }
                        | RecycleLineKind::Let { gate, .. }
                        | RecycleLineKind::Assign { gate, .. } => {
                            validate_expr_tuple_contract(gate, line.span)?;
                        }
                    }
                    validate_headed_modifier_tuple_contract(line.modifier.as_ref(), line.span)?;
                }
            }
            StatementKind::Bind {
                default_modifier,
                lines,
            } => {
                validate_headed_modifier_tuple_contract(default_modifier.as_ref(), statement.span)?;
                for line in lines {
                    match &line.kind {
                        BindLineKind::Let { gate, .. } | BindLineKind::Assign { gate, .. } => {
                            validate_expr_tuple_contract(gate, line.span)?;
                        }
                        BindLineKind::Require { expr } => {
                            validate_expr_tuple_contract(expr, line.span)?;
                        }
                    }
                    validate_headed_modifier_tuple_contract(line.modifier.as_ref(), line.span)?;
                }
            }
            StatementKind::Record(region) => {
                validate_expr_tuple_contract(&region.target, region.span)?;
                if let Some(base) = &region.base {
                    validate_expr_tuple_contract(base, region.span)?;
                }
                validate_headed_modifier_tuple_contract(
                    region.default_modifier.as_ref(),
                    region.span,
                )?;
                if let Some(ConstructDestination::Place { target }) = &region.destination {
                    validate_assign_target_tuple_contract(target, region.span)?;
                }
                for line in &region.lines {
                    validate_expr_tuple_contract(&line.value, line.span)?;
                    validate_headed_modifier_tuple_contract(line.modifier.as_ref(), line.span)?;
                }
            }
            StatementKind::Array(region) => {
                validate_expr_tuple_contract(&region.target, region.span)?;
                if let Some(base) = &region.base {
                    validate_expr_tuple_contract(base, region.span)?;
                }
                validate_headed_modifier_tuple_contract(
                    region.default_modifier.as_ref(),
                    region.span,
                )?;
                if let Some(ConstructDestination::Place { target }) = &region.destination {
                    validate_assign_target_tuple_contract(target, region.span)?;
                }
                for line in &region.lines {
                    validate_expr_tuple_contract(&line.value, line.span)?;
                    validate_headed_modifier_tuple_contract(line.modifier.as_ref(), line.span)?;
                }
            }
            StatementKind::Construct(region) => {
                validate_expr_tuple_contract(&region.target, region.span)?;
                validate_headed_modifier_tuple_contract(
                    region.default_modifier.as_ref(),
                    region.span,
                )?;
                if let Some(ConstructDestination::Place { target }) = &region.destination {
                    validate_assign_target_tuple_contract(target, region.span)?;
                }
                for line in &region.lines {
                    validate_expr_tuple_contract(&line.value, line.span)?;
                    validate_headed_modifier_tuple_contract(line.modifier.as_ref(), line.span)?;
                }
            }
            StatementKind::MemorySpec(spec) => {
                validate_headed_modifier_tuple_contract(spec.default_modifier.as_ref(), spec.span)?;
                for detail in &spec.details {
                    validate_expr_tuple_contract(&detail.value, detail.span)?;
                    validate_headed_modifier_tuple_contract(detail.modifier.as_ref(), detail.span)?;
                }
            }
            StatementKind::Break | StatementKind::Continue => {}
        }
    }
    Ok(())
}

fn validate_expr_tuple_contract(expr: &Expr, span: Span) -> Result<(), String> {
    match expr {
        Expr::Path { .. }
        | Expr::BoolLiteral { .. }
        | Expr::IntLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StrLiteral { .. } => Ok(()),
        Expr::Pair { left, right } => {
            validate_expr_tuple_contract(left, span)?;
            validate_expr_tuple_contract(right, span)
        }
        Expr::CollectionLiteral { items } => {
            for item in items {
                validate_expr_tuple_contract(item, span)?;
            }
            Ok(())
        }
        Expr::Match { subject, arms } => {
            validate_expr_tuple_contract(subject, span)?;
            for arm in arms {
                validate_expr_tuple_contract(&arm.value, arm.span)?;
            }
            Ok(())
        }
        Expr::RecordRegion(region) => {
            validate_expr_tuple_contract(&region.target, region.span)?;
            if let Some(base) = &region.base {
                validate_expr_tuple_contract(base, region.span)?;
            }
            validate_headed_modifier_tuple_contract(region.default_modifier.as_ref(), region.span)?;
            for line in &region.lines {
                validate_expr_tuple_contract(&line.value, line.span)?;
                validate_headed_modifier_tuple_contract(line.modifier.as_ref(), line.span)?;
            }
            Ok(())
        }
        Expr::ArrayRegion(region) => {
            validate_expr_tuple_contract(&region.target, region.span)?;
            if let Some(base) = &region.base {
                validate_expr_tuple_contract(base, region.span)?;
            }
            validate_headed_modifier_tuple_contract(region.default_modifier.as_ref(), region.span)?;
            if let Some(ConstructDestination::Place { target }) = &region.destination {
                validate_assign_target_tuple_contract(target, region.span)?;
            }
            for line in &region.lines {
                validate_expr_tuple_contract(&line.value, line.span)?;
                validate_headed_modifier_tuple_contract(line.modifier.as_ref(), line.span)?;
            }
            Ok(())
        }
        Expr::ConstructRegion(region) => {
            validate_expr_tuple_contract(&region.target, region.span)?;
            validate_headed_modifier_tuple_contract(region.default_modifier.as_ref(), region.span)?;
            for line in &region.lines {
                validate_expr_tuple_contract(&line.value, line.span)?;
                validate_headed_modifier_tuple_contract(line.modifier.as_ref(), line.span)?;
            }
            Ok(())
        }
        Expr::Chain { steps, .. } => {
            for step in steps {
                validate_expr_tuple_contract(&step.stage, span)?;
                for arg in &step.bind_args {
                    validate_expr_tuple_contract(arg, span)?;
                }
            }
            Ok(())
        }
        Expr::MemoryPhrase {
            arena,
            init_args,
            attached,
            ..
        } => {
            validate_expr_tuple_contract(arena, span)?;
            for arg in init_args {
                match arg {
                    PhraseArg::Positional(expr) => validate_expr_tuple_contract(expr, span)?,
                    PhraseArg::Named { value, .. } => validate_expr_tuple_contract(value, span)?,
                }
            }
            for attachment in attached {
                validate_header_attachment_tuple_contract(attachment, span)?;
            }
            Ok(())
        }
        Expr::GenericApply { expr, type_args } => {
            validate_expr_tuple_contract(expr, span)?;
            for type_arg in type_args {
                validate_tuple_type_contract(type_arg, span, "generic argument")?;
            }
            Ok(())
        }
        Expr::QualifiedPhrase {
            subject,
            args,
            attached,
            ..
        } => {
            validate_expr_tuple_contract(subject, span)?;
            for arg in args {
                match arg {
                    PhraseArg::Positional(expr) => validate_expr_tuple_contract(expr, span)?,
                    PhraseArg::Named { value, .. } => validate_expr_tuple_contract(value, span)?,
                }
            }
            for attachment in attached {
                validate_header_attachment_tuple_contract(attachment, span)?;
            }
            Ok(())
        }
        Expr::Await { expr } | Expr::Unary { expr, .. } => validate_expr_tuple_contract(expr, span),
        Expr::Binary { left, right, .. } => {
            validate_expr_tuple_contract(left, span)?;
            validate_expr_tuple_contract(right, span)
        }
        Expr::MemberAccess { expr, member } => {
            validate_expr_tuple_contract(expr, span)?;
            validate_tuple_member_access(member, span)
        }
        Expr::Index { expr, index } => {
            validate_expr_tuple_contract(expr, span)?;
            validate_expr_tuple_contract(index, span)
        }
        Expr::Slice {
            expr,
            start,
            end,
            len,
            stride,
            ..
        } => {
            validate_expr_tuple_contract(expr, span)?;
            if let Some(start) = start {
                validate_expr_tuple_contract(start, span)?;
            }
            if let Some(end) = end {
                validate_expr_tuple_contract(end, span)?;
            }
            if let Some(len) = len {
                validate_expr_tuple_contract(len, span)?;
            }
            if let Some(stride) = stride {
                validate_expr_tuple_contract(stride, span)?;
            }
            Ok(())
        }
        Expr::Range { start, end, .. } => {
            if let Some(start) = start {
                validate_expr_tuple_contract(start, span)?;
            }
            if let Some(end) = end {
                validate_expr_tuple_contract(end, span)?;
            }
            Ok(())
        }
    }
}

fn validate_header_attachment_tuple_contract(
    attachment: &HeaderAttachment,
    span: Span,
) -> Result<(), String> {
    match attachment {
        HeaderAttachment::Named { value, .. } | HeaderAttachment::Chain { expr: value, .. } => {
            validate_expr_tuple_contract(value, span)
        }
    }
}

fn validate_headed_modifier_tuple_contract(
    modifier: Option<&HeadedModifier>,
    span: Span,
) -> Result<(), String> {
    if let Some(modifier) = modifier
        && let Some(payload) = &modifier.payload
    {
        validate_expr_tuple_contract(payload, span)?;
    }
    Ok(())
}

fn validate_assign_target_tuple_contract(target: &AssignTarget, span: Span) -> Result<(), String> {
    match target {
        AssignTarget::Name { .. } => Ok(()),
        AssignTarget::Deref { .. } => Ok(()),
        AssignTarget::MemberAccess { target, member } => {
            validate_assign_target_tuple_contract(target, span)?;
            if is_numeric_member_selector(member) {
                return Err(format!(
                    "{}:{}: tuple field assignment is not allowed in v1",
                    span.line, span.column
                ));
            }
            Ok(())
        }
        AssignTarget::Index { target, index } => {
            validate_assign_target_tuple_contract(target, span)?;
            validate_expr_tuple_contract(index, span)
        }
    }
}

fn validate_raw_function_header_tuple_contract(text: &str, span: Span) -> Result<(), String> {
    let Some((params, return_type)) = extract_raw_function_signature_parts(text) else {
        return Ok(());
    };
    validate_raw_param_list_tuple_contract(params, span)?;
    if let Some(return_type) = return_type {
        validate_tuple_group_text_contract(return_type, span, "return type")?;
    }
    Ok(())
}

fn extract_raw_function_signature_parts(text: &str) -> Option<(&str, Option<&str>)> {
    let rest = text.strip_prefix("export ").unwrap_or(text).trim_start();
    let rest = rest.strip_prefix("async ").unwrap_or(rest).trim_start();
    let rest = if rest.starts_with("behavior") {
        let open_idx = rest.find('[')?;
        if !rest[..open_idx].trim().eq("behavior") {
            return None;
        }
        let close_idx = find_matching_delim(rest, open_idx, '[', ']')?;
        rest[close_idx + 1..].trim_start().strip_prefix("fn ")?
    } else {
        rest.strip_prefix("fn ")?
    };

    let header = rest.strip_suffix(':').unwrap_or(rest).trim();
    let name = parse_symbol_name(header)?;
    let after_name = &header[name.len()..];
    let (_, _, remainder) = parse_type_params_and_where(after_name)?;
    let remainder = remainder.trim();
    let open_idx = remainder.find('(')?;
    let close_idx = find_matching_delim(remainder, open_idx, '(', ')')?;
    let params = &remainder[open_idx + 1..close_idx];
    let return_type = remainder[close_idx + 1..]
        .trim()
        .strip_prefix("->")
        .map(str::trim)
        .filter(|value| !value.is_empty());
    Some((params, return_type))
}

fn validate_raw_param_list_tuple_contract(source: &str, span: Span) -> Result<(), String> {
    for part in split_top_level(source, ',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if looks_like_tuple_binding(part) {
            return Err(format!(
                "{}:{}: tuple destructuring is not allowed in parameter lists",
                span.line, span.column
            ));
        }
        if let Some((binding, ty)) = part.split_once(':') {
            if looks_like_tuple_binding(binding.trim()) {
                return Err(format!(
                    "{}:{}: tuple destructuring is not allowed in parameter lists",
                    span.line, span.column
                ));
            }
            let ty = ty.trim();
            if !ty.is_empty() {
                validate_tuple_group_text_contract(ty, span, "parameter type")?;
            }
        }
    }
    Ok(())
}

fn validate_tuple_group_text_contract(text: &str, span: Span, context: &str) -> Result<(), String> {
    validate_tuple_groups_in_text(text, span, context, "tuple types")
}

fn validate_tuple_groups_in_text(
    text: &str,
    span: Span,
    context: &str,
    tuple_label: &str,
) -> Result<(), String> {
    let text = text.trim();
    let mut index = 0usize;
    let mut in_string = false;
    let mut escape = false;

    while index < text.len() {
        let mut chars = text[index..].chars();
        let Some(ch) = chars.next() else {
            break;
        };

        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += ch.len_utf8();
            continue;
        }

        if ch == '"' {
            in_string = true;
            index += ch.len_utf8();
            continue;
        }

        if ch == '(' {
            let Some(close_idx) = find_matching_delim(text, index, '(', ')') else {
                break;
            };
            let inner = text[index + 1..close_idx].trim();
            if contains_top_level_char(inner, ',') {
                let parts = split_top_level(inner, ',')
                    .into_iter()
                    .map(str::trim)
                    .collect::<Vec<_>>();
                if parts.len() != 2 || parts.iter().any(|part| part.is_empty()) {
                    return Err(format!(
                        "{}:{}: {tuple_label} must have exactly 2 elements in v1 ({context})",
                        span.line, span.column
                    ));
                }
                for part in parts {
                    validate_tuple_groups_in_text(part, span, context, tuple_label)?;
                }
            } else if !inner.is_empty() {
                validate_tuple_groups_in_text(inner, span, context, tuple_label)?;
            }
            index = close_idx + 1;
            continue;
        }

        index += ch.len_utf8();
    }

    Ok(())
}

fn validate_tuple_member_access(member: &str, span: Span) -> Result<(), String> {
    if is_numeric_member_selector(member) && member != "0" && member != "1" {
        return Err(format!(
            "{}:{}: tuple field access only supports `.0` and `.1` in v1",
            span.line, span.column
        ));
    }
    Ok(())
}

fn looks_like_tuple_binding(text: &str) -> bool {
    tuple_parts_if_whole(text).is_some()
}

fn is_valid_tuple_binding_pattern(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    if is_identifier(trimmed) {
        return true;
    }
    let Some(parts) = tuple_parts_if_whole(trimmed) else {
        return false;
    };
    parts.len() == 2
        && parts
            .iter()
            .all(|part| is_valid_tuple_binding_pattern(part.trim()))
}

fn tuple_parts_if_whole(text: &str) -> Option<Vec<&str>> {
    let trimmed = text.trim();
    if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
        return None;
    }
    let close_idx = find_matching_delim(trimmed, 0, '(', ')')?;
    if close_idx != trimmed.len() - 1 {
        return None;
    }
    let inner = trimmed[1..close_idx].trim();
    if !contains_top_level_char(inner, ',') {
        return None;
    }
    Some(
        split_top_level(inner, ',')
            .into_iter()
            .map(str::trim)
            .collect(),
    )
}

fn is_numeric_member_selector(member: &str) -> bool {
    !member.is_empty() && member.chars().all(|ch| ch.is_ascii_digit())
}

fn split_top_level(source: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut start = 0usize;

    for (idx, ch) in source.char_indices() {
        if in_string {
            match ch {
                '\\' if !escaped => {
                    escaped = true;
                }
                '"' if !escaped => {
                    in_string = false;
                }
                _ => {
                    escaped = false;
                }
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if ch == separator && depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 {
            parts.push(&source[start..idx]);
            start = idx + ch.len_utf8();
        }
    }

    parts.push(&source[start..]);
    parts
}

fn find_matching_delim(source: &str, open_idx: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, ch) in source.char_indices().skip_while(|(idx, _)| *idx < open_idx) {
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

fn parse_symbol_name(rest: &str) -> Option<String> {
    let mut chars = rest.chars();
    let first = chars.next()?;
    if !is_identifier_start(first) {
        return None;
    }

    let mut name = String::new();
    name.push(first);
    for ch in chars {
        if !is_identifier_continue(ch) {
            break;
        }
        name.push(ch);
    }
    Some(name)
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_identifier_start(first) {
        return false;
    }
    chars.all(is_identifier_continue)
}

fn is_path_like(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed.split('.').all(is_identifier)
}

fn is_int_literal(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit())
}

fn parse_float_literal_kind(value: &str) -> FloatLiteralKind {
    let trimmed = value.trim();
    if trimmed.ends_with("f32") {
        FloatLiteralKind::F32
    } else {
        FloatLiteralKind::F64
    }
}

fn is_float_literal(value: &str) -> bool {
    let trimmed = value.trim();
    let core = trimmed
        .strip_suffix("f32")
        .or_else(|| trimmed.strip_suffix("f64"))
        .unwrap_or(trimmed);
    let Some((left, right)) = core.split_once('.') else {
        return false;
    };
    !left.is_empty()
        && !right.is_empty()
        && left.chars().all(|ch| ch.is_ascii_digit())
        && right.chars().all(|ch| ch.is_ascii_digit())
}

fn is_string_literal(value: &str) -> bool {
    if !value.starts_with('"') || !value.ends_with('"') || value.len() < 2 {
        return false;
    }
    let mut escape = false;
    let inner = &value[1..value.len() - 1];
    for ch in inner.chars() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' {
            escape = true;
        } else if ch == '"' {
            return false;
        }
    }
    !escape
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::freeze::{FROZEN_AST_NODE_KINDS, FROZEN_TOKEN_KINDS};
    use super::{
        AssignTarget, BUILTIN_TYPE_INFOS, BinaryOp, ChainConnector, ChainIntroducer, ChainStep,
        DeferAction, DirectiveKind, Expr, ForewordAliasKind, ForewordApp, ForewordArg,
        ForewordDefinitionTarget, HeaderAttachment, MatchPattern, OpaqueBoundaryPolicy,
        OpaqueOwnershipPolicy, OpaqueTypePolicy, ParamMode, PhraseArg,
        QualifiedPhraseQualifierKind, ShackleDeclKind, ShackleRawDecl, Statement, StatementKind,
        SurfaceTraitRef, SurfaceType, SurfaceWhereClause, SymbolBody, SymbolKind, UnaryOp,
        builtin_type_info, parse_module,
    };

    fn expr_is_path(expr: &Expr, name: &str) -> bool {
        matches!(expr, Expr::Path { segments } if segments == &vec![name.to_string()])
    }

    fn expr_is_int_literal(expr: &Expr, text: &str) -> bool {
        matches!(expr, Expr::IntLiteral { text: value } if value == text)
    }

    fn expr_is_str_literal(expr: &Expr, text: &str) -> bool {
        matches!(expr, Expr::StrLiteral { text: value } if value == text)
    }

    fn chain_step_texts(steps: &[ChainStep]) -> Vec<String> {
        steps.iter().map(|step| step.text.clone()).collect()
    }

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root should resolve")
    }

    fn collect_arc_files(root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let entries = fs::read_dir(&dir).expect("dir should be readable");
            for entry in entries {
                let entry = entry.expect("entry should be readable");
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().and_then(|ext| ext.to_str()) == Some("arc") {
                    files.push(path);
                }
            }
        }
        files.sort();
        files
    }

    #[test]
    fn frozen_lists_are_unique() {
        let mut tokens = FROZEN_TOKEN_KINDS.to_vec();
        tokens.sort_unstable();
        tokens.dedup();
        assert_eq!(tokens.len(), FROZEN_TOKEN_KINDS.len());

        let mut nodes = FROZEN_AST_NODE_KINDS.to_vec();
        nodes.sort_unstable();
        nodes.dedup();
        assert_eq!(nodes.len(), FROZEN_AST_NODE_KINDS.len());
    }

    #[test]
    fn parse_module_rejects_tabs() {
        let err = parse_module("fn main()\n\treturn 0\n").expect_err("expected tab rejection");
        assert!(err.contains("tabs are not allowed"));
    }

    #[test]
    fn rewrite_owned_and_conformance_corpus_parses_as_supported_syntax() {
        for root in [
            repo_root().join("std").join("src"),
            repo_root()
                .join("grimoires")
                .join("arcana")
                .join("process")
                .join("src"),
            repo_root()
                .join("grimoires")
                .join("arcana")
                .join("winapi")
                .join("src"),
            repo_root()
                .join("conformance")
                .join("fixtures")
                .join("types_guard_workspace")
                .join("app")
                .join("src"),
            repo_root()
                .join("conformance")
                .join("fixtures")
                .join("types_guard_workspace")
                .join("core")
                .join("src"),
        ] {
            for file in collect_arc_files(&root) {
                let source = fs::read_to_string(&file).expect("source should be readable");
                parse_module(&source)
                    .unwrap_or_else(|err| panic!("{} should parse: {err}", file.display()));
            }
        }
    }

    #[test]
    fn parse_module_collects_directives_and_symbols() {
        let parsed = parse_module(
            "import arcana_process.io\nuse std.result.Result\nreexport types\nexport record Counter:\n    value: Int\nexport enum Result[T]:\n    Ok(Int)\n    Err(Str)\nexport trait CounterOps[T]:\n    type Output\n    fn tick(edit self: T) -> Int:\n        return 0\nfn main() -> Int:\n",
        )
        .expect("parse should pass");

        assert_eq!(parsed.directives.len(), 3);
        assert_eq!(parsed.directives[0].kind, DirectiveKind::Import);
        assert_eq!(parsed.directives[0].path, ["arcana_process", "io"]);
        assert_eq!(parsed.directives[1].kind, DirectiveKind::Use);
        assert_eq!(parsed.directives[1].path, ["std", "result", "Result"]);
        assert_eq!(parsed.symbols.len(), 4);
        assert_eq!(parsed.symbols[0].name, "Counter");
        assert_eq!(parsed.symbols[0].kind.as_str(), "record");
        assert!(parsed.symbols[0].exported);
        assert_eq!(
            parsed.symbols[0].surface_text,
            "record Counter:\nvalue: Int"
        );
        assert_eq!(parsed.symbols[0].type_params, Vec::<String>::new());
        match &parsed.symbols[0].body {
            SymbolBody::Record { fields } => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].name, "value");
                assert_eq!(fields[0].ty.render(), "Int");
            }
            other => panic!("expected record body, got {other:?}"),
        }
        assert_eq!(parsed.symbols[1].name, "Result");
        match &parsed.symbols[1].body {
            SymbolBody::Enum { variants } => {
                assert_eq!(variants.len(), 2);
                assert_eq!(variants[0].name, "Ok");
                assert_eq!(
                    variants[0].payload.as_ref().map(SurfaceType::render),
                    Some("Int".to_string())
                );
                assert_eq!(
                    variants[1].payload.as_ref().map(SurfaceType::render),
                    Some("Str".to_string())
                );
            }
            other => panic!("expected enum body, got {other:?}"),
        }
        assert_eq!(parsed.symbols[2].name, "CounterOps");
        match &parsed.symbols[2].body {
            SymbolBody::Trait {
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
        assert_eq!(parsed.symbols[3].name, "main");
        assert_eq!(parsed.symbols[3].kind.as_str(), "fn");
        assert!(!parsed.symbols[3].exported);
        assert_eq!(parsed.symbols[3].surface_text, "fn main() -> Int:");
        assert_eq!(
            parsed.symbols[3]
                .return_type
                .as_ref()
                .map(SurfaceType::render),
            Some("Int".to_string())
        );
        assert!(parsed.symbols[3].statements.is_empty());
    }

    #[test]
    fn parse_module_collects_async_functions_and_impls() {
        let parsed = parse_module(
            "async fn worker[T, where std.iter.Iterator[T]](read it: T, count: Int) -> Int:\n    return count\nbehavior[phase=update, affinity=worker] fn tick():\n    return 0\nimpl[T] std.iter.Iterator[T] for RangeIter:\n    type Item = Int\n    fn next(edit self: RangeIter) -> (Bool, Int):\n        return (false, 0)\n",
        )
        .expect("parse should pass");

        assert_eq!(parsed.symbols.len(), 2);
        let worker = &parsed.symbols[0];
        assert!(worker.is_async);
        assert_eq!(worker.type_params, vec!["T".to_string()]);
        assert_eq!(
            worker.where_clause.as_ref().map(SurfaceWhereClause::render),
            Some("std.iter.Iterator[T]".to_string())
        );
        assert_eq!(worker.params.len(), 2);
        assert_eq!(worker.params[0].mode, Some(ParamMode::Read));
        assert_eq!(worker.params[0].name, "it");
        assert_eq!(worker.params[0].ty.render(), "T");
        assert_eq!(worker.params[1].mode, None);
        assert_eq!(
            worker.return_type.as_ref().map(SurfaceType::render),
            Some("Int".to_string())
        );
        let tick = &parsed.symbols[1];
        assert_eq!(tick.kind, SymbolKind::Behavior);
        assert_eq!(tick.name, "tick");
        assert_eq!(tick.behavior_attrs.len(), 2);
        assert_eq!(tick.behavior_attrs[0].name, "phase");
        assert_eq!(tick.behavior_attrs[0].value, "update");
        assert_eq!(tick.behavior_attrs[1].value, "worker");

        assert_eq!(parsed.impls.len(), 1);
        let impl_decl = &parsed.impls[0];
        assert_eq!(
            impl_decl.trait_path.as_ref().map(SurfaceTraitRef::render),
            Some("std.iter.Iterator[T]".to_string())
        );
        assert_eq!(impl_decl.type_params, vec!["T".to_string()]);
        assert_eq!(impl_decl.target_type.render(), "RangeIter");
        assert_eq!(impl_decl.body_entries.len(), 2);
        assert!(impl_decl.body_entries[0].starts_with("type Item"));
        assert_eq!(impl_decl.assoc_types.len(), 1);
        assert_eq!(impl_decl.assoc_types[0].name, "Item");
        assert_eq!(
            impl_decl.assoc_types[0]
                .value_ty
                .as_ref()
                .map(SurfaceType::render),
            Some("Int".to_string())
        );
        assert_eq!(impl_decl.methods.len(), 1);
        assert_eq!(impl_decl.methods[0].name, "next");
    }

    #[test]
    fn parse_module_collects_structured_statements() {
        let parsed = parse_module(
            "fn main() -> Int:\n    let mut frames = 0\n    while frames < 10:\n        if frames % 2 == 0:\n            frames += 1\n        else:\n            continue\n    return match frames:\n        10 => 1\n        _ => 0\n",
        )
        .expect("parse should pass");

        let statements = &parsed.symbols[0].statements;
        assert_eq!(statements.len(), 3);
        match &statements[0].kind {
            StatementKind::Let {
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
            StatementKind::While { condition, body } => {
                match condition {
                    Expr::Binary { left, op, right } => {
                        assert_eq!(*op, BinaryOp::Lt);
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
                    StatementKind::If {
                        condition,
                        then_branch,
                        else_branch,
                    } => {
                        match condition {
                            Expr::Binary { left, op, right } => {
                                assert_eq!(*op, BinaryOp::EqEq);
                                match left.as_ref() {
                                    Expr::Binary { left, op, right } => {
                                        assert_eq!(*op, BinaryOp::Mod);
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
                        assert!(matches!(
                            then_branch[0].kind,
                            StatementKind::Assign {
                                target: AssignTarget::Name { ref text },
                                ..
                            } if text == "frames"
                        ));
                        let else_branch = else_branch.as_ref().expect("else branch should exist");
                        assert_eq!(else_branch.len(), 1);
                        assert!(matches!(else_branch[0].kind, StatementKind::Continue));
                    }
                    other => panic!("expected nested if statement, got {other:?}"),
                }
            }
            other => panic!("expected while statement, got {other:?}"),
        }
        match &statements[2].kind {
            StatementKind::Return { value } => {
                match value.as_ref().expect("return should carry a value") {
                    Expr::Match { subject, arms } => {
                        assert!(matches!(
                            subject.as_ref(),
                            expr if expr_is_path(expr, "frames")
                        ));
                        assert_eq!(arms.len(), 2);
                        assert_eq!(
                            arms[0].patterns,
                            vec![MatchPattern::Literal {
                                text: "10".to_string()
                            }]
                        );
                        assert!(matches!(
                            arms[0].value,
                            ref expr if expr_is_int_literal(expr, "1")
                        ));
                        assert_eq!(arms[1].patterns, vec![MatchPattern::Wildcard]);
                    }
                    other => panic!("expected match expression, got {other:?}"),
                }
            }
            other => panic!("expected return statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_collects_match_expressions() {
        let parsed = parse_module(
            "fn score(t: Token) -> Int:\n    return match t:\n        Token.Plus | Token.Minus => 1\n        Token.IntLit(v) => v\nfn main() -> Int:\n    let out = score :: Token.Minus :: call\n    let v = match out:\n        0 => 0\n        _ => 1\n    return v\n",
        )
        .expect("parse should pass");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Return { value } => match value.as_ref().expect("match return expected")
            {
                Expr::Match { subject, arms } => {
                    assert!(matches!(
                        subject.as_ref(),
                        expr if expr_is_path(expr, "t")
                    ));
                    assert_eq!(
                        arms[0].patterns,
                        vec![
                            MatchPattern::Name {
                                text: "Token.Plus".to_string()
                            },
                            MatchPattern::Name {
                                text: "Token.Minus".to_string()
                            }
                        ]
                    );
                    assert_eq!(
                        arms[1].patterns,
                        vec![MatchPattern::Variant {
                            path: "Token.IntLit".to_string(),
                            args: vec![MatchPattern::Name {
                                text: "v".to_string()
                            }]
                        }]
                    );
                }
                other => panic!("expected match expression, got {other:?}"),
            },
            other => panic!("expected return statement, got {other:?}"),
        }

        match &parsed.symbols[1].statements[1].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "v");
                match value {
                    Expr::Match { subject, arms } => {
                        assert!(matches!(
                            subject.as_ref(),
                            expr if expr_is_path(expr, "out")
                        ));
                        assert_eq!(arms.len(), 2);
                        assert_eq!(
                            arms[0].patterns,
                            vec![MatchPattern::Literal {
                                text: "0".to_string()
                            }]
                        );
                        assert_eq!(arms[1].patterns, vec![MatchPattern::Wildcard]);
                    }
                    other => panic!("expected match expression, got {other:?}"),
                }
            }
            other => panic!("expected let statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_rejects_break_outside_loop() {
        let err = parse_module("fn main() -> Int:\n    break\n").expect_err("break should fail");
        assert!(err.contains("`break` is only valid inside loops"), "{err}");
    }

    #[test]
    fn parse_module_rejects_stray_else() {
        let err = parse_module("fn main() -> Int:\n    else:\n        return 0\n")
            .expect_err("else should fail");
        assert!(err.contains("`else` without a preceding `if`"), "{err}");
    }

    #[test]
    fn parse_module_collects_cleanup_footers() {
        let parsed = parse_module(
            "fn cleanup(take value: Int) -> Result[Unit, Str]:\n    return Result.Ok[Unit, Str] :: :: call\nfn run(seed: Int) -> Int:\n    let local = seed\n    while local > 0:\n        let scratch = local\n        local -= 1\n    -cleanup[target = scratch, handler = cleanup]\n    return local\n-cleanup[target = seed, handler = cleanup]\n",
        )
        .expect("cleanup footers should parse");

        let run = parsed
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
            Statement {
                kind: StatementKind::While { .. },
                cleanup_footers,
                ..
            } => {
                assert_eq!(cleanup_footers.len(), 1);
                assert_eq!(cleanup_footers[0].subject, "scratch");
                assert_eq!(cleanup_footers[0].kind.as_str(), "cleanup");
            }
            other => panic!("expected while statement with cleanup footer, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_rejects_ownerless_cleanup_footers() {
        let err = parse_module(
            "-cleanup[target = value, handler = cleanup]\nfn cleanup(value: Int):\n    return\n",
        )
        .expect_err("ownerless cleanup footer should fail");
        assert!(
            err.contains("cleanup footer without a valid owning header"),
            "{err}"
        );
    }

    #[test]
    fn parse_module_allows_unknown_cleanup_footer_targets_for_later_semantic_validation() {
        let parsed = parse_module(
            "fn cleanup(take value: Int) -> Result[Unit, Str]:\n    return Result.Ok[Unit, Str] :: :: call\nfn main() -> Int:\n    let value = 1\n    return value\n-cleanup[target = missing, handler = cleanup]\n",
        )
        .expect("unknown cleanup footer target should defer to semantic validation");
        assert_eq!(parsed.symbols[1].cleanup_footers[0].subject, "missing");
    }

    #[test]
    fn parse_module_allows_reassigned_cleanup_footer_targets_for_later_semantic_validation() {
        let parsed = parse_module(
            "fn cleanup(take value: Int) -> Result[Unit, Str]:\n    return Result.Ok[Unit, Str] :: :: call\nfn main(seed: Int) -> Int:\n    let local = seed\n    local += 1\n    return local\n-cleanup[target = local, handler = cleanup]\n",
        )
        .expect("cleanup footer target reassignment should defer to semantic validation");
        assert_eq!(parsed.symbols[1].cleanup_footers[0].subject, "local");
    }

    #[test]
    fn parse_module_allows_cleanup_footers_on_systems() {
        let parsed = parse_module(
            "enum Result[T, E]:\n    Ok(T)\n    Err(E)\nfn cleanup(take value: Int) -> Result[Unit, Str]:\n    return Result.Ok[Unit, Str] :: :: call\nsystem[phase=startup] fn boot() -> Int:\n    let value = 1\n    return value\n-cleanup[target = value, handler = cleanup]\n",
        )
        .expect("system cleanup footer should parse");
        assert_eq!(parsed.symbols[2].kind, SymbolKind::System);
        assert_eq!(parsed.symbols[2].cleanup_footers.len(), 1);
        assert_eq!(parsed.symbols[2].cleanup_footers[0].subject, "value");
    }

    #[test]
    fn parse_module_rejects_unsupported_attached_dash_footer_forms() {
        let err = parse_module("fn main() -> Int:\n    return 0\n-defer\n")
            .expect_err("unsupported attached dash footer should fail");
        assert!(
            err.contains(
                "attached `-defer` is unsupported here; this footer position currently accepts only `-cleanup`"
            ),
            "{err}"
        );
    }

    #[test]
    fn parse_module_rejects_duplicate_bare_cleanup_footers() {
        let err = parse_module("fn main() -> Int:\n    return 0\n-cleanup\n-cleanup\n")
            .expect_err("duplicate bare cleanup footers should fail");
        assert!(err.contains("duplicate bare `-cleanup` footer"), "{err}");
    }

    #[test]
    fn parse_module_rejects_redundant_targeted_cleanup_under_bare_cleanup() {
        let err = parse_module(
            "fn cleanup(take value: Int) -> Result[Unit, Str]:\n    return Result.Ok[Unit, Str] :: :: call\nfn main(seed: Int) -> Int:\n    return seed\n-cleanup\n-cleanup[target = seed]\n",
        )
        .expect_err("redundant targeted cleanup footer should fail");
        assert!(
            err.contains(
                "`-cleanup[target = seed]` is redundant when bare `-cleanup` is already present"
            ),
            "{err}"
        );
    }

    #[test]
    fn parse_module_collects_forewords_lang_items_intrinsics_and_systems() {
        let parsed = parse_module(
            "use std.result as result\n#test\n#inline\nfn smoke() -> Int:\n    return 0\n#stage[thread=worker, deterministic=true]\nsystem[phase=startup, affinity=main] fn boot():\n    #chain[phase=startup, deterministic=true]\n    forward :=> seed => step\nlang result = smoke\nintrinsic fn host_len(read text: Str) -> Int = HostTextLenBytes\nfn seed() -> Int:\n    return 1\nfn step(v: Int) -> Int:\n    return v\n",
        )
        .expect("module should parse");

        assert_eq!(parsed.directives[0].forewords.len(), 0);
        assert_eq!(parsed.lang_items[0].name, "result");
        assert_eq!(parsed.symbols[0].forewords.len(), 2);
        assert_eq!(parsed.symbols[1].kind, SymbolKind::System);
        assert_eq!(parsed.symbols[1].forewords[0].name, "stage");
        assert_eq!(parsed.symbols[1].statements[0].forewords[0].name, "chain");
        assert!(parsed.symbols[2].intrinsic_impl.is_some());
    }

    #[test]
    fn parse_module_collects_foreword_handlers_aliases_reexports_and_member_targets() {
        let parsed = parse_module(concat!(
            "foreword tool.exec.trace:\n",
            "    tier = executable\n",
            "    visibility = public\n",
            "    action = metadata\n",
            "    targets = [fn, field, param]\n",
            "    retention = runtime\n",
            "    payload = [label: Str]\n",
            "    handler = tool.exec.trace_handler\n",
            "foreword handler tool.exec.trace_handler:\n",
            "    protocol = \"stdio-v1\"\n",
            "    product = \"trace\"\n",
            "    entry = \"main\"\n",
            "foreword alias tool.exec.local = tool.exec.trace\n",
            "foreword reexport tool.exec.public = tool.exec.trace\n",
            "record Box:\n",
            "    #tool.exec.local[label = \"field\"]\n",
            "    value: Int\n",
            "#tool.exec.trace[label = \"fn\"]\n",
            "fn helper(#tool.exec.public[label = \"param\"] value: Int) -> Int:\n",
            "    return value\n",
        ))
        .expect("module should parse");

        assert_eq!(parsed.foreword_definitions.len(), 1);
        let definition = &parsed.foreword_definitions[0];
        assert_eq!(definition.qualified_name, vec!["tool", "exec", "trace"]);
        assert_eq!(
            definition.handler.as_ref(),
            Some(&vec![
                "tool".to_string(),
                "exec".to_string(),
                "trace_handler".to_string(),
            ])
        );
        assert_eq!(
            definition.targets,
            vec![
                ForewordDefinitionTarget::Function,
                ForewordDefinitionTarget::Field,
                ForewordDefinitionTarget::Param,
            ]
        );

        assert_eq!(parsed.foreword_handlers.len(), 1);
        assert_eq!(
            parsed.foreword_handlers[0].qualified_name,
            vec!["tool", "exec", "trace_handler"]
        );
        assert_eq!(parsed.foreword_handlers[0].protocol, "stdio-v1");
        assert_eq!(parsed.foreword_handlers[0].product, "trace");
        assert_eq!(parsed.foreword_handlers[0].entry, "main");

        assert_eq!(parsed.foreword_aliases.len(), 2);
        assert_eq!(parsed.foreword_aliases[0].kind, ForewordAliasKind::Alias);
        assert_eq!(
            parsed.foreword_aliases[0].alias_name,
            vec!["tool", "exec", "local"]
        );
        assert_eq!(parsed.foreword_aliases[1].kind, ForewordAliasKind::Reexport);
        assert_eq!(
            parsed.foreword_aliases[1].alias_name,
            vec!["tool", "exec", "public"]
        );

        let record = parsed
            .symbols
            .iter()
            .find(|symbol| symbol.name == "Box")
            .expect("record should parse");
        match &record.body {
            SymbolBody::Record { fields } => {
                assert_eq!(fields[0].forewords[0].path, vec!["tool", "exec", "local"]);
            }
            other => panic!("expected record body, got {other:?}"),
        }

        let helper = parsed
            .symbols
            .iter()
            .find(|symbol| symbol.name == "helper")
            .expect("helper should parse");
        assert_eq!(helper.forewords[0].path, vec!["tool", "exec", "trace"]);
        assert_eq!(
            helper.params[0].forewords[0].path,
            vec!["tool", "exec", "public"]
        );
    }

    #[test]
    fn parse_module_collects_chain_collection_and_memory_expressions() {
        let parsed = parse_module(
            "fn main() -> Int:\n    let xs = [1, 2, 3]\n    let id = arena: store :> value = 1 <: Item\n    forward :=> seed => step\n    return xs[0]\n",
        )
        .expect("module should parse");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Let {
                value: Expr::CollectionLiteral { items },
                ..
            } => {
                assert_eq!(items.len(), 3);
            }
            other => panic!("expected collection literal, got {other:?}"),
        }
        match &parsed.symbols[0].statements[1].kind {
            StatementKind::Let {
                value:
                    Expr::MemoryPhrase {
                        family,
                        constructor,
                        ..
                    },
                ..
            } => {
                assert_eq!(family, "arena");
                match constructor.as_ref() {
                    Expr::Path { segments } => assert_eq!(segments, &vec!["Item".to_string()]),
                    other => panic!("expected constructor path, got {other:?}"),
                }
            }
            other => panic!("expected memory phrase, got {other:?}"),
        }
        match &parsed.symbols[0].statements[2].kind {
            StatementKind::Expr {
                expr:
                    Expr::Chain {
                        style,
                        introducer,
                        steps,
                    },
            } => {
                assert_eq!(style, "forward");
                assert_eq!(*introducer, ChainIntroducer::Forward);
                assert_eq!(chain_step_texts(steps), vec!["seed", "step"]);
                assert!(steps[0].incoming.is_none());
                assert_eq!(steps[1].incoming, Some(ChainConnector::Forward));
            }
            other => panic!("expected chain expression, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_collects_mixed_and_bound_chain_steps() {
        let parsed = parse_module(
            "fn main(seed: Int) -> Int:\n    let score = forward :=> stage.seed with (seed) => stage.inc <= stage.dec <= stage.emit\n    return score\n",
        )
        .expect("parse should pass");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Let {
                value:
                    Expr::Chain {
                        style,
                        introducer,
                        steps,
                    },
                ..
            } => {
                assert_eq!(style, "forward");
                assert_eq!(*introducer, ChainIntroducer::Forward);
                assert_eq!(
                    steps.iter().map(|step| step.incoming).collect::<Vec<_>>(),
                    vec![
                        None,
                        Some(ChainConnector::Forward),
                        Some(ChainConnector::Reverse),
                        Some(ChainConnector::Reverse)
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
                assert!(matches!(&steps[0].stage, Expr::MemberAccess { .. }));
                assert_eq!(steps[0].bind_args.len(), 1);
                assert!(expr_is_path(&steps[0].bind_args[0], "seed"));
                assert!(steps[1].bind_args.is_empty());
            }
            other => panic!("expected bound mixed chain expression, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_rejects_unsupported_top_level_syntax() {
        let err = parse_module("widget Gizmo:\n    value: Int\n")
            .expect_err("unsupported top-level syntax should fail");
        assert!(err.contains("unsupported top-level syntax"), "{err}");
    }

    #[test]
    fn parse_module_rejects_bad_memory_family_trailing_commas_and_phrase_over_arity() {
        for (source, expected) in [
            (
                "fn main() -> Int:\n    return io.print :: 1, :: call\n",
                "trailing comma is not allowed before phrase qualifier",
            ),
            (
                "fn main() -> Int:\n    return io.print :: 1, 2, 3, 4 :: call\n",
                "qualified phrase allows at most 3 top-level arguments",
            ),
            (
                "fn main() -> Int:\n    let node = arena: store :> a = 1, <: Node\n    return 0\n",
                "trailing comma is not allowed before memory phrase qualifier",
            ),
            (
                "fn main() -> Int:\n    let node = arena: store :> a = 1, b = 2, c = 3, d = 4 <: Node\n    return 0\n",
                "memory phrase allows at most 3 top-level arguments",
            ),
            (
                "fn main() -> Int:\n    let item = weird: store :> value = 1 <: Item\n    return 0\n",
                "unknown memory type `weird`; supported now: arena, frame, pool, temp, session, ring, slab",
            ),
            (
                "fn main() -> Int:\n    let item = arena: store :> value = 1 <: Node()\n    return 0\n",
                "invalid memory phrase constructor `Node()`; expected path or path[type_args]",
            ),
            (
                "fn main() -> Int:\n    let item = io.print :: 1 :: call\n        value = 2\n    return 0\n",
                "attached blocks are only valid on standalone qualified/memory phrase statements",
            ),
            (
                "fn main() -> Int:\n    io.print :: 1 :: arcana_process.io.print\n        value = 2\n    return 0\n",
                "qualifier `path qualifier` does not support named header entries",
            ),
            (
                "fn main() -> Int:\n    value :: :: ?\n        fallback = 1\n    return 0\n",
                "qualifier `?` does not support named header entries",
            ),
        ] {
            let err = parse_module(source).expect_err("source should fail");
            assert!(err.contains(expected), "{source}: {err}");
        }
    }

    #[test]
    fn parse_module_rejects_invalid_chain_styles() {
        for (source, expected) in [
            (
                "fn main() -> Int:\n    mystery :=> seed => step\n    return 0\n",
                "unknown chain style `mystery`; supported: forward, lazy, parallel, async, plan, broadcast, collect",
            ),
            (
                "fn main() -> Int:\n    parallel :=< emit <= seed\n    return 0\n",
                "chain style `parallel` does not support reverse-introduced chains",
            ),
        ] {
            let err = parse_module(source).expect_err("source should fail");
            assert!(err.contains(expected), "{source}: {err}");
        }
    }

    #[test]
    fn parse_module_validates_boundary_and_test_forewords() {
        let parsed = parse_module(
            "#boundary[target = \"lua\"]\nfn bridge(read text: Str) -> Str:\n    return text\nfn main() -> Int:\n    return 0\n",
        )
        .expect("boundary foreword should be accepted");
        assert_eq!(parsed.symbols[0].forewords[0].name, "boundary");

        for (source, expected) in [
            (
                "#boundary[target = lua.sql]\nfn bad() -> Int:\n    return 0\n",
                "invalid payload for foreword `#boundary`: `target` must be a string or symbol",
            ),
            (
                "#stage[bad_key=true]\nfn seed() -> Int:\n    return 1\n",
                "invalid #stage contract key 'bad_key'",
            ),
            (
                "#test[smoke]\nfn bad() -> Int:\n    return 0\n",
                "invalid payload for foreword `#test`: expected no payload",
            ),
            (
                "#test\nexport fn smoke() -> Int:\n    return 0\n",
                "`#test` functions must not be exported in v1",
            ),
        ] {
            let err = parse_module(source).expect_err("foreword contract should fail");
            assert!(err.contains(expected), "{source}: {err}");
        }
    }

    #[test]
    fn parse_module_filters_only_forewords_for_current_target() {
        let parsed = parse_module(
            "#only[os = \"definitely_not_host\"]\nfn skipped() -> Missing:\n    return 0\nfn main() -> Int:\n    return 0\n",
        )
        .expect("non-matching #only target should filter declaration");
        assert_eq!(
            parsed
                .symbols
                .iter()
                .map(|symbol| symbol.name.as_str())
                .collect::<Vec<_>>(),
            vec!["main"]
        );
    }

    #[test]
    fn parse_module_rejects_malformed_intrinsic_declaration() {
        let err = parse_module("intrinsic fn host_len(read text: Str) -> Int\n")
            .expect_err("malformed intrinsic should fail");
        assert!(
            err.contains("malformed intrinsic function declaration"),
            "{err}"
        );
    }

    #[test]
    fn parse_module_handles_native_binding_declarations() {
        let parsed = parse_module(concat!(
            "export opaque type HiddenWindow as move, boundary_unsafe\n",
            "export native fn create_hidden_window() -> HiddenWindow = host.create_hidden_window\n",
            "native callback window_proc(read window: HiddenWindow, message: Int, wparam: Int, lparam: Int) -> Int = app.callbacks.handle_window_proc\n",
            "fn handle_window_proc(read window: HiddenWindow, message: Int, wparam: Int, lparam: Int) -> Int:\n",
            "    return wparam\n",
        ))
        .expect("native binding declarations should parse");

        assert_eq!(parsed.symbols.len(), 3);
        assert_eq!(parsed.symbols[0].kind, SymbolKind::OpaqueType);
        assert_eq!(parsed.symbols[0].name, "HiddenWindow");
        assert_eq!(
            parsed.symbols[0].opaque_policy.expect("opaque policy"),
            OpaqueTypePolicy {
                ownership: OpaqueOwnershipPolicy::Move,
                boundary: OpaqueBoundaryPolicy::Unsafe,
            }
        );
        assert_eq!(parsed.symbols[1].kind, SymbolKind::Fn);
        assert_eq!(parsed.symbols[1].name, "create_hidden_window");
        assert_eq!(
            parsed.symbols[1].native_impl.as_deref(),
            Some("host.create_hidden_window")
        );
        assert_eq!(parsed.native_callbacks.len(), 1);
        assert_eq!(parsed.native_callbacks[0].name, "window_proc");
        assert_eq!(
            parsed.native_callbacks[0].target,
            vec![
                "app".to_string(),
                "callbacks".to_string(),
                "handle_window_proc".to_string()
            ]
        );
    }

    #[test]
    fn parse_module_handles_typed_native_callbacks_and_shackle_declarations() {
        let parsed = parse_module(concat!(
            "export shackle callback WNDPROC(read hwnd: Int, message: Int) -> Int\n",
            "export shackle import fn CreateWindowExW() -> Int = user32.CreateWindowExW\n",
            "shackle fn helper(read code: Int) -> Int:\n",
            "    return code\n",
            "native callback proc: arcana_winapi.raw.user32.WNDPROC = app.callbacks.handle_proc\n",
            "fn handle_proc(read code: Int) -> Int:\n",
            "    return code\n",
        ))
        .expect("typed native callbacks and shackle declarations should parse");

        assert_eq!(parsed.shackle_decls.len(), 3);
        assert_eq!(parsed.shackle_decls[0].kind, ShackleDeclKind::Callback);
        assert_eq!(parsed.shackle_decls[0].name, "WNDPROC");
        assert!(matches!(
            parsed.shackle_decls[0].raw_decl,
            Some(ShackleRawDecl::Callback { .. })
        ));
        assert_eq!(parsed.shackle_decls[1].kind, ShackleDeclKind::ImportFn);
        assert_eq!(
            parsed.shackle_decls[1].binding.as_deref(),
            Some("user32.CreateWindowExW")
        );
        let import_target = parsed.shackle_decls[1]
            .import_target
            .as_ref()
            .expect("typed import target should parse at syntax layer");
        assert_eq!(import_target.library, "user32");
        assert_eq!(import_target.symbol, "CreateWindowExW");
        assert_eq!(parsed.shackle_decls[2].kind, ShackleDeclKind::Fn);
        assert_eq!(parsed.shackle_decls[2].body_entries, vec!["return code"]);
        assert_eq!(parsed.native_callbacks.len(), 1);
        assert_eq!(parsed.native_callbacks[0].name, "proc");
        assert!(parsed.native_callbacks[0].params.is_empty());
        assert!(parsed.native_callbacks[0].return_type.is_none());
        assert_eq!(
            parsed.native_callbacks[0]
                .callback_type
                .as_ref()
                .expect("typed callback ref")
                .render(),
            "arcana_winapi.raw.user32.WNDPROC"
        );
    }

    #[test]
    fn parse_module_lowers_typed_shackle_raw_decl_metadata() {
        let parsed = parse_module(concat!(
            "export shackle struct RECT:\n",
            "    left: I32\n",
            "    flags: U32 bits 3\n",
            "export shackle type MODE = U32:\n",
            "    None = 0\n",
            "    Windowed = 1\n",
            "export shackle type HANDLE_ARRAY = [U16; 4]\n",
            "export shackle type IUnknown = *mut c_void\n",
        ))
        .expect("typed shackle raw decls should parse");

        assert!(matches!(
            parsed.shackle_decls[0].raw_decl,
            Some(ShackleRawDecl::Struct { .. })
        ));
        assert!(matches!(
            parsed.shackle_decls[1].raw_decl,
            Some(ShackleRawDecl::Enum { .. })
        ));
        assert!(matches!(
            parsed.shackle_decls[2].raw_decl,
            Some(ShackleRawDecl::Array { .. })
        ));
        assert!(matches!(
            parsed.shackle_decls[3].raw_decl,
            Some(ShackleRawDecl::Alias { .. })
        ));
    }

    #[test]
    fn parse_module_rejects_tuple_field_access_outside_pair_contract() {
        let err = parse_module("fn main() -> Int:\n    return pair.2\n")
            .expect_err("tuple field access should fail");
        assert!(
            err.contains("tuple field access only supports `.0` and `.1` in v1"),
            "{err}"
        );
    }

    #[test]
    fn parse_module_accepts_decimal_float_literals_in_binary_expressions() {
        let parsed =
            parse_module("fn main() -> F64:\n    let value = 1.5 + 2.5\n    return value\n")
                .expect("decimal float literals should parse without tuple-member access");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Let { value, .. } => match value {
                Expr::Binary { left, right, .. } => {
                    assert!(matches!(left.as_ref(), Expr::FloatLiteral { .. }));
                    assert!(matches!(right.as_ref(), Expr::FloatLiteral { .. }));
                }
                other => panic!("expected binary float expression, got {other:?}"),
            },
            other => panic!("expected float let binding, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_accepts_member_access_plus_decimal_float_literal() {
        parse_module(
            "struct Vec2:\n    x: F32\n    y: F32\nfn main() -> F64:\n    let point = Vec2 :: x = 1.5f32, y = 2.5f32 :: call\n    let sum = (F64 :: point.x :: call) + 2.5\n    return sum\n",
        )
        .expect("member access and decimal float literal should parse together");
    }

    #[test]
    fn parse_module_accepts_nested_tuple_member_access_after_numeric_selector() {
        let parsed =
            parse_module("fn main() -> Int:\n    let value = pair.1.0\n    return value\n")
                .expect("nested tuple member access should parse");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Let { value, .. } => match value {
                Expr::MemberAccess { expr, member } if member == "0" => {
                    assert!(matches!(
                        expr.as_ref(),
                        Expr::MemberAccess { member, .. } if member == "1"
                    ));
                }
                other => panic!("expected nested tuple member access, got {other:?}"),
            },
            other => panic!("expected let statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_accepts_nested_qualified_phrase_inside_named_arg() {
        parse_module(
            "struct Flags:\n    mask: U32 bits 3\nfn main() -> Int:\n    let flags = Flags :: mask = U32 :: 3 :: call :: call\n    return Int :: flags.mask :: call\n",
        )
        .expect("nested qualified phrases inside named args should parse");
    }

    #[test]
    fn parse_module_accepts_exact_pair_destructuring_in_let_and_for_statements() {
        let parsed = parse_module(
            "fn main() -> Int:\n    let (left, right) = pair\n    for (first, second) in pairs:\n        return first\n    return 0\n",
        )
        .expect("tuple destructuring should parse");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Let { name, .. } => assert_eq!(name, "(left, right)"),
            other => panic!("expected tuple let statement, got {other:?}"),
        }
        match &parsed.symbols[0].statements[1].kind {
            StatementKind::For { binding, .. } => assert_eq!(binding, "(first, second)"),
            other => panic!("expected tuple for statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_rejects_tuple_field_assignment() {
        let err = parse_module("fn main() -> Int:\n    pair.0 = 1\n    return 0\n")
            .expect_err("tuple field assignment should fail");
        assert!(
            err.contains("tuple field assignment is not allowed in v1"),
            "{err}"
        );
    }

    #[test]
    fn parse_module_rejects_three_element_tuple_contracts() {
        let err = parse_module("fn main() -> (Int, Int, Int):\n    return (1, 2, 3)\n")
            .expect_err("triple tuples should fail");
        assert!(
            err.contains("tuple types must have exactly 2 elements in v1"),
            "{err}"
        );
    }

    #[test]
    fn parse_module_collects_structured_phrases_and_operators() {
        let parsed = parse_module(
            "fn main() -> Int:\n    defer io.print[Str] :: \"bye\" :: call\n    let task = weave worker :: 41 :: call\n    let ready = task >> await\n    let ok = not false and ((1 + 2) << 3) >= 8\n    let cfg = winspell.loop.FrameConfig :: clear = 0 :: call\n    let printed = io.print[Int] :: ready, ok :: call\n    return printed\n",
        )
        .expect("parse should pass");

        let statements = &parsed.symbols[0].statements;
        assert_eq!(statements.len(), 7);

        match &statements[0].kind {
            StatementKind::Defer {
                action: DeferAction::Expr { expr },
            } => match expr {
                Expr::QualifiedPhrase {
                    subject,
                    args,
                    qualifier,
                    attached,
                    ..
                } => {
                    assert_eq!(qualifier, "call");
                    assert!(attached.is_empty());
                    assert!(matches!(
                        subject.as_ref(),
                        Expr::GenericApply { expr, type_args }
                            if type_args.iter().map(SurfaceType::render).collect::<Vec<_>>()
                                == vec!["Str".to_string()]
                                && matches!(
                                    expr.as_ref(),
                                    Expr::MemberAccess { member, .. } if member == "print"
                                )
                    ));
                    assert_eq!(args.len(), 1);
                    assert!(matches!(
                        &args[0],
                        PhraseArg::Positional(expr) if expr_is_str_literal(expr, "\"bye\"")
                    ));
                }
                other => panic!("expected defer phrase expression, got {other:?}"),
            },
            other => panic!("expected defer statement, got {other:?}"),
        }

        match &statements[1].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "task");
                match value {
                    Expr::Unary { op, expr } => {
                        assert_eq!(*op, UnaryOp::Weave);
                        assert!(matches!(
                            expr.as_ref(),
                            Expr::QualifiedPhrase { qualifier, .. } if qualifier == "call"
                        ));
                    }
                    other => panic!("expected weave unary expression, got {other:?}"),
                }
            }
            other => panic!("expected let task statement, got {other:?}"),
        }

        match &statements[2].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "ready");
                match value {
                    Expr::Await { expr } => {
                        assert!(matches!(expr.as_ref(), expr if expr_is_path(expr, "task")));
                    }
                    other => panic!("expected await expression, got {other:?}"),
                }
            }
            other => panic!("expected let ready statement, got {other:?}"),
        }

        match &statements[3].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "ok");
                match value {
                    Expr::Binary { left, op, right } => {
                        assert_eq!(*op, BinaryOp::And);
                        assert!(matches!(
                            left.as_ref(),
                            Expr::Unary {
                                op: UnaryOp::Not,
                                ..
                            }
                        ));
                        match right.as_ref() {
                            Expr::Binary { left, op, right } => {
                                assert_eq!(*op, BinaryOp::GtEq);
                                match left.as_ref() {
                                    Expr::Binary { left, op, right } => {
                                        assert_eq!(*op, BinaryOp::Shl);
                                        match left.as_ref() {
                                            Expr::Binary { op, .. } => {
                                                assert_eq!(*op, BinaryOp::Add);
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
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "cfg");
                match value {
                    Expr::QualifiedPhrase {
                        subject,
                        args,
                        qualifier,
                        attached,
                        ..
                    } => {
                        assert_eq!(qualifier, "call");
                        assert!(attached.is_empty());
                        assert!(matches!(
                            subject.as_ref(),
                            Expr::MemberAccess { member, .. } if member == "FrameConfig"
                        ));
                        assert_eq!(args.len(), 1);
                        assert!(matches!(
                            &args[0],
                            PhraseArg::Named { name, value }
                                if name == "clear" && expr_is_int_literal(value, "0")
                        ));
                    }
                    other => panic!("expected named-arg phrase, got {other:?}"),
                }
            }
            other => panic!("expected let cfg statement, got {other:?}"),
        }

        match &statements[5].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "printed");
                match value {
                    Expr::QualifiedPhrase {
                        subject,
                        args,
                        qualifier,
                        attached,
                        ..
                    } => {
                        assert_eq!(qualifier, "call");
                        assert!(attached.is_empty());
                        assert!(matches!(
                                subject.as_ref(),
                                Expr::GenericApply { expr, type_args }
                                if type_args.iter().map(SurfaceType::render).collect::<Vec<_>>()
                                    == vec!["Int".to_string()]
                                    && matches!(
                                        expr.as_ref(),
                                        Expr::MemberAccess { member, .. } if member == "print"
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
            StatementKind::Return { value } => {
                assert!(matches!(
                    value.as_ref().expect("return should have value"),
                    expr if expr_is_path(expr, "printed")
                ));
            }
            other => panic!("expected return statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_keeps_parenthesized_phrase_args_structured() {
        let parsed = parse_module(
            "fn main(text: Str, i: Int) -> Int:\n    let b = (std.text.byte_at :: text, i :: call)\n    return 0\n",
        )
        .expect("parse should pass");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Let { value, .. } => {
                assert!(
                    matches!(value, Expr::QualifiedPhrase { qualifier, .. } if qualifier == "call")
                );
            }
            other => panic!("expected let statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_keeps_parenthesized_pairs_with_phrase_pair_args() {
        let parsed = parse_module(
            "fn main(a: Int, b: Int) -> (Bool, Int):\n    return (true, widget.emit :: (a, b) :: call)\n",
        )
        .expect("parse should pass");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Return { value } => match value.as_ref().expect("return value expected")
            {
                Expr::Pair { left, right } => {
                    assert!(matches!(left.as_ref(), Expr::BoolLiteral { value: true }));
                    match right.as_ref() {
                        Expr::QualifiedPhrase {
                            args, qualifier, ..
                        } => {
                            assert_eq!(qualifier, "call");
                            assert_eq!(args.len(), 1);
                            assert!(matches!(&args[0], PhraseArg::Positional(Expr::Pair { .. })));
                        }
                        other => panic!("expected qualified phrase rhs, got {other:?}"),
                    }
                }
                other => panic!("expected pair return, got {other:?}"),
            },
            other => panic!("expected return statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_keeps_pair_rhs_phrase_calls_from_std_ecs_shape() {
        let parsed = parse_module(
            "fn main() -> (Bool, Int):\n    return (true, get_component[Int] :: :: call)\n",
        )
        .expect("parse should pass");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Return { value } => match value.as_ref().expect("return value expected")
            {
                Expr::Pair { left, right } => {
                    assert!(matches!(left.as_ref(), Expr::BoolLiteral { value: true }));
                    assert!(matches!(
                        right.as_ref(),
                        Expr::QualifiedPhrase { qualifier, .. } if qualifier == "call"
                    ));
                }
                other => panic!("expected pair return, got {other:?}"),
            },
            other => panic!("expected return statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_keeps_grouped_phrase_call_as_phrase_arg() {
        let parsed = parse_module(
            "fn main(edit out: List[Int], a: Int) -> Int:\n    out :: (Widget.emit :: a != 0 :: call) :: push\n    return 0\n",
        )
        .expect("parse should pass");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Expr {
                expr:
                    Expr::QualifiedPhrase {
                        qualifier, args, ..
                    },
            } => {
                assert_eq!(qualifier, "push");
                assert_eq!(args.len(), 1);
                assert!(matches!(
                    &args[0],
                    PhraseArg::Positional(Expr::QualifiedPhrase { qualifier, .. }) if qualifier == "call"
                ));
            }
            other => panic!("expected push phrase, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_keeps_grouped_phrase_with_comma_args() {
        let parsed = parse_module(
            "fn main(text: Str, i: Int) -> Bool:\n    return (std.text.byte_at :: text, i :: call) == 0\n",
        )
        .expect("parse should pass");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Return { value } => match value.as_ref().expect("return value expected")
            {
                Expr::Binary { left, op, right } => {
                    assert_eq!(*op, BinaryOp::EqEq);
                    match left.as_ref() {
                        Expr::QualifiedPhrase { qualifier, .. } if qualifier == "call" => {}
                        other => panic!("expected qualified phrase lhs, got {other:?}"),
                    }
                    assert!(matches!(right.as_ref(), Expr::IntLiteral { text } if text == "0"));
                }
                other => panic!("expected equality return, got {other:?}"),
            },
            other => panic!("expected return statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_expression_core_keeps_grouped_phrase_subjects() {
        let inner = super::parse_expression_core("(Widget.emit :: a != 0 :: call)")
            .expect("grouped phrase should parse");
        match inner {
            Expr::QualifiedPhrase { ref qualifier, .. } if qualifier == "call" => {}
            other => panic!("expected grouped qualified phrase, got {other:?}"),
        }

        let outer = super::parse_qualified_phrase("out :: (Widget.emit :: a != 0 :: call) :: push")
            .expect("qualified phrase parse should not error");
        assert!(
            matches!(outer, Some(Expr::QualifiedPhrase { qualifier, .. }) if qualifier == "push")
        );
    }

    #[test]
    fn parse_expression_core_keeps_grouped_phrase_with_comma_args() {
        let inner = super::parse_qualified_phrase("std.text.byte_at :: text, i :: call")
            .expect("qualified phrase parse should not error");
        assert!(
            matches!(inner, Some(Expr::QualifiedPhrase { qualifier, .. }) if qualifier == "call")
        );

        let grouped = super::parse_expression_core("(std.text.byte_at :: text, i :: call)")
            .expect("grouped phrase should parse");
        assert!(matches!(grouped, Expr::QualifiedPhrase { qualifier, .. } if qualifier == "call"));
    }

    #[test]
    fn parse_module_preserves_parenthesized_phrase_args_with_nested_commas() {
        let parsed = parse_module(
            "fn has_magic(read bytes: Array[Int], left: (Int, Int), right: (Int, Int)) -> Bool:\n    return true\nfn main(read bytes: Array[Int]) -> Int:\n    if not (has_magic :: bytes, (65, 82), (67, 66) :: call):\n        return 1\n    return 0\n",
        )
        .expect("parenthesized phrase args should parse");

        match &parsed.symbols[1].statements[0].kind {
            StatementKind::If { condition, .. } => {
                assert!(matches!(
                    condition,
                    Expr::Unary { op: UnaryOp::Not, expr }
                        if !matches!(expr.as_ref(), Expr::Pair { .. })
                ));
            }
            other => panic!("expected if statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_collects_access_and_range_expressions() {
        let parsed = parse_module(
            "fn main() -> Int:\n    let tuple_head = pair.0\n    let color = spec.color\n    let xs = [1, 2, 3, 4]\n    let first = xs[0]\n    let tail = xs[1..]\n    let mid = xs[1..=2]\n    let whole = xs[..]\n    let r1 = 0..3\n    let r2 = ..=3\n    return first\n",
        )
        .expect("parse should pass");

        let statements = &parsed.symbols[0].statements;
        match &statements[0].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "tuple_head");
                assert!(matches!(
                    value,
                    Expr::MemberAccess { member, .. } if member == "0"
                ));
            }
            other => panic!("expected tuple_head let, got {other:?}"),
        }
        match &statements[1].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "color");
                assert!(matches!(
                    value,
                    Expr::MemberAccess { member, .. } if member == "color"
                ));
            }
            other => panic!("expected color let, got {other:?}"),
        }
        match &statements[3].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "first");
                match value {
                    Expr::Index { expr, index } => {
                        assert!(matches!(expr.as_ref(), expr if expr_is_path(expr, "xs")));
                        assert!(matches!(index.as_ref(), expr if expr_is_int_literal(expr, "0")));
                    }
                    other => panic!("expected index expression, got {other:?}"),
                }
            }
            other => panic!("expected first let, got {other:?}"),
        }
        match &statements[4].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "tail");
                match value {
                    Expr::Slice {
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
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "mid");
                match value {
                    Expr::Slice {
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
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "whole");
                assert!(matches!(
                    value,
                    Expr::Slice {
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
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "r1");
                assert!(matches!(
                    value,
                    Expr::Range {
                        start: Some(_),
                        end: Some(_),
                        inclusive_end: false
                    }
                ));
            }
            other => panic!("expected r1 let, got {other:?}"),
        }
        match &statements[8].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "r2");
                assert!(matches!(
                    value,
                    Expr::Range {
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
    fn parse_module_collects_pair_tuple_expressions() {
        let parsed =
            parse_module("fn main() -> Int:\n    let pair = (left, right)\n    return pair.0\n")
                .expect("parse should pass");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "pair");
                assert!(matches!(
                    value,
                    Expr::Pair {
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
    fn parse_module_collects_pair_tuple_expressions_with_binary_left_side() {
        let parsed = parse_module(
            "fn main() -> Int:\n    let pair = (left + right, tail)\n    return pair.1\n",
        )
        .expect("parse should pass");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "pair");
                assert!(matches!(
                    value,
                    Expr::Pair { left, right }
                        if matches!(left.as_ref(), Expr::Binary { .. })
                            && matches!(right.as_ref(), expr if expr_is_path(expr, "tail"))
                ));
            }
            other => panic!("expected pair let, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_distinguishes_generic_apply_from_indexing() {
        let parsed = parse_module(
            "fn main() -> Int:\n    let out = std.collections.list.new[(K, V)] :: :: call\n    return 0\n",
        )
        .expect("parse should pass");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Let {
                value:
                    Expr::QualifiedPhrase {
                        subject, qualifier, ..
                    },
                ..
            } => {
                assert_eq!(qualifier, "call");
                assert!(matches!(
                    subject.as_ref(),
                    Expr::GenericApply { type_args, .. }
                        if type_args.iter().map(SurfaceType::render).collect::<Vec<_>>()
                            == vec!["(K, V)".to_string()]
                ));
            }
            other => panic!("expected generic qualified phrase, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_collects_extended_phrase_qualifier_kinds() {
        let parsed = parse_module(concat!(
            "fn main() -> Int:\n",
            "    let called = target :: :: call[Int]\n",
            "    let awaited = task :: :: await\n",
            "    let woven = worker :: 1 :: weave\n",
            "    let split_out = helper :: 2 :: split\n",
            "    let required = maybe :: :: must\n",
            "    let fallback_value = maybe :: 7 :: fallback\n",
            "    return 0\n",
        ))
        .expect("extended qualifier forms should parse");

        let statements = &parsed.symbols[0].statements;
        let expected = [
            (
                QualifiedPhraseQualifierKind::Call,
                "call",
                vec!["Int".to_string()],
            ),
            (QualifiedPhraseQualifierKind::Await, "await", Vec::new()),
            (QualifiedPhraseQualifierKind::Weave, "weave", Vec::new()),
            (QualifiedPhraseQualifierKind::Split, "split", Vec::new()),
            (QualifiedPhraseQualifierKind::Must, "must", Vec::new()),
            (
                QualifiedPhraseQualifierKind::Fallback,
                "fallback",
                Vec::new(),
            ),
        ];
        for (statement, (expected_kind, expected_qualifier, expected_type_args)) in
            statements.iter().take(6).zip(expected.iter())
        {
            let StatementKind::Let {
                value:
                    Expr::QualifiedPhrase {
                        qualifier_kind,
                        qualifier,
                        qualifier_type_args,
                        ..
                    },
                ..
            } = &statement.kind
            else {
                panic!(
                    "expected qualified phrase let statement, got {:?}",
                    statement.kind
                );
            };
            assert_eq!(qualifier_kind, expected_kind);
            assert_eq!(qualifier, expected_qualifier);
            assert_eq!(
                qualifier_type_args
                    .iter()
                    .map(SurfaceType::render)
                    .collect::<Vec<_>>(),
                *expected_type_args
            );
        }
    }

    #[test]
    fn parse_module_collects_owner_context_clause() {
        let parsed = parse_module(concat!(
            "obj SessionCtx:\n",
            "    base: Int\n",
            "\n",
            "obj Counter:\n",
            "    value: Int\n",
            "\n",
            "create Session [Counter] context: SessionCtx scope-exit:\n",
            "    done: when false retain [Counter]\n",
            "\n",
            "fn main() -> Int:\n",
            "    return 0\n",
        ))
        .expect("owner context clause should parse");

        let owner = parsed
            .symbols
            .iter()
            .find(|symbol| symbol.name == "Session")
            .expect("owner symbol should exist");
        let SymbolBody::Owner { context_type, .. } = &owner.body else {
            panic!("expected owner symbol body");
        };
        assert_eq!(
            context_type.as_ref().map(SurfaceType::render).as_deref(),
            Some("SessionCtx")
        );
    }

    #[test]
    fn parse_module_preserves_phrase_calls_inside_pair_expressions() {
        let parsed = parse_module(
            "fn main() -> Bool:\n    return (std.collections.list.new[Str] :: :: call, \"\")\n",
        )
        .expect("pair expression should parse");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Return { value } => match value
                .as_ref()
                .expect("return should carry a value")
            {
                Expr::Pair { left, right } => {
                    assert!(matches!(
                        left.as_ref(),
                        Expr::QualifiedPhrase { qualifier, .. } if qualifier == "call"
                    ));
                    assert!(matches!(right.as_ref(), expr if expr_is_str_literal(expr, "\"\"")));
                }
                other => panic!("expected pair expression, got {other:?}"),
            },
            other => panic!("expected return statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_collects_structured_header_attachments() {
        let parsed = parse_module(
            "fn main() -> Int:\n    Node :: :: call\n        value = 10\n        #chain[phase=update]\n        forward :=> show_node => bump_node => show_node\n    arena: arena_nodes :> 21 <: make_node\n        #chain[phase=update]\n        plan :=> touch_id\n        forward :=> touch_id\n    return 0\n",
        )
        .expect("parse should pass");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Expr {
                expr:
                    Expr::QualifiedPhrase {
                        qualifier,
                        attached,
                        ..
                    },
            } => {
                assert_eq!(qualifier, "call");
                assert_eq!(attached.len(), 2);
                assert!(matches!(
                    &attached[0],
                    HeaderAttachment::Named {
                        name,
                        value,
                        forewords,
                        ..
                    } if name == "value" && expr_is_int_literal(value, "10")
                        && forewords.is_empty()
                ));
                assert!(matches!(
                    &attached[1],
                    HeaderAttachment::Chain {
                        expr:
                            Expr::Chain {
                                style,
                                introducer,
                                steps,
                            },
                        forewords,
                        ..
                    } if style == "forward"
                        && *introducer == ChainIntroducer::Forward
                        && chain_step_texts(steps)
                            == vec!["show_node", "bump_node", "show_node"]
                        && matches!(
                            forewords.as_slice(),
                            [ForewordApp { name, args, .. }]
                                if name == "chain"
                                    && matches!(
                                        args.as_slice(),
                                        [ForewordArg {
                                            name: Some(arg_name),
                                            value,
                                            ..
                                        }]
                                            if arg_name == "phase" && value == "update"
                                    )
                        )
                ));
            }
            other => panic!("expected qualified phrase statement, got {other:?}"),
        }

        match &parsed.symbols[0].statements[1].kind {
            StatementKind::Expr {
                expr:
                    Expr::MemoryPhrase {
                        family,
                        constructor,
                        attached,
                        ..
                    },
            } => {
                assert_eq!(family, "arena");
                match constructor.as_ref() {
                    Expr::Path { segments } => {
                        assert_eq!(segments, &vec!["make_node".to_string()])
                    }
                    other => panic!("expected constructor path, got {other:?}"),
                }
                assert_eq!(attached.len(), 2);
                assert!(matches!(
                    &attached[0],
                    HeaderAttachment::Chain {
                        expr:
                            Expr::Chain {
                                style,
                                introducer,
                                steps,
                            },
                        forewords,
                        ..
                    } if style == "plan"
                        && *introducer == ChainIntroducer::Forward
                        && chain_step_texts(steps) == vec!["touch_id"]
                        && matches!(
                            forewords.as_slice(),
                            [ForewordApp { name, args, .. }]
                                if name == "chain"
                                    && matches!(
                                        args.as_slice(),
                                        [ForewordArg {
                                            name: Some(arg_name),
                                            value,
                                            ..
                                        }]
                                            if arg_name == "phase" && value == "update"
                                    )
                        )
                ));
                assert!(matches!(
                    &attached[1],
                    HeaderAttachment::Chain {
                        expr:
                            Expr::Chain {
                                style,
                                introducer,
                                steps,
                            },
                        forewords,
                        ..
                    } if style == "forward"
                        && *introducer == ChainIntroducer::Forward
                        && chain_step_texts(steps) == vec!["touch_id"]
                        && forewords.is_empty()
                ));
            }
            other => panic!("expected memory phrase statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_collects_assignment_targets() {
        let parsed = parse_module(
            "fn main() -> Int:\n    self.tick.value = self.tick.value + 1\n    xs[1] = 9\n    value += 3\n    *held = 7\n    (*held).value = 4\n    return 0\n",
        )
        .expect("parse should pass");

        let statements = &parsed.symbols[0].statements;
        match &statements[0].kind {
            StatementKind::Assign { target, .. } => match target {
                AssignTarget::MemberAccess { target, member } => {
                    assert_eq!(member, "value");
                    assert!(matches!(
                        target.as_ref(),
                        AssignTarget::MemberAccess { member, .. } if member == "tick"
                    ));
                }
                other => panic!("expected member assignment target, got {other:?}"),
            },
            other => panic!("expected first assignment statement, got {other:?}"),
        }
        match &statements[1].kind {
            StatementKind::Assign { target, .. } => match target {
                AssignTarget::Index { target, index } => {
                    assert!(matches!(
                        target.as_ref(),
                        AssignTarget::Name { text } if text == "xs"
                    ));
                    assert!(expr_is_int_literal(index, "1"));
                }
                other => panic!("expected indexed assignment target, got {other:?}"),
            },
            other => panic!("expected second assignment statement, got {other:?}"),
        }
        match &statements[2].kind {
            StatementKind::Assign { target, .. } => match target {
                AssignTarget::Name { text } => {
                    assert_eq!(text, "value");
                }
                other => panic!("expected named compound-assignment target, got {other:?}"),
            },
            other => panic!("expected third assignment statement, got {other:?}"),
        }
        match &statements[3].kind {
            StatementKind::Assign { target, .. } => match target {
                AssignTarget::Deref { expr } => {
                    assert!(expr_is_path(expr, "held"));
                }
                other => panic!("expected deref assignment target, got {other:?}"),
            },
            other => panic!("expected fourth assignment statement, got {other:?}"),
        }
        match &statements[4].kind {
            StatementKind::Assign { target, .. } => match target {
                AssignTarget::MemberAccess { target, member } => {
                    assert_eq!(member, "value");
                    assert!(matches!(
                        target.as_ref(),
                        AssignTarget::Deref { expr } if expr_is_path(expr, "held")
                    ));
                }
                other => panic!("expected deref member assignment target, got {other:?}"),
            },
            other => panic!("expected fifth assignment statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_collects_capability_and_deref_expressions() {
        let parsed = parse_module(
            "fn main() -> Int:\n    let local_x = 1\n    let mut local_y = 2\n    let x_ref = &read local_x\n    let y_cap = &edit local_y\n    let sum = *x_ref + *y_cap\n    return sum\n",
        )
        .expect("parse should pass");

        let statements = &parsed.symbols[0].statements;
        match &statements[2].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "x_ref");
                assert!(matches!(
                    value,
                    Expr::Unary {
                        op: UnaryOp::CapabilityRead,
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
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "y_cap");
                assert!(matches!(
                    value,
                    Expr::Unary {
                        op: UnaryOp::CapabilityEdit,
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
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "sum");
                assert!(matches!(
                    value,
                    Expr::Binary {
                        left,
                        op: BinaryOp::Add,
                        right
                    } if matches!(
                        left.as_ref(),
                        Expr::Unary {
                            op: UnaryOp::Deref,
                            expr
                        } if matches!(
                            expr.as_ref(),
                            expr if expr_is_path(expr, "x_ref")
                        )
                    ) && matches!(
                        right.as_ref(),
                        Expr::Unary {
                            op: UnaryOp::Deref,
                            expr
                        } if matches!(
                            expr.as_ref(),
                            expr if expr_is_path(expr, "y_cap")
                        )
                    )
                ));
            }
            other => panic!("expected sum let, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_collects_reclaim_statement() {
        let parsed = parse_module(
            "fn main() -> Int:\n    let mut local = 1\n    let held = &hold local\n    reclaim held\n    return 0\n",
        )
        .expect("reclaim should parse");

        assert!(matches!(
            parsed.symbols[0].statements[2].kind,
            StatementKind::Reclaim { .. }
        ));
    }

    #[test]
    fn parse_module_collects_deferred_reclaim_statement() {
        let parsed = parse_module(
            "fn main() -> Int:\n    let mut local = 1\n    let held = &hold local\n    defer reclaim held\n    return 0\n",
        )
        .expect("defer reclaim should parse");

        assert!(matches!(
            parsed.symbols[0].statements[2].kind,
            StatementKind::Defer {
                action: DeferAction::Reclaim { .. }
            }
        ));
    }

    #[test]
    fn parse_module_rejects_match_without_arms() {
        let err = parse_module("fn main() -> Int:\n    return match value:\n")
            .expect_err("match should fail");
        assert!(err.contains("malformed `match` expression"), "{err}");
    }

    #[test]
    fn builtin_type_info_names_are_unique() {
        let mut seen = BTreeSet::new();
        for info in BUILTIN_TYPE_INFOS {
            assert!(
                seen.insert(info.name),
                "duplicate builtin type entry `{}`",
                info.name
            );
        }
    }

    #[test]
    fn parse_module_handles_opaque_type_declarations() {
        let parsed = parse_module(
            "export opaque type Window as move, boundary_unsafe\nopaque type Token[T] as move, boundary_safe\n",
        )
        .expect("opaque types should parse");

        assert_eq!(parsed.symbols.len(), 2);
        assert_eq!(parsed.symbols[0].kind, SymbolKind::OpaqueType);
        assert!(parsed.symbols[0].exported);
        assert_eq!(parsed.symbols[0].name, "Window");
        assert_eq!(
            parsed.symbols[0].opaque_policy.expect("policy"),
            OpaqueTypePolicy {
                ownership: OpaqueOwnershipPolicy::Move,
                boundary: OpaqueBoundaryPolicy::Unsafe,
            }
        );

        assert_eq!(parsed.symbols[1].kind, SymbolKind::OpaqueType);
        assert_eq!(parsed.symbols[1].name, "Token");
        assert_eq!(parsed.symbols[1].type_params, vec!["T".to_string()]);
        assert_eq!(
            parsed.symbols[1].opaque_policy.expect("policy"),
            OpaqueTypePolicy {
                ownership: OpaqueOwnershipPolicy::Move,
                boundary: OpaqueBoundaryPolicy::Safe,
            }
        );
    }

    #[test]
    fn parse_module_rejects_invalid_opaque_type_declarations() {
        for source in [
            "opaque type Window\n",
            "opaque type Window as move\n",
            "opaque type Window as boundary_safe\n",
            "opaque type Window as move, move, boundary_safe\n",
            "opaque type Window as move, boundary_safe, nope\n",
            "opaque type Window as move, boundary_safe:\n    fn build() -> Int:\n        return 0\n",
        ] {
            let err = parse_module(source).expect_err("opaque declaration should fail");
            assert!(err.contains("opaque type"), "{err}");
        }
    }

    #[test]
    fn runtime_handles_are_not_builtin_types() {
        for name in [
            "Window",
            "Image",
            "FileStream",
            "AudioDevice",
            "AudioBuffer",
            "AudioPlayback",
            "AppFrame",
        ] {
            assert!(
                builtin_type_info(name).is_none(),
                "{name} should be source-declared opaque type, not builtin"
            );
        }
    }

    #[test]
    fn parse_module_collects_headed_regions_v1_shapes() {
        let parsed = parse_module(concat!(
            "record Widget:\n",
            "    value: Int\n",
            "enum Result[T, E]:\n",
            "    Ok(T)\n",
            "    Err(E)\n",
            "Memory arena:cache -alloc\n",
            "    capacity = 8\n",
            "    pressure = bounded\n",
            "fn main() -> Int:\n",
            "    Memory frame:scratch -alloc\n",
            "        capacity = 2\n",
            "    bind -return 0\n",
            "        let value = Result.Ok[Int, Str] :: 1 :: call\n",
            "    recycle -return 0\n",
            "        true\n",
            "    let built = construct yield Widget -return 0\n",
            "        value = value\n",
            "    let copied = record yield Widget from built -return 0\n",
            "        value = value\n",
            "    record deliver Widget from copied -> mirrored -return 0\n",
            "        value = value\n",
            "    construct deliver Widget -> delivered -return 0\n",
            "        value = value\n",
            "    let mut placed = Widget :: value = 0 :: call\n",
            "    record place Widget from copied -> placed -return 0\n",
            "        value = value\n",
            "    construct place Widget -> placed -return 0\n",
            "        value = value\n",
            "    return built.value\n",
        ))
        .expect("headed regions should parse");

        assert_eq!(parsed.memory_specs.len(), 1);
        assert_eq!(parsed.memory_specs[0].family.as_str(), "arena");
        assert_eq!(parsed.memory_specs[0].name, "cache");

        let main = parsed
            .symbols
            .iter()
            .find(|symbol| symbol.name == "main")
            .expect("main should parse");
        assert!(matches!(
            main.statements[0].kind,
            StatementKind::MemorySpec(_)
        ));
        assert!(matches!(
            main.statements[1].kind,
            StatementKind::Bind { .. }
        ));
        assert!(matches!(
            main.statements[2].kind,
            StatementKind::Recycle { .. }
        ));
        assert!(matches!(
            main.statements[3].kind,
            StatementKind::Let {
                value: Expr::ConstructRegion(_),
                ..
            }
        ));
        assert!(matches!(
            main.statements[4].kind,
            StatementKind::Let {
                value: Expr::RecordRegion(_),
                ..
            }
        ));
        assert!(matches!(main.statements[5].kind, StatementKind::Record(_)));
        assert!(matches!(
            main.statements[6].kind,
            StatementKind::Construct(_)
        ));
        assert!(matches!(main.statements[8].kind, StatementKind::Record(_)));
        assert!(matches!(
            main.statements[9].kind,
            StatementKind::Construct(_)
        ));
    }
}
