import std.collections.list
import std.collections.map
import std.result
import std.text
use std.result.Result

export record ConfigEntry:
    key: Str
    value: Str

export record ConfigSection:
    name: Str
    values: Map[Str, Str]
    order: List[Str]

export record ConfigDoc:
    root: std.config.ConfigSection
    sections: Map[Str, std.config.ConfigSection]
    order: List[Str]

fn empty_string_list() -> List[Str]:
    return std.collections.list.new[Str] :: :: call

fn empty_string_map() -> Map[Str, Str]:
    return std.collections.map.new[Str, Str] :: :: call

fn empty_section_map() -> Map[Str, std.config.ConfigSection]:
    return std.collections.map.new[Str, std.config.ConfigSection] :: :: call

fn empty_entry_list() -> List[std.config.ConfigEntry]:
    return std.collections.list.new[std.config.ConfigEntry] :: :: call

fn empty_section_named(read name: Str) -> std.config.ConfigSection:
    return std.config.ConfigSection :: name = name, values = (std.config.empty_string_map :: :: call), order = (std.config.empty_string_list :: :: call) :: call

export fn empty_document() -> std.config.ConfigDoc:
    let root = std.config.empty_section_named :: "" :: call
    return std.config.ConfigDoc :: root = root, sections = (std.config.empty_section_map :: :: call), order = (std.config.empty_string_list :: :: call) :: call

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
            let close = std.config.find_quote_end :: text, i + 1 :: call
            if close :: :: is_err:
                return Result.Err[Str, Str] :: "unterminated quoted string" :: call
            let value = close :: 0 :: unwrap_or
            return std.config.strip_comment_after_quote :: text, value + 1 :: call
        else:
            if b == 35:
                return Result.Ok[Str, Str] :: (std.config.trim_ws :: (std.text.slice_bytes :: text, 0, i :: call) :: call) :: call
        i += 1
    return Result.Ok[Str, Str] :: (std.config.trim_ws :: text :: call) :: call

fn strip_comment_after_quote(read text: Str, start: Int) -> Result[Str, Str]:
    let n = std.text.len_bytes :: text :: call
    let mut i = start
    while i < n:
        let b = std.text.byte_at :: text, i :: call
        if b == 34:
            let close = std.config.find_quote_end :: text, i + 1 :: call
            if close :: :: is_err:
                return Result.Err[Str, Str] :: "unterminated quoted string" :: call
            let value = close :: 0 :: unwrap_or
            return std.config.strip_comment_after_quote :: text, value + 1 :: call
        else:
            if b == 35:
                return Result.Ok[Str, Str] :: (std.config.trim_ws :: (std.text.slice_bytes :: text, 0, i :: call) :: call) :: call
        i += 1
    return Result.Ok[Str, Str] :: (std.config.trim_ws :: text :: call) :: call

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
    let t = std.config.trim_ws :: text :: call
    let n = std.text.len_bytes :: t :: call
    if n < 2 or (std.text.byte_at :: t, 0 :: call) != 34 or (std.text.byte_at :: t, n - 1 :: call) != 34:
        return Result.Err[Str, Str] :: "expected quoted string" :: call
    return std.config.decode_quoted_span :: t, 1, n - 1 :: call

fn trim_or_decode_string(read text: Str) -> Result[Str, Str]:
    let t = std.config.trim_ws :: text :: call
    let n = std.text.len_bytes :: t :: call
    if n >= 2 and (std.text.byte_at :: t, 0 :: call) == 34 and (std.text.byte_at :: t, n - 1 :: call) == 34:
        return std.config.decode_quoted_value :: t :: call
    return Result.Ok[Str, Str] :: t :: call

fn parse_kv(read line: Str) -> (Bool, (Str, Str)):
    let eq = std.text.find_byte :: line, 0, 61 :: call
    if eq < 0:
        return (false, ("", ""))
    let key = std.config.trim_ws :: (std.text.slice_bytes :: line, 0, eq :: call) :: call
    let value = std.config.trim_ws :: (std.text.slice_bytes :: line, eq + 1, (std.text.len_bytes :: line :: call) :: call) :: call
    return (true, (key, value))

fn parse_int_or(read text: Str, fallback: Int) -> Int:
    let s = std.config.trim_ws :: text :: call
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

