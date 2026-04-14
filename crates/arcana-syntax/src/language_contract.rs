#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeadedRegionHead {
    Recycle,
    Bind,
    Array,
    Record,
    Struct,
    Union,
    Construct,
    Memory,
}

impl HeadedRegionHead {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Recycle => "recycle",
            Self::Bind => "bind",
            Self::Array => "array",
            Self::Record => "record",
            Self::Struct => "struct",
            Self::Union => "union",
            Self::Construct => "construct",
            Self::Memory => "Memory",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "recycle" => Some(Self::Recycle),
            "bind" => Some(Self::Bind),
            "array" => Some(Self::Array),
            "record" => Some(Self::Record),
            "struct" => Some(Self::Struct),
            "union" => Some(Self::Union),
            "construct" => Some(Self::Construct),
            "Memory" => Some(Self::Memory),
            _ => None,
        }
    }

    pub const fn is_record_like(self) -> bool {
        matches!(self, Self::Record | Self::Struct | Self::Union)
    }

    pub const fn supports_expression_yield(self) -> bool {
        matches!(
            self,
            Self::Construct | Self::Array | Self::Record | Self::Struct | Self::Union
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstructCompletionKind {
    Yield,
    Deliver,
    Place,
}

impl ConstructCompletionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Yield => "yield",
            Self::Deliver => "deliver",
            Self::Place => "place",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadedModifierKeyword {
    Return,
    Break,
    Continue,
    Default,
    Preserve,
    Replace,
    Skip,
    NamedExit,
}

impl HeadedModifierKeyword {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Return => "return",
            Self::Break => "break",
            Self::Continue => "continue",
            Self::Default => "default",
            Self::Preserve => "preserve",
            Self::Replace => "replace",
            Self::Skip => "skip",
            Self::NamedExit => "named_exit",
        }
    }

    pub fn parse(text: &str) -> Self {
        match text {
            "return" => Self::Return,
            "break" => Self::Break,
            "continue" => Self::Continue,
            "default" => Self::Default,
            "preserve" => Self::Preserve,
            "replace" => Self::Replace,
            "skip" => Self::Skip,
            _ => Self::NamedExit,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryFamily {
    Arena,
    Frame,
    Pool,
    Temp,
    Session,
    Ring,
    Slab,
}

impl MemoryFamily {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Arena => "arena",
            Self::Frame => "frame",
            Self::Pool => "pool",
            Self::Temp => "temp",
            Self::Session => "session",
            Self::Ring => "ring",
            Self::Slab => "slab",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "arena" => Some(Self::Arena),
            "frame" => Some(Self::Frame),
            "pool" => Some(Self::Pool),
            "temp" => Some(Self::Temp),
            "session" => Some(Self::Session),
            "ring" => Some(Self::Ring),
            "slab" => Some(Self::Slab),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryDetailKey {
    Capacity,
    Growth,
    Recycle,
    Handle,
    Pressure,
    ResetOn,
    Page,
    Overwrite,
    Window,
}

impl MemoryDetailKey {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Capacity => "capacity",
            Self::Growth => "growth",
            Self::Recycle => "recycle",
            Self::Handle => "handle",
            Self::Pressure => "pressure",
            Self::ResetOn => "reset_on",
            Self::Page => "page",
            Self::Overwrite => "overwrite",
            Self::Window => "window",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "capacity" => Some(Self::Capacity),
            "growth" => Some(Self::Growth),
            "recycle" => Some(Self::Recycle),
            "handle" => Some(Self::Handle),
            "pressure" => Some(Self::Pressure),
            "reset_on" => Some(Self::ResetOn),
            "page" => Some(Self::Page),
            "overwrite" => Some(Self::Overwrite),
            "window" => Some(Self::Window),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryDetailValueKind {
    IntExpr,
    Atom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryDetailDescriptor {
    pub key: MemoryDetailKey,
    pub value_kind: MemoryDetailValueKind,
    pub operational: bool,
    pub atoms: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryFamilyDescriptor {
    pub family: MemoryFamily,
    pub module_specs: bool,
    pub block_specs: bool,
    pub detail_keys: &'static [MemoryDetailDescriptor],
    pub supported_modifiers: &'static [&'static str],
    pub lazy_materialization_hook_id: &'static str,
    pub phrase_consumers: &'static [&'static str],
}

const MEMORY_PHRASE_CONSUMERS: &[&str] = &["memory_phrase"];
const ARENA_MODIFIERS: &[&str] = &["alloc", "grow", "fixed"];
const FRAME_MODIFIERS: &[&str] = &["alloc", "grow", "recycle"];
const POOL_MODIFIERS: &[&str] = &["alloc", "grow", "fixed", "recycle"];
const TEMP_MODIFIERS: &[&str] = &["alloc", "grow", "fixed"];
const SESSION_MODIFIERS: &[&str] = &["alloc", "grow", "fixed"];
const RING_MODIFIERS: &[&str] = &["alloc", "grow", "fixed"];
const SLAB_MODIFIERS: &[&str] = &["alloc", "grow", "fixed"];

const ARENA_DETAILS: &[MemoryDetailDescriptor] = &[
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Capacity,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Growth,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Pressure,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["bounded", "elastic"],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Handle,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["stable", "unstable"],
    },
];

const FRAME_DETAILS: &[MemoryDetailDescriptor] = &[
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Capacity,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Growth,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Pressure,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["bounded", "elastic"],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Recycle,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["manual", "frame"],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::ResetOn,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["manual", "frame", "owner_exit"],
    },
];

const POOL_DETAILS: &[MemoryDetailDescriptor] = &[
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Capacity,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Growth,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Pressure,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["bounded", "elastic"],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Recycle,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["free_list", "strict"],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Handle,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["stable", "unstable"],
    },
];

const TEMP_DETAILS: &[MemoryDetailDescriptor] = &[
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Capacity,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Growth,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Pressure,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["bounded", "elastic"],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::ResetOn,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["manual", "frame", "owner_exit"],
    },
];

const SESSION_DETAILS: &[MemoryDetailDescriptor] = &[
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Capacity,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Growth,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Pressure,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["bounded", "elastic"],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Handle,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["stable"],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::ResetOn,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["manual"],
    },
];

