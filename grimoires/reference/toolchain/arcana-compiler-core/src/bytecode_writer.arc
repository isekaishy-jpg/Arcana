import std.bytes
import std.collections.array
import std.text
use std.collections.list as list
import std.fs
import arcana_compiler_core.core
import arcana_compiler_core.types
import arcana_compiler_core.bytecode_writer_typed
import fs_support

fn empty_bytes() -> Array[Int]:
    return std.bytes.buf_to_array :: (std.bytes.new_buf :: :: call) :: call

fn trim_line(text: Str) -> Str:
    let n = std.text.len_bytes :: text :: call
    if n > 0 and (std.text.byte_at :: text, n - 1 :: call) == 13:
        let trimmed = arcana_compiler_core.core.trim_ws :: (std.text.slice_bytes :: text, 0, n - 1 :: call) :: call
        return bytecode_writer.strip_bom :: trimmed :: call
    return bytecode_writer.strip_bom :: (arcana_compiler_core.core.trim_ws :: text :: call) :: call

fn strip_line_end(text: Str) -> Str:
    let n = std.text.len_bytes :: text :: call
    if n > 0 and (std.text.byte_at :: text, n - 1 :: call) == 13:
        return bytecode_writer.strip_bom :: (std.text.slice_bytes :: text, 0, n - 1 :: call) :: call
    return bytecode_writer.strip_bom :: text :: call

fn strip_bom(text: Str) -> Str:
    if (std.text.len_bytes :: text :: call) >= 3:
        if (std.text.byte_at :: text, 0 :: call) == 239 and (std.text.byte_at :: text, 1 :: call) == 187 and (std.text.byte_at :: text, 2 :: call) == 191:
            return std.text.slice_bytes :: text, 3, (std.text.len_bytes :: text :: call) :: call
    return text

fn parse_int_or(text: Str, fallback: Int) -> Int:
    let s = arcana_compiler_core.core.trim_ws :: text :: call
    let n = std.text.len_bytes :: s :: call
    if n == 0:
        return fallback
    let mut sign = 1
    let mut i = 0
    if (std.text.byte_at :: s, 0 :: call) == 45:
        sign = -1
        i = 1
    let mut value = 0
    while i < n:
        let b = std.text.byte_at :: s, i :: call
        if not (std.text.is_digit_byte :: b :: call):
            return fallback
        value = value * 10 + (b - 48)
        i += 1
    return value * sign

fn body_after_prefix(line: Str, prefix: Str) -> Str:
    let n = std.text.len_bytes :: prefix :: call
    return std.text.slice_bytes :: line, n, (std.text.len_bytes :: line :: call) :: call

fn spec_kind(spec_text: Str) -> Str:
    let mut lines_scan = list.new[Str] :: :: call
    let mut lines_rev = std.text.split_lines :: spec_text :: call
    while (lines_rev :: :: len) > 0:
        lines_scan :: (lines_rev :: :: pop) :: push
    while (lines_scan :: :: len) > 0:
        let raw = lines_scan :: :: pop
        let line = bytecode_writer.strip_line_end :: raw :: call
        let trimmed = arcana_compiler_core.core.trim_ws :: line :: call
        if (std.text.len_bytes :: trimmed :: call) <= 0:
            continue
        if std.text.starts_with :: trimmed, "#" :: call:
            continue
        if std.text.starts_with :: trimmed, "kind=" :: call:
            return trimmed
        break
    return "kind=module"

export fn detect_spec_kind(spec_text: Str) -> Str:
    return bytecode_writer.spec_kind :: spec_text :: call

fn clone_text(text: Str) -> Str:
    let n = std.text.len_bytes :: text :: call
    return std.text.slice_bytes :: text, 0, n :: call

fn unescape_text(text: Str) -> Str:
    let mut buf = std.bytes.new_buf :: :: call
    let n = std.text.len_bytes :: text :: call
    let mut i = 0
    while i < n:
        let b = std.text.byte_at :: text, i :: call
        if b == 92 and (i + 1) < n:
            let next = std.text.byte_at :: text, i + 1 :: call
            if next == 110:
                let _ = std.bytes.buf_push :: buf, 10 :: call
                i += 2
                continue
            if next == 114:
                let _ = std.bytes.buf_push :: buf, 13 :: call
                i += 2
                continue
            if next == 116:
                let _ = std.bytes.buf_push :: buf, 9 :: call
                i += 2
                continue
            let _ = std.bytes.buf_push :: buf, next :: call
            i += 2
            continue
        let _ = std.bytes.buf_push :: buf, b :: call
        i += 1
    return std.bytes.to_str_utf8 :: (std.bytes.buf_to_array :: buf :: call) :: call

fn split_escaped(text: Str, delim: Int) -> List[Str]:
    let mut out = list.new[Str] :: :: call
    let n = std.text.len_bytes :: text :: call
    let mut start = 0
    let mut i = 0
    let mut escaped = false
    while i < n:
        let b = std.text.byte_at :: text, i :: call
        if escaped:
            escaped = false
            i += 1
            continue
        if b == 92:
            escaped = true
            i += 1
            continue
        if b == delim:
            let raw = std.text.slice_bytes :: text, start, i :: call
            out :: (bytecode_writer.unescape_text :: raw :: call) :: push
            start = i + 1
        i += 1
    let raw = std.text.slice_bytes :: text, start, n :: call
    out :: (bytecode_writer.unescape_text :: raw :: call) :: push
    return out

export fn append_line(text: Str, line: Str) -> Str:
    if (std.text.len_bytes :: text :: call) <= 0:
        return line
    return text + "\n" + line

fn ordered_rows(read rows: List[Str]) -> List[Str]:
    let mut scan = rows
    let mut restore = list.new[Str] :: :: call
    let mut rev = list.new[Str] :: :: call
    while (scan :: :: len) > 0:
        let item = scan :: :: pop
        restore :: (bytecode_writer.clone_text :: item :: call) :: push
        rev :: item :: push
    while (restore :: :: len) > 0:
        scan :: (restore :: :: pop) :: push
    return rev

fn ordered_ints(read xs: List[Int]) -> List[Int]:
    let mut scan = xs
    let mut restore = list.new[Int] :: :: call
    let mut rev = list.new[Int] :: :: call
    while (scan :: :: len) > 0:
        let item = scan :: :: pop
        restore :: item :: push
        rev :: item :: push
    while (restore :: :: len) > 0:
        scan :: (restore :: :: pop) :: push
    return rev

fn pipe_field_at(text: Str, want: Int, fallback: Str) -> Str:
    let mut parts = bytecode_writer.ordered_rows :: (bytecode_writer.split_escaped :: text, 124 :: call) :: call
    let mut idx = 0
    while (parts :: :: len) > 0:
        let item = parts :: :: pop
        if idx == want:
            return arcana_compiler_core.core.trim_ws :: item :: call
        idx += 1
    return fallback

fn fields_after(text: Str, needle: Int, skip: Int) -> List[Str]:
    let mut out = list.new[Str] :: :: call
    let mut parts = bytecode_writer.ordered_rows :: (bytecode_writer.split_escaped :: text, needle :: call) :: call
    let mut idx = 0
    while (parts :: :: len) > 0:
        let item = arcana_compiler_core.core.trim_ws :: (parts :: :: pop) :: call
        if idx >= skip and (std.text.len_bytes :: item :: call) > 0:
            out :: item :: push
        idx += 1
    return out

fn csv_strs(text: Str) -> List[Str]:
    let mut out = list.new[Str] :: :: call
    let mut parts = bytecode_writer.ordered_rows :: (bytecode_writer.split_escaped :: text, 44 :: call) :: call
    while (parts :: :: len) > 0:
        let item = arcana_compiler_core.core.trim_ws :: (parts :: :: pop) :: call
        if (std.text.len_bytes :: item :: call) > 0:
            out :: item :: push
    return out

fn csv_ints(text: Str) -> List[Int]:
    let mut out = list.new[Int] :: :: call
    let mut parts = bytecode_writer.ordered_rows :: (bytecode_writer.split_escaped :: text, 44 :: call) :: call
    while (parts :: :: len) > 0:
        let item = arcana_compiler_core.core.trim_ws :: (parts :: :: pop) :: call
        if (std.text.len_bytes :: item :: call) > 0:
            out :: (bytecode_writer.parse_int_or :: item, 0 :: call) :: push
    return out

fn push_u8(edit out: List[Int], value: Int):
    out :: (value & 255) :: push

fn push_u16(edit out: List[Int], value: Int):
    bytecode_writer.push_u8 :: out, value :: call
    bytecode_writer.push_u8 :: out, (value shr 8) :: call

