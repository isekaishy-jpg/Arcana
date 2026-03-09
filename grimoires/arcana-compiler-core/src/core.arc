import std.collections.list
import std.fs
import std.path
import std.text
import arcana_frontend.frontend

export fn trim_ws(text: Str) -> Str:
    let n = std.text.len_bytes :: text :: call
    let mut start = 0
    while start < n and (std.text.is_space_byte :: (std.text.byte_at :: text, start :: call) :: call):
        start += 1
    let mut end = n
    while end > start and (std.text.is_space_byte :: (std.text.byte_at :: text, end - 1 :: call) :: call):
        end -= 1
    return std.text.slice_bytes :: text, start, end :: call

export fn strip_quotes(text: Str) -> Str:
    let t = arcana_compiler_core.core.trim_ws :: text :: call
    let n = std.text.len_bytes :: t :: call
    if n >= 2 and (std.text.byte_at :: t, 0 :: call) == 34 and (std.text.byte_at :: t, n - 1 :: call) == 34:
        return std.text.slice_bytes :: t, 1, n - 1 :: call
    return t

export fn collect_lines(text: Str) -> List[Str]:
    let mut out = std.collections.list.new[Str] :: :: call
    let mut raw = std.text.split_lines :: text :: call
    while (raw :: :: len) > 0:
        let line = raw :: :: pop
        let n = std.text.len_bytes :: line :: call
        if n <= 0:
            continue
        let mut trimmed = line
        if (std.text.byte_at :: line, n - 1 :: call) == 13:
            trimmed = std.text.slice_bytes :: line, 0, n - 1 :: call
        if (std.text.len_bytes :: trimmed :: call) <= 0:
            continue
        out :: trimmed :: push
    return out

fn parse_members_value(value: Str) -> List[Str]:
    let t = arcana_compiler_core.core.trim_ws :: value :: call
    let n = std.text.len_bytes :: t :: call
    let mut out = std.collections.list.new[Str] :: :: call
    if n < 2:
        return out
    if (std.text.byte_at :: t, 0 :: call) != 91 or (std.text.byte_at :: t, n - 1 :: call) != 93:
        return out
    let body = std.text.slice_bytes :: t, 1, n - 1 :: call
    let body_n = std.text.len_bytes :: body :: call
    let mut start = 0
    let mut i = 0
    while i <= body_n:
        if i == body_n or (std.text.byte_at :: body, i :: call) == 44:
            let part = std.text.slice_bytes :: body, start, i :: call
            let item = arcana_compiler_core.core.strip_quotes :: part :: call
            if (std.text.len_bytes :: item :: call) > 0:
                out :: item :: push
            start = i + 1
        i += 1
    return out

export fn parse_root_members(book_text: Str) -> List[Str]:
    let mut lines = arcana_compiler_core.core.collect_lines :: book_text :: call
    let mut collecting = false
    let mut collected = ""
    while (lines :: :: len) > 0:
        let raw_line = lines :: :: pop
        let line = arcana_compiler_core.core.trim_ws :: raw_line :: call
        if collecting:
            collected = collected + line
            if (std.text.find_byte :: line, 0, 93 :: call) >= 0:
                return arcana_compiler_core.core.parse_members_value :: collected :: call
            continue
        if not (std.text.starts_with :: line, "members" :: call):
            continue
        let eq = std.text.find_byte :: line, 0, 61 :: call
        if eq < 0:
            continue
        let rhs = std.text.slice_bytes :: line, eq + 1, (std.text.len_bytes :: line :: call) :: call
        collected = rhs
        if (std.text.find_byte :: rhs, 0, 93 :: call) >= 0:
            return arcana_compiler_core.core.parse_members_value :: rhs :: call
        collecting = true
    return std.collections.list.new[Str] :: :: call

