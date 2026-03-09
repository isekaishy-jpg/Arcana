import std.bytes
import std.collections.array
import std.collections.list
import std.fs
import std.path
import std.text
import arcana_compiler_core.types

fn empty_instr() -> arcana_compiler_core.types.DecodedInstr:
    let meta = ("", false)
    let tail = ("Unknown", ("", -1))
    return arcana_compiler_core.types.DecodedInstr :: meta = meta, tail = tail :: call

fn empty_function() -> arcana_compiler_core.types.DecodedFunction:
    let fill = empty_instr :: :: call
    let code = std.collections.array.new[arcana_compiler_core.types.DecodedInstr] :: 0, fill :: call
    let meta = (false, (0, 0))
    return arcana_compiler_core.types.DecodedFunction :: name = "", meta = meta, code = code :: call

fn empty_module() -> arcana_compiler_core.types.DecodedModule:
    let fill = empty_function :: :: call
    let functions = std.collections.array.new[arcana_compiler_core.types.DecodedFunction] :: 0, fill :: call
    let counts = (0, (0, 0))
    return arcana_compiler_core.types.DecodedModule :: counts = counts, functions = functions :: call

fn empty_phi() -> arcana_compiler_core.types.IrPhi:
    let fill = arcana_compiler_core.types.IrPhiInput :: pred_block = 0, value_id = 0 :: call
    let inputs = std.collections.array.new[arcana_compiler_core.types.IrPhiInput] :: 0, fill :: call
    return arcana_compiler_core.types.IrPhi :: output = 0, inputs = inputs :: call

fn empty_ir_instr() -> arcana_compiler_core.types.IrInstr:
    let meta = (false, -1)
    let tail = ("Unknown", "")
    return arcana_compiler_core.types.IrInstr :: meta = meta, tail = tail :: call

fn empty_block() -> arcana_compiler_core.types.IrBlock:
    let phi_fill = empty_phi :: :: call
    let instr_fill = empty_ir_instr :: :: call
    let phis = std.collections.array.new[arcana_compiler_core.types.IrPhi] :: 0, phi_fill :: call
    let instrs = std.collections.array.new[arcana_compiler_core.types.IrInstr] :: 0, instr_fill :: call
    let content = (phis, instrs)
    let term = ("return", (-1, (-1, -1)))
    return arcana_compiler_core.types.IrBlock :: id = 0, content = content, term = term :: call

fn empty_ir_function() -> arcana_compiler_core.types.IrFunction:
    let fill = empty_block :: :: call
    let blocks = std.collections.array.new[arcana_compiler_core.types.IrBlock] :: 0, fill :: call
    let meta = (false, (0, 0))
    let tail = (0, (0, blocks))
    return arcana_compiler_core.types.IrFunction :: name = "", meta = meta, tail = tail :: call

fn ok_module(value: arcana_compiler_core.types.DecodedModule) -> arcana_compiler_core.types.Outcome[arcana_compiler_core.types.DecodedModule]:
    return arcana_compiler_core.types.Outcome[arcana_compiler_core.types.DecodedModule] :: ok = true, value = value, message = "" :: call

fn err_module(message: Str) -> arcana_compiler_core.types.Outcome[arcana_compiler_core.types.DecodedModule]:
    return arcana_compiler_core.types.Outcome[arcana_compiler_core.types.DecodedModule] :: ok = false, value = empty_module :: :: call, message = message :: call

fn ok_text(value: Str) -> arcana_compiler_core.types.Outcome[Str]:
    return arcana_compiler_core.types.Outcome[Str] :: ok = true, value = value, message = "" :: call

fn err_text(message: Str) -> arcana_compiler_core.types.Outcome[Str]:
    return arcana_compiler_core.types.Outcome[Str] :: ok = false, value = "", message = message :: call

fn ok_bool() -> arcana_compiler_core.types.Outcome[Bool]:
    return arcana_compiler_core.types.Outcome[Bool] :: ok = true, value = true, message = "" :: call

fn err_bool(message: Str) -> arcana_compiler_core.types.Outcome[Bool]:
    return arcana_compiler_core.types.Outcome[Bool] :: ok = false, value = false, message = message :: call

