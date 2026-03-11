import std.collections.list
import std.fs
import std.path
import std.text
import arcana_compiler_core.core
import fs_support

fn read_workspace_book_text(workspace_dir: Str) -> Str:
    let book_path = std.path.join :: workspace_dir, "book.toml" :: call
    return fs_support.read_text_or :: book_path, "" :: call

fn workspace_member_rels(workspace_dir: Str) -> List[Str]:
    let book_text = read_workspace_book_text :: workspace_dir :: call
    if (std.text.len_bytes :: book_text :: call) <= 0:
        return std.collections.list.new[Str] :: :: call
    return arcana_compiler_core.core.parse_root_members :: book_text :: call

export fn render_name_list(read names: List[Str]) -> Str:
    let mut out = "["
    let mut scan = names
    let mut rev = std.collections.list.new[Str] :: :: call
    while (scan :: :: len) > 0:
        rev :: (scan :: :: pop) :: push
    let mut first = true
    while (rev :: :: len) > 0:
        let name = rev :: :: pop
        if not first:
            out = out + ", "
        out = out + "\"" + name + "\""
        first = false
    out = out + "]"
    return out

export fn render_row_lines(read rows: List[Str]) -> Str:
    let mut out = ""
    let mut scan = rows
    let mut rev = std.collections.list.new[Str] :: :: call
    while (scan :: :: len) > 0:
        rev :: (scan :: :: pop) :: push
    while (rev :: :: len) > 0:
        out = out + (rev :: :: pop) + "\n"
    return out

export fn workspace_name_or_default(workspace_dir: Str) -> Str:
    let book_text = read_workspace_book_text :: workspace_dir :: call
    if (std.text.len_bytes :: book_text :: call) <= 0:
        return ""
    let name = arcana_compiler_core.core.parse_workspace_name :: book_text :: call
    if (std.text.len_bytes :: name :: call) <= 0:
        return "workspace"
    return name

export fn workspace_dep_rows(workspace_dir: Str) -> List[Str]:
    let mut out = std.collections.list.new[Str] :: :: call
    let mut rel_members = workspace_member_rels :: workspace_dir :: call
    let mut members_rev = std.collections.list.new[Str] :: :: call
    while (rel_members :: :: len) > 0:
        members_rev :: (rel_members :: :: pop) :: push
    while (members_rev :: :: len) > 0:
        let rel = members_rev :: :: pop
        let member_dir = std.path.normalize :: (std.path.join :: workspace_dir, rel :: call) :: call
        let member_book = std.path.join :: member_dir, "book.toml" :: call
        let member_text = fs_support.read_text_or :: member_book, "" :: call
        if (std.text.len_bytes :: member_text :: call) <= 0:
            continue
        let member_name = arcana_compiler_core.core.parse_member_name :: member_text :: call
        if (std.text.len_bytes :: member_name :: call) <= 0:
            continue
        let deps = arcana_compiler_core.core.parse_deps_value :: member_text :: call
        let deps_row = arcana_compiler_core.core.row_encode :: member_name, deps :: call
        out :: deps_row :: push
    return out

export fn workspace_meta_rows(workspace_dir: Str) -> List[Str]:
    let mut out = std.collections.list.new[Str] :: :: call
    let mut rel_members = workspace_member_rels :: workspace_dir :: call
    let mut members_rev = std.collections.list.new[Str] :: :: call
    while (rel_members :: :: len) > 0:
        members_rev :: (rel_members :: :: pop) :: push
    while (members_rev :: :: len) > 0:
        let rel = members_rev :: :: pop
        let member_dir = std.path.normalize :: (std.path.join :: workspace_dir, rel :: call) :: call
        let member_book = std.path.join :: member_dir, "book.toml" :: call
        let member_text = fs_support.read_text_or :: member_book, "" :: call
        if (std.text.len_bytes :: member_text :: call) <= 0:
            continue
        let member_name = arcana_compiler_core.core.parse_member_name :: member_text :: call
        let member_kind = arcana_compiler_core.core.parse_member_kind :: member_text :: call
        if (std.text.len_bytes :: member_name :: call) <= 0:
            continue
        let meta_row = arcana_compiler_core.core.member_meta_encode :: member_name, rel, member_kind :: call
        out :: meta_row :: push
    return out

export fn resolve_workspace_plan_names(workspace_dir: Str) -> List[Str]:
    let rows = arcana_compiler_core.workspace.workspace_dep_rows :: workspace_dir :: call
    return arcana_compiler_core.core.topo_from_rows :: rows :: call

export fn write_lock_text(workspace_dir: Str, text: Str) -> Bool:
    let lock_path = std.path.join :: workspace_dir, "Arcana.lock" :: call
    return fs_support.write_text_or_false :: lock_path, text :: call