fn push_u32(edit out: List[Int], value: Int):
    bytecode_writer.push_u8 :: out, value :: call
    bytecode_writer.push_u8 :: out, (value shr 8) :: call
    bytecode_writer.push_u8 :: out, (value shr 16) :: call
    bytecode_writer.push_u8 :: out, (value shr 24) :: call

fn push_i64(edit out: List[Int], value: Int):
    let mut shift = 0
    while shift < 64:
        bytecode_writer.push_u8 :: out, (value shr shift) :: call
        shift += 8

fn push_string(edit out: List[Int], value: Str):
    let bytes = std.bytes.from_str_utf8 :: value :: call
    let n = std.bytes.len :: bytes :: call
    bytecode_writer.push_u32 :: out, n :: call
    let mut i = 0
    while i < n:
        out :: (std.bytes.at :: bytes, i :: call) :: push
        i += 1

fn split_code_blob(blob: Str) -> List[Str]:
    let mut out = list.new[Str] :: :: call
    let n = std.text.len_bytes :: blob :: call
    let mut start = 0
    let mut i = 0
    while i <= n:
        if i == n or (std.text.byte_at :: blob, i :: call) == 10:
            let part = std.text.slice_bytes :: blob, start, i :: call
            let line = bytecode_writer.trim_line :: part :: call
            if (std.text.len_bytes :: line :: call) > 0:
                out :: line :: push
            start = i + 1
        i += 1
    return out

fn encode_int_list_u16(edit out: List[Int], read xs: List[Int]):
    bytecode_writer.push_u16 :: out, (xs :: :: len) :: call
    let mut scan = bytecode_writer.ordered_ints :: xs :: call
    while (scan :: :: len) > 0:
        let value = scan :: :: pop
        bytecode_writer.push_u16 :: out, value :: call

fn encode_int_list_u32(edit out: List[Int], read xs: List[Int]):
    bytecode_writer.push_u16 :: out, (xs :: :: len) :: call
    let mut scan = bytecode_writer.ordered_ints :: xs :: call
    while (scan :: :: len) > 0:
        let value = scan :: :: pop
        bytecode_writer.push_u32 :: out, value :: call

fn encode_str_list_u16(edit out: List[Int], read xs: List[Str]):
    bytecode_writer.push_u16 :: out, (xs :: :: len) :: call
    let mut scan = bytecode_writer.ordered_rows :: xs :: call
    while (scan :: :: len) > 0:
        let value = scan :: :: pop
        bytecode_writer.push_string :: out, value :: call

fn encode_opcode_row(edit out: List[Int], row: Str) -> Bool:
    return (std.text.len_bytes :: (bytecode_writer.encode_opcode_row_error :: out, row :: call) :: call) <= 0

fn encode_opcode_row_error(edit out: List[Int], row: Str) -> Str:
    let tag_text = bytecode_writer.pipe_field_at :: row, 0, "-1" :: call
    let a_text = bytecode_writer.pipe_field_at :: row, 1, "0" :: call
    let b_text = bytecode_writer.pipe_field_at :: row, 2, "0" :: call
    let tag = bytecode_writer.parse_int_or :: tag_text, -1 :: call
    let a = bytecode_writer.parse_int_or :: a_text, 0 :: call
    let b = bytecode_writer.parse_int_or :: b_text, 0 :: call
    if tag < 0:
        return "invalid opcode row `" + row + "`"
    bytecode_writer.push_u8 :: out, tag :: call
    if tag == 0:
        bytecode_writer.push_i64 :: out, a :: call
        return ""
    if tag == 1 or tag == 67:
        bytecode_writer.push_u8 :: out, a :: call
        return ""
    if tag == 2 or tag == 15 or tag == 16:
        bytecode_writer.push_u32 :: out, a :: call
        return ""
    if tag == 3 or tag == 21 or tag == 4 or tag == 5 or tag == 6 or tag == 7 or tag == 125 or tag == 66 or tag == 79 or tag == 17 or tag == 29 or tag == 30 or tag == 34:
        bytecode_writer.push_u16 :: out, a :: call
        return ""
    if tag == 123 or tag == 131:
        bytecode_writer.push_u16 :: out, a :: call
        bytecode_writer.push_u16 :: out, b :: call
        return ""
    if tag == 124 or tag == 78 or tag == 80 or tag == 126 or tag == 127 or tag == 128 or tag == 129 or tag == 130:
        return ""
    if tag == 68 or tag == 8 or tag == 65 or tag == 9 or tag == 10 or tag == 11 or tag == 56 or tag == 57 or tag == 58 or tag == 59 or tag == 60 or tag == 61 or tag == 62 or tag == 63 or tag == 64:
        return ""
    if tag == 12 or tag == 13 or tag == 14 or tag == 19 or tag == 20 or tag == 31:
        return ""
    if tag == 69 or tag == 70 or tag == 71 or tag == 72 or tag == 73 or tag == 74 or tag == 75:
        return ""
    if tag == 81 or tag == 82 or tag == 83 or tag == 84 or tag == 85 or tag == 86:
        return ""
    if tag == 87 or tag == 88 or tag == 89 or tag == 90 or tag == 91 or tag == 92:
        return ""
    if tag == 76 or tag == 77:
        return ""
    return "unsupported opcode row `" + row + "`"

fn encode_instr_error(edit out: List[Int], read instr: arcana_compiler_core.types.BytecodeInstr) -> Str:
    let tag = instr.tag
    let a = instr.a
    let b = instr.b
    if tag < 0:
        return "invalid opcode tag"
    bytecode_writer.push_u8 :: out, tag :: call
    if tag == 0:
        bytecode_writer.push_i64 :: out, a :: call
        return ""
    if tag == 1 or tag == 67:
        bytecode_writer.push_u8 :: out, a :: call
        return ""
    if tag == 2 or tag == 15 or tag == 16:
        bytecode_writer.push_u32 :: out, a :: call
        return ""
    if tag == 3 or tag == 21 or tag == 4 or tag == 5 or tag == 6 or tag == 7 or tag == 125 or tag == 66 or tag == 79 or tag == 17 or tag == 29 or tag == 30 or tag == 34:
        bytecode_writer.push_u16 :: out, a :: call
        return ""
    if tag == 123 or tag == 131:
        bytecode_writer.push_u16 :: out, a :: call
        bytecode_writer.push_u16 :: out, b :: call
        return ""
    if tag == 124 or tag == 78 or tag == 80 or tag == 126 or tag == 127 or tag == 128 or tag == 129 or tag == 130:
        return ""
    if tag == 68 or tag == 8 or tag == 65 or tag == 9 or tag == 10 or tag == 11 or tag == 56 or tag == 57 or tag == 58 or tag == 59 or tag == 60 or tag == 61 or tag == 62 or tag == 63 or tag == 64:
        return ""
    if tag == 12 or tag == 13 or tag == 14 or tag == 19 or tag == 20 or tag == 31:
        return ""
    if tag == 69 or tag == 70 or tag == 71 or tag == 72 or tag == 73 or tag == 74 or tag == 75:
        return ""
    if tag == 81 or tag == 82 or tag == 83 or tag == 84 or tag == 85 or tag == 86:
        return ""
    if tag == 87 or tag == 88 or tag == 89 or tag == 90 or tag == 91 or tag == 92:
        return ""
    if tag == 76 or tag == 77:
        return ""
    return "unsupported opcode tag `" + (std.text.from_int :: tag :: call) + "`"

