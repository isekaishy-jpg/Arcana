import arcana_frontend.parse
import arcana_frontend.tokenize
import arcana_frontend.typecheck
import fs_support
import std.collections.list
import std.fs
import std.path

fn collect_arc_files(target: Str) -> List[Str]:
    let mut pending = std.collections.list.new[Str] :: :: call
    let mut files = std.collections.list.new[Str] :: :: call
    pending :: target :: push
    while (pending :: :: len) > 0:
        let path = pending :: :: pop
        if std.fs.is_dir :: path :: call:
            let mut entries = fs_support.list_dir_or_empty :: path :: call
            while (entries :: :: len) > 0:
                pending :: (entries :: :: pop) :: push
            continue
        if (std.path.ext :: path :: call) != "arc":
            continue
        files :: path :: push
    return files

fn check_file(path: Str) -> (Int, Int):
    let text = fs_support.read_text_or :: path, "" :: call
    let tokens = arcana_frontend.tokenize.scan :: path, text :: call
    let parse_result = arcana_frontend.parse.check :: path, text, tokens :: call
    let type_result = arcana_frontend.typecheck.check :: path, text, tokens :: call
    let errors = parse_result.0 + type_result.0
    let mut checksum = 0
    checksum = ((checksum * 131) + parse_result.1 + 7) % 2147483647
    checksum = ((checksum * 131) + type_result.1 + 7) % 2147483647
    return (errors, checksum)

export fn check_target(target: Str) -> (Int, Int):
    let mut files = arcana_frontend.frontend.collect_arc_files :: target :: call
    let mut errors = 0
    let mut checksum = 0
    while (files :: :: len) > 0:
        let file = files :: :: pop
        let file_result = arcana_frontend.frontend.check_file :: file :: call
        errors += file_result.0
        checksum = ((checksum * 131) + file_result.1 + 7) % 2147483647
    return (errors, checksum)
