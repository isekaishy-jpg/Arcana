import std.collections.array
import std.text
use std.collections.list as list
import arcana_compiler_core.types

fn append_line(text: Str, line: Str) -> Str:
    if (std.text.len_bytes :: text :: call) <= 0:
        return line
    return text + "\n" + line

fn empty_record_type() -> arcana_compiler_core.types.BytecodeRecordType:
    let fields = std.collections.array.new[Str] :: 0, "" :: call
    return arcana_compiler_core.types.BytecodeRecordType :: name = "", fields = fields :: call

fn empty_function_sig() -> arcana_compiler_core.types.BytecodeFunctionSig:
    let params = std.collections.array.new[Int] :: 0, 0 :: call
    return arcana_compiler_core.types.BytecodeFunctionSig :: params = params, ret = 0 :: call

fn empty_instr() -> arcana_compiler_core.types.BytecodeInstr:
    return arcana_compiler_core.types.BytecodeInstr :: tag = 20, a = 0, b = 0 :: call

fn empty_function() -> arcana_compiler_core.types.BytecodeFunction:
    let modes = std.collections.array.new[Int] :: 0, 0 :: call
    let fill = arcana_compiler_core.bytecode_writer_typed.empty_instr :: :: call
    let code = std.collections.array.new[arcana_compiler_core.types.BytecodeInstr] :: 0, fill :: call
    let meta = (false, (0, modes))
    let tail = (0, code)
    return arcana_compiler_core.types.BytecodeFunction :: name = "", meta = meta, tail = tail :: call

fn empty_behavior_contract() -> arcana_compiler_core.types.BytecodeBehaviorContract:
    return arcana_compiler_core.types.BytecodeBehaviorContract :: meta = (false, 0), tags = (0, (0, 0)), flags = (false, false) :: call

fn empty_access_sets() -> arcana_compiler_core.types.BytecodeAccessSets:
    let reads = std.collections.array.new[Int] :: 0, 0 :: call
    let writes = std.collections.array.new[Int] :: 0, 0 :: call
    let excludes = std.collections.array.new[Int] :: 0, 0 :: call
    return arcana_compiler_core.types.BytecodeAccessSets :: reads = reads, writes = writes, excludes = excludes :: call

fn empty_behavior() -> arcana_compiler_core.types.BytecodeBehavior:
    let component_types = std.collections.array.new[Str] :: 0, "" :: call
    let contract = arcana_compiler_core.bytecode_writer_typed.empty_behavior_contract :: :: call
    let access = arcana_compiler_core.bytecode_writer_typed.empty_access_sets :: :: call
    let tail = (component_types, (contract, access))
    let meta = (0, (0, 0))
    return arcana_compiler_core.types.BytecodeBehavior :: name = "", meta = meta, tail = tail :: call

fn empty_export() -> arcana_compiler_core.types.BytecodeLibExport:
    let modes = std.collections.array.new[Int] :: 0, 0 :: call
    let param_types = std.collections.array.new[Str] :: 0, "" :: call
    let meta = (0, (false, modes))
    let tail = (param_types, "")
    return arcana_compiler_core.types.BytecodeLibExport :: name = "", meta = meta, tail = tail :: call

fn empty_dep() -> arcana_compiler_core.types.BytecodeDepFingerprint:
    return arcana_compiler_core.types.BytecodeDepFingerprint :: dep = "", fingerprint = "" :: call

fn bool_digit(value: Bool) -> Str:
    if value:
        return "1"
    return "0"

fn csv_int_array(read xs: Array[Int]) -> Str:
    let mut out = ""
    let mut i = 0
    while i < (xs :: :: len):
        if i > 0:
            out = out + ","
        let value = xs[i]
        let part = std.text.from_int :: value :: call
        out = out + part
        i += 1
    return out

fn csv_str_array(read xs: Array[Str]) -> Str:
    let mut out = ""
    let mut i = 0
    while i < (xs :: :: len):
        if i > 0:
            out = out + ","
        out = out + xs[i]
        i += 1
    return out