fn parse_string_array_literal(read text: Str) -> Result[List[Str], Str]:
    let value = std.config.trim_ws :: text :: call
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
        let close = std.config.find_quote_end :: value, i + 1 :: call
        if close :: :: is_err:
            return Result.Err[List[Str], Str] :: "unterminated quoted string in array literal" :: call
        let end = close :: 0 :: unwrap_or
        if end >= n:
            return Result.Err[List[Str], Str] :: "unterminated quoted string in array literal" :: call
        let item = std.config.decode_quoted_span :: value, i + 1, end :: call
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
    let value = std.config.trim_ws :: text :: call
    let n = std.text.len_bytes :: value :: call
    if n >= 2 and (std.text.byte_at :: value, 0 :: call) == 34 and (std.text.byte_at :: value, n - 1 :: call) == 34:
        return std.config.decode_quoted_value :: value :: call
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
        let key = std.config.trim_ws :: (std.text.slice_bytes :: value, key_start, key_end :: call) :: call
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
            let close = std.config.find_quote_end :: value, i + 1 :: call
            if close :: :: is_err:
                return Result.Err[Str, Str] :: "unterminated quoted string in inline table" :: call
            let end = close :: 0 :: unwrap_or
            if end >= n:
                return Result.Err[Str, Str] :: "unterminated quoted string in inline table" :: call
            let decoded = std.config.decode_quoted_span :: value, i + 1, end :: call
            if decoded :: :: is_err:
                return Result.Err[Str, Str] :: (decoded :: "" :: unwrap_or) :: call
            field_value = decoded :: "" :: unwrap_or
            i = end + 1
        else:
            let value_start = i
            while i < (n - 1) and (std.text.byte_at :: value, i :: call) != 44:
                i += 1
            field_value = std.config.trim_ws :: (std.text.slice_bytes :: value, value_start, i :: call) :: call
        if key == wanted_key:
            return Result.Ok[Str, Str] :: field_value :: call
    return Result.Err[Str, Str] :: ("missing inline table field `" + wanted_key + "`") :: call

fn push_unique_name(edit names: List[Str], read name: Str, read duplicate_label: Str) -> Result[Unit, Str]:
    for current in names:
        if current == name:
            return Result.Err[Unit, Str] :: duplicate_label :: call
    names :: name :: push
    return Result.Ok[Unit, Str] :: :: call

fn add_root_entry(edit root: std.config.ConfigSection, read pair: (Str, Str)) -> Result[Unit, Str]:
    if root.values :: pair.0 :: has:
        return Result.Err[Unit, Str] :: ("duplicate config key `" + pair.0 + "`") :: call
    root.values :: pair.0, pair.1 :: set
    root.order :: pair.0 :: push
    return Result.Ok[Unit, Str] :: :: call

fn add_section_entry(edit sections: Map[Str, std.config.ConfigSection], read section_name: Str, read pair: (Str, Str)) -> Result[Unit, Str]:
    if not (sections :: section_name :: has):
        return Result.Err[Unit, Str] :: ("missing section `[" + section_name + "]`") :: call
    let mut section = sections :: section_name :: get
    if section.values :: pair.0 :: has:
        return Result.Err[Unit, Str] :: ("duplicate config key `" + pair.0 + "` in `[" + section_name + "]`") :: call
    section.values :: pair.0, pair.1 :: set
    section.order :: pair.0 :: push
    sections :: section_name, section :: set
    return Result.Ok[Unit, Str] :: :: call

fn copy_string_list(read values: List[Str]) -> List[Str]:
    let mut out = std.config.empty_string_list :: :: call
    for value in values:
        out :: value :: push
    return out

fn copy_string_map(read values: Map[Str, Str]) -> Map[Str, Str]:
    let mut out = std.config.empty_string_map :: :: call
    for pair in values:
        out :: pair.0, pair.1 :: set
    return out

fn copy_section(read section: std.config.ConfigSection) -> std.config.ConfigSection:
    return std.config.ConfigSection :: name = section.name, values = (std.config.copy_string_map :: section.values :: call), order = (std.config.copy_string_list :: section.order :: call) :: call

fn section_entries(read section: std.config.ConfigSection) -> List[std.config.ConfigEntry]:
    let mut out = std.config.empty_entry_list :: :: call
    for key in section.order:
        let entry = std.config.ConfigEntry :: key = key, value = (section.values :: key :: get) :: call
        out :: entry :: push
    return out

fn root_raw_value(read self: ConfigDoc, read key: Str) -> (Bool, Str):
    return self.root.values :: key, "" :: try_get_or

fn section_named_or_empty(read self: ConfigDoc, read section: Str) -> std.config.ConfigSection:
    let lookup = self.sections :: section, (std.config.empty_section_named :: section :: call) :: try_get_or
    if lookup.0:
        return std.config.copy_section :: lookup.1 :: call
    return lookup.1

fn section_raw_value(read self: ConfigDoc, read section: Str, read key: Str) -> (Bool, Str):
    let current = std.config.section_named_or_empty :: self, section :: call
    return current.values :: key, "" :: try_get_or

fn lookup_required(raw: (Bool, Str), read label: Str, read key: Str) -> Result[Str, Str]:
    if raw.0:
        return Result.Ok[Str, Str] :: raw.1 :: call
    return Result.Err[Str, Str] :: ("missing " + label + " `" + key + "`") :: call

