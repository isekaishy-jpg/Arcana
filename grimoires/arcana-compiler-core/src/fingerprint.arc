import std.bytes
import std.collections.list
import std.fs
import std.path
import std.text

fn fold(acc: Int, delta: Int) -> Int:
    return ((acc * 131) + delta + 7) % 2147483647

fn fold_text(acc: Int, text: Str) -> Int:
    let mut out = acc
    let n = std.text.len_bytes :: text :: call
    let mut i = 0
    while i < n:
        out = arcana_compiler_core.fingerprint.fold :: out, (std.text.byte_at :: text, i :: call) :: call
        i += 1
    return out

fn fold_bytes(acc: Int, read bytes: Array[Int]) -> Int:
    let mut out = acc
    let n = std.bytes.len :: bytes :: call
    let mut i = 0
    while i < n:
        out = arcana_compiler_core.fingerprint.fold :: out, (std.bytes.at :: bytes, i :: call) :: call
        i += 1
    return out

fn collect_arc_rel_paths(dir: Str, rel_prefix: Str) -> List[Str]:
    let mut out = std.collections.list.new[Str] :: :: call
    let mut names = std.fs.list_dir_or_empty :: dir :: call
    let mut names_rev = std.collections.list.new[Str] :: :: call
    while (names :: :: len) > 0:
        names_rev :: (names :: :: pop) :: push
    while (names_rev :: :: len) > 0:
        let name = names_rev :: :: pop
        let child = std.path.join :: dir, name :: call
        let mut rel = name
        if (std.text.len_bytes :: rel_prefix :: call) > 0:
            rel = rel_prefix + "/" + name
        if std.fs.is_dir :: child :: call:
            let sub = arcana_compiler_core.fingerprint.collect_arc_rel_paths :: child, rel :: call
            let mut sub_scan = sub
            let mut sub_rev = std.collections.list.new[Str] :: :: call
            while (sub_scan :: :: len) > 0:
                sub_rev :: (sub_scan :: :: pop) :: push
            while (sub_rev :: :: len) > 0:
                out :: (sub_rev :: :: pop) :: push
        else:
            if std.fs.is_file :: child :: call:
                if (std.path.ext :: child :: call) == "arc":
                    out :: rel :: push
    return out

export fn member_source_hash_or_zero(member_dir: Str) -> Int:
    let mut checksum = 0
    let manifest_path = std.path.join :: member_dir, "book.toml" :: call
    let manifest_text = std.fs.read_text_or :: manifest_path, "" :: call
    checksum = arcana_compiler_core.fingerprint.fold_text :: checksum, "manifest\n" :: call
    checksum = arcana_compiler_core.fingerprint.fold_text :: checksum, manifest_text :: call
    checksum = arcana_compiler_core.fingerprint.fold_text :: checksum, "\n" :: call

    let src_dir = std.path.join :: member_dir, "src" :: call
    if not (std.fs.is_dir :: src_dir :: call):
        return checksum
    let files = arcana_compiler_core.fingerprint.collect_arc_rel_paths :: src_dir, "" :: call
    let mut scan = files
    let mut rev = std.collections.list.new[Str] :: :: call
    while (scan :: :: len) > 0:
        rev :: (scan :: :: pop) :: push
    while (rev :: :: len) > 0:
        let rel = rev :: :: pop
        let full = std.path.join :: src_dir, rel :: call
        let bytes = std.fs.read_bytes_or_empty :: full :: call
        checksum = arcana_compiler_core.fingerprint.fold_text :: checksum, "file:" :: call
        checksum = arcana_compiler_core.fingerprint.fold_text :: checksum, rel :: call
        checksum = arcana_compiler_core.fingerprint.fold_text :: checksum, "\n" :: call
        checksum = arcana_compiler_core.fingerprint.fold_bytes :: checksum, bytes :: call
        checksum = arcana_compiler_core.fingerprint.fold_text :: checksum, "\n" :: call
    return checksum

export fn member_source_fingerprint(member_dir: Str) -> Str:
    let hash = arcana_compiler_core.fingerprint.member_source_hash_or_zero :: member_dir :: call
    return "fold_" + (std.text.from_int :: hash :: call)

export fn workspace_sources_hash_or_zero(workspace_dir: Str) -> Int:
    let mut checksum = 0
    let book_path = std.path.join :: workspace_dir, "book.toml" :: call
    let book_text = std.fs.read_text_or :: book_path, "" :: call
    checksum = arcana_compiler_core.fingerprint.fold_text :: checksum, book_text :: call
    let mut members = std.fs.list_dir_or_empty :: workspace_dir :: call
    let mut members_rev = std.collections.list.new[Str] :: :: call
    while (members :: :: len) > 0:
        members_rev :: (members :: :: pop) :: push
    while (members_rev :: :: len) > 0:
        let name = members_rev :: :: pop
        let member_dir = std.path.join :: workspace_dir, name :: call
        if not (std.fs.is_dir :: member_dir :: call):
            continue
        let member_hash = arcana_compiler_core.fingerprint.member_source_hash_or_zero :: member_dir :: call
        checksum = arcana_compiler_core.fingerprint.fold_text :: checksum, name :: call
        checksum = arcana_compiler_core.fingerprint.fold :: checksum, member_hash :: call
    return checksum