export fn parse_deps_value(book_text: Str) -> List[Str]:
    let mut out = std.collections.list.new[Str] :: :: call
    let mut in_deps = false
    let mut lines = arcana_compiler_core.core.collect_lines :: book_text :: call
    while (lines :: :: len) > 0:
        let line = arcana_compiler_core.core.trim_ws :: (lines :: :: pop) :: call
        let n = std.text.len_bytes :: line :: call
        if n <= 0:
            continue
        if (std.text.byte_at :: line, 0 :: call) == 91:
            in_deps = (line == "[deps]")
            continue
        if not in_deps:
            continue
        let eq = std.text.find_byte :: line, 0, 61 :: call
        if eq <= 0:
            continue
        let key = arcana_compiler_core.core.trim_ws :: (std.text.slice_bytes :: line, 0, eq :: call) :: call
        if (std.text.len_bytes :: key :: call) > 0:
            out :: key :: push
    return out

export fn parse_workspace_name(book_text: Str) -> Str:
    let mut lines = arcana_compiler_core.core.collect_lines :: book_text :: call
    while (lines :: :: len) > 0:
        let line = arcana_compiler_core.core.trim_ws :: (lines :: :: pop) :: call
        if std.text.starts_with :: line, "name" :: call:
            let eq = std.text.find_byte :: line, 0, 61 :: call
            if eq >= 0:
                let rhs = std.text.slice_bytes :: line, eq + 1, (std.text.len_bytes :: line :: call) :: call
                return arcana_compiler_core.core.strip_quotes :: rhs :: call
    return ""

export fn parse_member_name(book_text: Str) -> Str:
    let mut lines = arcana_compiler_core.core.collect_lines :: book_text :: call
    while (lines :: :: len) > 0:
        let line = arcana_compiler_core.core.trim_ws :: (lines :: :: pop) :: call
        if std.text.starts_with :: line, "name" :: call:
            let eq = std.text.find_byte :: line, 0, 61 :: call
            if eq >= 0:
                let rhs = std.text.slice_bytes :: line, eq + 1, (std.text.len_bytes :: line :: call) :: call
                return arcana_compiler_core.core.strip_quotes :: rhs :: call
    return ""

export fn parse_member_kind(book_text: Str) -> Str:
    let mut lines = arcana_compiler_core.core.collect_lines :: book_text :: call
    while (lines :: :: len) > 0:
        let line = arcana_compiler_core.core.trim_ws :: (lines :: :: pop) :: call
        if std.text.starts_with :: line, "kind" :: call:
            let eq = std.text.find_byte :: line, 0, 61 :: call
            if eq >= 0:
                let rhs = std.text.slice_bytes :: line, eq + 1, (std.text.len_bytes :: line :: call) :: call
                return arcana_compiler_core.core.strip_quotes :: rhs :: call
    return ""

export fn list_has(read values: List[Str], read needle: Str) -> Bool:
    let mut scan = values
    let mut restore = std.collections.list.new[Str] :: :: call
    let mut found = false
    while (scan :: :: len) > 0:
        let item = scan :: :: pop
        if item == needle:
            found = true
        restore :: item :: push
    while (restore :: :: len) > 0:
        scan :: (restore :: :: pop) :: push
    return found

export fn row_name(read row: Str) -> Str:
    let sep = std.text.find_byte :: row, 0, 124 :: call
    if sep < 0:
        return row
    return std.text.slice_bytes :: row, 0, sep :: call

export fn row_deps(read row: Str) -> List[Str]:
    let mut out = std.collections.list.new[Str] :: :: call
    let sep = std.text.find_byte :: row, 0, 124 :: call
    if sep < 0:
        return out
    let n = std.text.len_bytes :: row :: call
    if (sep + 1) >= n:
        return out
    let body = std.text.slice_bytes :: row, sep + 1, n :: call
    let body_n = std.text.len_bytes :: body :: call
    let mut start = 0
    let mut i = 0
    while i <= body_n:
        if i == body_n or (std.text.byte_at :: body, i :: call) == 44:
            let part = arcana_compiler_core.core.trim_ws :: (std.text.slice_bytes :: body, start, i :: call) :: call
            if (std.text.len_bytes :: part :: call) > 0:
                out :: part :: push
            start = i + 1
        i += 1
    return out

