import std.collections.list
import std.collections.map
import std.result
import std.text
use std.result.Result

record BookManifest:
    name: Str
    kind: Str
    data: Map[Str, Str]

record LockManifestV1:
    version: Int
    workspace: Str
    data: Map[Str, Str]

fn empty_string_list() -> List[Str]:
    return std.collections.list.new[Str] :: :: call

fn trim_ws(read text: Str) -> Str:
    let n = std.text.len_bytes :: text :: call
    let mut start = 0
    while start < n and (std.text.is_space_byte :: (std.text.byte_at :: text, start :: call) :: call):
        start += 1
    let mut end = n
    while end > start and (std.text.is_space_byte :: (std.text.byte_at :: text, end - 1 :: call) :: call):
        end -= 1
    return std.text.slice_bytes :: text, start, end :: call

fn find_quote_end(read text: Str, start: Int) -> Result[Int, Str]:
    let n = std.text.len_bytes :: text :: call
    let mut i = start
    let mut escaped = false
    while i < n:
        let b = std.text.byte_at :: text, i :: call
        if escaped:
            escaped = false
        else:
            if b == 92:
                escaped = true
            else:
                if b == 34:
                    return Result.Ok[Int, Str] :: i :: call
        i += 1
    return Result.Err[Int, Str] :: "unterminated quoted string" :: call

fn strip_comment(read text: Str) -> Result[Str, Str]:
    let n = std.text.len_bytes :: text :: call
    let mut i = 0
    while i < n:
        let b = std.text.byte_at :: text, i :: call
        if b == 34:
            let close = std.manifest.find_quote_end :: text, i + 1 :: call
            if close :: :: is_err:
                return Result.Err[Str, Str] :: "unterminated quoted string" :: call
            let value = close :: 0 :: unwrap_or
            return std.manifest.strip_comment_after_quote :: text, value + 1 :: call
        else:
            if b == 35:
                return Result.Ok[Str, Str] :: (std.manifest.trim_ws :: (std.text.slice_bytes :: text, 0, i :: call) :: call) :: call
        i += 1
    return Result.Ok[Str, Str] :: (std.manifest.trim_ws :: text :: call) :: call

fn strip_comment_after_quote(read text: Str, start: Int) -> Result[Str, Str]:
    let n = std.text.len_bytes :: text :: call
    let mut i = start
    while i < n:
        let b = std.text.byte_at :: text, i :: call
        if b == 34:
            let close = std.manifest.find_quote_end :: text, i + 1 :: call
            if close :: :: is_err:
                return Result.Err[Str, Str] :: "unterminated quoted string" :: call
            let value = close :: 0 :: unwrap_or
            return std.manifest.strip_comment_after_quote :: text, value + 1 :: call
        else:
            if b == 35:
                return Result.Ok[Str, Str] :: (std.manifest.trim_ws :: (std.text.slice_bytes :: text, 0, i :: call) :: call) :: call
        i += 1
    return Result.Ok[Str, Str] :: (std.manifest.trim_ws :: text :: call) :: call

fn decode_quoted_span(read text: Str, start: Int, end: Int) -> Result[Str, Str]:
    let mut out = ""
    let mut i = start
    while i < end:
        let b = std.text.byte_at :: text, i :: call
        if b == 92:
            if i + 1 >= end:
                return Result.Err[Str, Str] :: "unterminated escape sequence" :: call
            let esc = std.text.byte_at :: text, i + 1 :: call
            if esc == 34:
                out = out + "\""
            else:
                if esc == 92:
                    out = out + "\\"
                else:
                    if esc == 110:
                        out = out + "\n"
                    else:
                        if esc == 114:
                            out = out + "\r"
                        else:
                            if esc == 116:
                                out = out + "\t"
                            else:
                                return Result.Err[Str, Str] :: ("unsupported escape sequence `\\" + (std.text.slice_bytes :: text, i + 1, i + 2 :: call) + "`") :: call
            i += 2
        else:
            out = out + (std.text.slice_bytes :: text, i, i + 1 :: call)
            i += 1
    return Result.Ok[Str, Str] :: out :: call

fn decode_quoted_value(read text: Str) -> Result[Str, Str]:
    let t = std.manifest.trim_ws :: text :: call
    let n = std.text.len_bytes :: t :: call
    if n < 2 or (std.text.byte_at :: t, 0 :: call) != 34 or (std.text.byte_at :: t, n - 1 :: call) != 34:
        return Result.Err[Str, Str] :: "expected quoted string" :: call
    return std.manifest.decode_quoted_span :: t, 1, n - 1 :: call