fn encode_module_typed_diagnostic(read module: arcana_compiler_core.types.BytecodeModule, edit out: List[Int]) -> Str:
    let version = module.head.version
    let strings = module.head.strings
    let records = module.head.records
    let functions = module.tail.functions
    let sigs = module.tail.function_sigs
    let behaviors = module.tail.behaviors
    if (functions :: :: len) != (sigs :: :: len):
        return "typed module function/signature count mismatch"

    bytecode_writer.push_u8 :: out, 65 :: call
    bytecode_writer.push_u8 :: out, 82 :: call
    bytecode_writer.push_u8 :: out, 67 :: call
    bytecode_writer.push_u8 :: out, 66 :: call
    bytecode_writer.push_u16 :: out, version :: call

    bytecode_writer.push_u32 :: out, (strings :: :: len) :: call
    let mut i = 0
    while i < (strings :: :: len):
        let item = strings[i]
        bytecode_writer.push_string :: out, item :: call
        i += 1

    bytecode_writer.push_u32 :: out, (records :: :: len) :: call
    i = 0
    while i < (records :: :: len):
        let rec = records[i]
        bytecode_writer.push_string :: out, rec.name :: call
        bytecode_writer.push_u32 :: out, (rec.fields :: :: len) :: call
        let mut fi = 0
        while fi < (rec.fields :: :: len):
            let field = rec.fields[fi]
            bytecode_writer.push_string :: out, field :: call
            fi += 1
        i += 1

    bytecode_writer.push_u32 :: out, (functions :: :: len) :: call
    i = 0
    while i < (functions :: :: len):
        let fun = functions[i]
        let modes = fun.meta.1.1
        let code = fun.tail.1
        bytecode_writer.push_string :: out, fun.name :: call
        if fun.meta.0:
            bytecode_writer.push_u8 :: out, 1 :: call
        else:
            bytecode_writer.push_u8 :: out, 0 :: call
        bytecode_writer.push_u16 :: out, fun.meta.1.0 :: call
        bytecode_writer.push_u16 :: out, (modes :: :: len) :: call
        let mut mi = 0
        while mi < (modes :: :: len):
            let mode = modes[mi]
            bytecode_writer.push_u8 :: out, mode :: call
            mi += 1
        bytecode_writer.push_u16 :: out, fun.tail.0 :: call
        bytecode_writer.push_u32 :: out, (code :: :: len) :: call
        let mut ci = 0
        while ci < (code :: :: len):
            let instr = code[ci]
            let code_error = bytecode_writer.encode_instr_error :: out, instr :: call
            if (std.text.len_bytes :: code_error :: call) > 0:
                return "typed module opcode encode failed"
            ci += 1
        i += 1

    bytecode_writer.push_u32 :: out, (sigs :: :: len) :: call
    i = 0
    while i < (sigs :: :: len):
        let sig = sigs[i]
        bytecode_writer.push_u16 :: out, (sig.params :: :: len) :: call
        let mut pi = 0
        while pi < (sig.params :: :: len):
            let param = sig.params[pi]
            bytecode_writer.push_u8 :: out, param :: call
            pi += 1
        bytecode_writer.push_u8 :: out, sig.ret :: call
        i += 1

    bytecode_writer.push_u32 :: out, (behaviors :: :: len) :: call
    i = 0
    while i < (behaviors :: :: len):
        let item = behaviors[i]
        let component_types = item.tail.0
        let access = item.tail.1.1
        bytecode_writer.push_string :: out, item.name :: call
        bytecode_writer.push_u8 :: out, item.meta.0 :: call
        bytecode_writer.push_u8 :: out, item.meta.1.0 :: call
        bytecode_writer.push_u16 :: out, item.meta.1.1 :: call
        bytecode_writer.push_u16 :: out, (component_types :: :: len) :: call
        let mut cti = 0
        while cti < (component_types :: :: len):
            let component = component_types[cti]
            bytecode_writer.push_string :: out, component :: call
            cti += 1
        if item.tail.1.0.meta.0:
            bytecode_writer.push_u8 :: out, 1 :: call
        else:
            bytecode_writer.push_u8 :: out, 0 :: call
        bytecode_writer.push_u16 :: out, item.tail.1.0.meta.1 :: call
        bytecode_writer.push_u8 :: out, item.tail.1.0.tags.0 :: call
        bytecode_writer.push_u8 :: out, item.tail.1.0.tags.1.0 :: call
        bytecode_writer.push_u8 :: out, item.tail.1.0.tags.1.1 :: call
        if item.tail.1.0.flags.0:
            bytecode_writer.push_u8 :: out, 1 :: call
        else:
            bytecode_writer.push_u8 :: out, 0 :: call
        if item.tail.1.0.flags.1:
            bytecode_writer.push_u8 :: out, 1 :: call
        else:
            bytecode_writer.push_u8 :: out, 0 :: call
        bytecode_writer.push_u16 :: out, (access.reads :: :: len) :: call
        let mut ri = 0
        while ri < (access.reads :: :: len):
            let read_id = access.reads[ri]
            bytecode_writer.push_u32 :: out, read_id :: call
            ri += 1
        bytecode_writer.push_u16 :: out, (access.writes :: :: len) :: call
        let mut wi = 0
        while wi < (access.writes :: :: len):
            let write_id = access.writes[wi]
            bytecode_writer.push_u32 :: out, write_id :: call
            wi += 1
        bytecode_writer.push_u16 :: out, (access.excludes :: :: len) :: call
        let mut ei = 0
        while ei < (access.excludes :: :: len):
            let exclude_id = access.excludes[ei]
            bytecode_writer.push_u32 :: out, exclude_id :: call
            ei += 1
        i += 1
    return ""

fn encode_lib_typed_diagnostic(read artifact: arcana_compiler_core.types.BytecodeLibArtifact, edit out: List[Int]) -> Str:
    let mut module_payload = std.bytes.new_buf :: :: call
    let module_error = bytecode_writer.encode_module_typed_diagnostic :: artifact.tail.1, module_payload :: call
    if (std.text.len_bytes :: module_error :: call) > 0:
        return "embedded module encode failed"

    bytecode_writer.push_u8 :: out, 65 :: call
    bytecode_writer.push_u8 :: out, 82 :: call
    bytecode_writer.push_u8 :: out, 67 :: call
    bytecode_writer.push_u8 :: out, 76 :: call
    bytecode_writer.push_u16 :: out, artifact.meta.format_version :: call
    bytecode_writer.push_u16 :: out, artifact.meta.bytecode_version :: call
    bytecode_writer.push_string :: out, artifact.meta.std_abi :: call

    let exports = artifact.exports
    let deps = artifact.tail.0
    bytecode_writer.push_u32 :: out, (exports :: :: len) :: call
    let mut i = 0
    while i < (exports :: :: len):
        let item = exports[i]
        let modes = item.meta.1.1
        let param_types = item.tail.0
        bytecode_writer.push_string :: out, item.name :: call
        bytecode_writer.push_u16 :: out, item.meta.0 :: call
        if item.meta.1.0:
            bytecode_writer.push_u8 :: out, 1 :: call
        else:
            bytecode_writer.push_u8 :: out, 0 :: call
        bytecode_writer.push_u16 :: out, (modes :: :: len) :: call
        let mut mi = 0
        while mi < (modes :: :: len):
            let mode = modes[mi]
            bytecode_writer.push_u8 :: out, mode :: call
            mi += 1
        bytecode_writer.push_u16 :: out, (param_types :: :: len) :: call
        let mut pi = 0
        while pi < (param_types :: :: len):
            let param_type = param_types[pi]
            bytecode_writer.push_string :: out, param_type :: call
            pi += 1
        bytecode_writer.push_string :: out, item.tail.1 :: call
        i += 1

    bytecode_writer.push_u32 :: out, (deps :: :: len) :: call
    i = 0
    while i < (deps :: :: len):
        let dep = deps[i]
        bytecode_writer.push_string :: out, dep.dep :: call
        bytecode_writer.push_string :: out, dep.fingerprint :: call
        i += 1

    bytecode_writer.push_u32 :: out, (module_payload :: :: len) :: call
    i = 0
    while i < (module_payload :: :: len):
        let byte = module_payload[i]
        out :: byte :: push
        i += 1
    return ""

fn empty_record_type() -> arcana_compiler_core.types.BytecodeRecordType:
    let fields = std.collections.array.new[Str] :: 0, "" :: call
    return arcana_compiler_core.types.BytecodeRecordType :: name = "", fields = fields :: call

fn empty_function_sig() -> arcana_compiler_core.types.BytecodeFunctionSig:
    let params = std.collections.array.new[Int] :: 0, 0 :: call
    return arcana_compiler_core.types.BytecodeFunctionSig :: params = params, ret = 0 :: call

fn empty_instr_typed() -> arcana_compiler_core.types.BytecodeInstr:
    return arcana_compiler_core.types.BytecodeInstr :: tag = 20, a = 0, b = 0 :: call

fn empty_function_typed() -> arcana_compiler_core.types.BytecodeFunction:
    let modes = std.collections.array.new[Int] :: 0, 0 :: call
    let code = std.collections.array.new[arcana_compiler_core.types.BytecodeInstr] :: 0, empty_instr_typed :: :: call :: call
    return arcana_compiler_core.types.BytecodeFunction :: name = "", meta = (false, (0, modes)), tail = (0, code) :: call

fn empty_behavior_contract() -> arcana_compiler_core.types.BytecodeBehaviorContract:
    return arcana_compiler_core.types.BytecodeBehaviorContract :: meta = (false, 0), tags = (0, (0, 0)), flags = (false, false) :: call

fn empty_access_sets() -> arcana_compiler_core.types.BytecodeAccessSets:
    let reads = std.collections.array.new[Int] :: 0, 0 :: call
    let writes = std.collections.array.new[Int] :: 0, 0 :: call
    let excludes = std.collections.array.new[Int] :: 0, 0 :: call
    return arcana_compiler_core.types.BytecodeAccessSets :: reads = reads, writes = writes, excludes = excludes :: call

