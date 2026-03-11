import std.collections.list
import std.path
import std.text
import lockfile
import protocol
import tokenize
import types

fn split_space_tokens(text: Str) -> List[Str]:
    let mut out = std.collections.list.new[Str] :: :: call
    let n = std.text.len_bytes :: text :: call
    let mut start = 0
    let mut i = 0
    while i <= n:
        let at_end = i == n
        let at_space = false
        let mut sep = at_space
        if not at_end:
            sep = (std.text.byte_at :: text, i :: call) == 32
        if at_end or sep:
            if i > start:
                let token = std.text.slice_bytes :: text, start, i :: call
                out :: token :: push
            start = i + 1
        i += 1
    return out

export fn collect_lines(text: Str) -> List[Str]:
    let mut out = std.collections.list.new[Str] :: :: call
    let mut raw = std.text.split_lines :: text :: call
    while (raw :: :: len) > 0:
        let line = raw :: :: pop
        let trimmed = tokenize.trim_cr :: line :: call
        if (std.text.len_bytes :: trimmed :: call) <= 0:
            continue
        out :: trimmed :: push
    return out

export fn parse_diag_line(line: Str) -> (Bool, types.ParsedDiag):
    let fallback_pos = (1, 1)
    let fallback_loc = ("", fallback_pos)
    let fallback_meta = ("NO_CODE", "error")
    let fallback = types.ParsedDiag :: loc = fallback_loc, meta = fallback_meta, message = line :: call
    let n = std.text.len_bytes :: line :: call
    let mut i = 0
    let mut found = false
    let mut path_end = -1
    let mut line_start = -1
    let mut line_end = -1
    let mut col_start = -1
    let mut col_end = -1
    let mut rest_start = -1

    while i < n:
        if (std.text.byte_at :: line, i :: call) == 58 and (i + 1) < n:
            let b = std.text.byte_at :: line, i + 1 :: call
            if tokenize.is_digit :: b :: call:
                let mut j = i + 1
                while j < n:
                    let d = std.text.byte_at :: line, j :: call
                    if not (tokenize.is_digit :: d :: call):
                        break
                    j += 1
                if j < n and (std.text.byte_at :: line, j :: call) == 58 and (j + 1) < n:
                    let b2 = std.text.byte_at :: line, j + 1 :: call
                    if tokenize.is_digit :: b2 :: call:
                        let mut k = j + 1
                        while k < n:
                            let d2 = std.text.byte_at :: line, k :: call
                            if not (tokenize.is_digit :: d2 :: call):
                                break
                            k += 1
                        if k < n and (std.text.byte_at :: line, k :: call) == 58 and (k + 1) < n and (std.text.byte_at :: line, k + 1 :: call) == 32:
                            found = true
                            path_end = i
                            line_start = i + 1
                            line_end = j
                            col_start = j + 1
                            col_end = k
                            rest_start = k + 2
                            break
        i += 1

    if not found:
        return (false, fallback)

    let path = std.text.slice_bytes :: line, 0, path_end :: call
    let line_text = std.text.slice_bytes :: line, line_start, line_end :: call
    let col_text = std.text.slice_bytes :: line, col_start, col_end :: call
    let line_num_pair = tokenize.parse_int_ascii :: line_text :: call
    let col_num_pair = tokenize.parse_int_ascii :: col_text :: call
    if not line_num_pair.0 or not col_num_pair.0:
        return (false, fallback)
    let mut line_num = line_num_pair.1
    let mut col_num = col_num_pair.1
    if line_num <= 0:
        line_num = 1
    if col_num <= 0:
        col_num = 1

    let rest = std.text.slice_bytes :: line, rest_start, n :: call
    let mut severity = ""
    let mut idx = 0
    if std.text.starts_with :: rest, "error" :: call:
        severity = "error"
        idx = 5
    else:
        if std.text.starts_with :: rest, "warning" :: call:
            severity = "warning"
            idx = 7
        else:
            return (false, fallback)

    let rest_n = std.text.len_bytes :: rest :: call
    let mut code = "NO_CODE"
    if idx < rest_n and (std.text.byte_at :: rest, idx :: call) == 91:
        let close = tokenize.find_byte :: rest, idx + 1, 93 :: call
        if close < 0:
            return (false, fallback)
        code = std.text.slice_bytes :: rest, idx + 1, close :: call
        idx = close + 1

    if (idx + 1) >= rest_n:
        return (false, fallback)
    if (std.text.byte_at :: rest, idx :: call) != 58 or (std.text.byte_at :: rest, idx + 1 :: call) != 32:
        return (false, fallback)
    let message = std.text.slice_bytes :: rest, idx + 2, rest_n :: call
    let pos = (line_num, col_num)
    let loc = (path, pos)
    let meta = (code, severity)
    let parsed = types.ParsedDiag :: loc = loc, meta = meta, message = message :: call
    return (true, parsed)