fn trim_or_decode_string(read text: Str) -> Result[Str, Str]:
    let t = std.manifest.trim_ws :: text :: call
    let n = std.text.len_bytes :: t :: call
    if n >= 2 and (std.text.byte_at :: t, 0 :: call) == 34 and (std.text.byte_at :: t, n - 1 :: call) == 34:
        return std.manifest.decode_quoted_value :: t :: call
    return Result.Ok[Str, Str] :: t :: call

fn parse_kv(read line: Str) -> (Bool, (Str, Str)):
    let eq = std.text.find_byte :: line, 0, 61 :: call
    if eq < 0:
        return (false, ("", ""))
    let key = std.manifest.trim_ws :: (std.text.slice_bytes :: line, 0, eq :: call) :: call
    let value = std.manifest.trim_ws :: (std.text.slice_bytes :: line, eq + 1, (std.text.len_bytes :: line :: call) :: call) :: call
    return (true, (key, value))

fn parse_int_or(read text: Str, fallback: Int) -> Int:
    let s = std.manifest.trim_ws :: text :: call
    let n = std.text.len_bytes :: s :: call
    if n == 0:
        return fallback
    let mut sign = 1
    let mut i = 0
    if (std.text.byte_at :: s, 0 :: call) == 45:
        sign = -1
        i = 1
    let mut value = 0
    while i < n:
        let b = std.text.byte_at :: s, i :: call
        if not (std.text.is_digit_byte :: b :: call):
            return fallback
        value = value * 10 + (b - 48)
        i += 1
    return value * sign

fn lookup_required(read data: Map[Str, Str], read key: Str, read label: Str) -> Result[Str, Str]:
    if data :: key :: has:
        return Result.Ok[Str, Str] :: (data :: key :: get) :: call
    return Result.Err[Str, Str] :: ("missing " + label + " `" + key + "`") :: call

fn parse_string_array_literal(read text: Str) -> Result[List[Str], Str]:
    let value = std.manifest.trim_ws :: text :: call
    let n = std.text.len_bytes :: value :: call
    if n < 2 or (std.text.byte_at :: value, 0 :: call) != 91 or (std.text.byte_at :: value, n - 1 :: call) != 93:
        return Result.Err[List[Str], Str] :: "expected string array literal" :: call
    let mut out = std.collections.list.new[Str] :: :: call
    let mut i = 1
    while i < (n - 1):
        while i < (n - 1) and (std.text.is_space_byte :: (std.text.byte_at :: value, i :: call) :: call):
            i += 1
        if i >= (n - 1):
            return Result.Ok[List[Str], Str] :: out :: call
        if (std.text.byte_at :: value, i :: call) != 34:
            return Result.Err[List[Str], Str] :: "expected quoted string in array literal" :: call
        let close = std.manifest.find_quote_end :: value, i + 1 :: call
        if close :: :: is_err:
            return Result.Err[List[Str], Str] :: "unterminated quoted string in array literal" :: call
        let end = close :: 0 :: unwrap_or
        if end >= n:
            return Result.Err[List[Str], Str] :: "unterminated quoted string in array literal" :: call
        let item = std.manifest.decode_quoted_span :: value, i + 1, end :: call
        if item :: :: is_err:
            return Result.Err[List[Str], Str] :: (item :: "" :: unwrap_or) :: call
        out :: (item :: "" :: unwrap_or) :: push
        i = end + 1
        while i < (n - 1) and (std.text.is_space_byte :: (std.text.byte_at :: value, i :: call) :: call):
            i += 1
        if i >= (n - 1):
            return Result.Ok[List[Str], Str] :: out :: call
        if (std.text.byte_at :: value, i :: call) != 44:
            return Result.Err[List[Str], Str] :: "expected comma in array literal" :: call
        i += 1
    return Result.Ok[List[Str], Str] :: out :: call

