use std::ffi::{c_char, c_void};

use serde::{Deserialize, Serialize};

pub const ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL: &str = "arcana_cabi_get_product_api_v1";
pub const ARCANA_CABI_LAST_ERROR_ALLOC_V1_SYMBOL: &str = "arcana_cabi_last_error_alloc_v1";
pub const ARCANA_CABI_OWNED_BYTES_FREE_V1_SYMBOL: &str = "arcana_cabi_owned_bytes_free_v1";
pub const ARCANA_CABI_OWNED_STR_FREE_V1_SYMBOL: &str = "arcana_cabi_owned_str_free_v1";

pub const ARCANA_CABI_EXPORT_CONTRACT_ID: &str = "arcana.cabi.export.v1";
pub const ARCANA_CABI_CHILD_CONTRACT_ID: &str = "arcana.cabi.child.v1";
pub const ARCANA_CABI_PLUGIN_CONTRACT_ID: &str = "arcana.cabi.plugin.v1";
pub const ARCANA_CABI_PROVIDER_CONTRACT_ID: &str = "arcana.cabi.provider.v1";
pub const ARCANA_CABI_CONTRACT_VERSION_V1: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArcanaCabiProductRole {
    Export,
    Child,
    Plugin,
    Provider,
}

impl ArcanaCabiProductRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Export => "export",
            Self::Child => "child",
            Self::Plugin => "plugin",
            Self::Provider => "provider",
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "export" => Ok(Self::Export),
            "child" => Ok(Self::Child),
            "plugin" => Ok(Self::Plugin),
            "provider" => Ok(Self::Provider),
            other => Err(format!(
                "`role` must be \"export\", \"child\", \"plugin\", or \"provider\" (found `{other}`)"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArcanaCabiParamSourceMode {
    Read,
    Take,
    Edit,
}

impl ArcanaCabiParamSourceMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Take => "take",
            Self::Edit => "edit",
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "read" => Ok(Self::Read),
            "take" => Ok(Self::Take),
            "edit" => Ok(Self::Edit),
            other => Err(format!("unsupported provider param source mode `{other}`")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArcanaCabiPassMode {
    In,
    InWithWriteBack,
}

impl ArcanaCabiPassMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::In => "in",
            Self::InWithWriteBack => "in_with_write_back",
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "in" => Ok(Self::In),
            "in_with_write_back" => Ok(Self::InWithWriteBack),
            other => Err(format!("unsupported provider pass mode `{other}`")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArcanaCabiType {
    Int,
    Bool,
    Str,
    Bytes,
    Pair(Box<ArcanaCabiType>, Box<ArcanaCabiType>),
    Unit,
}

impl ArcanaCabiType {
    pub fn render(&self) -> String {
        match self {
            Self::Int => "Int".to_string(),
            Self::Bool => "Bool".to_string(),
            Self::Str => "Str".to_string(),
            Self::Bytes => "Array[Int]".to_string(),
            Self::Unit => "Unit".to_string(),
            Self::Pair(left, right) => format!("Pair[{}, {}]", left.render(), right.render()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiExportParam {
    pub name: String,
    pub source_mode: ArcanaCabiParamSourceMode,
    pub pass_mode: ArcanaCabiPassMode,
    pub input_type: ArcanaCabiType,
    pub write_back_type: Option<ArcanaCabiType>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiExport {
    pub export_name: String,
    pub routine_key: String,
    pub symbol_name: String,
    pub return_type: ArcanaCabiType,
    pub params: Vec<ArcanaCabiExportParam>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArcanaCabiProviderDescriptorViewOwner {
    Runtime {
        package_id: String,
    },
    ProviderBinding {
        consumer_package_id: String,
        dependency_alias: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArcanaCabiProviderDescriptorViewBackingKind {
    ReadElements,
    ReadBytes,
    ReadUtf8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiProviderDescriptorView {
    pub owner: ArcanaCabiProviderDescriptorViewOwner,
    pub backing_kind: ArcanaCabiProviderDescriptorViewBackingKind,
    pub family: String,
    pub id: u64,
    pub element_type: String,
    pub element_layout: String,
    pub start: u64,
    pub len: u64,
    pub mutable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArcanaCabiProviderValue {
    Int(i64),
    Bool(bool),
    Str(String),
    Bytes(Vec<u8>),
    Pair(Box<ArcanaCabiProviderValue>, Box<ArcanaCabiProviderValue>),
    List(Vec<ArcanaCabiProviderValue>),
    Map(Vec<(ArcanaCabiProviderValue, ArcanaCabiProviderValue)>),
    Range {
        start: Option<i64>,
        end: Option<i64>,
        inclusive_end: bool,
    },
    Record {
        name: String,
        fields: Vec<(String, ArcanaCabiProviderValue)>,
    },
    Variant {
        name: String,
        payload: Vec<ArcanaCabiProviderValue>,
    },
    DescriptorView(ArcanaCabiProviderDescriptorView),
    SubstrateOpaque {
        family: String,
        id: u64,
    },
    ProviderOpaque {
        family: String,
        id: u64,
    },
    Unit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiProviderWriteBack {
    pub index: usize,
    pub name: String,
    pub value: ArcanaCabiProviderValue,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiProviderCallOutcome {
    pub result: ArcanaCabiProviderValue,
    pub write_backs: Vec<ArcanaCabiProviderWriteBack>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiProviderParam {
    pub name: String,
    pub source_mode: ArcanaCabiParamSourceMode,
    pub pass_mode: ArcanaCabiPassMode,
    pub input_type: String,
    pub write_back_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiProviderCallable {
    pub callable_key: String,
    pub path: String,
    pub routine_key: Option<String>,
    pub return_type: String,
    pub params: Vec<ArcanaCabiProviderParam>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiProviderOpaqueFamily {
    pub family_key: String,
    pub type_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiProviderDescriptor {
    pub format: String,
    pub package_name: String,
    pub product_name: String,
    pub callables: Vec<ArcanaCabiProviderCallable>,
    pub opaque_families: Vec<ArcanaCabiProviderOpaqueFamily>,
}

const PROVIDER_CODEC_VERSION: u8 = 1;
const PROVIDER_TAG_INT: u8 = 1;
const PROVIDER_TAG_BOOL: u8 = 2;
const PROVIDER_TAG_STR: u8 = 3;
const PROVIDER_TAG_BYTES: u8 = 4;
const PROVIDER_TAG_PAIR: u8 = 5;
const PROVIDER_TAG_LIST: u8 = 6;
const PROVIDER_TAG_MAP: u8 = 7;
const PROVIDER_TAG_RANGE: u8 = 8;
const PROVIDER_TAG_RECORD: u8 = 9;
const PROVIDER_TAG_VARIANT: u8 = 10;
const PROVIDER_TAG_DESCRIPTOR_VIEW: u8 = 11;
const PROVIDER_TAG_SUBSTRATE_OPAQUE: u8 = 12;
const PROVIDER_TAG_PROVIDER_OPAQUE: u8 = 13;
const PROVIDER_TAG_UNIT: u8 = 14;
const PROVIDER_DESCRIPTOR_MAGIC: &[u8] = b"arcana.provider.descriptor.v1\0";

pub fn encode_provider_value(value: &ArcanaCabiProviderValue) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.push(PROVIDER_CODEC_VERSION);
    encode_provider_value_inner(value, &mut out)?;
    Ok(out)
}

pub fn decode_provider_value(bytes: &[u8]) -> Result<ArcanaCabiProviderValue, String> {
    let mut cursor = ProviderDecodeCursor::new(bytes);
    let version = cursor.read_u8("provider value version")?;
    if version != PROVIDER_CODEC_VERSION {
        return Err(format!(
            "unsupported provider value codec version `{version}`"
        ));
    }
    let value = decode_provider_value_inner(&mut cursor)?;
    cursor.finish()?;
    Ok(value)
}

pub fn encode_provider_values(values: &[ArcanaCabiProviderValue]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.push(PROVIDER_CODEC_VERSION);
    write_len(values.len(), &mut out)?;
    for value in values {
        encode_provider_value_inner(value, &mut out)?;
    }
    Ok(out)
}

pub fn decode_provider_values(bytes: &[u8]) -> Result<Vec<ArcanaCabiProviderValue>, String> {
    let mut cursor = ProviderDecodeCursor::new(bytes);
    let version = cursor.read_u8("provider values version")?;
    if version != PROVIDER_CODEC_VERSION {
        return Err(format!(
            "unsupported provider values codec version `{version}`"
        ));
    }
    let count = cursor.read_len("provider value count")?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(decode_provider_value_inner(&mut cursor)?);
    }
    cursor.finish()?;
    Ok(values)
}

pub fn encode_provider_call_outcome(
    outcome: &ArcanaCabiProviderCallOutcome,
) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.push(PROVIDER_CODEC_VERSION);
    encode_provider_value_inner(&outcome.result, &mut out)?;
    write_len(outcome.write_backs.len(), &mut out)?;
    for write_back in &outcome.write_backs {
        write_u64(
            u64::try_from(write_back.index).map_err(|_| {
                format!(
                    "write-back index `{}` does not fit in u64",
                    write_back.index
                )
            })?,
            &mut out,
        );
        write_string(&write_back.name, &mut out)?;
        encode_provider_value_inner(&write_back.value, &mut out)?;
    }
    Ok(out)
}

pub fn decode_provider_call_outcome(bytes: &[u8]) -> Result<ArcanaCabiProviderCallOutcome, String> {
    let mut cursor = ProviderDecodeCursor::new(bytes);
    let version = cursor.read_u8("provider outcome version")?;
    if version != PROVIDER_CODEC_VERSION {
        return Err(format!(
            "unsupported provider outcome codec version `{version}`"
        ));
    }
    let result = decode_provider_value_inner(&mut cursor)?;
    let write_back_count = cursor.read_len("provider write-back count")?;
    let mut write_backs = Vec::with_capacity(write_back_count);
    for _ in 0..write_back_count {
        let index = usize::try_from(cursor.read_u64("provider write-back index")?)
            .map_err(|_| "provider write-back index does not fit in usize".to_string())?;
        let name = cursor.read_string("provider write-back name")?;
        let value = decode_provider_value_inner(&mut cursor)?;
        write_backs.push(ArcanaCabiProviderWriteBack { index, name, value });
    }
    cursor.finish()?;
    Ok(ArcanaCabiProviderCallOutcome {
        result,
        write_backs,
    })
}

pub fn encode_provider_descriptor(
    descriptor: &ArcanaCabiProviderDescriptor,
) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.extend_from_slice(PROVIDER_DESCRIPTOR_MAGIC);
    write_string(&descriptor.format, &mut out)?;
    write_string(&descriptor.package_name, &mut out)?;
    write_string(&descriptor.product_name, &mut out)?;
    write_len(descriptor.callables.len(), &mut out)?;
    for callable in &descriptor.callables {
        write_string(&callable.callable_key, &mut out)?;
        write_string(&callable.path, &mut out)?;
        write_bool(callable.routine_key.is_some(), &mut out);
        if let Some(routine_key) = &callable.routine_key {
            write_string(routine_key, &mut out)?;
        }
        write_string(&callable.return_type, &mut out)?;
        write_len(callable.params.len(), &mut out)?;
        for param in &callable.params {
            write_string(&param.name, &mut out)?;
            write_string(param.source_mode.as_str(), &mut out)?;
            write_string(param.pass_mode.as_str(), &mut out)?;
            write_string(&param.input_type, &mut out)?;
            write_bool(param.write_back_type.is_some(), &mut out);
            if let Some(write_back_type) = &param.write_back_type {
                write_string(write_back_type, &mut out)?;
            }
        }
    }
    write_len(descriptor.opaque_families.len(), &mut out)?;
    for family in &descriptor.opaque_families {
        write_string(&family.family_key, &mut out)?;
        write_string(&family.type_path, &mut out)?;
    }
    Ok(out)
}

pub fn decode_provider_descriptor(bytes: &[u8]) -> Result<ArcanaCabiProviderDescriptor, String> {
    let mut cursor = ProviderDecodeCursor::new(bytes);
    cursor.expect_exact_bytes(PROVIDER_DESCRIPTOR_MAGIC, "provider descriptor header")?;
    let format = cursor.read_string("provider descriptor format")?;
    let package_name = cursor.read_string("provider descriptor package_name")?;
    let product_name = cursor.read_string("provider descriptor product_name")?;
    let callable_count = cursor.read_len("provider descriptor callable count")?;
    let mut callables = Vec::with_capacity(callable_count);
    for _ in 0..callable_count {
        let callable_key = cursor.read_string("provider callable key")?;
        let path = cursor.read_string("provider callable path")?;
        let has_routine_key = cursor.read_bool("provider callable has routine_key")?;
        let routine_key = if has_routine_key {
            Some(cursor.read_string("provider callable routine_key")?)
        } else {
            None
        };
        let return_type = cursor.read_string("provider callable return_type")?;
        let param_count = cursor.read_len("provider callable param count")?;
        let mut params = Vec::with_capacity(param_count);
        for _ in 0..param_count {
            let name = cursor.read_string("provider param name")?;
            let source_mode = ArcanaCabiParamSourceMode::parse(
                &cursor.read_string("provider param source_mode")?,
            )?;
            let pass_mode =
                ArcanaCabiPassMode::parse(&cursor.read_string("provider param pass_mode")?)?;
            let input_type = cursor.read_string("provider param input_type")?;
            let has_write_back_type = cursor.read_bool("provider param has write_back_type")?;
            let write_back_type = if has_write_back_type {
                Some(cursor.read_string("provider param write_back_type")?)
            } else {
                None
            };
            params.push(ArcanaCabiProviderParam {
                name,
                source_mode,
                pass_mode,
                input_type,
                write_back_type,
            });
        }
        callables.push(ArcanaCabiProviderCallable {
            callable_key,
            path,
            routine_key,
            return_type,
            params,
        });
    }
    let family_count = cursor.read_len("provider descriptor opaque family count")?;
    let mut opaque_families = Vec::with_capacity(family_count);
    for _ in 0..family_count {
        opaque_families.push(ArcanaCabiProviderOpaqueFamily {
            family_key: cursor.read_string("provider opaque family key")?,
            type_path: cursor.read_string("provider opaque type path")?,
        });
    }
    cursor.finish()?;
    Ok(ArcanaCabiProviderDescriptor {
        format,
        package_name,
        product_name,
        callables,
        opaque_families,
    })
}

fn encode_provider_value_inner(
    value: &ArcanaCabiProviderValue,
    out: &mut Vec<u8>,
) -> Result<(), String> {
    match value {
        ArcanaCabiProviderValue::Int(value) => {
            out.push(PROVIDER_TAG_INT);
            write_i64(*value, out);
        }
        ArcanaCabiProviderValue::Bool(value) => {
            out.push(PROVIDER_TAG_BOOL);
            out.push(u8::from(*value));
        }
        ArcanaCabiProviderValue::Str(value) => {
            out.push(PROVIDER_TAG_STR);
            write_string(value, out)?;
        }
        ArcanaCabiProviderValue::Bytes(value) => {
            out.push(PROVIDER_TAG_BYTES);
            write_bytes(value, out)?;
        }
        ArcanaCabiProviderValue::Pair(left, right) => {
            out.push(PROVIDER_TAG_PAIR);
            encode_provider_value_inner(left, out)?;
            encode_provider_value_inner(right, out)?;
        }
        ArcanaCabiProviderValue::List(values) => {
            out.push(PROVIDER_TAG_LIST);
            write_len(values.len(), out)?;
            for value in values {
                encode_provider_value_inner(value, out)?;
            }
        }
        ArcanaCabiProviderValue::Map(entries) => {
            out.push(PROVIDER_TAG_MAP);
            write_len(entries.len(), out)?;
            for (key, value) in entries {
                encode_provider_value_inner(key, out)?;
                encode_provider_value_inner(value, out)?;
            }
        }
        ArcanaCabiProviderValue::Range {
            start,
            end,
            inclusive_end,
        } => {
            out.push(PROVIDER_TAG_RANGE);
            write_optional_i64(*start, out);
            write_optional_i64(*end, out);
            out.push(u8::from(*inclusive_end));
        }
        ArcanaCabiProviderValue::Record { name, fields } => {
            out.push(PROVIDER_TAG_RECORD);
            write_string(name, out)?;
            write_len(fields.len(), out)?;
            for (field, value) in fields {
                write_string(field, out)?;
                encode_provider_value_inner(value, out)?;
            }
        }
        ArcanaCabiProviderValue::Variant { name, payload } => {
            out.push(PROVIDER_TAG_VARIANT);
            write_string(name, out)?;
            write_len(payload.len(), out)?;
            for value in payload {
                encode_provider_value_inner(value, out)?;
            }
        }
        ArcanaCabiProviderValue::DescriptorView(view) => {
            out.push(PROVIDER_TAG_DESCRIPTOR_VIEW);
            encode_provider_descriptor_view_owner(&view.owner, out)?;
            encode_provider_descriptor_view_backing_kind(&view.backing_kind, out);
            write_string(&view.family, out)?;
            write_u64(view.id, out);
            write_string(&view.element_type, out)?;
            write_string(&view.element_layout, out)?;
            write_u64(view.start, out);
            write_u64(view.len, out);
            write_bool(view.mutable, out);
        }
        ArcanaCabiProviderValue::SubstrateOpaque { family, id } => {
            out.push(PROVIDER_TAG_SUBSTRATE_OPAQUE);
            write_string(family, out)?;
            write_u64(*id, out);
        }
        ArcanaCabiProviderValue::ProviderOpaque { family, id } => {
            out.push(PROVIDER_TAG_PROVIDER_OPAQUE);
            write_string(family, out)?;
            write_u64(*id, out);
        }
        ArcanaCabiProviderValue::Unit => out.push(PROVIDER_TAG_UNIT),
    }
    Ok(())
}

fn decode_provider_value_inner(
    cursor: &mut ProviderDecodeCursor<'_>,
) -> Result<ArcanaCabiProviderValue, String> {
    let tag = cursor.read_u8("provider value tag")?;
    match tag {
        PROVIDER_TAG_INT => Ok(ArcanaCabiProviderValue::Int(
            cursor.read_i64("provider Int payload")?,
        )),
        PROVIDER_TAG_BOOL => Ok(ArcanaCabiProviderValue::Bool(
            cursor.read_u8("provider Bool payload")? != 0,
        )),
        PROVIDER_TAG_STR => Ok(ArcanaCabiProviderValue::Str(
            cursor.read_string("provider Str payload")?,
        )),
        PROVIDER_TAG_BYTES => Ok(ArcanaCabiProviderValue::Bytes(
            cursor.read_bytes("provider Bytes payload")?,
        )),
        PROVIDER_TAG_PAIR => Ok(ArcanaCabiProviderValue::Pair(
            Box::new(decode_provider_value_inner(cursor)?),
            Box::new(decode_provider_value_inner(cursor)?),
        )),
        PROVIDER_TAG_LIST => {
            let count = cursor.read_len("provider List length")?;
            let mut values = Vec::with_capacity(count);
            for _ in 0..count {
                values.push(decode_provider_value_inner(cursor)?);
            }
            Ok(ArcanaCabiProviderValue::List(values))
        }
        PROVIDER_TAG_MAP => {
            let count = cursor.read_len("provider Map length")?;
            let mut values = Vec::with_capacity(count);
            for _ in 0..count {
                values.push((
                    decode_provider_value_inner(cursor)?,
                    decode_provider_value_inner(cursor)?,
                ));
            }
            Ok(ArcanaCabiProviderValue::Map(values))
        }
        PROVIDER_TAG_RANGE => Ok(ArcanaCabiProviderValue::Range {
            start: cursor.read_optional_i64("provider Range.start")?,
            end: cursor.read_optional_i64("provider Range.end")?,
            inclusive_end: cursor.read_u8("provider Range.inclusive_end")? != 0,
        }),
        PROVIDER_TAG_RECORD => {
            let name = cursor.read_string("provider Record name")?;
            let field_count = cursor.read_len("provider Record field count")?;
            let mut fields = Vec::with_capacity(field_count);
            for _ in 0..field_count {
                fields.push((
                    cursor.read_string("provider Record field name")?,
                    decode_provider_value_inner(cursor)?,
                ));
            }
            Ok(ArcanaCabiProviderValue::Record { name, fields })
        }
        PROVIDER_TAG_VARIANT => {
            let name = cursor.read_string("provider Variant name")?;
            let payload_count = cursor.read_len("provider Variant payload count")?;
            let mut payload = Vec::with_capacity(payload_count);
            for _ in 0..payload_count {
                payload.push(decode_provider_value_inner(cursor)?);
            }
            Ok(ArcanaCabiProviderValue::Variant { name, payload })
        }
        PROVIDER_TAG_DESCRIPTOR_VIEW => Ok(ArcanaCabiProviderValue::DescriptorView(
            ArcanaCabiProviderDescriptorView {
                owner: decode_provider_descriptor_view_owner(cursor)?,
                backing_kind: decode_provider_descriptor_view_backing_kind(cursor)?,
                family: cursor.read_string("provider DescriptorView family")?,
                id: cursor.read_u64("provider DescriptorView id")?,
                element_type: cursor.read_string("provider DescriptorView element_type")?,
                element_layout: cursor.read_string("provider DescriptorView element_layout")?,
                start: cursor.read_u64("provider DescriptorView start")?,
                len: cursor.read_u64("provider DescriptorView len")?,
                mutable: cursor.read_bool("provider DescriptorView mutable")?,
            },
        )),
        PROVIDER_TAG_SUBSTRATE_OPAQUE => Ok(ArcanaCabiProviderValue::SubstrateOpaque {
            family: cursor.read_string("provider SubstrateOpaque family")?,
            id: cursor.read_u64("provider SubstrateOpaque id")?,
        }),
        PROVIDER_TAG_PROVIDER_OPAQUE => Ok(ArcanaCabiProviderValue::ProviderOpaque {
            family: cursor.read_string("provider ProviderOpaque family")?,
            id: cursor.read_u64("provider ProviderOpaque id")?,
        }),
        PROVIDER_TAG_UNIT => Ok(ArcanaCabiProviderValue::Unit),
        other => Err(format!("unsupported provider value tag `{other}`")),
    }
}

fn write_len(len: usize, out: &mut Vec<u8>) -> Result<(), String> {
    write_u64(
        u64::try_from(len).map_err(|_| format!("length `{len}` does not fit in u64"))?,
        out,
    );
    Ok(())
}

fn write_bool(value: bool, out: &mut Vec<u8>) {
    out.push(u8::from(value));
}

fn encode_provider_descriptor_view_owner(
    owner: &ArcanaCabiProviderDescriptorViewOwner,
    out: &mut Vec<u8>,
) -> Result<(), String> {
    match owner {
        ArcanaCabiProviderDescriptorViewOwner::Runtime { package_id } => {
            out.push(1);
            write_string(package_id, out)?;
        }
        ArcanaCabiProviderDescriptorViewOwner::ProviderBinding {
            consumer_package_id,
            dependency_alias,
        } => {
            out.push(2);
            write_string(consumer_package_id, out)?;
            write_string(dependency_alias, out)?;
        }
    }
    Ok(())
}

fn decode_provider_descriptor_view_owner(
    cursor: &mut ProviderDecodeCursor<'_>,
) -> Result<ArcanaCabiProviderDescriptorViewOwner, String> {
    match cursor.read_u8("provider DescriptorView owner tag")? {
        1 => Ok(ArcanaCabiProviderDescriptorViewOwner::Runtime {
            package_id: cursor.read_string("provider DescriptorView owner package_id")?,
        }),
        2 => Ok(ArcanaCabiProviderDescriptorViewOwner::ProviderBinding {
            consumer_package_id: cursor
                .read_string("provider DescriptorView owner consumer_package_id")?,
            dependency_alias: cursor
                .read_string("provider DescriptorView owner dependency_alias")?,
        }),
        other => Err(format!(
            "unsupported provider DescriptorView owner tag `{other}`"
        )),
    }
}

fn encode_provider_descriptor_view_backing_kind(
    kind: &ArcanaCabiProviderDescriptorViewBackingKind,
    out: &mut Vec<u8>,
) {
    let tag = match kind {
        ArcanaCabiProviderDescriptorViewBackingKind::ReadElements => 1,
        ArcanaCabiProviderDescriptorViewBackingKind::ReadBytes => 2,
        ArcanaCabiProviderDescriptorViewBackingKind::ReadUtf8 => 3,
    };
    out.push(tag);
}

fn decode_provider_descriptor_view_backing_kind(
    cursor: &mut ProviderDecodeCursor<'_>,
) -> Result<ArcanaCabiProviderDescriptorViewBackingKind, String> {
    match cursor.read_u8("provider DescriptorView backing kind")? {
        1 => Ok(ArcanaCabiProviderDescriptorViewBackingKind::ReadElements),
        2 => Ok(ArcanaCabiProviderDescriptorViewBackingKind::ReadBytes),
        3 => Ok(ArcanaCabiProviderDescriptorViewBackingKind::ReadUtf8),
        other => Err(format!(
            "unsupported provider DescriptorView backing kind `{other}`"
        )),
    }
}

fn write_i64(value: i64, out: &mut Vec<u8>) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(value: u64, out: &mut Vec<u8>) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_optional_i64(value: Option<i64>, out: &mut Vec<u8>) {
    match value {
        Some(value) => {
            out.push(1);
            write_i64(value, out);
        }
        None => out.push(0),
    }
}

fn write_string(value: &str, out: &mut Vec<u8>) -> Result<(), String> {
    write_bytes(value.as_bytes(), out)
}

fn write_bytes(value: &[u8], out: &mut Vec<u8>) -> Result<(), String> {
    write_len(value.len(), out)?;
    out.extend_from_slice(value);
    Ok(())
}

struct ProviderDecodeCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ProviderDecodeCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_u8(&mut self, context: &str) -> Result<u8, String> {
        let value = *self
            .bytes
            .get(self.offset)
            .ok_or_else(|| format!("truncated {context}"))?;
        self.offset += 1;
        Ok(value)
    }

    fn read_u64(&mut self, context: &str) -> Result<u64, String> {
        let bytes = self.read_exact(8, context)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("u64 slice should fit"),
        ))
    }

    fn read_bool(&mut self, context: &str) -> Result<bool, String> {
        match self.read_u8(context)? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(format!("{context} must be 0 or 1, found `{other}`")),
        }
    }

    fn expect_exact_bytes(&mut self, expected: &[u8], context: &str) -> Result<(), String> {
        let bytes = self.read_exact(expected.len(), context)?;
        if bytes != expected {
            return Err(format!("{context} is invalid"));
        }
        Ok(())
    }

    fn read_i64(&mut self, context: &str) -> Result<i64, String> {
        let bytes = self.read_exact(8, context)?;
        Ok(i64::from_le_bytes(
            bytes.try_into().expect("i64 slice should fit"),
        ))
    }

    fn read_len(&mut self, context: &str) -> Result<usize, String> {
        usize::try_from(self.read_u64(context)?)
            .map_err(|_| format!("{context} does not fit in usize"))
    }

    fn read_optional_i64(&mut self, context: &str) -> Result<Option<i64>, String> {
        match self.read_u8(context)? {
            0 => Ok(None),
            1 => Ok(Some(self.read_i64(context)?)),
            other => Err(format!("invalid optional flag `{other}` for {context}")),
        }
    }

    fn read_string(&mut self, context: &str) -> Result<String, String> {
        let bytes = self.read_bytes(context)?;
        String::from_utf8(bytes).map_err(|e| format!("{context} is not valid utf8: {e}"))
    }

    fn read_bytes(&mut self, context: &str) -> Result<Vec<u8>, String> {
        let len = self.read_len(context)?;
        Ok(self.read_exact(len, context)?.to_vec())
    }

    fn read_exact(&mut self, len: usize, context: &str) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| format!("{context} length overflowed"))?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| format!("truncated {context}"))?;
        self.offset = end;
        Ok(bytes)
    }

    fn finish(&self) -> Result<(), String> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err("provider codec payload contains trailing bytes".to_string())
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArcanaBytesView {
    pub ptr: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArcanaStrView {
    pub ptr: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArcanaOwnedBytes {
    pub ptr: *mut u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArcanaOwnedStr {
    pub ptr: *mut u8,
    pub len: usize,
}

pub type ArcanaCabiLastErrorAllocFn = unsafe extern "system" fn(out_len: *mut usize) -> *mut u8;
pub type ArcanaCabiOwnedBytesFreeFn = unsafe extern "system" fn(ptr: *mut u8, len: usize);
pub type ArcanaCabiOwnedStrFreeFn = unsafe extern "system" fn(ptr: *mut u8, len: usize);
pub type ArcanaCabiChildRunEntrypointFn = unsafe extern "system" fn(
    instance: *mut c_void,
    package_image_ptr: *const u8,
    package_image_len: usize,
    main_routine_key: *const c_char,
    out_exit_code: *mut i32,
) -> i32;
pub type ArcanaCabiPluginDescribeInstanceFn =
    unsafe extern "system" fn(instance: *mut c_void, out_len: *mut usize) -> *mut u8;
pub type ArcanaCabiPluginUseInstanceFn = unsafe extern "system" fn(
    instance: *mut c_void,
    request_ptr: *const u8,
    request_len: usize,
    out_len: *mut usize,
) -> *mut u8;
pub type ArcanaCabiProviderDescribeFn =
    unsafe extern "system" fn(instance: *mut c_void, out_len: *mut usize) -> *mut u8;
pub type ArcanaCabiProviderInvokeFn = unsafe extern "system" fn(
    instance: *mut c_void,
    host_ops: *const ArcanaCabiProviderHostOpsV1,
    host_context: *mut c_void,
    callable_key: *const c_char,
    args_ptr: *const u8,
    args_len: usize,
    out_len: *mut usize,
) -> *mut u8;
pub type ArcanaCabiProviderRetainOpaqueFn = unsafe extern "system" fn(
    instance: *mut c_void,
    family_key: *const c_char,
    opaque_id: u64,
) -> i32;
pub type ArcanaCabiProviderReleaseOpaqueFn = unsafe extern "system" fn(
    instance: *mut c_void,
    family_key: *const c_char,
    opaque_id: u64,
) -> i32;
pub type ArcanaCabiProviderHostResolvePackageAssetRootFn =
    unsafe extern "system" fn(
        host_context: *mut c_void,
        package_id: *const c_char,
    ) -> ArcanaOwnedStr;
pub type ArcanaCabiProviderHostOwnedStrFreeFn = unsafe extern "system" fn(ptr: *mut u8, len: usize);
pub type ArcanaCabiProviderHostReadDescriptorValuesFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    family: *const c_char,
    view_id: u64,
    start: u64,
    len: u64,
    out_len: *mut usize,
) -> *mut u8;
pub type ArcanaCabiProviderHostReadDescriptorBytesFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    family: *const c_char,
    view_id: u64,
    start: u64,
    len: u64,
    out_len: *mut usize,
) -> *mut u8;
pub type ArcanaCabiProviderHostCanvasImageCreateFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    width: i64,
    height: i64,
    out_image_id: *mut u64,
) -> i32;
pub type ArcanaCabiProviderHostCanvasImageReplaceRgbaFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    image_id: u64,
    rgba_ptr: *const u8,
    rgba_len: usize,
) -> i32;
pub type ArcanaCabiProviderHostCanvasBlitFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    window_id: u64,
    image_id: u64,
    x: i64,
    y: i64,
) -> i32;
pub type ArcanaCabiProviderHostLastErrorAllocFn =
    unsafe extern "system" fn(host_context: *mut c_void, out_len: *mut usize) -> *mut u8;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiProductApiV1 {
    pub descriptor_size: usize,
    pub package_name: *const c_char,
    pub product_name: *const c_char,
    pub role: *const c_char,
    pub contract_id: *const c_char,
    pub contract_version: u32,
    pub role_ops: *const c_void,
    pub reserved0: *const c_void,
    pub reserved1: *const c_void,
}
unsafe impl Sync for ArcanaCabiProductApiV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiExportParamV1 {
    pub name: *const c_char,
    pub source_mode: *const c_char,
    pub pass_mode: *const c_char,
    pub input_type: *const c_char,
    pub write_back_type: *const c_char,
}
unsafe impl Sync for ArcanaCabiExportParamV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiExportEntryV1 {
    pub export_name: *const c_char,
    pub routine_key: *const c_char,
    pub symbol_name: *const c_char,
    pub return_type: *const c_char,
    pub params: *const ArcanaCabiExportParamV1,
    pub param_count: usize,
}
unsafe impl Sync for ArcanaCabiExportEntryV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiExportOpsV1 {
    pub ops_size: usize,
    pub exports: *const ArcanaCabiExportEntryV1,
    pub export_count: usize,
    pub last_error_alloc: ArcanaCabiLastErrorAllocFn,
    pub owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
    pub owned_str_free: ArcanaCabiOwnedStrFreeFn,
    pub reserved0: *const c_void,
    pub reserved1: *const c_void,
}
unsafe impl Sync for ArcanaCabiExportOpsV1 {}

pub type ArcanaCabiCreateInstanceFn = unsafe extern "system" fn() -> *mut c_void;
pub type ArcanaCabiDestroyInstanceFn = unsafe extern "system" fn(instance: *mut c_void);

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiInstanceOpsV1 {
    pub ops_size: usize,
    pub create_instance: ArcanaCabiCreateInstanceFn,
    pub destroy_instance: ArcanaCabiDestroyInstanceFn,
    pub reserved0: *const c_void,
    pub reserved1: *const c_void,
}
unsafe impl Sync for ArcanaCabiInstanceOpsV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiChildOpsV1 {
    pub base: ArcanaCabiInstanceOpsV1,
    pub run_entrypoint: ArcanaCabiChildRunEntrypointFn,
    pub last_error_alloc: ArcanaCabiLastErrorAllocFn,
    pub owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
    pub reserved0: *const c_void,
    pub reserved1: *const c_void,
}
unsafe impl Sync for ArcanaCabiChildOpsV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiPluginOpsV1 {
    pub base: ArcanaCabiInstanceOpsV1,
    pub describe_instance: ArcanaCabiPluginDescribeInstanceFn,
    pub use_instance: ArcanaCabiPluginUseInstanceFn,
    pub last_error_alloc: ArcanaCabiLastErrorAllocFn,
    pub owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
    pub reserved0: *const c_void,
    pub reserved1: *const c_void,
}
unsafe impl Sync for ArcanaCabiPluginOpsV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiProviderHostOpsV1 {
    pub ops_size: usize,
    pub resolve_package_asset_root: ArcanaCabiProviderHostResolvePackageAssetRootFn,
    pub host_owned_str_free: ArcanaCabiProviderHostOwnedStrFreeFn,
    pub read_descriptor_values: ArcanaCabiProviderHostReadDescriptorValuesFn,
    pub read_descriptor_bytes: ArcanaCabiProviderHostReadDescriptorBytesFn,
    pub canvas_image_create: ArcanaCabiProviderHostCanvasImageCreateFn,
    pub canvas_image_replace_rgba: ArcanaCabiProviderHostCanvasImageReplaceRgbaFn,
    pub canvas_blit: ArcanaCabiProviderHostCanvasBlitFn,
    pub last_error_alloc: ArcanaCabiProviderHostLastErrorAllocFn,
    pub reserved0: *const c_void,
    pub reserved1: *const c_void,
}
unsafe impl Sync for ArcanaCabiProviderHostOpsV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiProviderOpsV1 {
    pub base: ArcanaCabiInstanceOpsV1,
    pub describe: ArcanaCabiProviderDescribeFn,
    pub invoke_callable: ArcanaCabiProviderInvokeFn,
    pub retain_opaque: ArcanaCabiProviderRetainOpaqueFn,
    pub release_opaque: ArcanaCabiProviderReleaseOpaqueFn,
    pub last_error_alloc: ArcanaCabiLastErrorAllocFn,
    pub owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
    pub reserved0: *const c_void,
    pub reserved1: *const c_void,
}
unsafe impl Sync for ArcanaCabiProviderOpsV1 {}

pub fn render_c_value_type_defs() -> String {
    concat!(
        "typedef struct ArcanaBytesView {\n",
        "    const uint8_t* ptr;\n",
        "    size_t len;\n",
        "} ArcanaBytesView;\n\n",
        "typedef struct ArcanaStrView {\n",
        "    const uint8_t* ptr;\n",
        "    size_t len;\n",
        "} ArcanaStrView;\n\n",
        "typedef struct ArcanaOwnedBytes {\n",
        "    uint8_t* ptr;\n",
        "    size_t len;\n",
        "} ArcanaOwnedBytes;\n\n",
        "typedef struct ArcanaOwnedStr {\n",
        "    uint8_t* ptr;\n",
        "    size_t len;\n",
        "} ArcanaOwnedStr;\n\n",
    )
    .to_string()
}

pub fn render_c_descriptor_type_defs() -> String {
    concat!(
        "typedef struct ArcanaCabiProductApiV1 {\n",
        "    size_t descriptor_size;\n",
        "    const char* package_name;\n",
        "    const char* product_name;\n",
        "    const char* role;\n",
        "    const char* contract_id;\n",
        "    uint32_t contract_version;\n",
        "    const void* role_ops;\n",
        "    const void* reserved0;\n",
        "    const void* reserved1;\n",
        "} ArcanaCabiProductApiV1;\n\n",
        "typedef struct ArcanaCabiExportParamV1 {\n",
        "    const char* name;\n",
        "    const char* source_mode;\n",
        "    const char* pass_mode;\n",
        "    const char* input_type;\n",
        "    const char* write_back_type;\n",
        "} ArcanaCabiExportParamV1;\n\n",
        "typedef struct ArcanaCabiExportEntryV1 {\n",
        "    const char* export_name;\n",
        "    const char* routine_key;\n",
        "    const char* symbol_name;\n",
        "    const char* return_type;\n",
        "    const ArcanaCabiExportParamV1* params;\n",
        "    size_t param_count;\n",
        "} ArcanaCabiExportEntryV1;\n\n",
        "typedef struct ArcanaCabiExportOpsV1 {\n",
        "    size_t ops_size;\n",
        "    const ArcanaCabiExportEntryV1* exports;\n",
        "    size_t export_count;\n",
        "    uint8_t* (*last_error_alloc)(size_t* out_len);\n",
        "    void (*owned_bytes_free)(uint8_t* ptr, size_t len);\n",
        "    void (*owned_str_free)(uint8_t* ptr, size_t len);\n",
        "    const void* reserved0;\n",
        "    const void* reserved1;\n",
        "} ArcanaCabiExportOpsV1;\n\n",
        "typedef struct ArcanaCabiInstanceOpsV1 {\n",
        "    size_t ops_size;\n",
        "    void* (*create_instance)(void);\n",
        "    void (*destroy_instance)(void* instance);\n",
        "    const void* reserved0;\n",
        "    const void* reserved1;\n",
        "} ArcanaCabiInstanceOpsV1;\n\n",
        "typedef struct ArcanaCabiChildOpsV1 {\n",
        "    ArcanaCabiInstanceOpsV1 base;\n",
        "    int32_t (*run_entrypoint)(void* instance, const uint8_t* package_image_ptr, size_t package_image_len, const char* main_routine_key, int32_t* out_exit_code);\n",
        "    uint8_t* (*last_error_alloc)(size_t* out_len);\n",
        "    void (*owned_bytes_free)(uint8_t* ptr, size_t len);\n",
        "    const void* reserved0;\n",
        "    const void* reserved1;\n",
        "} ArcanaCabiChildOpsV1;\n\n",
        "typedef struct ArcanaCabiPluginOpsV1 {\n",
        "    ArcanaCabiInstanceOpsV1 base;\n",
        "    uint8_t* (*describe_instance)(void* instance, size_t* out_len);\n",
        "    uint8_t* (*use_instance)(void* instance, const uint8_t* request_ptr, size_t request_len, size_t* out_len);\n",
        "    uint8_t* (*last_error_alloc)(size_t* out_len);\n",
        "    void (*owned_bytes_free)(uint8_t* ptr, size_t len);\n",
        "    const void* reserved0;\n",
        "    const void* reserved1;\n",
        "} ArcanaCabiPluginOpsV1;\n\n",
        "typedef struct ArcanaCabiProviderHostOpsV1 {\n",
        "    size_t ops_size;\n",
        "    ArcanaOwnedStr (*resolve_package_asset_root)(void* host_context, const char* package_id);\n",
        "    void (*host_owned_str_free)(uint8_t* ptr, size_t len);\n",
        "    uint8_t* (*read_descriptor_values)(void* host_context, const char* family, uint64_t view_id, uint64_t start, uint64_t len, size_t* out_len);\n",
        "    uint8_t* (*read_descriptor_bytes)(void* host_context, const char* family, uint64_t view_id, uint64_t start, uint64_t len, size_t* out_len);\n",
        "    int32_t (*canvas_image_create)(void* host_context, int64_t width, int64_t height, uint64_t* out_image_id);\n",
        "    int32_t (*canvas_image_replace_rgba)(void* host_context, uint64_t image_id, const uint8_t* rgba_ptr, size_t rgba_len);\n",
        "    int32_t (*canvas_blit)(void* host_context, uint64_t window_id, uint64_t image_id, int64_t x, int64_t y);\n",
        "    uint8_t* (*last_error_alloc)(void* host_context, size_t* out_len);\n",
        "    const void* reserved0;\n",
        "    const void* reserved1;\n",
        "} ArcanaCabiProviderHostOpsV1;\n\n",
        "typedef struct ArcanaCabiProviderOpsV1 {\n",
        "    ArcanaCabiInstanceOpsV1 base;\n",
        "    uint8_t* (*describe)(void* instance, size_t* out_len);\n",
        "    uint8_t* (*invoke_callable)(void* instance, const ArcanaCabiProviderHostOpsV1* host_ops, void* host_context, const char* callable_key, const uint8_t* args_ptr, size_t args_len, size_t* out_len);\n",
        "    int32_t (*retain_opaque)(void* instance, const char* family_key, uint64_t opaque_id);\n",
        "    int32_t (*release_opaque)(void* instance, const char* family_key, uint64_t opaque_id);\n",
        "    uint8_t* (*last_error_alloc)(size_t* out_len);\n",
        "    void (*owned_bytes_free)(uint8_t* ptr, size_t len);\n",
        "    const void* reserved0;\n",
        "    const void* reserved1;\n",
        "} ArcanaCabiProviderOpsV1;\n\n",
    )
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        ArcanaCabiProviderCallOutcome, ArcanaCabiProviderDescriptorView,
        ArcanaCabiProviderDescriptorViewBackingKind, ArcanaCabiProviderDescriptorViewOwner,
        ArcanaCabiProviderValue, ArcanaCabiProviderWriteBack, decode_provider_call_outcome,
        decode_provider_value, decode_provider_values, encode_provider_call_outcome,
        encode_provider_value, encode_provider_values, render_c_descriptor_type_defs,
        render_c_value_type_defs,
    };

    #[test]
    fn render_c_value_type_defs_includes_owned_and_view_buffers() {
        let text = render_c_value_type_defs();
        assert!(text.contains("typedef struct ArcanaBytesView"));
        assert!(text.contains("typedef struct ArcanaOwnedStr"));
    }

    #[test]
    fn render_c_descriptor_type_defs_includes_export_and_instance_ops() {
        let text = render_c_descriptor_type_defs();
        assert!(text.contains("typedef struct ArcanaCabiProductApiV1"));
        assert!(text.contains("typedef struct ArcanaCabiExportOpsV1"));
        assert!(text.contains("typedef struct ArcanaCabiInstanceOpsV1"));
        assert!(text.contains("typedef struct ArcanaCabiChildOpsV1"));
        assert!(text.contains("typedef struct ArcanaCabiPluginOpsV1"));
        assert!(text.contains("typedef struct ArcanaCabiProviderOpsV1"));
        assert!(text.contains("use_instance"));
        assert!(text.contains("owned_str_free"));
    }

    #[test]
    fn provider_value_codec_roundtrips_nested_shapes_and_opaques() {
        let value = ArcanaCabiProviderValue::Record {
            name: "arcana_text.types.TextBox".to_string(),
            fields: vec![
                ("x".to_string(), ArcanaCabiProviderValue::Int(10)),
                ("rtl".to_string(), ArcanaCabiProviderValue::Bool(true)),
                (
                    "bytes".to_string(),
                    ArcanaCabiProviderValue::DescriptorView(ArcanaCabiProviderDescriptorView {
                        owner: ArcanaCabiProviderDescriptorViewOwner::Runtime {
                            package_id: "path:app".to_string(),
                        },
                        backing_kind: ArcanaCabiProviderDescriptorViewBackingKind::ReadBytes,
                        family: "std.memory.ByteView".to_string(),
                        id: 5,
                        element_type: "Int".to_string(),
                        element_layout: "Int".to_string(),
                        start: 0,
                        len: 4,
                        mutable: false,
                    }),
                ),
                (
                    "family".to_string(),
                    ArcanaCabiProviderValue::ProviderOpaque {
                        family: "arcana_text.types.Paragraph".to_string(),
                        id: 7,
                    },
                ),
                (
                    "window".to_string(),
                    ArcanaCabiProviderValue::SubstrateOpaque {
                        family: "std.window.Window".to_string(),
                        id: 9,
                    },
                ),
            ],
        };
        let bytes = encode_provider_value(&value).expect("value should encode");
        let decoded = decode_provider_value(&bytes).expect("value should decode");
        assert_eq!(decoded, value);
    }

    #[test]
    fn provider_values_codec_roundtrips_multiple_arguments() {
        let values = vec![
            ArcanaCabiProviderValue::Int(1),
            ArcanaCabiProviderValue::Str("two".to_string()),
            ArcanaCabiProviderValue::List(vec![
                ArcanaCabiProviderValue::Int(3),
                ArcanaCabiProviderValue::Unit,
            ]),
        ];
        let bytes = encode_provider_values(&values).expect("values should encode");
        let decoded = decode_provider_values(&bytes).expect("values should decode");
        assert_eq!(decoded, values);
    }

    #[test]
    fn provider_call_outcome_codec_roundtrips_write_backs() {
        let outcome = ArcanaCabiProviderCallOutcome {
            result: ArcanaCabiProviderValue::Unit,
            write_backs: vec![ArcanaCabiProviderWriteBack {
                index: 0,
                name: "paragraph".to_string(),
                value: ArcanaCabiProviderValue::ProviderOpaque {
                    family: "arcana_text.types.Paragraph".to_string(),
                    id: 11,
                },
            }],
        };
        let bytes = encode_provider_call_outcome(&outcome).expect("outcome should encode");
        let decoded = decode_provider_call_outcome(&bytes).expect("outcome should decode");
        assert_eq!(decoded, outcome);
    }
}