fn instr_render(read instr: arcana_compiler_core.types.DecodedInstr) -> Str:
    return instr.meta.0

fn instr_pushes(read instr: arcana_compiler_core.types.DecodedInstr) -> Bool:
    return instr.meta.1

fn instr_type(read instr: arcana_compiler_core.types.DecodedInstr) -> Str:
    return instr.tail.0

fn instr_control(read instr: arcana_compiler_core.types.DecodedInstr) -> Str:
    return instr.tail.1.0

fn instr_target(read instr: arcana_compiler_core.types.DecodedInstr) -> Int:
    return instr.tail.1.1

fn make_instr(render: Str, meta_tail: (Bool, Str), control_tail: (Str, Int)) -> arcana_compiler_core.types.DecodedInstr:
    let meta = (render, meta_tail.0)
    let tail = (meta_tail.1, control_tail)
    return arcana_compiler_core.types.DecodedInstr :: meta = meta, tail = tail :: call

fn new_cursor(read bytes: Array[Int]) -> arcana_compiler_core.types.ByteCursor:
    return arcana_compiler_core.types.ByteCursor :: bytes = bytes, pos = 0, error = "" :: call

fn cursor_fail(edit cursor: arcana_compiler_core.types.ByteCursor, message: Str):
    if (std.text.len_bytes :: cursor.error :: call) <= 0:
        cursor.error = message

fn need(edit cursor: arcana_compiler_core.types.ByteCursor, count: Int) -> Bool:
    if (cursor.pos + count) > (std.bytes.len :: cursor.bytes :: call):
        cursor_fail :: cursor, "unexpected end of bytecode" :: call
        return false
    return true

fn read_u8(edit cursor: arcana_compiler_core.types.ByteCursor) -> Int:
    if not (need :: cursor, 1 :: call):
        return 0
    let value = std.bytes.at :: cursor.bytes, cursor.pos :: call
    cursor.pos += 1
    return value

fn read_u16(edit cursor: arcana_compiler_core.types.ByteCursor) -> Int:
    if not (need :: cursor, 2 :: call):
        return 0
    let lo = std.bytes.at :: cursor.bytes, cursor.pos :: call
    let hi = std.bytes.at :: cursor.bytes, cursor.pos + 1 :: call
    cursor.pos += 2
    return lo + hi * 256

fn read_u32(edit cursor: arcana_compiler_core.types.ByteCursor) -> Int:
    if not (need :: cursor, 4 :: call):
        return 0
    let b0 = std.bytes.at :: cursor.bytes, cursor.pos :: call
    let b1 = std.bytes.at :: cursor.bytes, cursor.pos + 1 :: call
    let b2 = std.bytes.at :: cursor.bytes, cursor.pos + 2 :: call
    let b3 = std.bytes.at :: cursor.bytes, cursor.pos + 3 :: call
    cursor.pos += 4
    return b0 + b1 * 256 + b2 * 65536 + b3 * 16777216

fn read_i64(edit cursor: arcana_compiler_core.types.ByteCursor) -> Int:
    if not (need :: cursor, 8 :: call):
        return 0
    let start = cursor.pos
    cursor.pos += 8
    let mut value = std.bytes.at :: cursor.bytes, start + 7 :: call
    if value >= 128:
        value -= 256
    let mut i = 6
    while i >= 0:
        value = value * 256 + (std.bytes.at :: cursor.bytes, start + i :: call)
        i -= 1
    return value

fn read_string(edit cursor: arcana_compiler_core.types.ByteCursor) -> Str:
    let count = read_u32 :: cursor :: call
    if not (need :: cursor, count :: call):
        return ""
    let slice = std.bytes.slice :: cursor.bytes, cursor.pos, cursor.pos + count :: call
    cursor.pos += count
    return std.bytes.to_str_utf8 :: slice :: call

fn read_exact_bytes(edit cursor: arcana_compiler_core.types.ByteCursor, count: Int) -> Array[Int]:
    if not (need :: cursor, count :: call):
        return std.collections.array.new[Int] :: 0, 0 :: call
    let slice = std.bytes.slice :: cursor.bytes, cursor.pos, cursor.pos + count :: call
    cursor.pos += count
    return slice

fn bool_text(flag: Bool) -> Str:
    if flag:
        return "true"
    return "false"