fn instr_row(read instr: arcana_compiler_core.types.BytecodeInstr) -> Str:
    let tag = std.text.from_int :: instr.tag :: call
    let a = std.text.from_int :: instr.a :: call
    let b = std.text.from_int :: instr.b :: call
    return tag + "|" + a + "|" + b

fn record_row(read rec: arcana_compiler_core.types.BytecodeRecordType) -> Str:
    let mut out = rec.name
    let mut i = 0
    while i < (rec.fields :: :: len):
        out = out + "|" + rec.fields[i]
        i += 1
    return out

fn function_row(read fun: arcana_compiler_core.types.BytecodeFunction, read sig: arcana_compiler_core.types.BytecodeFunctionSig) -> Str:
    let flag = arcana_compiler_core.bytecode_writer_typed.bool_digit :: fun.meta.0 :: call
    let arity = std.text.from_int :: fun.meta.1.0 :: call
    let modes = arcana_compiler_core.bytecode_writer_typed.csv_int_array :: fun.meta.1.1 :: call
    let entry = std.text.from_int :: fun.tail.0 :: call
    let params = arcana_compiler_core.bytecode_writer_typed.csv_int_array :: sig.params :: call
    let ret = std.text.from_int :: sig.ret :: call
    return fun.name + "|" + flag + "|" + arity + "|" + modes + "|" + entry + "|" + params + "|" + ret

fn behavior_row(read item: arcana_compiler_core.types.BytecodeBehavior) -> Str:
    let contract = item.tail.1.0
    let access = item.tail.1.1
    let a = std.text.from_int :: item.meta.0 :: call
    let b = std.text.from_int :: item.meta.1.0 :: call
    let c = std.text.from_int :: item.meta.1.1 :: call
    let components = arcana_compiler_core.bytecode_writer_typed.csv_str_array :: item.tail.0 :: call
    let async_flag = arcana_compiler_core.bytecode_writer_typed.bool_digit :: contract.meta.0 :: call
    let every = std.text.from_int :: contract.meta.1 :: call
    let priority = std.text.from_int :: contract.tags.0 :: call
    let phase = std.text.from_int :: contract.tags.1.0 :: call
    let ord = std.text.from_int :: contract.tags.1.1 :: call
    let det = arcana_compiler_core.bytecode_writer_typed.bool_digit :: contract.flags.0 :: call
    let narrow = arcana_compiler_core.bytecode_writer_typed.bool_digit :: contract.flags.1 :: call
    let reads = arcana_compiler_core.bytecode_writer_typed.csv_int_array :: access.reads :: call
    let writes = arcana_compiler_core.bytecode_writer_typed.csv_int_array :: access.writes :: call
    let excludes = arcana_compiler_core.bytecode_writer_typed.csv_int_array :: access.excludes :: call
    return item.name + "|" + a + "|" + b + "|" + c + "|" + components + "|" + async_flag + "|" + every + "|" + priority + "|" + phase + "|" + ord + "|" + det + "|" + narrow + "|" + reads + "|" + writes + "|" + excludes

fn export_row(read item: arcana_compiler_core.types.BytecodeLibExport) -> Str:
    let arity = std.text.from_int :: item.meta.0 :: call
    let extern_flag = arcana_compiler_core.bytecode_writer_typed.bool_digit :: item.meta.1.0 :: call
    let modes = arcana_compiler_core.bytecode_writer_typed.csv_int_array :: item.meta.1.1 :: call
    let params = arcana_compiler_core.bytecode_writer_typed.csv_str_array :: item.tail.0 :: call
    return item.name + "|" + arity + "|" + extern_flag + "|" + modes + "|" + params + "|" + item.tail.1

fn dep_row(read dep: arcana_compiler_core.types.BytecodeDepFingerprint) -> Str:
    return dep.dep + "|" + dep.fingerprint