fn parse_inline_table_string_field(read text: Str, read wanted_key: Str) -> Result[Str, Str]:
    let value = std.manifest.trim_ws :: text :: call
    let n = std.text.len_bytes :: value :: call
    if n >= 2 and (std.text.byte_at :: value, 0 :: call) == 34 and (std.text.byte_at :: value, n - 1 :: call) == 34:
        return std.manifest.decode_quoted_value :: value :: call
    if n < 2 or (std.text.byte_at :: value, 0 :: call) != 123 or (std.text.byte_at :: value, n - 1 :: call) != 125:
        return Result.Err[Str, Str] :: "expected quoted string or inline table" :: call
    let mut i = 1
    while i < (n - 1):
        while i < (n - 1):
            let b = std.text.byte_at :: value, i :: call
            if (std.text.is_space_byte :: b :: call) or b == 44:
                i += 1
            else:
                break
        if i >= (n - 1):
            break
        let key_start = i
        while i < (n - 1):
            let b = std.text.byte_at :: value, i :: call
            if b == 61 or (std.text.is_space_byte :: b :: call):
                break
            i += 1
        let key_end = i
        let key = std.manifest.trim_ws :: (std.text.slice_bytes :: value, key_start, key_end :: call) :: call
        while i < (n - 1) and (std.text.is_space_byte :: (std.text.byte_at :: value, i :: call) :: call):
            i += 1
        if i >= (n - 1) or (std.text.byte_at :: value, i :: call) != 61:
            return Result.Err[Str, Str] :: "expected `=` in inline table" :: call
        i += 1
        while i < (n - 1) and (std.text.is_space_byte :: (std.text.byte_at :: value, i :: call) :: call):
            i += 1
        if i >= (n - 1):
            return Result.Err[Str, Str] :: "missing inline table value" :: call
        let mut field_value = ""
        if (std.text.byte_at :: value, i :: call) == 34:
            let close = std.manifest.find_quote_end :: value, i + 1 :: call
            if close :: :: is_err:
                return Result.Err[Str, Str] :: "unterminated quoted string in inline table" :: call
            let end = close :: 0 :: unwrap_or
            if end >= n:
                return Result.Err[Str, Str] :: "unterminated quoted string in inline table" :: call
            let decoded = std.manifest.decode_quoted_span :: value, i + 1, end :: call
            if decoded :: :: is_err:
                return Result.Err[Str, Str] :: (decoded :: "" :: unwrap_or) :: call
            field_value = decoded :: "" :: unwrap_or
            i = end + 1
        else:
            let value_start = i
            while i < (n - 1) and (std.text.byte_at :: value, i :: call) != 44:
                i += 1
            field_value = std.manifest.trim_ws :: (std.text.slice_bytes :: value, value_start, i :: call) :: call
        if key == wanted_key:
            return Result.Ok[Str, Str] :: field_value :: call
    return Result.Err[Str, Str] :: ("missing inline table field `" + wanted_key + "`") :: call

impl BookManifest:
    fn data_value_or(read self: BookManifest, read key: Str, fallback: Str) -> Str:
        if self.data :: key :: has:
            return self.data :: key :: get
        return fallback

    fn workspace_members(read self: BookManifest) -> Result[List[Str], Str]:
        if not (self.data :: "[workspace].members" :: has):
            return Result.Ok[List[Str], Str] :: (std.manifest.empty_string_list :: :: call) :: call
        return std.manifest.parse_string_array_literal :: (self.data :: "[workspace].members" :: get) :: call

    fn dep_path(read self: BookManifest, dep_name: Str) -> Result[Str, Str]:
        let key = "[deps]." + dep_name
        let raw = std.manifest.lookup_required :: self.data, key, "dependency entry" :: call
        if raw :: :: is_err:
            return Result.Err[Str, Str] :: ("missing dependency entry `" + key + "`") :: call
        let value = raw :: "" :: unwrap_or
        return std.manifest.parse_inline_table_string_field :: value, "path" :: call