fn empty_behavior_typed() -> arcana_compiler_core.types.BytecodeBehavior:
    let component_types = std.collections.array.new[Str] :: 0, "" :: call
    let reads = std.collections.array.new[Int] :: 0, 0 :: call
    let writes = std.collections.array.new[Int] :: 0, 0 :: call
    let excludes = std.collections.array.new[Int] :: 0, 0 :: call
    let contract = arcana_compiler_core.types.BytecodeBehaviorContract :: meta = (false, 0), tags = (0, (0, 0)), flags = (false, false) :: call
    let access = arcana_compiler_core.types.BytecodeAccessSets :: reads = reads, writes = writes, excludes = excludes :: call
    return arcana_compiler_core.types.BytecodeBehavior :: name = "", meta = (0, (0, 0)), tail = (component_types, (contract, access)) :: call

fn empty_dep_fingerprint() -> arcana_compiler_core.types.BytecodeDepFingerprint:
    return arcana_compiler_core.types.BytecodeDepFingerprint :: dep = "", fingerprint = "" :: call

fn empty_lib_export() -> arcana_compiler_core.types.BytecodeLibExport:
    let modes = std.collections.array.new[Int] :: 0, 0 :: call
    let param_types = std.collections.array.new[Str] :: 0, "" :: call
    return arcana_compiler_core.types.BytecodeLibExport :: name = "", meta = (0, (false, modes)), tail = (param_types, "") :: call

fn empty_module_typed() -> arcana_compiler_core.types.BytecodeModule:
    let strings = std.collections.array.new[Str] :: 0, "" :: call
    let record_fields = std.collections.array.new[Str] :: 0, "" :: call
    let record_fill = arcana_compiler_core.types.BytecodeRecordType :: name = "", fields = record_fields :: call
    let sig_params = std.collections.array.new[Int] :: 0, 0 :: call
    let sig_fill = arcana_compiler_core.types.BytecodeFunctionSig :: params = sig_params, ret = 0 :: call
    let modes = std.collections.array.new[Int] :: 0, 0 :: call
    let code_fill = arcana_compiler_core.types.BytecodeInstr :: tag = 20, a = 0, b = 0 :: call
    let code = std.collections.array.new[arcana_compiler_core.types.BytecodeInstr] :: 0, code_fill :: call
    let function_fill = arcana_compiler_core.types.BytecodeFunction :: name = "", meta = (false, (0, modes)), tail = (0, code) :: call
    let component_types = std.collections.array.new[Str] :: 0, "" :: call
    let reads = std.collections.array.new[Int] :: 0, 0 :: call
    let writes = std.collections.array.new[Int] :: 0, 0 :: call
    let excludes = std.collections.array.new[Int] :: 0, 0 :: call
    let contract = arcana_compiler_core.types.BytecodeBehaviorContract :: meta = (false, 0), tags = (0, (0, 0)), flags = (false, false) :: call
    let access = arcana_compiler_core.types.BytecodeAccessSets :: reads = reads, writes = writes, excludes = excludes :: call
    let behavior_fill = arcana_compiler_core.types.BytecodeBehavior :: name = "", meta = (0, (0, 0)), tail = (component_types, (contract, access)) :: call
    let records = std.collections.array.new[arcana_compiler_core.types.BytecodeRecordType] :: 0, record_fill :: call
    let sigs = std.collections.array.new[arcana_compiler_core.types.BytecodeFunctionSig] :: 0, sig_fill :: call
    let functions = std.collections.array.new[arcana_compiler_core.types.BytecodeFunction] :: 0, function_fill :: call
    let behaviors = std.collections.array.new[arcana_compiler_core.types.BytecodeBehavior] :: 0, behavior_fill :: call
    let head = arcana_compiler_core.types.BytecodeModuleHead :: version = 29, strings = strings, records = records :: call
    let tail = arcana_compiler_core.types.BytecodeModuleTail :: function_sigs = sigs, functions = functions, behaviors = behaviors :: call
    return arcana_compiler_core.types.BytecodeModule :: head = head, tail = tail :: call

fn empty_lib_typed() -> arcana_compiler_core.types.BytecodeLibArtifact:
    let export_modes = std.collections.array.new[Int] :: 0, 0 :: call
    let export_param_types = std.collections.array.new[Str] :: 0, "" :: call
    let export_fill = arcana_compiler_core.types.BytecodeLibExport :: name = "", meta = (0, (false, export_modes)), tail = (export_param_types, "") :: call
    let dep_fill = arcana_compiler_core.types.BytecodeDepFingerprint :: dep = "", fingerprint = "" :: call
    let exports = std.collections.array.new[arcana_compiler_core.types.BytecodeLibExport] :: 0, export_fill :: call
    let deps = std.collections.array.new[arcana_compiler_core.types.BytecodeDepFingerprint] :: 0, dep_fill :: call
    let module = bytecode_writer.empty_module_typed :: :: call
    let meta = arcana_compiler_core.types.BytecodeLibMeta :: format_version = 1, bytecode_version = 29, std_abi = "std-abi-v1" :: call
    return arcana_compiler_core.types.BytecodeLibArtifact :: meta = meta, exports = exports, tail = (deps, module) :: call

fn ok_module_typed(value: arcana_compiler_core.types.BytecodeModule) -> arcana_compiler_core.types.Outcome[arcana_compiler_core.types.BytecodeModule]:
    return arcana_compiler_core.types.Outcome[arcana_compiler_core.types.BytecodeModule] :: ok = true, value = value, message = "" :: call

fn err_module_typed(message: Str) -> arcana_compiler_core.types.Outcome[arcana_compiler_core.types.BytecodeModule]:
    return arcana_compiler_core.types.Outcome[arcana_compiler_core.types.BytecodeModule] :: ok = false, value = bytecode_writer.empty_module_typed :: :: call, message = message :: call

fn ok_lib_typed(value: arcana_compiler_core.types.BytecodeLibArtifact) -> arcana_compiler_core.types.Outcome[arcana_compiler_core.types.BytecodeLibArtifact]:
    return arcana_compiler_core.types.Outcome[arcana_compiler_core.types.BytecodeLibArtifact] :: ok = true, value = value, message = "" :: call

fn err_lib_typed(message: Str) -> arcana_compiler_core.types.Outcome[arcana_compiler_core.types.BytecodeLibArtifact]:
    return arcana_compiler_core.types.Outcome[arcana_compiler_core.types.BytecodeLibArtifact] :: ok = false, value = bytecode_writer.empty_lib_typed :: :: call, message = message :: call

fn parse_instr_row_typed(row: Str) -> arcana_compiler_core.types.BytecodeInstr:
    let tag_text = bytecode_writer.pipe_field_at :: row, 0, "-1" :: call
    let a_text = bytecode_writer.pipe_field_at :: row, 1, "0" :: call
    let b_text = bytecode_writer.pipe_field_at :: row, 2, "0" :: call
    let tag = bytecode_writer.parse_int_or :: tag_text, -1 :: call
    let a = bytecode_writer.parse_int_or :: a_text, 0 :: call
    let b = bytecode_writer.parse_int_or :: b_text, 0 :: call
    return arcana_compiler_core.types.BytecodeInstr :: tag = tag, a = a, b = b :: call

fn parse_function_item_typed(read function_row: Str, read code_rows: List[Str]) -> arcana_compiler_core.types.BytecodeFunction:
    let name = bytecode_writer.pipe_field_at :: function_row, 0, "" :: call
    let is_async = (bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: function_row, 1, "0" :: call), 0 :: call) != 0
    let arity = bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: function_row, 2, "0" :: call), 0 :: call
    let modes = std.collections.array.from_list[Int] :: (bytecode_writer.csv_ints :: (bytecode_writer.pipe_field_at :: function_row, 3, "" :: call) :: call) :: call
    let locals = bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: function_row, 4, "0" :: call), 0 :: call
    let mut instr_items = list.new[arcana_compiler_core.types.BytecodeInstr] :: :: call
    let mut code_scan = bytecode_writer.ordered_rows :: code_rows :: call
    while (code_scan :: :: len) > 0:
        instr_items :: (bytecode_writer.parse_instr_row_typed :: (code_scan :: :: pop) :: call) :: push
    let code = std.collections.array.from_list[arcana_compiler_core.types.BytecodeInstr] :: instr_items :: call
    return arcana_compiler_core.types.BytecodeFunction :: name = name, meta = (is_async, (arity, modes)), tail = (locals, code) :: call

fn parse_sig_item_typed(function_row: Str) -> arcana_compiler_core.types.BytecodeFunctionSig:
    let sig_params = std.collections.array.from_list[Int] :: (bytecode_writer.csv_ints :: (bytecode_writer.pipe_field_at :: function_row, 5, "" :: call) :: call) :: call
    let sig_ret = bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: function_row, 6, "0" :: call), 0 :: call
    return arcana_compiler_core.types.BytecodeFunctionSig :: params = sig_params, ret = sig_ret :: call