fn render_call(name: Str, value: Int) -> Str:
    return name + "(" + (std.text.from_int :: value :: call) + ")"

fn decode_instr(edit cursor: arcana_compiler_core.types.ByteCursor) -> arcana_compiler_core.types.DecodedInstr:
    let tag = read_u8 :: cursor :: call
    if tag == 0:
        return make_instr :: ("ConstInt(" + (std.text.from_int :: (read_i64 :: cursor :: call) :: call) + ")"), (true, "Int"), ("", -1) :: call
    if tag == 1:
        return make_instr :: ("ConstBool(" + (bool_text :: ((read_u8 :: cursor :: call) != 0) :: call) + ")"), (true, "Bool"), ("", -1) :: call
    if tag == 2:
        return make_instr :: (render_call :: "ConstStr", (read_u32 :: cursor :: call) :: call), (true, "Str"), ("", -1) :: call
    if tag == 3:
        return make_instr :: (render_call :: "LoadLocal", (read_u16 :: cursor :: call) :: call), (true, "Unknown"), ("", -1) :: call
    if tag == 21:
        return make_instr :: (render_call :: "LoadLocalRef", (read_u16 :: cursor :: call) :: call), (true, "Unknown"), ("", -1) :: call
    if tag == 4:
        return make_instr :: (render_call :: "StoreLocal", (read_u16 :: cursor :: call) :: call), (false, "Unknown"), ("", -1) :: call
    if tag == 5:
        return make_instr :: (render_call :: "LoadField", (read_u16 :: cursor :: call) :: call), (true, "Unknown"), ("", -1) :: call
    if tag == 6:
        return make_instr :: (render_call :: "StoreField", (read_u16 :: cursor :: call) :: call), (false, "Unknown"), ("", -1) :: call
    if tag == 7:
        return make_instr :: (render_call :: "MakeRecord", (read_u16 :: cursor :: call) :: call), (true, "Unknown"), ("", -1) :: call
    if tag == 123:
        let discr = read_u16 :: cursor :: call
        let fields = read_u16 :: cursor :: call
        return make_instr :: ("MakeEnum(" + (std.text.from_int :: discr :: call) + ", " + (std.text.from_int :: fields :: call) + ")"), (true, "Unknown"), ("", -1) :: call
    if tag == 124:
        return make_instr :: "EnumTag", (true, "Unknown"), ("", -1) :: call
    if tag == 125:
        return make_instr :: (render_call :: "EnumGetField", (read_u16 :: cursor :: call) :: call), (true, "Unknown"), ("", -1) :: call
    if tag == 66:
        return make_instr :: (render_call :: "MakeList", (read_u16 :: cursor :: call) :: call), (true, "Unknown"), ("", -1) :: call
    if tag == 78:
        return make_instr :: "MakeArray", (true, "Unknown"), ("", -1) :: call
    if tag == 79:
        return make_instr :: (render_call :: "MakeMap", (read_u16 :: cursor :: call) :: call), (true, "Unknown"), ("", -1) :: call
    if tag == 67:
        return make_instr :: (render_call :: "MakeRange", (read_u8 :: cursor :: call) :: call), (true, "Unknown"), ("", -1) :: call
    if tag == 80:
        return make_instr :: "RangeToList", (true, "Unknown"), ("", -1) :: call
    if tag == 126:
        return make_instr :: "RangeIterInit", (true, "Unknown"), ("", -1) :: call
    if tag == 127:
        return make_instr :: "ListIterInit", (true, "Unknown"), ("", -1) :: call
    if tag == 128:
        return make_instr :: "ArrayIterInit", (true, "Unknown"), ("", -1) :: call
    if tag == 129:
        return make_instr :: "MapIterInit", (true, "Unknown"), ("", -1) :: call
    if tag == 130:
        return make_instr :: "IterNext", (true, "Unknown"), ("", -1) :: call
    if tag == 68:
        return make_instr :: "MakeTuple2", (true, "Unknown"), ("", -1) :: call
    if tag == 8:
        return make_instr :: "AddInt", (true, "Int"), ("", -1) :: call
    if tag == 65:
        return make_instr :: "ConcatStr", (true, "Str"), ("", -1) :: call
    if tag == 9:
        return make_instr :: "SubInt", (true, "Int"), ("", -1) :: call
    if tag == 10:
        return make_instr :: "MulInt", (true, "Int"), ("", -1) :: call
    if tag == 11:
        return make_instr :: "DivInt", (true, "Int"), ("", -1) :: call
    if tag == 56:
        return make_instr :: "ModInt", (true, "Int"), ("", -1) :: call
    if tag == 57:
        return make_instr :: "NegInt", (true, "Int"), ("", -1) :: call
    if tag == 58:
        return make_instr :: "NotBool", (true, "Bool"), ("", -1) :: call
    if tag == 59:
        return make_instr :: "BitAndInt", (true, "Int"), ("", -1) :: call
    if tag == 60:
        return make_instr :: "BitOrInt", (true, "Int"), ("", -1) :: call
    if tag == 61:
        return make_instr :: "BitXorInt", (true, "Int"), ("", -1) :: call
    if tag == 62:
        return make_instr :: "BitNotInt", (true, "Int"), ("", -1) :: call
    if tag == 63:
        return make_instr :: "ShlInt", (true, "Int"), ("", -1) :: call
    if tag == 64:
        return make_instr :: "ShrInt", (true, "Int"), ("", -1) :: call
    if tag == 12:
        return make_instr :: "Eq", (true, "Bool"), ("", -1) :: call
    if tag == 13:
        return make_instr :: "LtInt", (true, "Int"), ("", -1) :: call
    if tag == 14:
        return make_instr :: "GtInt", (true, "Int"), ("", -1) :: call
    if tag == 15:
        return make_instr :: "", (false, "Unknown"), ("jump", (read_u32 :: cursor :: call)) :: call
    if tag == 16:
        return make_instr :: "", (false, "Unknown"), ("branch", (read_u32 :: cursor :: call)) :: call
    if tag == 17:
        return make_instr :: (render_call :: "Call", (read_u16 :: cursor :: call) :: call), (true, "Unknown"), ("", -1) :: call
    if tag == 131:
        let id = read_u16 :: cursor :: call
        let argc = read_u16 :: cursor :: call
        return make_instr :: ("CallIntrinsic(" + (std.text.from_int :: id :: call) + ", " + (std.text.from_int :: argc :: call) + ")"), (true, "Unknown"), ("", -1) :: call
    if tag == 29:
        return make_instr :: (render_call :: "MakeTask", (read_u16 :: cursor :: call) :: call), (true, "Unknown"), ("", -1) :: call
    if tag == 30:
        return make_instr :: (render_call :: "WeaveTask", (read_u16 :: cursor :: call) :: call), (true, "Unknown"), ("", -1) :: call
    if tag == 31:
        return make_instr :: "AwaitTask", (true, "Unknown"), ("", -1) :: call
    if tag == 34:
        return make_instr :: (render_call :: "SplitThread", (read_u16 :: cursor :: call) :: call), (true, "Unknown"), ("", -1) :: call
    if tag == 69:
        return make_instr :: "ListLen", (true, "Int"), ("", -1) :: call
    if tag == 70:
        return make_instr :: "ListPush", (false, "Unknown"), ("", -1) :: call
    if tag == 71:
        return make_instr :: "ListPop", (true, "Unknown"), ("", -1) :: call
    if tag == 72:
        return make_instr :: "ListTryPopOr", (true, "Unknown"), ("", -1) :: call
    if tag == 73:
        return make_instr :: "ListGet", (true, "Unknown"), ("", -1) :: call
    if tag == 74:
        return make_instr :: "ListSet", (false, "Unknown"), ("", -1) :: call
    if tag == 75:
        return make_instr :: "ListSlice", (true, "Unknown"), ("", -1) :: call
    if tag == 81:
        return make_instr :: "ArrayLen", (true, "Int"), ("", -1) :: call
    if tag == 82:
        return make_instr :: "ArrayGet", (true, "Unknown"), ("", -1) :: call
    if tag == 83:
        return make_instr :: "ArraySet", (false, "Unknown"), ("", -1) :: call
    if tag == 84:
        return make_instr :: "ArraySlice", (true, "Unknown"), ("", -1) :: call
    if tag == 85:
        return make_instr :: "ArrayFromList", (true, "Unknown"), ("", -1) :: call
    if tag == 86:
        return make_instr :: "ArrayToList", (true, "Unknown"), ("", -1) :: call
    if tag == 87:
        return make_instr :: "MapLen", (true, "Unknown"), ("", -1) :: call
    if tag == 88:
        return make_instr :: "MapHas", (true, "Bool"), ("", -1) :: call
    if tag == 89:
        return make_instr :: "MapGet", (true, "Unknown"), ("", -1) :: call
    if tag == 90:
        return make_instr :: "MapSet", (false, "Unknown"), ("", -1) :: call
    if tag == 91:
        return make_instr :: "MapTryGetOr", (true, "Unknown"), ("", -1) :: call
    if tag == 92:
        return make_instr :: "MapItemsSnapshot", (true, "Unknown"), ("", -1) :: call
    if tag == 76:
        return make_instr :: "Tuple2Get0", (true, "Int"), ("", -1) :: call
    if tag == 77:
        return make_instr :: "Tuple2Get1", (true, "Int"), ("", -1) :: call
    if tag == 19:
        return make_instr :: "Pop", (false, "Unknown"), ("", -1) :: call
    if tag == 20:
        return make_instr :: "", (false, "Unknown"), ("return", -1) :: call
    cursor_fail :: cursor, "unsupported opcode in bootstrap ir dump: " + (std.text.from_int :: tag :: call) :: call
    return empty_instr :: :: call