const RING_DETAILS: &[MemoryDetailDescriptor] = &[
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Capacity,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Growth,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Pressure,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["bounded", "elastic"],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Overwrite,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["oldest"],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Window,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
];

const SLAB_DETAILS: &[MemoryDetailDescriptor] = &[
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Capacity,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Growth,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Pressure,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["bounded", "elastic"],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Handle,
        value_kind: MemoryDetailValueKind::Atom,
        operational: true,
        atoms: &["stable"],
    },
    MemoryDetailDescriptor {
        key: MemoryDetailKey::Page,
        value_kind: MemoryDetailValueKind::IntExpr,
        operational: true,
        atoms: &[],
    },
];

const MEMORY_FAMILY_DESCRIPTORS: &[MemoryFamilyDescriptor] = &[
    MemoryFamilyDescriptor {
        family: MemoryFamily::Arena,
        module_specs: true,
        block_specs: true,
        detail_keys: ARENA_DETAILS,
        supported_modifiers: ARENA_MODIFIERS,
        lazy_materialization_hook_id: "arena_new",
        phrase_consumers: MEMORY_PHRASE_CONSUMERS,
    },
    MemoryFamilyDescriptor {
        family: MemoryFamily::Frame,
        module_specs: true,
        block_specs: true,
        detail_keys: FRAME_DETAILS,
        supported_modifiers: FRAME_MODIFIERS,
        lazy_materialization_hook_id: "frame_new",
        phrase_consumers: MEMORY_PHRASE_CONSUMERS,
    },
    MemoryFamilyDescriptor {
        family: MemoryFamily::Pool,
        module_specs: true,
        block_specs: true,
        detail_keys: POOL_DETAILS,
        supported_modifiers: POOL_MODIFIERS,
        lazy_materialization_hook_id: "pool_new",
        phrase_consumers: MEMORY_PHRASE_CONSUMERS,
    },
    MemoryFamilyDescriptor {
        family: MemoryFamily::Temp,
        module_specs: true,
        block_specs: true,
        detail_keys: TEMP_DETAILS,
        supported_modifiers: TEMP_MODIFIERS,
        lazy_materialization_hook_id: "temp_new",
        phrase_consumers: MEMORY_PHRASE_CONSUMERS,
    },
    MemoryFamilyDescriptor {
        family: MemoryFamily::Session,
        module_specs: true,
        block_specs: true,
        detail_keys: SESSION_DETAILS,
        supported_modifiers: SESSION_MODIFIERS,
        lazy_materialization_hook_id: "session_new",
        phrase_consumers: MEMORY_PHRASE_CONSUMERS,
    },
    MemoryFamilyDescriptor {
        family: MemoryFamily::Ring,
        module_specs: true,
        block_specs: true,
        detail_keys: RING_DETAILS,
        supported_modifiers: RING_MODIFIERS,
        lazy_materialization_hook_id: "ring_new",
        phrase_consumers: MEMORY_PHRASE_CONSUMERS,
    },
    MemoryFamilyDescriptor {
        family: MemoryFamily::Slab,
        module_specs: true,
        block_specs: true,
        detail_keys: SLAB_DETAILS,
        supported_modifiers: SLAB_MODIFIERS,
        lazy_materialization_hook_id: "slab_new",
        phrase_consumers: MEMORY_PHRASE_CONSUMERS,
    },
];

pub fn memory_family_descriptors() -> &'static [MemoryFamilyDescriptor] {
    MEMORY_FAMILY_DESCRIPTORS
}

pub fn memory_family_descriptor(family: MemoryFamily) -> &'static MemoryFamilyDescriptor {
    MEMORY_FAMILY_DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.family == family)
        .expect("seeded memory family descriptor should exist")
}

pub fn memory_detail_descriptor(
    family: MemoryFamily,
    key: MemoryDetailKey,
) -> Option<&'static MemoryDetailDescriptor> {
    memory_family_descriptor(family)
        .detail_keys
        .iter()
        .find(|descriptor| descriptor.key == key)
}

pub fn memory_modifier_allowed(family: MemoryFamily, modifier: &str) -> bool {
    memory_family_descriptor(family)
        .supported_modifiers
        .contains(&modifier)
}
