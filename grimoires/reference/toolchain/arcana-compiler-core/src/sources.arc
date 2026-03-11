import std.collections.list
import std.fs
import std.path
import std.text
import arcana_compiler_core.core
import fs_support

export fn collect_compile_sources(read source_path: Str) -> List[Str]:
    let mut out = std.collections.list.new[Str] :: :: call
    let source_for_dir = arcana_compiler_core.core.copy_text :: source_path :: call
    if not (std.fs.is_dir :: source_for_dir :: call):
        let source_for_file = arcana_compiler_core.core.copy_text :: source_path :: call
        if std.fs.is_file :: source_for_file :: call:
            let source_for_ext = arcana_compiler_core.core.copy_text :: source_path :: call
            if (std.path.ext :: source_for_ext :: call) == "arc":
                out :: (arcana_compiler_core.core.copy_text :: source_path :: call) :: push
        return out
    let src_dir = std.path.join :: source_for_dir, "src" :: call
    let src_dir_for_check = arcana_compiler_core.core.copy_text :: src_dir :: call
    if not (std.fs.is_dir :: src_dir_for_check :: call):
        return out

    let mut pending = std.collections.list.new[Str] :: :: call
    pending :: src_dir :: push
    while (pending :: :: len) > 0:
        let path = pending :: :: pop
        if std.fs.is_dir :: path :: call:
            let mut entries = fs_support.list_dir_or_empty :: path :: call
            let mut entries_rev = std.collections.list.new[Str] :: :: call
            while (entries :: :: len) > 0:
                entries_rev :: (entries :: :: pop) :: push
            while (entries_rev :: :: len) > 0:
                pending :: (entries_rev :: :: pop) :: push
            continue
        if std.fs.is_file :: path :: call:
            if (std.path.ext :: path :: call) == "arc":
                out :: path :: push
    return out

export fn count_files_and_bytes(target: Str) -> (Int, Int):
    let mut files = arcana_compiler_core.sources.collect_compile_sources :: target :: call
    let mut count = 0
    let mut total_bytes = 0
    while (files :: :: len) > 0:
        let path = files :: :: pop
        count += 1
        let text = fs_support.read_text_or :: path, "" :: call
        total_bytes += std.text.len_bytes :: text :: call
    return (count, total_bytes)
