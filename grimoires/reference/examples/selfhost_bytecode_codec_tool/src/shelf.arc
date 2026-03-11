import std.args
import std.bytes
import std.fs
import std.path
import fs_support

fn read_u16_at(read bytes: Array[Int], offset: Int) -> Int:
    if (offset + 1) >= (std.bytes.len :: bytes :: call):
        return 0
    let lo = std.bytes.at :: bytes, offset :: call
    let hi = std.bytes.at :: bytes, offset + 1 :: call
    return lo + hi * 256

fn valid_artifact(read bytes: Array[Int], ext: Str) -> Bool:
    if ext == "arcbc":
        if (std.bytes.len :: bytes :: call) < 6:
            return false
        if (std.bytes.at :: bytes, 0 :: call) != 65:
            return false
        if (std.bytes.at :: bytes, 1 :: call) != 82:
            return false
        if (std.bytes.at :: bytes, 2 :: call) != 67:
            return false
        if (std.bytes.at :: bytes, 3 :: call) != 66:
            return false
        let version = read_u16_at :: bytes, 4 :: call
        return version == 28 or version == 29
    if ext == "arclib":
        if (std.bytes.len :: bytes :: call) < 8:
            return false
        if (std.bytes.at :: bytes, 0 :: call) != 65:
            return false
        if (std.bytes.at :: bytes, 1 :: call) != 82:
            return false
        if (std.bytes.at :: bytes, 2 :: call) != 67:
            return false
        if (std.bytes.at :: bytes, 3 :: call) != 76:
            return false
        let format_version = read_u16_at :: bytes, 4 :: call
        if format_version != 1:
            return false
        let bytecode_version = read_u16_at :: bytes, 6 :: call
        return bytecode_version == 28 or bytecode_version == 29
    return false

fn main() -> Int:
    if (std.args.count :: :: call) < 2:
        return 1
    let input_path = std.args.get :: 0 :: call
    let output_path = std.args.get :: 1 :: call
    let bytes = fs_support.read_bytes_or_empty :: input_path :: call
    let ext = std.path.ext :: input_path :: call
    if not (valid_artifact :: bytes, ext :: call):
        return 1
    if not (fs_support.write_bytes_or_false :: output_path, bytes :: call):
        return 1
    return 0