export fn row_encode(name: Str, read deps: List[Str]) -> Str:
    let mut out = name + "|"
    let mut rev = std.collections.list.new[Str] :: :: call
    let mut scan = deps
    while (scan :: :: len) > 0:
        rev :: (scan :: :: pop) :: push
    let mut first = true
    while (rev :: :: len) > 0:
        let dep = rev :: :: pop
        if not first:
            out = out + ","
        out = out + dep
        first = false
    return out

export fn member_meta_encode(name: Str, rel: Str, kind: Str) -> Str:
    return name + "|" + rel + "|" + kind

export fn member_meta_name(read row: Str) -> Str:
    let p0 = std.text.find_byte :: row, 0, 124 :: call
    if p0 < 0:
        return row
    return std.text.slice_bytes :: row, 0, p0 :: call

export fn member_meta_rel(read row: Str) -> Str:
    let p0 = std.text.find_byte :: row, 0, 124 :: call
    if p0 < 0:
        return ""
    let p1 = std.text.find_byte :: row, p0 + 1, 124 :: call
    if p1 < 0:
        return ""
    return std.text.slice_bytes :: row, p0 + 1, p1 :: call

export fn member_meta_kind(read row: Str) -> Str:
    let p0 = std.text.find_byte :: row, 0, 124 :: call
    if p0 < 0:
        return ""
    let p1 = std.text.find_byte :: row, p0 + 1, 124 :: call
    if p1 < 0:
        return ""
    let n = std.text.len_bytes :: row :: call
    return std.text.slice_bytes :: row, p1 + 1, n :: call

export fn find_member_meta_row(read rows: List[Str], read name: Str) -> Str:
    let mut scan = rows
    let mut restore = std.collections.list.new[Str] :: :: call
    let mut found = ""
    while (scan :: :: len) > 0:
        let row = scan :: :: pop
        if ((std.text.len_bytes :: found :: call) <= 0) and ((arcana_compiler_core.core.member_meta_name :: row :: call) == name):
            found = row
        restore :: row :: push
    while (restore :: :: len) > 0:
        scan :: (restore :: :: pop) :: push
    return found

export fn format_dep_row(name: Str, read deps: List[Str]) -> Str:
    let mut out = "\"" + name + "\" = ["
    let mut scan = deps
    let mut rev = std.collections.list.new[Str] :: :: call
    while (scan :: :: len) > 0:
        rev :: (scan :: :: pop) :: push
    let mut first = true
    while (rev :: :: len) > 0:
        let dep = rev :: :: pop
        if not first:
            out = out + ", "
        out = out + "\"" + dep + "\""
        first = false
    out = out + "]"
    return out

fn deps_ready(read deps: List[Str], read planned: List[Str], read members: List[Str]) -> Bool:
    let mut scan = deps
    while (scan :: :: len) > 0:
        let dep = scan :: :: pop
        if (arcana_compiler_core.core.list_has :: members, dep :: call) and (not (arcana_compiler_core.core.list_has :: planned, dep :: call)):
            return false
    return true

export fn topo_from_rows(read rows: List[Str]) -> List[Str]:
    let mut planned = std.collections.list.new[Str] :: :: call
    let mut members = std.collections.list.new[Str] :: :: call
    let mut pending = rows

    let mut init_rev = std.collections.list.new[Str] :: :: call
    while (pending :: :: len) > 0:
        init_rev :: (pending :: :: pop) :: push
    while (init_rev :: :: len) > 0:
        let row = init_rev :: :: pop
        let name = arcana_compiler_core.core.row_name :: row :: call
        if not (arcana_compiler_core.core.list_has :: members, name :: call):
            members :: name :: push
        pending :: row :: push

    while true:
        if (pending :: :: len) <= 0:
            return planned
        let mut pending_rev = std.collections.list.new[Str] :: :: call
        while (pending :: :: len) > 0:
            pending_rev :: (pending :: :: pop) :: push
        let mut next_pending = std.collections.list.new[Str] :: :: call
        let mut progress = false
        while (pending_rev :: :: len) > 0:
            let row = pending_rev :: :: pop
            let name = arcana_compiler_core.core.row_name :: row :: call
            if arcana_compiler_core.core.list_has :: planned, name :: call:
                continue
            let deps = arcana_compiler_core.core.row_deps :: row :: call
            if arcana_compiler_core.core.deps_ready :: deps, planned, members :: call:
                planned :: name :: push
                progress = true
            else:
                next_pending :: row :: push
        if progress:
            pending = next_pending
            continue
        let mut remain_rev = std.collections.list.new[Str] :: :: call
        while (next_pending :: :: len) > 0:
            remain_rev :: (next_pending :: :: pop) :: push
        while (remain_rev :: :: len) > 0:
            let row = remain_rev :: :: pop
            let name = arcana_compiler_core.core.row_name :: row :: call
            if not (arcana_compiler_core.core.list_has :: planned, name :: call):
                planned :: name :: push
        return planned
    return planned