impl LockManifestV1:
    fn data_value_or(read self: LockManifestV1, read key: Str, fallback: Str) -> Str:
        if self.data :: key :: has:
            return self.data :: key :: get
        return fallback

    fn order(read self: LockManifestV1) -> Result[List[Str], Str]:
        if not (self.data :: "order" :: has):
            return Result.Ok[List[Str], Str] :: (std.manifest.empty_string_list :: :: call) :: call
        return std.manifest.parse_string_array_literal :: (self.data :: "order" :: get) :: call

    fn deps_for(read self: LockManifestV1, member: Str) -> Result[List[Str], Str]:
        let key = "[deps]." + member
        if not (self.data :: key :: has):
            return Result.Ok[List[Str], Str] :: (std.manifest.empty_string_list :: :: call) :: call
        return std.manifest.parse_string_array_literal :: (self.data :: key :: get) :: call

    fn path_for(read self: LockManifestV1, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_required :: self.data, "[paths]." + member, "lock path entry" :: call

    fn fingerprint_for(read self: LockManifestV1, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_required :: self.data, "[fingerprints]." + member, "lock fingerprint entry" :: call

    fn api_fingerprint_for(read self: LockManifestV1, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_required :: self.data, "[api_fingerprints]." + member, "lock api fingerprint entry" :: call

    fn artifact_for(read self: LockManifestV1, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_required :: self.data, "[artifacts]." + member, "lock artifact entry" :: call

    fn kind_for(read self: LockManifestV1, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_required :: self.data, "[kinds]." + member, "lock kind entry" :: call

    fn format_for(read self: LockManifestV1, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_required :: self.data, "[formats]." + member, "lock format entry" :: call

export fn parse_book(text: Str) -> Result[BookManifest, Str]:
    let mut name = ""
    let mut kind = ""
    let mut section = ""
    let mut data = std.collections.map.new[Str, Str] :: :: call
    let lines = std.text.split_lines :: text :: call
    for raw in lines:
        let line_result = std.manifest.strip_comment :: raw :: call
        if line_result :: :: is_err:
            return Result.Err[BookManifest, Str] :: "unterminated quoted string" :: call
        let line = line_result :: "" :: unwrap_or
        if (std.text.len_bytes :: line :: call) == 0:
            continue
        if std.text.starts_with :: line, "[" :: call:
            if not (std.text.ends_with :: line, "]" :: call):
                return Result.Err[BookManifest, Str] :: ("malformed section header `" + line + "`") :: call
            section = line
            continue
        let kv = std.manifest.parse_kv :: line :: call
        if not kv.0:
            return Result.Err[BookManifest, Str] :: ("malformed manifest line `" + line + "`") :: call
        let key_result = std.manifest.trim_or_decode_string :: kv.1.0 :: call
        if key_result :: :: is_err:
            return Result.Err[BookManifest, Str] :: (key_result :: "" :: unwrap_or) :: call
        let key = key_result :: "" :: unwrap_or
        let value = kv.1.1
        if section == "":
            if key == "name":
                let decoded = std.manifest.trim_or_decode_string :: value :: call
                if decoded :: :: is_err:
                    return Result.Err[BookManifest, Str] :: (decoded :: "" :: unwrap_or) :: call
                name = decoded :: "" :: unwrap_or
            else:
                if key == "kind":
                    let decoded = std.manifest.trim_or_decode_string :: value :: call
                    if decoded :: :: is_err:
                        return Result.Err[BookManifest, Str] :: (decoded :: "" :: unwrap_or) :: call
                    kind = decoded :: "" :: unwrap_or
        let mut full_key = key
        if section != "":
            full_key = section + "." + key
        data :: full_key, value :: set
    let manifest = std.manifest.BookManifest :: name = name, kind = kind, data = data :: call
    return Result.Ok[BookManifest, Str] :: manifest :: call

export fn parse_lock_v1(text: Str) -> Result[LockManifestV1, Str]:
    let mut version = 0
    let mut workspace = ""
    let mut section = ""
    let mut data = std.collections.map.new[Str, Str] :: :: call
    let lines = std.text.split_lines :: text :: call
    for raw in lines:
        let line_result = std.manifest.strip_comment :: raw :: call
        if line_result :: :: is_err:
            return Result.Err[LockManifestV1, Str] :: "unterminated quoted string" :: call
        let line = line_result :: "" :: unwrap_or
        if (std.text.len_bytes :: line :: call) == 0:
            continue
        if std.text.starts_with :: line, "[" :: call:
            if not (std.text.ends_with :: line, "]" :: call):
                return Result.Err[LockManifestV1, Str] :: ("malformed section header `" + line + "`") :: call
            section = line
            continue
        let kv = std.manifest.parse_kv :: line :: call
        if not kv.0:
            return Result.Err[LockManifestV1, Str] :: ("malformed lockfile line `" + line + "`") :: call
        let key_result = std.manifest.trim_or_decode_string :: kv.1.0 :: call
        if key_result :: :: is_err:
            return Result.Err[LockManifestV1, Str] :: (key_result :: "" :: unwrap_or) :: call
        let key = key_result :: "" :: unwrap_or
        let value = kv.1.1
        if section == "":
            if key == "version":
                version = std.manifest.parse_int_or :: value, 0 :: call
            else:
                if key == "workspace":
                    let decoded = std.manifest.trim_or_decode_string :: value :: call
                    if decoded :: :: is_err:
                        return Result.Err[LockManifestV1, Str] :: (decoded :: "" :: unwrap_or) :: call
                    workspace = decoded :: "" :: unwrap_or
        let mut full_key = key
        if section != "":
            full_key = section + "." + key
        data :: full_key, value :: set
    if version != 1:
        return Result.Err[LockManifestV1, Str] :: "Arcana.lock version must be 1" :: call
    let manifest = std.manifest.LockManifestV1 :: version = version, workspace = workspace, data = data :: call
    return Result.Ok[LockManifestV1, Str] :: manifest :: call
