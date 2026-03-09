import arcana_frontend.frontend
import std.fs
import std.path
import std.text

export fn derive_check_target(path: Str) -> Str:
    if not (std.fs.is_file :: path :: call):
        return path
    let parent = std.path.parent :: path :: call
    let parent_name = std.path.file_name :: parent :: call
    if parent_name == "src":
        let root = std.path.parent :: parent :: call
        if (std.text.len_bytes :: root :: call) > 0:
            return root
    return path

export fn validate_target_with_frontend(target: Str) -> (Int, Int):
    return arcana_frontend.frontend.check_target :: target :: call

export fn validate_semantics_target(path: Str) -> (Int, Int):
    let target = arcana_compiler_core.semantics.derive_check_target :: path :: call
    let mut checksum = ((0 * 131) + (std.text.len_bytes :: path :: call) + 7) % 2147483647
    checksum = ((checksum * 131) + (std.text.len_bytes :: target :: call) + 7) % 2147483647
    let sem = arcana_compiler_core.semantics.validate_target_with_frontend :: target :: call
    checksum = ((checksum * 131) + sem.1 + 7) % 2147483647
    return (sem.0, checksum)