export fn module_spec_from_typed(read module: arcana_compiler_core.types.BytecodeModule) -> Str:
    let version = std.text.from_int :: module.head.version :: call
    let mut spec = "kind=module\nversion=" + version
    let mut i = 0
    while i < (module.head.strings :: :: len):
        let row = "string=" + module.head.strings[i]
        spec = spec + "\n" + row
        i += 1
    i = 0
    while i < (module.head.records :: :: len):
        let rec = module.head.records[i]
        let row = "record=" + (arcana_compiler_core.bytecode_writer_typed.record_row :: rec :: call)
        spec = spec + "\n" + row
        i += 1
    i = 0
    while i < (module.tail.functions :: :: len):
        let fun = module.tail.functions[i]
        let sig = module.tail.function_sigs[i]
        let row = "function=" + (arcana_compiler_core.bytecode_writer_typed.function_row :: fun, sig :: call)
        spec = spec + "\n" + row
        let mut ci = 0
        let code = fun.tail.1
        while ci < (code :: :: len):
            let instr = code[ci]
            let code_row = "code=" + (arcana_compiler_core.bytecode_writer_typed.instr_row :: instr :: call)
            spec = spec + "\n" + code_row
            ci += 1
        spec += "\nendfn"
        i += 1
    i = 0
    while i < (module.tail.behaviors :: :: len):
        let item = module.tail.behaviors[i]
        let row = "behavior=" + (arcana_compiler_core.bytecode_writer_typed.behavior_row :: item :: call)
        spec = spec + "\n" + row
        i += 1
    return spec

export fn lib_spec_from_typed(read artifact: arcana_compiler_core.types.BytecodeLibArtifact) -> Str:
    let version = std.text.from_int :: artifact.meta.bytecode_version :: call
    let mut spec = "kind=lib\nbytecode_version=" + version
    spec += "\nstd_abi=" + artifact.meta.std_abi
    let mut i = 0
    while i < (artifact.exports :: :: len):
        let item = artifact.exports[i]
        let row = "export=" + (arcana_compiler_core.bytecode_writer_typed.export_row :: item :: call)
        spec = spec + "\n" + row
        i += 1
    i = 0
    let deps = artifact.tail.0
    while i < (deps :: :: len):
        let dep = deps[i]
        let row = "dep=" + (arcana_compiler_core.bytecode_writer_typed.dep_row :: dep :: call)
        spec = spec + "\n" + row
        i += 1
    let module_spec = arcana_compiler_core.bytecode_writer_typed.module_spec_from_typed :: artifact.tail.1 :: call
    let mut raw = std.text.split_lines :: module_spec :: call
    let mut lines = list.new[Str] :: :: call
    while (raw :: :: len) > 0:
        lines :: (raw :: :: pop) :: push
    while (lines :: :: len) > 0:
        spec = spec + "\n" + (lines :: :: pop)
    return spec

export fn module_hello_fixture() -> arcana_compiler_core.types.BytecodeModule:
    let mut strings = std.collections.array.new[Str] :: 1, "" :: call
    strings[0] = "hello"
    let mut fields = std.collections.array.new[Str] :: 1, "" :: call
    fields[0] = "mana"
    let rec = arcana_compiler_core.types.BytecodeRecordType :: name = "Mage", fields = fields :: call
    let record_fill = arcana_compiler_core.bytecode_writer_typed.empty_record_type :: :: call
    let mut records = std.collections.array.new[arcana_compiler_core.types.BytecodeRecordType] :: 1, record_fill :: call
    records[0] = rec
    let params = std.collections.array.new[Int] :: 0, 0 :: call
    let sig = arcana_compiler_core.types.BytecodeFunctionSig :: params = params, ret = 0 :: call
    let sig_fill = arcana_compiler_core.bytecode_writer_typed.empty_function_sig :: :: call
    let mut sigs = std.collections.array.new[arcana_compiler_core.types.BytecodeFunctionSig] :: 1, sig_fill :: call
    sigs[0] = sig
    let instr_fill = arcana_compiler_core.bytecode_writer_typed.empty_instr :: :: call
    let mut code = std.collections.array.new[arcana_compiler_core.types.BytecodeInstr] :: 3, instr_fill :: call
    code[0] = arcana_compiler_core.types.BytecodeInstr :: tag = 2, a = 0, b = 0 :: call
    code[1] = arcana_compiler_core.types.BytecodeInstr :: tag = 131, a = 76, b = 1 :: call
    code[2] = arcana_compiler_core.types.BytecodeInstr :: tag = 20, a = 0, b = 0 :: call
    let modes = std.collections.array.new[Int] :: 0, 0 :: call
    let fun = arcana_compiler_core.types.BytecodeFunction :: name = "main", meta = (false, (0, modes)), tail = (1, code) :: call
    let function_fill = arcana_compiler_core.bytecode_writer_typed.empty_function :: :: call
    let mut functions = std.collections.array.new[arcana_compiler_core.types.BytecodeFunction] :: 1, function_fill :: call
    functions[0] = fun
    let behavior_fill = arcana_compiler_core.bytecode_writer_typed.empty_behavior :: :: call
    let behaviors = std.collections.array.new[arcana_compiler_core.types.BytecodeBehavior] :: 0, behavior_fill :: call
    let head = arcana_compiler_core.types.BytecodeModuleHead :: version = 29, strings = strings, records = records :: call
    let tail = arcana_compiler_core.types.BytecodeModuleTail :: function_sigs = sigs, functions = functions, behaviors = behaviors :: call
    return arcana_compiler_core.types.BytecodeModule :: head = head, tail = tail :: call