fn parse_module_lines_typed(read raw_lines: List[Str], version_override: Int) -> arcana_compiler_core.types.Outcome[arcana_compiler_core.types.BytecodeModule]:
    let mut version = 29
    if version_override > 0:
        version = version_override
    let mut string_items = list.new[Str] :: :: call
    let mut record_items = list.new[arcana_compiler_core.types.BytecodeRecordType] :: :: call
    let mut function_items = list.new[arcana_compiler_core.types.BytecodeFunction] :: :: call
    let mut sig_items = list.new[arcana_compiler_core.types.BytecodeFunctionSig] :: :: call
    let mut behavior_items = list.new[arcana_compiler_core.types.BytecodeBehavior] :: :: call
    let mut current_function = ""
    let mut current_code_rows = list.new[Str] :: :: call
    let mut have_function = false
    let mut lines_scan = raw_lines
    while (lines_scan :: :: len) > 0:
        let raw = lines_scan :: :: pop
        let line = bytecode_writer.strip_line_end :: raw :: call
        let trimmed = arcana_compiler_core.core.trim_ws :: line :: call
        if (std.text.len_bytes :: trimmed :: call) <= 0:
            continue
        if std.text.starts_with :: trimmed, "#" :: call:
            continue
        if std.text.starts_with :: trimmed, "kind=" :: call:
            continue
        if version_override <= 0 and (std.text.starts_with :: trimmed, "version=" :: call):
            version = bytecode_writer.parse_int_or :: (bytecode_writer.body_after_prefix :: trimmed, "version=" :: call), 29 :: call
            continue
        if std.text.starts_with :: line, "string=" :: call:
            string_items :: (bytecode_writer.unescape_text :: (bytecode_writer.body_after_prefix :: line, "string=" :: call) :: call) :: push
            continue
        if std.text.starts_with :: trimmed, "record=" :: call:
            let row = bytecode_writer.body_after_prefix :: trimmed, "record=" :: call
            let name = bytecode_writer.pipe_field_at :: row, 0, "" :: call
            let fields = std.collections.array.from_list[Str] :: (bytecode_writer.fields_after :: row, 124, 1 :: call) :: call
            record_items :: (arcana_compiler_core.types.BytecodeRecordType :: name = name, fields = fields :: call) :: push
            continue
        if std.text.starts_with :: trimmed, "behavior=" :: call:
            let row = bytecode_writer.body_after_prefix :: trimmed, "behavior=" :: call
            let name = bytecode_writer.pipe_field_at :: row, 0, "" :: call
            let phase = bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: row, 1, "0" :: call), 0 :: call
            let affinity = bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: row, 2, "0" :: call), 0 :: call
            let fn_index = bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: row, 3, "0" :: call), 0 :: call
            let component_types = std.collections.array.from_list[Str] :: (bytecode_writer.csv_strs :: (bytecode_writer.pipe_field_at :: row, 4, "" :: call) :: call) :: call
            let contracted = (bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: row, 5, "0" :: call), 0 :: call) != 0
            let scheduler_group = bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: row, 6, "0" :: call), 0 :: call
            let contract_phase = bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: row, 7, "0" :: call), 0 :: call
            let contract_thread = bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: row, 8, "0" :: call), 0 :: call
            let contract_authority = bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: row, 9, "0" :: call), 0 :: call
            let deterministic = (bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: row, 10, "0" :: call), 0 :: call) != 0
            let rollback_safe = (bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: row, 11, "0" :: call), 0 :: call) != 0
            let reads = std.collections.array.from_list[Int] :: (bytecode_writer.csv_ints :: (bytecode_writer.pipe_field_at :: row, 12, "" :: call) :: call) :: call
            let writes = std.collections.array.from_list[Int] :: (bytecode_writer.csv_ints :: (bytecode_writer.pipe_field_at :: row, 13, "" :: call) :: call) :: call
            let excludes = std.collections.array.from_list[Int] :: (bytecode_writer.csv_ints :: (bytecode_writer.pipe_field_at :: row, 14, "" :: call) :: call) :: call
            let contract = arcana_compiler_core.types.BytecodeBehaviorContract :: meta = (contracted, scheduler_group), tags = (contract_phase, (contract_thread, contract_authority)), flags = (deterministic, rollback_safe) :: call
            let access = arcana_compiler_core.types.BytecodeAccessSets :: reads = reads, writes = writes, excludes = excludes :: call
            behavior_items :: (arcana_compiler_core.types.BytecodeBehavior :: name = name, meta = (phase, (affinity, fn_index)), tail = (component_types, (contract, access)) :: call) :: push
            continue
        if std.text.starts_with :: trimmed, "function=" :: call:
            if have_function:
                function_items :: (bytecode_writer.parse_function_item_typed :: current_function, current_code_rows :: call) :: push
                sig_items :: (bytecode_writer.parse_sig_item_typed :: current_function :: call) :: push
            current_function = bytecode_writer.body_after_prefix :: trimmed, "function=" :: call
            current_code_rows = list.new[Str] :: :: call
            have_function = true
            continue
        if trimmed == "endfn":
            if have_function:
                function_items :: (bytecode_writer.parse_function_item_typed :: current_function, current_code_rows :: call) :: push
                sig_items :: (bytecode_writer.parse_sig_item_typed :: current_function :: call) :: push
                current_function = ""
                current_code_rows = list.new[Str] :: :: call
                have_function = false
            continue
        if std.text.starts_with :: trimmed, "code=" :: call:
            if not have_function:
                return bytecode_writer.err_module_typed :: "encountered code row before function header" :: call
            let body = bytecode_writer.body_after_prefix :: trimmed, "code=" :: call
            current_code_rows :: body :: push
            continue
    if have_function:
        function_items :: (bytecode_writer.parse_function_item_typed :: current_function, current_code_rows :: call) :: push
        sig_items :: (bytecode_writer.parse_sig_item_typed :: current_function :: call) :: push

    let strings = std.collections.array.from_list[Str] :: string_items :: call
    let records = std.collections.array.from_list[arcana_compiler_core.types.BytecodeRecordType] :: record_items :: call
    let functions = std.collections.array.from_list[arcana_compiler_core.types.BytecodeFunction] :: function_items :: call
    let sigs = std.collections.array.from_list[arcana_compiler_core.types.BytecodeFunctionSig] :: sig_items :: call
    let behaviors = std.collections.array.from_list[arcana_compiler_core.types.BytecodeBehavior] :: behavior_items :: call

    let head = arcana_compiler_core.types.BytecodeModuleHead :: version = version, strings = strings, records = records :: call
    let tail = arcana_compiler_core.types.BytecodeModuleTail :: function_sigs = sigs, functions = functions, behaviors = behaviors :: call
    return bytecode_writer.ok_module_typed :: (arcana_compiler_core.types.BytecodeModule :: head = head, tail = tail :: call) :: call

fn parse_module_spec_typed(spec_text: Str) -> arcana_compiler_core.types.Outcome[arcana_compiler_core.types.BytecodeModule]:
    let mut lines_scan = list.new[Str] :: :: call
    let mut lines_rev = std.text.split_lines :: spec_text :: call
    while (lines_rev :: :: len) > 0:
        lines_scan :: (lines_rev :: :: pop) :: push
    return bytecode_writer.parse_module_lines_typed :: lines_scan, -1 :: call