export fn parse_build_event_line(line: Str) -> (Bool, types.ParsedBuildEvent):
    let fallback = types.ParsedBuildEvent :: member = "", status = "", hash = "" :: call
    let mut offset = -1
    let mut status = ""
    if std.text.starts_with :: line, "built " :: call:
        status = "compiled"
        offset = 6
    else:
        if std.text.starts_with :: line, "cached " :: call:
            status = "cache_hit"
            offset = 7
        else:
            return (false, fallback)

    let n = std.text.len_bytes :: line :: call
    let member_end = tokenize.find_byte :: line, offset, 32 :: call
    if member_end < 0:
        return (false, fallback)
    let member = std.text.slice_bytes :: line, offset, member_end :: call
    let hash_start = member_end + 1
    let prefix_n = std.text.len_bytes :: "sha256:" :: call
    if hash_start >= n:
        return (false, fallback)
    let tail = std.text.slice_bytes :: line, hash_start, n :: call
    if not (std.text.starts_with :: tail, "sha256:" :: call):
        return (false, fallback)
    let hash = std.text.slice_bytes :: line, hash_start + prefix_n, n :: call
    let parsed = types.ParsedBuildEvent :: member = member, status = status, hash = hash :: call
    return (true, parsed)

export fn parse_summary_diag_line(line: Str, default_path: Str) -> (Bool, types.Diag):
    let start = (1, 1)
    let meta = ("ARC-SHCOMP-COMPILE-FAILED", "error")
    let loc = (default_path, start)
    let tail = (start, "selfhost compile failed")
    let fallback = types.Diag :: meta = meta, loc = loc, tail = tail :: call
    if not (std.text.starts_with :: line, "DIAG " :: call):
        return (false, fallback)
    let n = std.text.len_bytes :: line :: call
    let body = std.text.slice_bytes :: line, 5, n :: call
    let mut tokens = split_space_tokens :: body :: call
    if (tokens :: :: len) < 7:
        return (false, fallback)
    let mut tokens_rev = std.collections.list.new[Str] :: :: call
    while (tokens :: :: len) > 0:
        tokens_rev :: (tokens :: :: pop) :: push

    let code_tok = tokens_rev :: :: pop
    let severity_tok = tokens_rev :: :: pop
    let path_tok = tokens_rev :: :: pop
    let line_tok = tokens_rev :: :: pop
    let col_tok = tokens_rev :: :: pop
    let end_line_tok = tokens_rev :: :: pop
    let end_col_tok = tokens_rev :: :: pop

    let code_prefix = "code="
    let sev_prefix = "severity="
    let path_prefix = "path="
    let line_prefix = "line="
    let col_prefix = "column="
    let end_line_prefix = "end_line="
    let end_col_prefix = "end_column="
    if not (std.text.starts_with :: code_tok, code_prefix :: call):
        return (false, fallback)
    if not (std.text.starts_with :: severity_tok, sev_prefix :: call):
        return (false, fallback)
    if not (std.text.starts_with :: path_tok, path_prefix :: call):
        return (false, fallback)
    if not (std.text.starts_with :: line_tok, line_prefix :: call):
        return (false, fallback)
    if not (std.text.starts_with :: col_tok, col_prefix :: call):
        return (false, fallback)
    if not (std.text.starts_with :: end_line_tok, end_line_prefix :: call):
        return (false, fallback)
    if not (std.text.starts_with :: end_col_tok, end_col_prefix :: call):
        return (false, fallback)

    let code = std.text.slice_bytes :: code_tok, (std.text.len_bytes :: code_prefix :: call), (std.text.len_bytes :: code_tok :: call) :: call
    let severity = std.text.slice_bytes :: severity_tok, (std.text.len_bytes :: sev_prefix :: call), (std.text.len_bytes :: severity_tok :: call) :: call
    let path = std.text.slice_bytes :: path_tok, (std.text.len_bytes :: path_prefix :: call), (std.text.len_bytes :: path_tok :: call) :: call
    let line_text = std.text.slice_bytes :: line_tok, (std.text.len_bytes :: line_prefix :: call), (std.text.len_bytes :: line_tok :: call) :: call
    let col_text = std.text.slice_bytes :: col_tok, (std.text.len_bytes :: col_prefix :: call), (std.text.len_bytes :: col_tok :: call) :: call
    let end_line_text = std.text.slice_bytes :: end_line_tok, (std.text.len_bytes :: end_line_prefix :: call), (std.text.len_bytes :: end_line_tok :: call) :: call
    let end_col_text = std.text.slice_bytes :: end_col_tok, (std.text.len_bytes :: end_col_prefix :: call), (std.text.len_bytes :: end_col_tok :: call) :: call

    let parsed_line = tokenize.parse_int_ascii :: line_text :: call
    let parsed_col = tokenize.parse_int_ascii :: col_text :: call
    let parsed_end_line = tokenize.parse_int_ascii :: end_line_text :: call
    let parsed_end_col = tokenize.parse_int_ascii :: end_col_text :: call
    if not parsed_line.0 or not parsed_col.0 or not parsed_end_line.0 or not parsed_end_col.0:
        return (false, fallback)

    let mut line_num = parsed_line.1
    let mut col_num = parsed_col.1
    let mut end_line_num = parsed_end_line.1
    let mut end_col_num = parsed_end_col.1
    if line_num <= 0:
        line_num = 1
    if col_num <= 0:
        col_num = 1
    if end_line_num <= 0:
        end_line_num = line_num
    if end_col_num <= 0:
        end_col_num = col_num
    let start_pos = (line_num, col_num)
    let end_pos = (end_line_num, end_col_num)
    let emitted_meta = (code, severity)
    let emitted_loc = (path, start_pos)
    let emitted_tail = (end_pos, "selfhost summary diagnostic")
    let diag = types.Diag :: meta = emitted_meta, loc = emitted_loc, tail = emitted_tail :: call
    return (true, diag)