export fn module_behavior_fixture() -> arcana_compiler_core.types.BytecodeModule:
    let strings = std.collections.array.new[Str] :: 0, "" :: call
    let record_fill = arcana_compiler_core.bytecode_writer_typed.empty_record_type :: :: call
    let records = std.collections.array.new[arcana_compiler_core.types.BytecodeRecordType] :: 0, record_fill :: call
    let params = std.collections.array.new[Int] :: 0, 0 :: call
    let sig = arcana_compiler_core.types.BytecodeFunctionSig :: params = params, ret = 0 :: call
    let sig_fill = arcana_compiler_core.bytecode_writer_typed.empty_function_sig :: :: call
    let mut sigs = std.collections.array.new[arcana_compiler_core.types.BytecodeFunctionSig] :: 1, sig_fill :: call
    sigs[0] = sig
    let instr_fill = arcana_compiler_core.bytecode_writer_typed.empty_instr :: :: call
    let mut code = std.collections.array.new[arcana_compiler_core.types.BytecodeInstr] :: 2, instr_fill :: call
    code[0] = arcana_compiler_core.types.BytecodeInstr :: tag = 0, a = 7, b = 0 :: call
    code[1] = arcana_compiler_core.types.BytecodeInstr :: tag = 20, a = 0, b = 0 :: call
    let modes = std.collections.array.new[Int] :: 0, 0 :: call
    let fun = arcana_compiler_core.types.BytecodeFunction :: name = "tick", meta = (false, (0, modes)), tail = (0, code) :: call
    let function_fill = arcana_compiler_core.bytecode_writer_typed.empty_function :: :: call
    let mut functions = std.collections.array.new[arcana_compiler_core.types.BytecodeFunction] :: 1, function_fill :: call
    functions[0] = fun
    let mut component_types = std.collections.array.new[Str] :: 2, "" :: call
    component_types[0] = "Player"
    component_types[1] = "Enemy"
    let mut reads = std.collections.array.new[Int] :: 2, 0 :: call
    reads[0] = 1
    reads[1] = 2
    let mut writes = std.collections.array.new[Int] :: 1, 0 :: call
    writes[0] = 3
    let mut excludes = std.collections.array.new[Int] :: 2, 0 :: call
    excludes[0] = 4
    excludes[1] = 5
    let contract = arcana_compiler_core.types.BytecodeBehaviorContract :: meta = (true, 7), tags = (2, (2, 0)), flags = (true, true) :: call
    let access = arcana_compiler_core.types.BytecodeAccessSets :: reads = reads, writes = writes, excludes = excludes :: call
    let behavior_item = arcana_compiler_core.types.BytecodeBehavior :: name = "tick", meta = (2, (2, 0)), tail = (component_types, (contract, access)) :: call
    let behavior_fill = arcana_compiler_core.bytecode_writer_typed.empty_behavior :: :: call
    let mut behaviors = std.collections.array.new[arcana_compiler_core.types.BytecodeBehavior] :: 1, behavior_fill :: call
    behaviors[0] = behavior_item
    let head = arcana_compiler_core.types.BytecodeModuleHead :: version = 29, strings = strings, records = records :: call
    let tail = arcana_compiler_core.types.BytecodeModuleTail :: function_sigs = sigs, functions = functions, behaviors = behaviors :: call
    return arcana_compiler_core.types.BytecodeModule :: head = head, tail = tail :: call