export fn has_flag(read extra: List[Str], flag: Str) -> Bool:
    let mut values = extra
    let mut restore = std.collections.list.new[Str] :: :: call
    let mut found = false
    while (values :: :: len) > 0:
        let item = values :: :: pop
        if item == flag:
            found = true
        restore :: item :: push
    while (restore :: :: len) > 0:
        values :: (restore :: :: pop) :: push
    return found

export fn parse_member_flag(read extra: List[Str]) -> Str:
    let mut seq = extra
    let mut rev = std.collections.list.new[Str] :: :: call
    while (seq :: :: len) > 0:
        rev :: (seq :: :: pop) :: push
    let mut pending_member = false
    while (rev :: :: len) > 0:
        let item = rev :: :: pop
        if pending_member:
            return item
        if item == "--member":
            pending_member = true
    return ""

export fn parse_flag_value(read extra: List[Str], flag: Str) -> Str:
    let mut seq = extra
    let mut rev = std.collections.list.new[Str] :: :: call
    while (seq :: :: len) > 0:
        rev :: (seq :: :: pop) :: push
    let mut pending = false
    while (rev :: :: len) > 0:
        let item = rev :: :: pop
        if pending:
            return item
        if item == flag:
            pending = true
    return ""

export fn build_set_for_target(read deps_rows: List[Str], target: Str) -> List[Str]:
    let mut selected = std.collections.list.new[Str] :: :: call
    if (std.text.len_bytes :: target :: call) <= 0:
        let mut scan = deps_rows
        let mut rev = std.collections.list.new[Str] :: :: call
        while (scan :: :: len) > 0:
            rev :: (scan :: :: pop) :: push
        while (rev :: :: len) > 0:
            let row = rev :: :: pop
            let name = arcana_compiler_core.core.row_name :: row :: call
            if not (arcana_compiler_core.core.list_has :: selected, name :: call):
                selected :: name :: push
        return selected

    let mut stack = std.collections.list.new[Str] :: :: call
    stack :: target :: push
    while (stack :: :: len) > 0:
        let name = stack :: :: pop
        if arcana_compiler_core.core.list_has :: selected, name :: call:
            continue
        let row = arcana_compiler_core.core.find_member_meta_row :: deps_rows, name :: call
        selected :: name :: push
        if (std.text.len_bytes :: row :: call) <= 0:
            continue
        let mut deps = arcana_compiler_core.core.row_deps :: row :: call
        while (deps :: :: len) > 0:
            stack :: (deps :: :: pop) :: push
    return selected

export fn artifact_ext_for_kind(kind: Str) -> Str:
    if kind == "lib":
        return "arclib"
    return "arcbc"

export fn artifact_rel_path(member_name: Str, fingerprint: Str, ext: Str) -> Str:
    return ".arcana/artifacts/" + member_name + "/" + fingerprint + "." + ext

export fn lock_fingerprint_row(name: Str, fingerprint: Str) -> Str:
    return "\"" + name + "\" = \"" + fingerprint + "\""

export fn lock_path_row(name: Str, rel: Str) -> Str:
    return "\"" + name + "\" = \"" + rel + "\""

export fn lock_artifact_row(name: Str, rel: Str) -> Str:
    return "\"" + name + "\" = \"" + rel + "\""

export fn copy_text(read text: Str) -> Str:
    let n = std.text.len_bytes :: text :: call
    return std.text.slice_bytes :: text, 0, n :: call