export fn ingest_diag_line(line: Str, default_path: Str, state: types.DiagState) -> types.DiagState:
    let mut errors = state.counts.0
    let mut warnings = state.counts.1
    let mut checksum = state.checksum
    let parsed = parse_diag_line :: line :: call
    if parsed.0:
        let diag_path = parsed.1.loc.0
        let diag_line = parsed.1.loc.1.0
        let diag_col = parsed.1.loc.1.1
        let diag_code = parsed.1.meta.0
        let diag_sev = parsed.1.meta.1
        let meta = (diag_code, diag_sev)
        let start = (diag_line, diag_col)
        let loc = (diag_path, start)
        let tail = (start, parsed.1.message)
        let diag = types.Diag :: meta = meta, loc = loc, tail = tail :: call
        protocol.emit_diag :: diag :: call
        if diag_sev == "warning":
            warnings += 1
        else:
            errors += 1
        checksum = protocol.fold_checksum :: checksum, (std.text.len_bytes :: line :: call) :: call
        checksum = protocol.fold_checksum :: checksum, diag_line :: call
        checksum = protocol.fold_checksum :: checksum, diag_col :: call
        checksum = protocol.fold_checksum :: checksum, (std.text.len_bytes :: default_path :: call) :: call
    else:
        let meta = ("ARC-SHCOMP-CHECK-FAILED", "error")
        let start = (1, 1)
        let loc = (default_path, start)
        let tail = (start, line)
        let diag = types.Diag :: meta = meta, loc = loc, tail = tail :: call
        protocol.emit_diag :: diag :: call
        errors += 1
        checksum = protocol.fold_checksum :: checksum, (std.text.len_bytes :: line :: call) :: call
    let counts = (errors, warnings)
    return types.DiagState :: counts = counts, checksum = checksum :: call