export fn lib_util_fixture() -> arcana_compiler_core.types.BytecodeLibArtifact:
    let mut strings = std.collections.array.new[Str] :: 1, "" :: call
    strings[0] = "hello"
    let record_fill = arcana_compiler_core.bytecode_writer_typed.empty_record_type :: :: call
    let records = std.collections.array.new[arcana_compiler_core.types.BytecodeRecordType] :: 0, record_fill :: call
    let mut params = std.collections.array.new[Int] :: 1, 0 :: call
    params[0] = 0
    let sig = arcana_compiler_core.types.BytecodeFunctionSig :: params = params, ret = 0 :: call
    let sig_fill = arcana_compiler_core.bytecode_writer_typed.empty_function_sig :: :: call
    let mut sigs = std.collections.array.new[arcana_compiler_core.types.BytecodeFunctionSig] :: 1, sig_fill :: call
    sigs[0] = sig
    let instr_fill = arcana_compiler_core.bytecode_writer_typed.empty_instr :: :: call
    let mut code = std.collections.array.new[arcana_compiler_core.types.BytecodeInstr] :: 2, instr_fill :: call
    code[0] = arcana_compiler_core.types.BytecodeInstr :: tag = 3, a = 0, b = 0 :: call
    code[1] = arcana_compiler_core.types.BytecodeInstr :: tag = 20, a = 0, b = 0 :: call
    let mut modes = std.collections.array.new[Int] :: 1, 0 :: call
    modes[0] = 0
    let fun = arcana_compiler_core.types.BytecodeFunction :: name = "util", meta = (false, (1, modes)), tail = (1, code) :: call
    let function_fill = arcana_compiler_core.bytecode_writer_typed.empty_function :: :: call
    let mut functions = std.collections.array.new[arcana_compiler_core.types.BytecodeFunction] :: 1, function_fill :: call
    functions[0] = fun
    let behavior_fill = arcana_compiler_core.bytecode_writer_typed.empty_behavior :: :: call
    let behaviors = std.collections.array.new[arcana_compiler_core.types.BytecodeBehavior] :: 0, behavior_fill :: call
    let head = arcana_compiler_core.types.BytecodeModuleHead :: version = 29, strings = strings, records = records :: call
    let module_tail = arcana_compiler_core.types.BytecodeModuleTail :: function_sigs = sigs, functions = functions, behaviors = behaviors :: call
    let bc_module = arcana_compiler_core.types.BytecodeModule :: head = head, tail = module_tail :: call
    let mut export_modes = std.collections.array.new[Int] :: 1, 0 :: call
    export_modes[0] = 0
    let mut export_types = std.collections.array.new[Str] :: 1, "" :: call
    export_types[0] = "Int"
    let lib_export = arcana_compiler_core.types.BytecodeLibExport :: name = "util", meta = (1, (false, export_modes)), tail = (export_types, "Int") :: call
    let export_fill = arcana_compiler_core.bytecode_writer_typed.empty_export :: :: call
    let mut exports = std.collections.array.new[arcana_compiler_core.types.BytecodeLibExport] :: 1, export_fill :: call
    exports[0] = lib_export
    let dep = arcana_compiler_core.types.BytecodeDepFingerprint :: dep = "core", fingerprint = "sha256:abc" :: call
    let dep_fill = arcana_compiler_core.bytecode_writer_typed.empty_dep :: :: call
    let mut deps = std.collections.array.new[arcana_compiler_core.types.BytecodeDepFingerprint] :: 1, dep_fill :: call
    deps[0] = dep
    let meta = arcana_compiler_core.types.BytecodeLibMeta :: format_version = 1, bytecode_version = 29, std_abi = "std-abi-v1" :: call
    let tail = (deps, bc_module)
    return arcana_compiler_core.types.BytecodeLibArtifact :: meta = meta, exports = exports, tail = tail :: call
