import std.bytes
import std.fs
import std.path

fn ok_bool() -> arcana_compiler_core.types.Outcome[Bool]:
    return arcana_compiler_core.types.Outcome[Bool] :: ok = true, value = true, message = "" :: call

fn err_bool(message: Str) -> arcana_compiler_core.types.Outcome[Bool]:
    return arcana_compiler_core.types.Outcome[Bool] :: ok = false, value = false, message = message :: call

fn valid_bytecode_version(version: Int) -> Bool:
    return version == 28 or version == 29

fn read_u16_at(read bytes: Array[Int], offset: Int) -> Int:
    if (offset + 1) >= (std.bytes.len :: bytes :: call):
        return 0
    let lo = std.bytes.at :: bytes, offset :: call
    let hi = std.bytes.at :: bytes, offset + 1 :: call
    return lo + hi * 256

fn has_magic(read bytes: Array[Int], left: (Int, Int), right: (Int, Int)) -> Bool:
    if (std.bytes.len :: bytes :: call) < 4:
        return false
    if (std.bytes.at :: bytes, 0 :: call) != left.0:
        return false
    if (std.bytes.at :: bytes, 1 :: call) != left.1:
        return false
    if (std.bytes.at :: bytes, 2 :: call) != right.0:
        return false
    if (std.bytes.at :: bytes, 3 :: call) != right.1:
        return false
    return true

fn validate_module_header(read bytes: Array[Int]) -> arcana_compiler_core.types.Outcome[Bool]:
    if (std.bytes.len :: bytes :: call) < 6:
        return err_bool :: "invalid bytecode module header" :: call
    if not (has_magic :: bytes, (65, 82), (67, 66) :: call):
        return err_bool :: "invalid bytecode module header" :: call
    let version = read_u16_at :: bytes, 4 :: call
    if not (valid_bytecode_version :: version :: call):
        return err_bool :: "invalid bytecode module header" :: call
    return ok_bool :: :: call

fn validate_lib_header(read bytes: Array[Int]) -> arcana_compiler_core.types.Outcome[Bool]:
    if (std.bytes.len :: bytes :: call) < 8:
        return err_bool :: "invalid library artifact header" :: call
    if not (has_magic :: bytes, (65, 82), (67, 76) :: call):
        return err_bool :: "invalid library artifact header" :: call
    let format_version = read_u16_at :: bytes, 4 :: call
    if format_version != 1:
        return err_bool :: "invalid library artifact header" :: call
    let bytecode_version = read_u16_at :: bytes, 6 :: call
    if not (valid_bytecode_version :: bytecode_version :: call):
        return err_bool :: "invalid library artifact header" :: call
    return ok_bool :: :: call

export fn roundtrip_file(input_path: Str, output_path: Str) -> arcana_compiler_core.types.Outcome[Bool]:
    let bytes = std.fs.read_bytes_or_empty :: input_path :: call
    if (std.bytes.len :: bytes :: call) <= 0:
        return err_bool :: ("failed to read artifact `" + input_path + "`") :: call
    let ext = std.path.ext :: input_path :: call
    if ext == "arcbc":
        let validated = validate_module_header :: bytes :: call
        if not validated.ok:
            return err_bool :: "failed to validate bytecode module" :: call
        if not (std.fs.write_bytes_or_false :: output_path, bytes :: call):
            return err_bool :: ("failed to write artifact `" + output_path + "`") :: call
        return ok_bool :: :: call
    if ext == "arclib":
        let validated = validate_lib_header :: bytes :: call
        if not validated.ok:
            return err_bool :: "failed to validate library artifact" :: call
        if not (std.fs.write_bytes_or_false :: output_path, bytes :: call):
            return err_bool :: ("failed to write artifact `" + output_path + "`") :: call
        return ok_bool :: :: call
    return err_bool :: ("unsupported artifact extension `" + ext + "`") :: call