export fn ingest_build_line(line: Str, workspace_dir: Str, checksum: Int) -> Int:
    if std.text.starts_with :: line, "built " :: call:
        let n = std.text.len_bytes :: line :: call
        let member_end = tokenize.find_byte :: line, 6, 32 :: call
        if member_end > 6:
            let member = std.text.slice_bytes :: line, 6, member_end :: call
            let artifact_path = std.path.join :: workspace_dir, member :: call
            protocol.emit_build_event :: member, "compiled", artifact_path :: call
            return protocol.fold_checksum :: checksum, n :: call
    else:
        if std.text.starts_with :: line, "cached " :: call:
            let n = std.text.len_bytes :: line :: call
            let member_end = tokenize.find_byte :: line, 7, 32 :: call
            if member_end > 7:
                let member = std.text.slice_bytes :: line, 7, member_end :: call
                let artifact_path = std.path.join :: workspace_dir, member :: call
                protocol.emit_build_event :: member, "cache_hit", artifact_path :: call
                return protocol.fold_checksum :: checksum, n :: call
    let lock_line = lockfile.parse_written_lock_line :: line :: call
    if lock_line.0:
        return protocol.fold_checksum :: checksum, (std.text.len_bytes :: line :: call) :: call
    return checksum

fn index_to_line_col(text: Str, index: Int) -> (Int, Int):
    let n = std.text.len_bytes :: text :: call
    let mut i = 0
    let mut line = 1
    let mut col = 1
    while i < n and i < index:
        let b = std.text.byte_at :: text, i :: call
        if b == 10:
            line += 1
            col = 1
        else:
            col += 1
        i += 1
    return (line, col)

fn emit_parse_diag(path: Str, read meta: (Str, Str), read pos: (Int, Int)):
    let diag_meta = (meta.0, "error")
    let start = (pos.0, pos.1)
    let loc = (path, start)
    let tail = (start, meta.1)
    let diag = types.Diag :: meta = diag_meta, loc = loc, tail = tail :: call
    protocol.emit_diag :: diag :: call

fn find_substring(text: Str, needle: Str) -> Int:
    let n = std.text.len_bytes :: text :: call
    let m = std.text.len_bytes :: needle :: call
    if m <= 0:
        return 0
    if n < m:
        return -1
    let mut i = 0
    while i <= (n - m):
        let part = std.text.slice_bytes :: text, i, i + m :: call
        if part == needle:
            return i
        i += 1
    return -1

export fn validate_parse_structure(path: Str, text: Str) -> (Int, Int):
    let mut errors = 0
    let mut checksum = 0
    let mut lines = parse.collect_lines :: text :: call
    while (lines :: :: len) > 0:
        let line = lines :: :: pop
        checksum = protocol.fold_checksum :: checksum, (std.text.len_bytes :: line :: call) :: call

        if std.text.starts_with :: line, "fn " :: call:
            let open = tokenize.find_byte :: line, 0, 40 :: call
            let close = tokenize.find_byte :: line, 0, 41 :: call
            let colon = tokenize.find_byte :: line, 0, 58 :: call
            if open < 0 or close < 0 or close <= open or colon < 0:
                emit_parse_diag :: path, ("ARC-SHCOMP-COMPILE-FAILED", "malformed function signature"), (1, 1) :: call
                errors += 1
            continue

        if std.text.starts_with :: line, "record " :: call:
            let colon = tokenize.find_byte :: line, 0, 58 :: call
            if colon < 0:
                emit_parse_diag :: path, ("ARC-SHCOMP-COMPILE-FAILED", "record declaration must end with ':'"), (1, 1) :: call
                errors += 1
            continue

        if std.text.starts_with :: line, "enum " :: call:
            let colon = tokenize.find_byte :: line, 0, 58 :: call
            if colon < 0:
                emit_parse_diag :: path, ("ARC-SHCOMP-COMPILE-FAILED", "enum declaration must end with ':'"), (1, 1) :: call
                errors += 1
            continue
    return (errors, checksum)