fn parse_lib_spec_typed(spec_text: Str) -> arcana_compiler_core.types.Outcome[arcana_compiler_core.types.BytecodeLibArtifact]:
    let mut bytecode_version = 29
    let mut std_abi = "std-abi-v1"
    let mut export_rows = list.new[Str] :: :: call
    let mut dep_rows = list.new[Str] :: :: call
    let mut module_lines = list.new[Str] :: :: call
    let mut lines_scan = list.new[Str] :: :: call
    let mut lines_rev = std.text.split_lines :: spec_text :: call
    while (lines_rev :: :: len) > 0:
        lines_scan :: (lines_rev :: :: pop) :: push
    while (lines_scan :: :: len) > 0:
        let raw = lines_scan :: :: pop
        let line = bytecode_writer.strip_line_end :: raw :: call
        let trimmed = arcana_compiler_core.core.trim_ws :: line :: call
        if (std.text.len_bytes :: trimmed :: call) <= 0:
            continue
        if std.text.starts_with :: trimmed, "#" :: call:
            continue
        if std.text.starts_with :: trimmed, "kind=" :: call:
            continue
        if std.text.starts_with :: trimmed, "bytecode_version=" :: call:
            bytecode_version = bytecode_writer.parse_int_or :: (bytecode_writer.body_after_prefix :: trimmed, "bytecode_version=" :: call), 29 :: call
            continue
        if std.text.starts_with :: line, "std_abi=" :: call:
            std_abi = bytecode_writer.unescape_text :: (bytecode_writer.body_after_prefix :: line, "std_abi=" :: call) :: call
            continue
        if std.text.starts_with :: trimmed, "export=" :: call:
            export_rows :: (bytecode_writer.body_after_prefix :: trimmed, "export=" :: call) :: push
            continue
        if std.text.starts_with :: trimmed, "dep=" :: call:
            dep_rows :: (bytecode_writer.body_after_prefix :: trimmed, "dep=" :: call) :: push
            continue
        module_lines :: line :: push

    let mut module_lines_scan = list.new[Str] :: :: call
    let mut module_lines_rev = module_lines
    while (module_lines_rev :: :: len) > 0:
        module_lines_scan :: (module_lines_rev :: :: pop) :: push
    let module_outcome = bytecode_writer.parse_module_lines_typed :: module_lines_scan, bytecode_version :: call
    if not module_outcome.ok:
        return bytecode_writer.err_lib_typed :: "embedded module parse failed" :: call

    let mut export_items = list.new[arcana_compiler_core.types.BytecodeLibExport] :: :: call
    let mut export_scan = bytecode_writer.ordered_rows :: export_rows :: call
    while (export_scan :: :: len) > 0:
        let row = export_scan :: :: pop
        let name = bytecode_writer.pipe_field_at :: row, 0, "" :: call
        let arity = bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: row, 1, "0" :: call), 0 :: call
        let is_async = (bytecode_writer.parse_int_or :: (bytecode_writer.pipe_field_at :: row, 2, "0" :: call), 0 :: call) != 0
        let modes = std.collections.array.from_list[Int] :: (bytecode_writer.csv_ints :: (bytecode_writer.pipe_field_at :: row, 3, "" :: call) :: call) :: call
        let param_types = std.collections.array.from_list[Str] :: (bytecode_writer.csv_strs :: (bytecode_writer.pipe_field_at :: row, 4, "" :: call) :: call) :: call
        let ret_type = bytecode_writer.pipe_field_at :: row, 5, "" :: call
        export_items :: (arcana_compiler_core.types.BytecodeLibExport :: name = name, meta = (arity, (is_async, modes)), tail = (param_types, ret_type) :: call) :: push
    let exports = std.collections.array.from_list[arcana_compiler_core.types.BytecodeLibExport] :: export_items :: call

    let mut dep_items = list.new[arcana_compiler_core.types.BytecodeDepFingerprint] :: :: call
    let mut dep_scan = bytecode_writer.ordered_rows :: dep_rows :: call
    while (dep_scan :: :: len) > 0:
        let row = dep_scan :: :: pop
        let dep_name = bytecode_writer.pipe_field_at :: row, 0, "" :: call
        let dep_fp = bytecode_writer.pipe_field_at :: row, 1, "" :: call
        dep_items :: (arcana_compiler_core.types.BytecodeDepFingerprint :: dep = dep_name, fingerprint = dep_fp :: call) :: push
    let deps = std.collections.array.from_list[arcana_compiler_core.types.BytecodeDepFingerprint] :: dep_items :: call

    let meta = arcana_compiler_core.types.BytecodeLibMeta :: format_version = 1, bytecode_version = bytecode_version, std_abi = std_abi :: call
    return bytecode_writer.ok_lib_typed :: (arcana_compiler_core.types.BytecodeLibArtifact :: meta = meta, exports = exports, tail = (deps, module_outcome.value) :: call) :: call

fn encode_module_spec(spec_text: Str) -> Array[Int]:
    let mut out = std.bytes.new_buf :: :: call
    let error = bytecode_writer.encode_module_spec_diagnostic :: spec_text, out :: call
    if (std.text.len_bytes :: error :: call) > 0:
        return empty_bytes :: :: call
    return std.bytes.buf_to_array :: out :: call