fn decode_module_bytes(read bytes: Array[Int]) -> arcana_compiler_core.types.Outcome[arcana_compiler_core.types.DecodedModule]:
    let mut cursor = new_cursor :: bytes :: call
    if (std.bytes.len :: bytes :: call) < 6:
        return err_module :: "invalid bytecode magic" :: call
    let m0 = read_u8 :: cursor :: call
    let m1 = read_u8 :: cursor :: call
    let m2 = read_u8 :: cursor :: call
    let m3 = read_u8 :: cursor :: call
    if m0 != 65 or m1 != 82 or m2 != 67 or m3 != 66:
        return err_module :: "invalid bytecode magic" :: call
    let version = read_u16 :: cursor :: call
    if version != 28 and version != 29:
        return err_module :: ("unsupported bytecode version " + (std.text.from_int :: version :: call) + ", expected 28 or 29") :: call
    let string_count = read_u32 :: cursor :: call
    for _ in 0..string_count:
        read_string :: cursor :: call
    let record_count = read_u32 :: cursor :: call
    for _ in 0..record_count:
        read_string :: cursor :: call
        let field_count = read_u32 :: cursor :: call
        for _ in 0..field_count:
            read_string :: cursor :: call
    let function_count = read_u32 :: cursor :: call
    let fill = empty_function :: :: call
    let mut functions = std.collections.array.new[arcana_compiler_core.types.DecodedFunction] :: function_count, fill :: call
    for i in 0..function_count:
        let name = read_string :: cursor :: call
        let is_async = (read_u8 :: cursor :: call) != 0
        let arity = read_u16 :: cursor :: call
        let mode_len = read_u16 :: cursor :: call
        for _ in 0..mode_len:
            read_u8 :: cursor :: call
        let locals = read_u16 :: cursor :: call
        let code_len = read_u32 :: cursor :: call
        let mut code = std.collections.array.new[arcana_compiler_core.types.DecodedInstr] :: code_len, empty_instr :: :: call :: call
        for pc in 0..code_len:
            code[pc] = decode_instr :: cursor :: call
        let meta = (is_async, (arity, locals))
        functions[i] = arcana_compiler_core.types.DecodedFunction :: name = name, meta = meta, code = code :: call
    let sig_len = read_u32 :: cursor :: call
    for _ in 0..sig_len:
        let param_len = read_u16 :: cursor :: call
        for _ in 0..param_len:
            read_u8 :: cursor :: call
        read_u8 :: cursor :: call
    let behavior_count = read_u32 :: cursor :: call
    for _ in 0..behavior_count:
        read_string :: cursor :: call
        read_u8 :: cursor :: call
        read_u8 :: cursor :: call
        read_u16 :: cursor :: call
        let comp_len = read_u16 :: cursor :: call
        for _ in 0..comp_len:
            read_string :: cursor :: call
        if version >= 29:
            read_u8 :: cursor :: call
            read_u16 :: cursor :: call
            read_u8 :: cursor :: call
            read_u8 :: cursor :: call
            read_u8 :: cursor :: call
            read_u8 :: cursor :: call
            read_u8 :: cursor :: call
            let reads_len = read_u16 :: cursor :: call
            for _ in 0..reads_len:
                read_u32 :: cursor :: call
            let writes_len = read_u16 :: cursor :: call
            for _ in 0..writes_len:
                read_u32 :: cursor :: call
            let excludes_len = read_u16 :: cursor :: call
            for _ in 0..excludes_len:
                read_u32 :: cursor :: call
    if (std.text.len_bytes :: cursor.error :: call) > 0:
        return err_module :: cursor.error :: call
    let counts = (string_count, (record_count, behavior_count))
    return ok_module :: (arcana_compiler_core.types.DecodedModule :: counts = counts, functions = functions :: call) :: call

