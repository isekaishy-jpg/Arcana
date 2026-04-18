use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeValue {
    Int(i64),
    Float {
        text: String,
        kind: ParsedFloatKind,
    },
    Bool(bool),
    Str(String),
    Bytes(Vec<u8>),
    ByteBuffer(Vec<u8>),
    Utf16(Vec<u16>),
    Utf16Buffer(Vec<u16>),
    Tuple(Vec<RuntimeValue>),
    Pair(Box<RuntimeValue>, Box<RuntimeValue>),
    Array(Vec<RuntimeValue>),
    List(Vec<RuntimeValue>),
    Map(Vec<(RuntimeValue, RuntimeValue)>),
    Range {
        start: Option<i64>,
        end: Option<i64>,
        inclusive_end: bool,
    },
    OwnerHandle(String),
    Ref(RuntimeReferenceValue),
    Opaque(RuntimeOpaqueValue),
    Record {
        name: String,
        fields: BTreeMap<String, RuntimeValue>,
    },
    Struct {
        name: String,
        fields: BTreeMap<String, RuntimeValue>,
    },
    Union {
        name: String,
        fields: BTreeMap<String, RuntimeValue>,
    },
    Variant {
        name: String,
        payload: Vec<RuntimeValue>,
    },
    Unit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeCallArg {
    pub(crate) name: Option<String>,
    pub(crate) value: RuntimeValue,
    pub(crate) source_expr: ParsedExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BoundRuntimeArg {
    pub(crate) value: RuntimeValue,
    pub(crate) source_expr: ParsedExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RoutineExecutionOutcome {
    pub(crate) value: RuntimeValue,
    pub(crate) final_args: Vec<RuntimeValue>,
    pub(crate) skip_write_back_edit_indices: BTreeSet<usize>,
    pub(crate) control: Option<FlowSignal>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeLocal {
    pub(crate) handle: RuntimeLocalHandle,
    pub(crate) binding_id: u64,
    pub(crate) mutable: bool,
    pub(crate) moved: bool,
    pub(crate) held: bool,
    pub(crate) take_reserved: bool,
    pub(crate) value: RuntimeValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeAttachedOwner {
    pub(crate) owner_path: Vec<String>,
    pub(crate) local_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeAttachedObject {
    pub(crate) object_path: Vec<String>,
    pub(crate) local_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeMemorySpecState {
    pub(crate) spec: ParsedMemorySpecDecl,
    pub(crate) handle: Option<RuntimeValue>,
    pub(crate) handle_policy: Option<RuntimeMemoryHandlePolicy>,
    pub(crate) owner_keys: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RuntimeScope {
    pub(crate) locals: BTreeMap<String, RuntimeLocal>,
    pub(crate) memory_specs: BTreeMap<String, RuntimeMemorySpecState>,
    pub(crate) deferred: Vec<ParsedDeferAction>,
    pub(crate) attached_object_names: BTreeSet<String>,
    pub(crate) attached_objects: Vec<RuntimeAttachedObject>,
    pub(crate) attached_owners: Vec<RuntimeAttachedOwner>,
    pub(crate) inherited_active_owner_keys: Vec<String>,
    pub(crate) activated_owner_keys: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FlowSignal {
    Next,
    Return(RuntimeValue),
    Break,
    Continue,
    OwnerExit {
        owner_key: String,
        exit_name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeEvalSignal {
    Message(String),
    Return(RuntimeValue),
    OwnerExit {
        owner_key: String,
        exit_name: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeMemoryStrategy {
    Alloc,
    Grow,
    Fixed,
    Recycle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeMemoryPressurePolicy {
    Bounded,
    Elastic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeMemoryHandlePolicy {
    Stable,
    Unstable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeFrameRecyclePolicy {
    Manual,
    Frame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimePoolRecyclePolicy {
    FreeList,
    Strict,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeResetOnPolicy {
    Manual,
    Frame,
    OwnerExit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeRingOverwritePolicy {
    Oldest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeArenaPolicy {
    pub(crate) base_capacity: usize,
    pub(crate) current_limit: usize,
    pub(crate) growth_step: usize,
    pub(crate) pressure: RuntimeMemoryPressurePolicy,
    pub(crate) handle: RuntimeMemoryHandlePolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeFrameArenaPolicy {
    pub(crate) base_capacity: usize,
    pub(crate) current_limit: usize,
    pub(crate) growth_step: usize,
    pub(crate) pressure: RuntimeMemoryPressurePolicy,
    pub(crate) recycle: RuntimeFrameRecyclePolicy,
    pub(crate) reset_on: RuntimeResetOnPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimePoolArenaPolicy {
    pub(crate) base_capacity: usize,
    pub(crate) current_limit: usize,
    pub(crate) growth_step: usize,
    pub(crate) pressure: RuntimeMemoryPressurePolicy,
    pub(crate) recycle: RuntimePoolRecyclePolicy,
    pub(crate) handle: RuntimeMemoryHandlePolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeTempArenaPolicy {
    pub(crate) base_capacity: usize,
    pub(crate) current_limit: usize,
    pub(crate) growth_step: usize,
    pub(crate) pressure: RuntimeMemoryPressurePolicy,
    pub(crate) reset_on: RuntimeResetOnPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeSessionArenaPolicy {
    pub(crate) base_capacity: usize,
    pub(crate) current_limit: usize,
    pub(crate) growth_step: usize,
    pub(crate) pressure: RuntimeMemoryPressurePolicy,
    pub(crate) handle: RuntimeMemoryHandlePolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeRingBufferPolicy {
    pub(crate) base_capacity: usize,
    pub(crate) current_limit: usize,
    pub(crate) growth_step: usize,
    pub(crate) pressure: RuntimeMemoryPressurePolicy,
    pub(crate) overwrite: RuntimeRingOverwritePolicy,
    pub(crate) window: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeSlabPolicy {
    pub(crate) base_capacity: usize,
    pub(crate) current_limit: usize,
    pub(crate) growth_step: usize,
    pub(crate) pressure: RuntimeMemoryPressurePolicy,
    pub(crate) handle: RuntimeMemoryHandlePolicy,
    pub(crate) page: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeMemorySpecMaterializationKind {
    Arena(RuntimeArenaPolicy),
    Frame(RuntimeFrameArenaPolicy),
    Pool(RuntimePoolArenaPolicy),
    Temp(RuntimeTempArenaPolicy),
    Session(RuntimeSessionArenaPolicy),
    Ring(RuntimeRingBufferPolicy),
    Slab(RuntimeSlabPolicy),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeMemorySpecMaterialization {
    pub(crate) hook_id: &'static str,
    pub(crate) handle_policy: RuntimeMemoryHandlePolicy,
    pub(crate) kind: RuntimeMemorySpecMaterializationKind,
}

pub(crate) type RuntimeEvalResult<T> = Result<T, RuntimeEvalSignal>;

impl From<String> for RuntimeEvalSignal {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}