fn encode_module_spec_diagnostic(spec_text: Str, edit out: List[Int]) -> Str:
    let mut version = 29
    let mut string_rows = list.new[Str] :: :: call
    let mut record_rows = list.new[Str] :: :: call
    let mut function_rows = list.new[Str] :: :: call
    let mut code_blobs = list.new[Str] :: :: call
    let mut behavior_rows = list.new[Str] :: :: call
    let mut current_function = ""
    let mut current_code = ""
    let mut have_function = false
    let mut lines_scan = list.new[Str] :: :: call
    let mut lines_rev = std.text.split_lines :: spec_text :: call
    while (lines_rev :: :: len) > 0:
        lines_scan :: (lines_rev :: :: pop) :: push
    while (lines_scan :: :: len) > 0:
        let raw = lines_scan :: :: pop
        let line = bytecode_writer.strip_line_end :: raw :: call
        let trimmed = arcana_compiler_core.core.trim_ws :: line :: call
        if (std.text.len_bytes :: trimmed :: call) <= 0:
            continue
        if std.text.starts_with :: trimmed, "#" :: call:
            continue
        if std.text.starts_with :: trimmed, "kind=" :: call:
            continue
        if std.text.starts_with :: trimmed, "version=" :: call:
            let version_text = bytecode_writer.body_after_prefix :: trimmed, "version=" :: call
            version = bytecode_writer.parse_int_or :: version_text, 29 :: call
            continue
        if std.text.starts_with :: line, "string=" :: call:
            let item = bytecode_writer.unescape_text :: (bytecode_writer.body_after_prefix :: line, "string=" :: call) :: call
            string_rows :: item :: push
            continue
        if std.text.starts_with :: trimmed, "record=" :: call:
            record_rows :: (bytecode_writer.body_after_prefix :: trimmed, "record=" :: call) :: push
            continue
        if std.text.starts_with :: trimmed, "behavior=" :: call:
            behavior_rows :: (bytecode_writer.body_after_prefix :: trimmed, "behavior=" :: call) :: push
            continue
        if std.text.starts_with :: trimmed, "function=" :: call:
            if have_function:
                function_rows :: (bytecode_writer.clone_text :: current_function :: call) :: push
                code_blobs :: (bytecode_writer.clone_text :: current_code :: call) :: push
            current_function = bytecode_writer.body_after_prefix :: trimmed, "function=" :: call
            current_code = ""
            have_function = true
            continue
        if trimmed == "endfn":
            if have_function:
                function_rows :: (bytecode_writer.clone_text :: current_function :: call) :: push
                code_blobs :: (bytecode_writer.clone_text :: current_code :: call) :: push
                current_function = ""
                current_code = ""
                have_function = false
            continue
        if std.text.starts_with :: trimmed, "code=" :: call:
            let body = bytecode_writer.body_after_prefix :: trimmed, "code=" :: call
            if not have_function:
                return "encountered code row before function header"
            if (std.text.len_bytes :: current_code :: call) <= 0:
                current_code = body
            else:
                current_code = current_code + "\n" + body
            continue
    if have_function:
        function_rows :: (bytecode_writer.clone_text :: current_function :: call) :: push
        code_blobs :: (bytecode_writer.clone_text :: current_code :: call) :: push
    if (function_rows :: :: len) != (code_blobs :: :: len):
        return "function/code blob count mismatch"
    bytecode_writer.push_u8 :: out, 65 :: call
    bytecode_writer.push_u8 :: out, 82 :: call
    bytecode_writer.push_u8 :: out, 67 :: call
    bytecode_writer.push_u8 :: out, 66 :: call
    bytecode_writer.push_u16 :: out, version :: call

    bytecode_writer.push_u32 :: out, (string_rows :: :: len) :: call
    let mut string_scan = bytecode_writer.ordered_rows :: string_rows :: call
    while (string_scan :: :: len) > 0:
        let item = string_scan :: :: pop
        bytecode_writer.push_string :: out, item :: call

    bytecode_writer.push_u32 :: out, (record_rows :: :: len) :: call
    let mut record_scan = bytecode_writer.ordered_rows :: record_rows :: call
    while (record_scan :: :: len) > 0:
        let row = record_scan :: :: pop
        let name = bytecode_writer.pipe_field_at :: row, 0, "" :: call
        bytecode_writer.push_string :: out, name :: call
        let fields = bytecode_writer.fields_after :: row, 124, 1 :: call
        bytecode_writer.push_u32 :: out, (fields :: :: len) :: call
        let mut field_scan = bytecode_writer.ordered_rows :: fields :: call
        while (field_scan :: :: len) > 0:
            let field = field_scan :: :: pop
            bytecode_writer.push_string :: out, field :: call

    bytecode_writer.push_u32 :: out, (function_rows :: :: len) :: call
    let mut function_scan = bytecode_writer.ordered_rows :: function_rows :: call
    let mut code_scan = bytecode_writer.ordered_rows :: code_blobs :: call
    while (function_scan :: :: len) > 0:
        let row = function_scan :: :: pop
        let name = bytecode_writer.pipe_field_at :: row, 0, "" :: call
        let is_async_text = bytecode_writer.pipe_field_at :: row, 1, "0" :: call
        let arity_text = bytecode_writer.pipe_field_at :: row, 2, "0" :: call
        let param_modes_text = bytecode_writer.pipe_field_at :: row, 3, "" :: call
        let locals_text = bytecode_writer.pipe_field_at :: row, 4, "0" :: call
        let is_async = bytecode_writer.parse_int_or :: is_async_text, 0 :: call
        let arity = bytecode_writer.parse_int_or :: arity_text, 0 :: call
        let param_modes = bytecode_writer.csv_ints :: param_modes_text :: call
        let locals = bytecode_writer.parse_int_or :: locals_text, 0 :: call
        let code_blob = code_scan :: :: pop
        let code_rows = bytecode_writer.split_code_blob :: code_blob :: call
        bytecode_writer.push_string :: out, name :: call
        if is_async != 0:
            bytecode_writer.push_u8 :: out, 1 :: call
        else:
            bytecode_writer.push_u8 :: out, 0 :: call
        bytecode_writer.push_u16 :: out, arity :: call
        bytecode_writer.push_u16 :: out, (param_modes :: :: len) :: call
        let mut mode_scan = bytecode_writer.ordered_ints :: param_modes :: call
        while (mode_scan :: :: len) > 0:
            let mode = mode_scan :: :: pop
            bytecode_writer.push_u8 :: out, mode :: call
        bytecode_writer.push_u16 :: out, locals :: call
        bytecode_writer.push_u32 :: out, (code_rows :: :: len) :: call
        let mut code_row_scan = bytecode_writer.ordered_rows :: code_rows :: call
        while (code_row_scan :: :: len) > 0:
            let code_row = code_row_scan :: :: pop
            let opcode_error = bytecode_writer.encode_opcode_row_error :: out, code_row :: call
            if (std.text.len_bytes :: opcode_error :: call) > 0:
                return "function `" + name + "`: " + opcode_error

    bytecode_writer.push_u32 :: out, (function_rows :: :: len) :: call
    let mut sig_scan = bytecode_writer.ordered_rows :: function_rows :: call
    while (sig_scan :: :: len) > 0:
        let row = sig_scan :: :: pop
        let sig_params_text = bytecode_writer.pipe_field_at :: row, 5, "" :: call
        let sig_ret_text = bytecode_writer.pipe_field_at :: row, 6, "0" :: call
        let sig_params = bytecode_writer.csv_ints :: sig_params_text :: call
        let sig_ret = bytecode_writer.parse_int_or :: sig_ret_text, 0 :: call
        bytecode_writer.push_u16 :: out, (sig_params :: :: len) :: call
        let mut sig_scan2 = bytecode_writer.ordered_ints :: sig_params :: call
        while (sig_scan2 :: :: len) > 0:
            let tag = sig_scan2 :: :: pop
            bytecode_writer.push_u8 :: out, tag :: call
        bytecode_writer.push_u8 :: out, sig_ret :: call

    bytecode_writer.push_u32 :: out, (behavior_rows :: :: len) :: call
    let mut behavior_scan = bytecode_writer.ordered_rows :: behavior_rows :: call
    while (behavior_scan :: :: len) > 0:
        let row = behavior_scan :: :: pop
        let name = bytecode_writer.pipe_field_at :: row, 0, "" :: call
        let phase_text = bytecode_writer.pipe_field_at :: row, 1, "0" :: call
        let affinity_text = bytecode_writer.pipe_field_at :: row, 2, "0" :: call
        let fn_index_text = bytecode_writer.pipe_field_at :: row, 3, "0" :: call
        let component_types_text = bytecode_writer.pipe_field_at :: row, 4, "" :: call
        let contracted_text = bytecode_writer.pipe_field_at :: row, 5, "0" :: call
        let scheduler_group_text = bytecode_writer.pipe_field_at :: row, 6, "0" :: call
        let contract_phase_text = bytecode_writer.pipe_field_at :: row, 7, "0" :: call
        let contract_thread_text = bytecode_writer.pipe_field_at :: row, 8, "0" :: call
        let contract_authority_text = bytecode_writer.pipe_field_at :: row, 9, "0" :: call
        let deterministic_text = bytecode_writer.pipe_field_at :: row, 10, "0" :: call
        let rollback_safe_text = bytecode_writer.pipe_field_at :: row, 11, "0" :: call
        let reads_text = bytecode_writer.pipe_field_at :: row, 12, "" :: call
        let writes_text = bytecode_writer.pipe_field_at :: row, 13, "" :: call
        let excludes_text = bytecode_writer.pipe_field_at :: row, 14, "" :: call
        let phase = bytecode_writer.parse_int_or :: phase_text, 0 :: call
        let affinity = bytecode_writer.parse_int_or :: affinity_text, 0 :: call
        let fn_index = bytecode_writer.parse_int_or :: fn_index_text, 0 :: call
        let component_types = bytecode_writer.csv_strs :: component_types_text :: call
        let contracted = bytecode_writer.parse_int_or :: contracted_text, 0 :: call
        let scheduler_group = bytecode_writer.parse_int_or :: scheduler_group_text, 0 :: call
        let contract_phase = bytecode_writer.parse_int_or :: contract_phase_text, 0 :: call
        let contract_thread = bytecode_writer.parse_int_or :: contract_thread_text, 0 :: call
        let contract_authority = bytecode_writer.parse_int_or :: contract_authority_text, 0 :: call
        let deterministic = bytecode_writer.parse_int_or :: deterministic_text, 0 :: call
        let rollback_safe = bytecode_writer.parse_int_or :: rollback_safe_text, 0 :: call
        let reads = bytecode_writer.csv_ints :: reads_text :: call
        let writes = bytecode_writer.csv_ints :: writes_text :: call
        let excludes = bytecode_writer.csv_ints :: excludes_text :: call
        bytecode_writer.push_string :: out, name :: call
        bytecode_writer.push_u8 :: out, phase :: call
        bytecode_writer.push_u8 :: out, affinity :: call
        bytecode_writer.push_u16 :: out, fn_index :: call
        bytecode_writer.encode_str_list_u16 :: out, component_types :: call
        if contracted != 0:
            bytecode_writer.push_u8 :: out, 1 :: call
        else:
            bytecode_writer.push_u8 :: out, 0 :: call
        bytecode_writer.push_u16 :: out, scheduler_group :: call
        bytecode_writer.push_u8 :: out, contract_phase :: call
        bytecode_writer.push_u8 :: out, contract_thread :: call
        bytecode_writer.push_u8 :: out, contract_authority :: call
        if deterministic != 0:
            bytecode_writer.push_u8 :: out, 1 :: call
        else:
            bytecode_writer.push_u8 :: out, 0 :: call
        if rollback_safe != 0:
            bytecode_writer.push_u8 :: out, 1 :: call
        else:
            bytecode_writer.push_u8 :: out, 0 :: call
        bytecode_writer.encode_int_list_u32 :: out, reads :: call
        bytecode_writer.encode_int_list_u32 :: out, writes :: call
        bytecode_writer.encode_int_list_u32 :: out, excludes :: call

    return ""

fn encode_lib_spec(spec_text: Str) -> Array[Int]:
    let mut out = std.bytes.new_buf :: :: call
    let error = bytecode_writer.encode_lib_spec_diagnostic :: spec_text, out :: call
    if (std.text.len_bytes :: error :: call) > 0:
        return empty_bytes :: :: call
    return std.bytes.buf_to_array :: out :: call