fn decode_lib_module_bytes(read bytes: Array[Int]) -> arcana_compiler_core.types.Outcome[arcana_compiler_core.types.DecodedModule]:
    let mut cursor = new_cursor :: bytes :: call
    if (std.bytes.len :: bytes :: call) < 6:
        return err_module :: "invalid library artifact magic" :: call
    let m0 = read_u8 :: cursor :: call
    let m1 = read_u8 :: cursor :: call
    let m2 = read_u8 :: cursor :: call
    let m3 = read_u8 :: cursor :: call
    if m0 != 65 or m1 != 82 or m2 != 67 or m3 != 76:
        return err_module :: "invalid library artifact magic" :: call
    let version = read_u16 :: cursor :: call
    if version != 1:
        return err_module :: ("unsupported library artifact version " + (std.text.from_int :: version :: call) + ", expected 1") :: call
    read_u16 :: cursor :: call
    read_string :: cursor :: call
    let export_count = read_u32 :: cursor :: call
    for _ in 0..export_count:
        read_string :: cursor :: call
        read_u16 :: cursor :: call
        read_u8 :: cursor :: call
        let mode_len = read_u16 :: cursor :: call
        for _ in 0..mode_len:
            read_u8 :: cursor :: call
        let type_len = read_u16 :: cursor :: call
        for _ in 0..type_len:
            read_string :: cursor :: call
        read_string :: cursor :: call
    let dep_count = read_u32 :: cursor :: call
    for _ in 0..dep_count:
        read_string :: cursor :: call
        read_string :: cursor :: call
    let module_len = read_u32 :: cursor :: call
    let module_bytes = read_exact_bytes :: cursor, module_len :: call
    if (std.text.len_bytes :: cursor.error :: call) > 0:
        return err_module :: cursor.error :: call
    return decode_module_bytes :: module_bytes :: call