export fn parse_document(text: Str) -> Result[ConfigDoc, Str]:
    let mut section = ""
    let mut doc = std.config.empty_document :: :: call
    let lines = std.text.split_lines :: text :: call
    for raw in lines:
        let line_result = std.config.strip_comment :: raw :: call
        if line_result :: :: is_err:
            return Result.Err[ConfigDoc, Str] :: "unterminated quoted string" :: call
        let line = line_result :: "" :: unwrap_or
        if (std.text.len_bytes :: line :: call) == 0:
            continue
        if std.text.starts_with :: line, "[" :: call:
            if not (std.text.ends_with :: line, "]" :: call):
                return Result.Err[ConfigDoc, Str] :: ("malformed section header `" + line + "`") :: call
            section = std.config.trim_ws :: (std.text.slice_bytes :: line, 1, (std.text.len_bytes :: line :: call) - 1 :: call) :: call
            if section == "":
                return Result.Err[ConfigDoc, Str] :: "malformed section header `[]`" :: call
            let push_result = std.config.push_unique_name :: doc.order, section, ("duplicate section header `[" + section + "]`") :: call
            if push_result :: :: is_err:
                return Result.Err[ConfigDoc, Str] :: (push_result :: "" :: unwrap_or) :: call
            let section_doc = std.config.empty_section_named :: section :: call
            doc.sections :: section, section_doc :: set
            continue
        let kv = std.config.parse_kv :: line :: call
        if not kv.0:
            return Result.Err[ConfigDoc, Str] :: ("malformed config line `" + line + "`") :: call
        let key_result = std.config.trim_or_decode_string :: kv.1.0 :: call
        if key_result :: :: is_err:
            return Result.Err[ConfigDoc, Str] :: (key_result :: "" :: unwrap_or) :: call
        let pair = ((key_result :: "" :: unwrap_or), kv.1.1)
        let mut add_result = Result.Ok[Unit, Str] :: :: call
        if section == "":
            add_result = std.config.add_root_entry :: doc.root, pair :: call
        else:
            add_result = std.config.add_section_entry :: doc.sections, section, pair :: call
        if add_result :: :: is_err:
            return Result.Err[ConfigDoc, Str] :: (add_result :: "" :: unwrap_or) :: call
    return Result.Ok[ConfigDoc, Str] :: doc :: call

impl ConfigDoc:
    fn has_section(read self: ConfigDoc, read section: Str) -> Bool:
        return self.sections :: section :: has

    fn entries_in_section(read self: ConfigDoc, read section: Str) -> List[std.config.ConfigEntry]:
        let current = std.config.section_named_or_empty :: self, section :: call
        return std.config.section_entries :: current :: call

    fn root_has_key(read self: ConfigDoc, read key: Str) -> Bool:
        return self.root.values :: key :: has

    fn root_string_or(read self: ConfigDoc, read key: Str, fallback: Str) -> Result[Str, Str]:
        let raw = std.config.root_raw_value :: self, key :: call
        if not raw.0:
            return Result.Ok[Str, Str] :: fallback :: call
        return std.config.trim_or_decode_string :: raw.1 :: call

    fn root_required_string(read self: ConfigDoc, read key: Str, read label: Str) -> Result[Str, Str]:
        let raw = std.config.lookup_required :: (std.config.root_raw_value :: self, key :: call), label, key :: call
        if raw :: :: is_err:
            return raw
        return std.config.trim_or_decode_string :: (raw :: "" :: unwrap_or) :: call

    fn root_int_or(read self: ConfigDoc, read key: Str, fallback: Int) -> Int:
        let raw = std.config.root_raw_value :: self, key :: call
        if not raw.0:
            return fallback
        return std.config.parse_int_or :: raw.1, fallback :: call

    fn root_string_array_or_empty(read self: ConfigDoc, read key: Str) -> Result[List[Str], Str]:
        let raw = std.config.root_raw_value :: self, key :: call
        if not raw.0:
            return Result.Ok[List[Str], Str] :: (std.config.empty_string_list :: :: call) :: call
        return std.config.parse_string_array_literal :: raw.1 :: call

    fn section_required_raw(read self: ConfigDoc, read section: Str, read key: Str, read label: Str) -> Result[Str, Str]:
        return std.config.lookup_required :: (std.config.section_raw_value :: self, section, key :: call), label, key :: call

    fn section_required(read self: ConfigDoc, read section: Str, read key: Str, read label: Str) -> Result[Str, Str]:
        let raw = self :: section, key, label :: section_required_raw
        if raw :: :: is_err:
            return raw
        return std.config.trim_or_decode_string :: (raw :: "" :: unwrap_or) :: call

    fn section_string_array_or_empty(read self: ConfigDoc, read section: Str, read key: Str) -> Result[List[Str], Str]:
        let raw = std.config.section_raw_value :: self, section, key :: call
        if not raw.0:
            return Result.Ok[List[Str], Str] :: (std.config.empty_string_list :: :: call) :: call
        return std.config.parse_string_array_literal :: raw.1 :: call

    fn section_inline_table_string_field(read self: ConfigDoc, read section: Str, read key_field: (Str, Str), read label: Str) -> Result[Str, Str]:
        let raw = self :: section, key_field.0, label :: section_required_raw
        if raw :: :: is_err:
            return raw
        return std.config.parse_inline_table_string_field :: (raw :: "" :: unwrap_or), key_field.1 :: call