fn encode_lib_spec_diagnostic(spec_text: Str, edit out: List[Int]) -> Str:
    let mut bytecode_version = 29
    let mut std_abi = "std-abi-v1"
    let mut export_rows = list.new[Str] :: :: call
    let mut dep_rows = list.new[Str] :: :: call
    let mut module_spec = "kind=module"
    let mut lines_scan = list.new[Str] :: :: call
    let mut lines_rev = std.text.split_lines :: spec_text :: call
    while (lines_rev :: :: len) > 0:
        lines_scan :: (lines_rev :: :: pop) :: push
    while (lines_scan :: :: len) > 0:
        let raw = lines_scan :: :: pop
        let line = bytecode_writer.strip_line_end :: raw :: call
        let trimmed = arcana_compiler_core.core.trim_ws :: line :: call
        if (std.text.len_bytes :: trimmed :: call) <= 0:
            continue
        if std.text.starts_with :: trimmed, "#" :: call:
            continue
        if std.text.starts_with :: trimmed, "kind=" :: call:
            continue
        if std.text.starts_with :: trimmed, "bytecode_version=" :: call:
            let version_text = bytecode_writer.body_after_prefix :: trimmed, "bytecode_version=" :: call
            bytecode_version = bytecode_writer.parse_int_or :: version_text, 29 :: call
            module_spec = module_spec + "\nversion=" + (std.text.from_int :: bytecode_version :: call)
            continue
        if std.text.starts_with :: line, "std_abi=" :: call:
            std_abi = bytecode_writer.unescape_text :: (bytecode_writer.body_after_prefix :: line, "std_abi=" :: call) :: call
            continue
        if std.text.starts_with :: trimmed, "export=" :: call:
            export_rows :: (bytecode_writer.body_after_prefix :: trimmed, "export=" :: call) :: push
            continue
        if std.text.starts_with :: trimmed, "dep=" :: call:
            dep_rows :: (bytecode_writer.body_after_prefix :: trimmed, "dep=" :: call) :: push
            continue
        module_spec = module_spec + "\n" + line
    let mut module_payload = std.bytes.new_buf :: :: call
    let module_error = bytecode_writer.encode_module_spec_diagnostic :: module_spec, module_payload :: call
    if (std.text.len_bytes :: module_error :: call) > 0:
        return "embedded module encode failed: " + module_error
    bytecode_writer.push_u8 :: out, 65 :: call
    bytecode_writer.push_u8 :: out, 82 :: call
    bytecode_writer.push_u8 :: out, 67 :: call
    bytecode_writer.push_u8 :: out, 76 :: call
    bytecode_writer.push_u16 :: out, 1 :: call
    bytecode_writer.push_u16 :: out, bytecode_version :: call
    bytecode_writer.push_string :: out, std_abi :: call

    bytecode_writer.push_u32 :: out, (export_rows :: :: len) :: call
    let mut export_scan = bytecode_writer.ordered_rows :: export_rows :: call
    while (export_scan :: :: len) > 0:
        let row = export_scan :: :: pop
        let name = bytecode_writer.pipe_field_at :: row, 0, "" :: call
        let arity_text = bytecode_writer.pipe_field_at :: row, 1, "0" :: call
        let is_async_text = bytecode_writer.pipe_field_at :: row, 2, "0" :: call
        let param_modes_text = bytecode_writer.pipe_field_at :: row, 3, "" :: call
        let param_types_text = bytecode_writer.pipe_field_at :: row, 4, "" :: call
        let arity = bytecode_writer.parse_int_or :: arity_text, 0 :: call
        let is_async = bytecode_writer.parse_int_or :: is_async_text, 0 :: call
        let param_modes = bytecode_writer.csv_ints :: param_modes_text :: call
        let param_types = bytecode_writer.csv_strs :: param_types_text :: call
        let ret_type = bytecode_writer.pipe_field_at :: row, 5, "" :: call
        bytecode_writer.push_string :: out, name :: call
        bytecode_writer.push_u16 :: out, arity :: call
        if is_async != 0:
            bytecode_writer.push_u8 :: out, 1 :: call
        else:
            bytecode_writer.push_u8 :: out, 0 :: call
        bytecode_writer.push_u16 :: out, (param_modes :: :: len) :: call
        let mut mode_scan = bytecode_writer.ordered_ints :: param_modes :: call
        while (mode_scan :: :: len) > 0:
            let mode = mode_scan :: :: pop
            bytecode_writer.push_u8 :: out, mode :: call
        bytecode_writer.encode_str_list_u16 :: out, param_types :: call
        bytecode_writer.push_string :: out, ret_type :: call

    bytecode_writer.push_u32 :: out, (dep_rows :: :: len) :: call
    let mut dep_scan = bytecode_writer.ordered_rows :: dep_rows :: call
    while (dep_scan :: :: len) > 0:
        let row = dep_scan :: :: pop
        bytecode_writer.push_string :: out, (bytecode_writer.pipe_field_at :: row, 0, "" :: call) :: call
        bytecode_writer.push_string :: out, (bytecode_writer.pipe_field_at :: row, 1, "" :: call) :: call

    bytecode_writer.push_u32 :: out, (module_payload :: :: len) :: call
    let mut mbi = 0
    while mbi < (module_payload :: :: len):
        let b = module_payload[mbi]
        out :: b :: push
        mbi += 1

    return ""

export fn write_spec_file(spec_text: Str, output_path: Str) -> Bool:
    let error = bytecode_writer.write_spec_file_error :: spec_text, output_path :: call
    return (std.text.len_bytes :: error :: call) <= 0

export fn write_spec_file_error(spec_text: Str, output_path: Str) -> Str:
    let kind = bytecode_writer.spec_kind :: spec_text :: call
    let mut out = std.bytes.new_buf :: :: call
    let mut error = ""
    if std.text.starts_with :: kind, "kind=lib" :: call:
        error = bytecode_writer.encode_lib_spec_diagnostic :: spec_text, out :: call
    else:
        error = bytecode_writer.encode_module_spec_diagnostic :: spec_text, out :: call
    if (std.text.len_bytes :: error :: call) > 0:
        return error
    let encoded = std.bytes.buf_to_array :: out :: call
    if (encoded :: :: len) <= 0:
        return "encoded artifact was empty"
    if not (fs_support.write_bytes_or_false :: output_path, encoded :: call):
        return "failed to write encoded artifact to `" + output_path + "`"
    return ""

export fn write_spec_file_diagnostic(spec_text: Str, output_path: Str) -> (Bool, Str):
    let error = bytecode_writer.write_spec_file_error :: spec_text, output_path :: call
    if (std.text.len_bytes :: error :: call) > 0:
        return (false, error)
    return (true, "")

export fn encode_module(read module: arcana_compiler_core.types.BytecodeModule) -> Array[Int]:
    let spec_text = arcana_compiler_core.bytecode_writer_typed.module_spec_from_typed :: module :: call
    let mut out = std.bytes.new_buf :: :: call
    let error = bytecode_writer.encode_module_spec_diagnostic :: spec_text, out :: call
    if (std.text.len_bytes :: error :: call) > 0:
        return empty_bytes :: :: call
    return std.bytes.buf_to_array :: out :: call

export fn encode_lib(read artifact: arcana_compiler_core.types.BytecodeLibArtifact) -> Array[Int]:
    let spec_text = arcana_compiler_core.bytecode_writer_typed.lib_spec_from_typed :: artifact :: call
    let mut out = std.bytes.new_buf :: :: call
    let error = bytecode_writer.encode_lib_spec_diagnostic :: spec_text, out :: call
    if (std.text.len_bytes :: error :: call) > 0:
        return empty_bytes :: :: call
    return std.bytes.buf_to_array :: out :: call

fn module_hello_spec() -> Str:
    let mut spec = "kind=module\nversion=29"
    spec += "\nstring=hello"
    spec += "\nrecord=Mage|mana"
    spec += "\nfunction=main|0|0||1||0"
    spec += "\ncode=2|0|0"
    spec += "\ncode=131|76|1"
    spec += "\ncode=20|0|0"
    spec += "\nendfn"
    return spec

fn module_behavior_spec() -> Str:
    let mut spec = "kind=module\nversion=29"
    spec += "\nfunction=tick|0|0||0||0"
    spec += "\ncode=0|7|0"
    spec += "\ncode=20|0|0"
    spec += "\nendfn"
    spec += "\nbehavior=tick|2|2|0|Player,Enemy|1|7|2|2|0|1|1|1,2|3|4,5"
    return spec

fn lib_util_spec() -> Str:
    let mut spec = "kind=lib\nbytecode_version=29\nstd_abi=std-abi-v1"
    spec += "\nexport=util|1|0|0|Int|Int"
    spec += "\ndep=core|sha256:abc"
    spec += "\nstring=hello"
    spec += "\nfunction=util|0|1|0|1|0|0"
    spec += "\ncode=3|0|0"
    spec += "\ncode=20|0|0"
    spec += "\nendfn"
    return spec

export fn write_fixture_file(fixture: Str, output_path: Str) -> Bool:
    if fixture == "module_hello":
        let bytes = bytecode_writer.encode_module :: (arcana_compiler_core.bytecode_writer_typed.module_hello_fixture :: :: call) :: call
        return (bytes :: :: len) > 0 and (fs_support.write_bytes_or_false :: output_path, bytes :: call)
    if fixture == "module_behavior":
        let bytes = bytecode_writer.encode_module :: (arcana_compiler_core.bytecode_writer_typed.module_behavior_fixture :: :: call) :: call
        return (bytes :: :: len) > 0 and (fs_support.write_bytes_or_false :: output_path, bytes :: call)
    if fixture == "lib_util":
        let bytes = bytecode_writer.encode_lib :: (arcana_compiler_core.bytecode_writer_typed.lib_util_fixture :: :: call) :: call
        return (bytes :: :: len) > 0 and (fs_support.write_bytes_or_false :: output_path, bytes :: call)
    return false