fn compute_block_starts(read code: Array[arcana_compiler_core.types.DecodedInstr]) -> Array[Int]:
    let code_len = code :: :: len
    let mut marks = std.collections.array.new[Bool] :: code_len + 1, false :: call
    if code_len > 0:
        marks[0] = true
    for pc in 0..code_len:
        let instr = code[pc]
        let control = instr_control :: instr :: call
        if control == "jump" or control == "branch":
            let target = instr_target :: instr :: call
            if target >= 0 and target < code_len:
                marks[target] = true
            if (pc + 1) < code_len:
                marks[pc + 1] = true
        else:
            if control == "return" and (pc + 1) < code_len:
                marks[pc + 1] = true
    let mut rows = std.collections.list.new[Int] :: :: call
    for i in 0..code_len:
        if marks[i]:
            rows :: i :: push
    return std.collections.array.from_list[Int] :: rows :: call

fn build_ir_function(read fun: arcana_compiler_core.types.DecodedFunction) -> arcana_compiler_core.types.IrFunction:
    let starts = compute_block_starts :: fun.code :: call
    let block_count = starts :: :: len
    let code_len = fun.code :: :: len
    let mut start_to_id = std.collections.array.new[Int] :: code_len + 1, -1 :: call
    for i in 0..block_count:
        start_to_id[starts[i]] = i
    let mut blocks = std.collections.array.new[arcana_compiler_core.types.IrBlock] :: block_count, empty_block :: :: call :: call
    let mut next_value = 0
    let mut value_type_count = 0
    for idx in 0..block_count:
        let start = starts[idx]
        let mut end = code_len
        if (idx + 1) < block_count:
            end = starts[idx + 1]
        let mut rows = std.collections.list.new[arcana_compiler_core.types.IrInstr] :: :: call
        let mut term_kind = ""
        let mut term_a = -1
        let mut term_b = -1
        let mut last_output = -1
        let mut pc = start
        while pc < end:
            let instr = fun.code[pc]
            let control = instr_control :: instr :: call
            if control == "jump":
                term_kind = "jump"
                term_a = start_to_id[instr_target :: instr :: call]
                break
            if control == "branch":
                term_kind = "branch"
                term_a = start_to_id[pc + 1]
                term_b = start_to_id[instr_target :: instr :: call]
                break
            if control == "return":
                term_kind = "return"
                break
            let mut output = -1
            if instr_pushes :: instr :: call:
                output = next_value
                next_value += 1
                value_type_count += 1
                last_output = output
            let meta = ((instr_pushes :: instr :: call), output)
            let tail = ((instr_type :: instr :: call), (instr_render :: instr :: call))
            rows :: (arcana_compiler_core.types.IrInstr :: meta = meta, tail = tail :: call) :: push
            pc += 1
        if (std.text.len_bytes :: term_kind :: call) <= 0:
            if (idx + 1) < block_count:
                term_kind = "fallthrough"
                term_a = idx + 1
            else:
                term_kind = "return"
        let phis = std.collections.array.new[arcana_compiler_core.types.IrPhi] :: 0, empty_phi :: :: call :: call
        let instrs = std.collections.array.from_list[arcana_compiler_core.types.IrInstr] :: rows :: call
        let content = (phis, instrs)
        let term = (term_kind, (term_a, (term_b, last_output)))
        blocks[idx] = arcana_compiler_core.types.IrBlock :: id = idx, content = content, term = term :: call
    for bi in 0..block_count:
        let mut input_rows = std.collections.list.new[arcana_compiler_core.types.IrPhiInput] :: :: call
        let mut incoming_count = 0
        let mut all_present = true
        let mut first_value = -1
        let mut all_same = true
        for pred_idx in 0..block_count:
            let pred = blocks[pred_idx]
            let term_kind = pred.term.0
            let term_a = pred.term.1.0
            let term_b = pred.term.1.1.0
            let last_output = pred.term.1.1.1
            let mut matches = false
            if term_kind == "jump" or term_kind == "fallthrough":
                matches = term_a == bi
            else:
                if term_kind == "branch":
                    matches = term_a == bi or term_b == bi
            if matches:
                incoming_count += 1
                if last_output < 0:
                    all_present = false
                else:
                    input_rows :: (arcana_compiler_core.types.IrPhiInput :: pred_block = pred_idx, value_id = last_output :: call) :: push
                    if first_value < 0:
                        first_value = last_output
                    else:
                        if last_output != first_value:
                            all_same = false
        if incoming_count > 1 and all_present and (not all_same):
            let mut block = blocks[bi]
            let inputs = std.collections.array.from_list[arcana_compiler_core.types.IrPhiInput] :: input_rows :: call
            let phi_output = next_value
            let phi = arcana_compiler_core.types.IrPhi :: output = phi_output, inputs = inputs :: call
            let mut phi_rows = std.collections.list.new[arcana_compiler_core.types.IrPhi] :: :: call
            phi_rows :: phi :: push
            block.content = (std.collections.array.from_list[arcana_compiler_core.types.IrPhi] :: phi_rows :: call, block.content.1)
            block.term = (block.term.0, (block.term.1.0, (block.term.1.1.0, phi_output)))
            blocks[bi] = block
            next_value += 1
    let meta = (fun.meta.0, (fun.meta.1.0, fun.meta.1.1))
    let tail = (0, (value_type_count, blocks))
    return arcana_compiler_core.types.IrFunction :: name = fun.name, meta = meta, tail = tail :: call

