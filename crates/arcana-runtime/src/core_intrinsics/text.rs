use super::*;
use crate::runtime_intrinsics::TextIntrinsic as RuntimeIntrinsic;

#[allow(unused_variables)]
pub(super) fn execute(
    intrinsic: RuntimeIntrinsic,
    type_args: &[String],
    final_args: &mut Vec<RuntimeValue>,
    plan: &RuntimePackagePlan,
    mut scopes: Option<&mut Vec<RuntimeScope>>,
    current_package_id: Option<&str>,
    current_module_id: Option<&str>,
    aliases: Option<&BTreeMap<String, Vec<String>>>,
    type_bindings: Option<&RuntimeTypeBindings>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    let args = final_args.clone();
    match intrinsic {
        RuntimeIntrinsic::TextLenBytes => {
            let text = expect_str(expect_single_arg(args, "text_len_bytes")?, "text_len_bytes")?;
            Ok(RuntimeValue::Int(runtime_text_len_bytes(&text)?))
        }
        RuntimeIntrinsic::TextByteAt => {
            if args.len() != 2 {
                return Err("text_byte_at expects two arguments".to_string());
            }
            let text = expect_str(args[0].clone(), "text_byte_at")?;
            let index = expect_int(args[1].clone(), "text_byte_at")?;
            if index < 0 {
                return Err("text_byte_at index must be non-negative".to_string());
            }
            Ok(RuntimeValue::Int(runtime_text_byte_at(
                &text,
                index as usize,
            )?))
        }
        RuntimeIntrinsic::TextSliceBytes => {
            if args.len() != 3 {
                return Err("text_slice_bytes expects three arguments".to_string());
            }
            let text = expect_str(args[0].clone(), "text_slice_bytes")?;
            let start = expect_int(args[1].clone(), "text_slice_bytes")?;
            let end = expect_int(args[2].clone(), "text_slice_bytes")?;
            if start < 0 || end < 0 {
                return Err("text_slice_bytes bounds must be non-negative".to_string());
            }
            if end < start {
                return Err(
                    "text_slice_bytes end must be greater than or equal to start".to_string(),
                );
            }
            Ok(RuntimeValue::Str(runtime_text_slice_bytes(
                &text,
                start as usize,
                end as usize,
            )?))
        }
        RuntimeIntrinsic::TextStartsWith => {
            if args.len() != 2 {
                return Err("text_starts_with expects two arguments".to_string());
            }
            Ok(RuntimeValue::Bool(runtime_text_starts_with(
                &expect_str(args[0].clone(), "text_starts_with")?,
                &expect_str(args[1].clone(), "text_starts_with")?,
            )))
        }
        RuntimeIntrinsic::TextEndsWith => {
            if args.len() != 2 {
                return Err("text_ends_with expects two arguments".to_string());
            }
            Ok(RuntimeValue::Bool(runtime_text_ends_with(
                &expect_str(args[0].clone(), "text_ends_with")?,
                &expect_str(args[1].clone(), "text_ends_with")?,
            )))
        }
        RuntimeIntrinsic::TextFind => {
            if args.len() != 3 {
                return Err("text_find expects three arguments".to_string());
            }
            let text = expect_str(args[0].clone(), "text_find")?;
            let start = expect_int(args[1].clone(), "text_find")?;
            let needle = expect_str(args[2].clone(), "text_find")?;
            Ok(RuntimeValue::Int(runtime_text_find(&text, start, &needle)?))
        }
        RuntimeIntrinsic::TextContains => {
            if args.len() != 2 {
                return Err("text_contains expects two arguments".to_string());
            }
            let text = expect_str(args[0].clone(), "text_contains")?;
            let needle = expect_str(args[1].clone(), "text_contains")?;
            Ok(RuntimeValue::Bool(
                runtime_text_find(&text, 0, &needle)? >= 0,
            ))
        }
        RuntimeIntrinsic::TextTrimStart => {
            let text = expect_str(
                expect_single_arg(args, "text_trim_start")?,
                "text_trim_start",
            )?;
            Ok(RuntimeValue::Str(runtime_text_trim_start(&text)?))
        }
        RuntimeIntrinsic::TextTrimEnd => {
            let text = expect_str(expect_single_arg(args, "text_trim_end")?, "text_trim_end")?;
            Ok(RuntimeValue::Str(runtime_text_trim_end(&text)?))
        }
        RuntimeIntrinsic::TextTrim => {
            let text = expect_str(expect_single_arg(args, "text_trim")?, "text_trim")?;
            Ok(RuntimeValue::Str(runtime_text_trim(&text)?))
        }
        RuntimeIntrinsic::TextSplit => {
            if args.len() != 2 {
                return Err("text_split expects two arguments".to_string());
            }
            let text = expect_str(args[0].clone(), "text_split")?;
            let delim = expect_str(args[1].clone(), "text_split")?;
            Ok(RuntimeValue::List(
                runtime_text_split(&text, &delim)?
                    .into_iter()
                    .map(RuntimeValue::Str)
                    .collect(),
            ))
        }
        RuntimeIntrinsic::TextJoin => {
            if args.len() != 2 {
                return Err("text_join expects two arguments".to_string());
            }
            let parts = expect_string_list(args[0].clone(), "text_join")?;
            let delim = expect_str(args[1].clone(), "text_join")?;
            Ok(RuntimeValue::Str(runtime_text_join(parts, &delim)))
        }
        RuntimeIntrinsic::TextRepeat => {
            if args.len() != 2 {
                return Err("text_repeat expects two arguments".to_string());
            }
            let text = expect_str(args[0].clone(), "text_repeat")?;
            let count = expect_int(args[1].clone(), "text_repeat")?;
            Ok(RuntimeValue::Str(runtime_text_repeat(&text, count)))
        }
        RuntimeIntrinsic::TextSplitLines => {
            let text = expect_str(
                expect_single_arg(args, "text_split_lines")?,
                "text_split_lines",
            )?;
            Ok(RuntimeValue::List(
                runtime_text_split_lines(&text)
                    .into_iter()
                    .map(RuntimeValue::Str)
                    .collect(),
            ))
        }
        RuntimeIntrinsic::TextFromInt => {
            let value = expect_int(expect_single_arg(args, "text_from_int")?, "text_from_int")?;
            Ok(RuntimeValue::Str(runtime_text_from_int(value)))
        }
        RuntimeIntrinsic::TextToIntTry => {
            let text = expect_str(expect_single_arg(args, "text_to_int")?, "text_to_int")?;
            Ok(match runtime_text_to_int(&text)? {
                Ok(value) => ok_variant(RuntimeValue::Int(value)),
                Err(message) => err_variant(message),
            })
        }
        RuntimeIntrinsic::BytesFromStrUtf8 => {
            let text = expect_str(
                expect_single_arg(args, "bytes_from_str_utf8")?,
                "bytes_from_str_utf8",
            )?;
            Ok(bytes_to_runtime_value(runtime_bytes_from_str_utf8(&text)))
        }
        RuntimeIntrinsic::BytesToStrUtf8 => {
            let bytes = expect_byte_array(
                expect_single_arg(args, "bytes_to_str_utf8")?,
                "bytes_to_str_utf8",
            )?;
            Ok(RuntimeValue::Str(runtime_bytes_to_str_utf8(&bytes)))
        }
        RuntimeIntrinsic::BytesLen => {
            let bytes = expect_byte_array(expect_single_arg(args, "bytes_len")?, "bytes_len")?;
            Ok(RuntimeValue::Int(i64::try_from(bytes.len()).map_err(
                |_| "bytes length does not fit in i64".to_string(),
            )?))
        }
        RuntimeIntrinsic::BytesAt => {
            if args.len() != 2 {
                return Err("bytes_at expects two arguments".to_string());
            }
            let bytes = expect_byte_array(args[0].clone(), "bytes_at")?;
            let index = expect_int(args[1].clone(), "bytes_at")?;
            if index < 0 {
                return Err("bytes_at index must be non-negative".to_string());
            }
            Ok(RuntimeValue::Int(i64::from(
                *bytes
                    .get(index as usize)
                    .ok_or_else(|| format!("bytes_at index `{index}` is out of bounds"))?,
            )))
        }
        RuntimeIntrinsic::BytesSlice => {
            if args.len() != 3 {
                return Err("bytes_slice expects three arguments".to_string());
            }
            let bytes = expect_byte_array(args[0].clone(), "bytes_slice")?;
            let start = expect_int(args[1].clone(), "bytes_slice")?;
            let end = expect_int(args[2].clone(), "bytes_slice")?;
            if start < 0 || end < 0 {
                return Err("bytes_slice bounds must be non-negative".to_string());
            }
            if end < start {
                return Err("bytes_slice end must be greater than or equal to start".to_string());
            }
            let slice = bytes
                .get(start as usize..end as usize)
                .ok_or_else(|| format!("bytes_slice `{start}..{end}` is out of bounds"))?;
            Ok(bytes_to_runtime_value(slice.iter().copied()))
        }
        RuntimeIntrinsic::BytesSha256Hex => {
            let bytes = expect_byte_array(
                expect_single_arg(args, "bytes_sha256_hex")?,
                "bytes_sha256_hex",
            )?;
            Ok(RuntimeValue::Str(runtime_bytes_sha256_hex(&bytes)))
        }
        RuntimeIntrinsic::BytesThaw => {
            let bytes = expect_byte_array(expect_single_arg(args, "bytes_thaw")?, "bytes_thaw")?;
            Ok(RuntimeValue::ByteBuffer(bytes))
        }
        RuntimeIntrinsic::ByteBufferNew => {
            if !args.is_empty() {
                return Err("byte_buffer_new expects zero arguments".to_string());
            }
            Ok(RuntimeValue::ByteBuffer(Vec::new()))
        }
        RuntimeIntrinsic::ByteBufferLen => {
            let bytes = if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes
                    .as_deref_mut()
                    .ok_or_else(|| "byte_buffer_len on refs requires runtime scopes".to_string())?;
                let current_package_id = current_package_id.ok_or_else(|| {
                    "byte_buffer_len on refs requires package context".to_string()
                })?;
                let current_module_id = current_module_id
                    .ok_or_else(|| "byte_buffer_len on refs requires module context".to_string())?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                expect_byte_array(
                    read_runtime_reference(
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        &reference,
                        host,
                    )?,
                    "byte_buffer_len",
                )?
            } else {
                expect_byte_array(
                    expect_single_arg(args, "byte_buffer_len")?,
                    "byte_buffer_len",
                )?
            };
            Ok(RuntimeValue::Int(i64::try_from(bytes.len()).map_err(
                |_| "byte buffer length does not fit in i64".to_string(),
            )?))
        }
        RuntimeIntrinsic::ByteBufferAt => {
            if args.len() != 2 {
                return Err("byte_buffer_at expects two arguments".to_string());
            }
            let bytes = if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes
                    .as_deref_mut()
                    .ok_or_else(|| "byte_buffer_at on refs requires runtime scopes".to_string())?;
                let current_package_id = current_package_id
                    .ok_or_else(|| "byte_buffer_at on refs requires package context".to_string())?;
                let current_module_id = current_module_id
                    .ok_or_else(|| "byte_buffer_at on refs requires module context".to_string())?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                expect_byte_array(
                    read_runtime_reference(
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        &reference,
                        host,
                    )?,
                    "byte_buffer_at",
                )?
            } else {
                expect_byte_array(args[0].clone(), "byte_buffer_at")?
            };
            let index = expect_int(args[1].clone(), "byte_buffer_at")?;
            if index < 0 {
                return Err("byte_buffer_at index must be non-negative".to_string());
            }
            Ok(RuntimeValue::Int(i64::from(
                *bytes
                    .get(index as usize)
                    .ok_or_else(|| format!("byte_buffer_at index `{index}` is out of bounds"))?,
            )))
        }
        RuntimeIntrinsic::ByteBufferSet => {
            if args.len() != 3 {
                return Err("byte_buffer_set expects three arguments".to_string());
            }
            let byte = expect_int(args[2].clone(), "byte_buffer_set value")?;
            if !(0..=255).contains(&byte) {
                return Err(format!(
                    "byte_buffer_set value `{byte}` is out of range `0..=255`"
                ));
            }
            let index = expect_int(args[1].clone(), "byte_buffer_set index")?;
            if index < 0 {
                return Err("byte_buffer_set index must be non-negative".to_string());
            }
            let index = index as usize;
            if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes
                    .as_deref_mut()
                    .ok_or_else(|| "byte_buffer_set on refs requires runtime scopes".to_string())?;
                let current_package_id = current_package_id.ok_or_else(|| {
                    "byte_buffer_set on refs requires package context".to_string()
                })?;
                let current_module_id = current_module_id
                    .ok_or_else(|| "byte_buffer_set on refs requires module context".to_string())?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                let mut values = expect_byte_array(
                    read_runtime_reference(
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        &reference,
                        host,
                    )?,
                    "byte_buffer_set",
                )?;
                let slot = values
                    .get_mut(index)
                    .ok_or_else(|| format!("byte_buffer_set index `{index}` is out of bounds"))?;
                *slot = byte as u8;
                write_runtime_reference(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &reference,
                    RuntimeValue::ByteBuffer(values),
                    host,
                )?;
                return Ok(RuntimeValue::Unit);
            }
            let Some(RuntimeValue::ByteBuffer(values)) = final_args.get_mut(0) else {
                return Err("byte_buffer_set expects ByteBuffer".to_string());
            };
            let slot = values
                .get_mut(index)
                .ok_or_else(|| format!("byte_buffer_set index `{index}` is out of bounds"))?;
            *slot = byte as u8;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::ByteBufferPush => {
            if args.len() != 2 {
                return Err("byte_buffer_push expects two arguments".to_string());
            }
            let byte = expect_int(args[1].clone(), "byte_buffer_push value")?;
            if !(0..=255).contains(&byte) {
                return Err(format!(
                    "byte_buffer_push value `{byte}` is out of range `0..=255`"
                ));
            }
            if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes.as_deref_mut().ok_or_else(|| {
                    "byte_buffer_push on refs requires runtime scopes".to_string()
                })?;
                let current_package_id = current_package_id.ok_or_else(|| {
                    "byte_buffer_push on refs requires package context".to_string()
                })?;
                let current_module_id = current_module_id.ok_or_else(|| {
                    "byte_buffer_push on refs requires module context".to_string()
                })?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                let mut values = expect_byte_array(
                    read_runtime_reference(
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        &reference,
                        host,
                    )?,
                    "byte_buffer_push",
                )?;
                values.push(byte as u8);
                write_runtime_reference(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &reference,
                    RuntimeValue::ByteBuffer(values),
                    host,
                )?;
                return Ok(RuntimeValue::Unit);
            }
            let Some(RuntimeValue::ByteBuffer(values)) = final_args.get_mut(0) else {
                return Err("byte_buffer_push expects ByteBuffer".to_string());
            };
            values.push(byte as u8);
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::ByteBufferFreeze => {
            let bytes = if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes.as_deref_mut().ok_or_else(|| {
                    "byte_buffer_freeze on refs requires runtime scopes".to_string()
                })?;
                let current_package_id = current_package_id.ok_or_else(|| {
                    "byte_buffer_freeze on refs requires package context".to_string()
                })?;
                let current_module_id = current_module_id.ok_or_else(|| {
                    "byte_buffer_freeze on refs requires module context".to_string()
                })?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                expect_byte_array(
                    read_runtime_reference(
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        &reference,
                        host,
                    )?,
                    "byte_buffer_freeze",
                )?
            } else {
                expect_byte_array(
                    expect_single_arg(args, "byte_buffer_freeze")?,
                    "byte_buffer_freeze",
                )?
            };
            Ok(RuntimeValue::Bytes(bytes))
        }
        RuntimeIntrinsic::Utf16FromStr => {
            let text = expect_str(expect_single_arg(args, "utf16_from_str")?, "utf16_from_str")?;
            Ok(RuntimeValue::Utf16(runtime_utf16_from_str(&text)))
        }
        RuntimeIntrinsic::Utf16ToStr => {
            let units =
                expect_utf16_units(expect_single_arg(args, "utf16_to_str")?, "utf16_to_str")?;
            Ok(match runtime_utf16_to_str(&units) {
                Ok(text) => ok_variant(RuntimeValue::Str(text)),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::Utf16Len => {
            let units = expect_utf16_units(expect_single_arg(args, "utf16_len")?, "utf16_len")?;
            Ok(RuntimeValue::Int(i64::try_from(units.len()).map_err(
                |_| "utf16 length does not fit in i64".to_string(),
            )?))
        }
        RuntimeIntrinsic::Utf16At => {
            if args.len() != 2 {
                return Err("utf16_at expects two arguments".to_string());
            }
            let units = expect_utf16_units(args[0].clone(), "utf16_at")?;
            let index = expect_int(args[1].clone(), "utf16_at")?;
            if index < 0 {
                return Err("utf16_at index must be non-negative".to_string());
            }
            Ok(RuntimeValue::Int(i64::from(
                *units
                    .get(index as usize)
                    .ok_or_else(|| format!("utf16_at index `{index}` is out of bounds"))?,
            )))
        }
        RuntimeIntrinsic::Utf16Slice => {
            if args.len() != 3 {
                return Err("utf16_slice expects three arguments".to_string());
            }
            let units = expect_utf16_units(args[0].clone(), "utf16_slice")?;
            let start = expect_int(args[1].clone(), "utf16_slice")?;
            let end = expect_int(args[2].clone(), "utf16_slice")?;
            if start < 0 || end < 0 {
                return Err("utf16_slice bounds must be non-negative".to_string());
            }
            if end < start {
                return Err("utf16_slice end must be greater than or equal to start".to_string());
            }
            let slice = units
                .get(start as usize..end as usize)
                .ok_or_else(|| format!("utf16_slice `{start}..{end}` is out of bounds"))?;
            Ok(RuntimeValue::Utf16(slice.to_vec()))
        }
        RuntimeIntrinsic::Utf16Thaw => {
            let units = expect_utf16_units(expect_single_arg(args, "utf16_thaw")?, "utf16_thaw")?;
            Ok(RuntimeValue::Utf16Buffer(units))
        }
        RuntimeIntrinsic::Utf16BufferNew => {
            if !args.is_empty() {
                return Err("utf16_buffer_new expects zero arguments".to_string());
            }
            Ok(RuntimeValue::Utf16Buffer(Vec::new()))
        }
        RuntimeIntrinsic::Utf16BufferLen => {
            let units = if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes.as_deref_mut().ok_or_else(|| {
                    "utf16_buffer_len on refs requires runtime scopes".to_string()
                })?;
                let current_package_id = current_package_id.ok_or_else(|| {
                    "utf16_buffer_len on refs requires package context".to_string()
                })?;
                let current_module_id = current_module_id.ok_or_else(|| {
                    "utf16_buffer_len on refs requires module context".to_string()
                })?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                expect_utf16_units(
                    read_runtime_reference(
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        &reference,
                        host,
                    )?,
                    "utf16_buffer_len",
                )?
            } else {
                expect_utf16_units(
                    expect_single_arg(args, "utf16_buffer_len")?,
                    "utf16_buffer_len",
                )?
            };
            Ok(RuntimeValue::Int(i64::try_from(units.len()).map_err(
                |_| "utf16 buffer length does not fit in i64".to_string(),
            )?))
        }
        RuntimeIntrinsic::Utf16BufferAt => {
            if args.len() != 2 {
                return Err("utf16_buffer_at expects two arguments".to_string());
            }
            let units = if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes
                    .as_deref_mut()
                    .ok_or_else(|| "utf16_buffer_at on refs requires runtime scopes".to_string())?;
                let current_package_id = current_package_id.ok_or_else(|| {
                    "utf16_buffer_at on refs requires package context".to_string()
                })?;
                let current_module_id = current_module_id
                    .ok_or_else(|| "utf16_buffer_at on refs requires module context".to_string())?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                expect_utf16_units(
                    read_runtime_reference(
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        &reference,
                        host,
                    )?,
                    "utf16_buffer_at",
                )?
            } else {
                expect_utf16_units(args[0].clone(), "utf16_buffer_at")?
            };
            let index = expect_int(args[1].clone(), "utf16_buffer_at")?;
            if index < 0 {
                return Err("utf16_buffer_at index must be non-negative".to_string());
            }
            Ok(RuntimeValue::Int(i64::from(
                *units
                    .get(index as usize)
                    .ok_or_else(|| format!("utf16_buffer_at index `{index}` is out of bounds"))?,
            )))
        }
        RuntimeIntrinsic::Utf16BufferSet => {
            if args.len() != 3 {
                return Err("utf16_buffer_set expects three arguments".to_string());
            }
            let unit = expect_int(args[2].clone(), "utf16_buffer_set value")?;
            let unit = u16::try_from(unit).map_err(|_| {
                format!("utf16_buffer_set value `{unit}` is out of range `0..=65535`")
            })?;
            let index = expect_int(args[1].clone(), "utf16_buffer_set index")?;
            if index < 0 {
                return Err("utf16_buffer_set index must be non-negative".to_string());
            }
            let index = index as usize;
            if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes.as_deref_mut().ok_or_else(|| {
                    "utf16_buffer_set on refs requires runtime scopes".to_string()
                })?;
                let current_package_id = current_package_id.ok_or_else(|| {
                    "utf16_buffer_set on refs requires package context".to_string()
                })?;
                let current_module_id = current_module_id.ok_or_else(|| {
                    "utf16_buffer_set on refs requires module context".to_string()
                })?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                let mut values = expect_utf16_units(
                    read_runtime_reference(
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        &reference,
                        host,
                    )?,
                    "utf16_buffer_set",
                )?;
                let slot = values
                    .get_mut(index)
                    .ok_or_else(|| format!("utf16_buffer_set index `{index}` is out of bounds"))?;
                *slot = unit;
                write_runtime_reference(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &reference,
                    RuntimeValue::Utf16Buffer(values),
                    host,
                )?;
                return Ok(RuntimeValue::Unit);
            }
            let Some(RuntimeValue::Utf16Buffer(values)) = final_args.get_mut(0) else {
                return Err("utf16_buffer_set expects Utf16Buffer".to_string());
            };
            let slot = values
                .get_mut(index)
                .ok_or_else(|| format!("utf16_buffer_set index `{index}` is out of bounds"))?;
            *slot = unit;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::Utf16BufferPush => {
            if args.len() != 2 {
                return Err("utf16_buffer_push expects two arguments".to_string());
            }
            let unit = expect_int(args[1].clone(), "utf16_buffer_push value")?;
            let unit = u16::try_from(unit).map_err(|_| {
                format!("utf16_buffer_push value `{unit}` is out of range `0..=65535`")
            })?;
            if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes.as_deref_mut().ok_or_else(|| {
                    "utf16_buffer_push on refs requires runtime scopes".to_string()
                })?;
                let current_package_id = current_package_id.ok_or_else(|| {
                    "utf16_buffer_push on refs requires package context".to_string()
                })?;
                let current_module_id = current_module_id.ok_or_else(|| {
                    "utf16_buffer_push on refs requires module context".to_string()
                })?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                let mut values = expect_utf16_units(
                    read_runtime_reference(
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        &reference,
                        host,
                    )?,
                    "utf16_buffer_push",
                )?;
                values.push(unit);
                write_runtime_reference(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &reference,
                    RuntimeValue::Utf16Buffer(values),
                    host,
                )?;
                return Ok(RuntimeValue::Unit);
            }
            let Some(RuntimeValue::Utf16Buffer(values)) = final_args.get_mut(0) else {
                return Err("utf16_buffer_push expects Utf16Buffer".to_string());
            };
            values.push(unit);
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::Utf16BufferFreeze => {
            let units = if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes.as_mut().ok_or_else(|| {
                    "utf16_buffer_freeze on refs requires runtime scopes".to_string()
                })?;
                let current_package_id = current_package_id.ok_or_else(|| {
                    "utf16_buffer_freeze on refs requires package context".to_string()
                })?;
                let current_module_id = current_module_id.ok_or_else(|| {
                    "utf16_buffer_freeze on refs requires module context".to_string()
                })?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                expect_utf16_units(
                    read_runtime_reference(
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        &reference,
                        host,
                    )?,
                    "utf16_buffer_freeze",
                )?
            } else {
                expect_utf16_units(
                    expect_single_arg(args, "utf16_buffer_freeze")?,
                    "utf16_buffer_freeze",
                )?
            };
            Ok(RuntimeValue::Utf16(units))
        }
    }
}
