use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecExpr {
    Int(i64),
    Bool(bool),
    Str(String),
    Path(Vec<String>),
    Pair {
        left: Box<ExecExpr>,
        right: Box<ExecExpr>,
    },
    Collection {
        items: Vec<ExecExpr>,
    },
    Match {
        subject: Box<ExecExpr>,
        arms: Vec<ExecMatchArm>,
    },
    Chain {
        style: String,
        introducer: ExecChainIntroducer,
        steps: Vec<ExecChainStep>,
    },
    MemoryPhrase {
        family: String,
        arena: Box<ExecExpr>,
        init_args: Vec<ExecPhraseArg>,
        constructor: Box<ExecExpr>,
        attached: Vec<ExecHeaderAttachment>,
    },
    Member {
        expr: Box<ExecExpr>,
        member: String,
    },
    Index {
        expr: Box<ExecExpr>,
        index: Box<ExecExpr>,
    },
    Slice {
        expr: Box<ExecExpr>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<Box<ExecExpr>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        end: Option<Box<ExecExpr>>,
        inclusive_end: bool,
    },
    Range {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<Box<ExecExpr>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        end: Option<Box<ExecExpr>>,
        inclusive_end: bool,
    },
    Generic {
        expr: Box<ExecExpr>,
        type_args: Vec<String>,
    },
    Phrase {
        subject: Box<ExecExpr>,
        args: Vec<ExecPhraseArg>,
        qualifier_kind: ExecPhraseQualifierKind,
        qualifier: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resolved_callable: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resolved_routine: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        dynamic_dispatch: Option<ExecDynamicDispatch>,
        attached: Vec<ExecHeaderAttachment>,
    },
    Await {
        expr: Box<ExecExpr>,
    },
    Unary {
        op: ExecUnaryOp,
        expr: Box<ExecExpr>,
    },
    Binary {
        left: Box<ExecExpr>,
        op: ExecBinaryOp,
        right: Box<ExecExpr>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecPhraseQualifierKind {
    Call,
    Try,
    Apply,
    AwaitApply,
    BareMethod,
    NamedPath,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecDynamicDispatch {
    TraitMethod { trait_path: Vec<String> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecMatchArm {
    pub patterns: Vec<ExecMatchPattern>,
    pub value: ExecExpr,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecCleanupFooter {
    pub kind: String,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub binding_id: u64,
    pub subject: String,
    pub handler_path: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_routine: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecNamedBindingId {
    pub name: String,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub binding_id: u64,
}

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecPhraseArg {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub value: ExecExpr,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecHeaderAttachment {
    Named { name: String, value: ExecExpr },
    Chain { expr: ExecExpr },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecAvailabilityKind {
    Owner,
    Object,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecAvailabilityAttachment {
    pub kind: ExecAvailabilityKind,
    pub path: Vec<String>,
    pub local_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecMatchPattern {
    Wildcard,
    Name(String),
    Literal(String),
    Variant {
        path: String,
        args: Vec<ExecMatchPattern>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecStmt {
    Let {
        #[serde(default, skip_serializing_if = "is_zero_u64")]
        binding_id: u64,
        mutable: bool,
        name: String,
        value: ExecExpr,
    },
    Expr {
        expr: ExecExpr,
        cleanup_footers: Vec<ExecCleanupFooter>,
    },
    ReturnVoid,
    ReturnValue {
        value: ExecExpr,
    },
    If {
        condition: ExecExpr,
        then_branch: Vec<ExecStmt>,
        else_branch: Vec<ExecStmt>,
        availability: Vec<ExecAvailabilityAttachment>,
        cleanup_footers: Vec<ExecCleanupFooter>,
    },
    While {
        condition: ExecExpr,
        body: Vec<ExecStmt>,
        availability: Vec<ExecAvailabilityAttachment>,
        cleanup_footers: Vec<ExecCleanupFooter>,
    },
    For {
        #[serde(default, skip_serializing_if = "is_zero_u64")]
        binding_id: u64,
        binding: String,
        iterable: ExecExpr,
        body: Vec<ExecStmt>,
        availability: Vec<ExecAvailabilityAttachment>,
        cleanup_footers: Vec<ExecCleanupFooter>,
    },
    ActivateOwner {
        owner_path: Vec<String>,
        owner_local_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        binding: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        object_binding_ids: Vec<ExecNamedBindingId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context: Option<ExecExpr>,
    },
    Defer(ExecExpr),
    Break,
    Continue,
    Assign {
        target: ExecAssignTarget,
        op: ExecAssignOp,
        value: ExecExpr,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecUnaryOp {
    Neg,
    Not,
    BitNot,
    BorrowRead,
    BorrowMut,
    Deref,
    Weave,
    Split,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecBinaryOp {
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecAssignTarget {
    Name(String),
    Member {
        target: Box<ExecAssignTarget>,
        member: String,
    },
    Index {
        target: Box<ExecAssignTarget>,
        index: ExecExpr,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecAssignOp {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecChainConnector {
    Forward,
    Reverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecChainIntroducer {
    Forward,
    Reverse,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecChainStep {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incoming: Option<ExecChainConnector>,
    pub stage: ExecExpr,
    pub bind_args: Vec<ExecExpr>,
    pub text: String,
}