fn render_term(kind: Str, term_a: Int, term_b: Int) -> Str:
    if kind == "jump":
        return "Jump(IrBlockId(" + (std.text.from_int :: term_a :: call) + "))"
    if kind == "fallthrough":
        return "Fallthrough(IrBlockId(" + (std.text.from_int :: term_a :: call) + "))"
    if kind == "branch":
        return "Branch { if_true: IrBlockId(" + (std.text.from_int :: term_a :: call) + "), if_false: IrBlockId(" + (std.text.from_int :: term_b :: call) + ") }"
    return "Return"

export fn render_artifact_ir_dump(path: Str) -> arcana_compiler_core.types.Outcome[Str]:
    let ext = std.path.ext :: path :: call
    let bytes = std.fs.read_bytes_or_empty :: path :: call
    let mut decoded = err_module :: "unsupported artifact extension" :: call
    if ext == "arcbc":
        decoded = decode_module_bytes :: bytes :: call
    else:
        if ext == "arclib":
            decoded = decode_lib_module_bytes :: bytes :: call
        else:
            return err_text :: ("unsupported artifact extension `" + ext + "`") :: call
    if not decoded.ok:
        return err_text :: decoded.message :: call
    let module = decoded.value
    let mut out = "arcana-ir-v2-ssa\n"
    out += "strings=" + (std.text.from_int :: module.counts.0 :: call)
    out += " records=" + (std.text.from_int :: module.counts.1.0 :: call)
    out += " functions=" + (std.text.from_int :: (module.functions :: :: len) :: call)
    out += " behaviors=" + (std.text.from_int :: module.counts.1.1 :: call) + "\n"
    for fi in 0..(module.functions :: :: len):
        let source_fun = module.functions[fi]
        let fun = build_ir_function :: source_fun :: call
        out += "fn " + fun.name + " async=" + (bool_text :: fun.meta.0 :: call)
        out += " arity=" + (std.text.from_int :: fun.meta.1.0 :: call)
        out += " locals=" + (std.text.from_int :: fun.meta.1.1 :: call)
        out += " entry=" + (std.text.from_int :: fun.tail.0 :: call)
        out += " values=" + (std.text.from_int :: fun.tail.1.0 :: call) + "\n"
        let blocks = fun.tail.1.1
        for bi in 0..(blocks :: :: len):
            let block = blocks[bi]
            out += "  block " + (std.text.from_int :: block.id :: call) + "\n"
            let phis = block.content.0
            for pi in 0..(phis :: :: len):
                let phi = phis[pi]
                out += "    v" + (std.text.from_int :: phi.output :: call) + " = phi("
                for ii in 0..(phi.inputs :: :: len):
                    let input = phi.inputs[ii]
                    if ii > 0:
                        out += ", "
                    out += "b" + (std.text.from_int :: input.pred_block :: call) + ":v" + (std.text.from_int :: input.value_id :: call)
                out += ")\n"
            let instrs = block.content.1
            for ii in 0..(instrs :: :: len):
                let instr = instrs[ii]
                if instr.meta.0:
                    out += "    v" + (std.text.from_int :: instr.meta.1 :: call) + ":" + instr.tail.0 + " = " + instr.tail.1 + "\n"
                else:
                    out += "    " + instr.tail.1 + "\n"
            out += "    term " + (render_term :: block.term.0, block.term.1.0, block.term.1.1.0 :: call) + "\n"
    return ok_text :: out :: call

export fn write_artifact_ir_dump(path: Str, dump_dir: Str, dump_name: Str) -> arcana_compiler_core.types.Outcome[Bool]:
    if (std.text.len_bytes :: dump_dir :: call) <= 0:
        return ok_bool :: :: call
    let rendered = render_artifact_ir_dump :: path :: call
    if not rendered.ok:
        return err_bool :: rendered.message :: call
    if not (std.fs.mkdir_all_or_false :: dump_dir :: call):
        return err_bool :: ("failed to create ir dump directory `" + dump_dir + "`") :: call
    let dump_path = std.path.join :: dump_dir, dump_name + ".ir.txt" :: call
    if not (std.fs.write_text_or_false :: dump_path, rendered.value :: call):
        return err_bool :: ("failed to write ir dump `" + dump_path + "`") :: call
    return ok_bool :: :: call


