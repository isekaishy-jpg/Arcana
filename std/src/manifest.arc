import std.collections.list
import std.config
import std.result
import std.text
use std.result.Result

export record NameValue:
    name: Str
    value: Str

export record NameList:
    name: Str
    values: List[Str]

export record BookState:
    name: Str
    kind: Str
    workspace_member_names: List[Str]

export record BookDependencyTables:
    path_entries: List[std.manifest.NameValue]
    raw_entries: List[std.manifest.NameValue]
    source_kind_entries: List[std.manifest.NameValue]

export record BookManifest:
    state: std.manifest.BookState
    package_version: Str
    dependency_tables: std.manifest.BookDependencyTables

export record LockMetadata:
    version: Int
    workspace: Str
    ordered_members: List[Str]

export record LockDependencyTables:
    dependency_lists: List[std.manifest.NameList]
    path_entries: List[std.manifest.NameValue]
    fingerprint_entries: List[std.manifest.NameValue]

export record LockLookupTables:
    dependencies: std.manifest.LockDependencyTables
    api_fingerprint_entries: List[std.manifest.NameValue]

export record LockOutputTables:
    artifact_entries: List[std.manifest.NameValue]
    kind_entries: List[std.manifest.NameValue]
    format_entries: List[std.manifest.NameValue]

export record LockManifestV1:
    metadata: std.manifest.LockMetadata
    lookup_tables: std.manifest.LockLookupTables
    output_tables: std.manifest.LockOutputTables

export record LockMemberTables:
    package_ids: List[Str]
    dependency_binding_entries: List[std.manifest.NameValue]
    field_entries: List[std.manifest.NameValue]

export record LockBuildIdentity:
    member: Str
    target: Str

export record LockBuildMetadata:
    fingerprint: Str
    api_fingerprint: Str
    artifact_hash: Str

export record LockBuildOutput:
    artifact: Str
    format: Str
    toolchain: Str

export record LockBuildEntry:
    address: std.manifest.LockBuildIdentity
    metadata: std.manifest.LockBuildMetadata
    output: std.manifest.LockBuildOutput

export record LockBuildManifestTables:
    build_entries: List[std.manifest.LockBuildEntry]
    native_product_field_entries: List[std.manifest.NameValue]
    native_product_sidecar_entries: List[std.manifest.NameList]

export record LockBuildTables:
    workspace_root: Str
    workspace_members: List[Str]
    manifest_tables: std.manifest.LockBuildManifestTables

export record LockManifestV2:
    metadata: std.manifest.LockMetadata
    member_tables: std.manifest.LockMemberTables
    build_tables: std.manifest.LockBuildTables

record DecodedNameSpan:
    value: Str
    next_index: Int

record ParsedBuildSection:
    member: Str
    target: Str

fn empty_name_values() -> List[std.manifest.NameValue]:
    return std.collections.list.new[std.manifest.NameValue] :: :: call

fn empty_name_lists() -> List[std.manifest.NameList]:
    return std.collections.list.new[std.manifest.NameList] :: :: call

fn empty_build_entries() -> List[std.manifest.LockBuildEntry]:
    return std.collections.list.new[std.manifest.LockBuildEntry] :: :: call

fn empty_strings() -> List[Str]:
    return std.collections.list.new[Str] :: :: call

fn empty_config_doc() -> std.config.ConfigDoc:
    return std.config.empty_document :: :: call

fn string_or_empty(read result: Result[Str, Str]) -> Str:
    return match result:
        Result.Ok(value) => value
        Result.Err(_) => ""

fn string_err_or_empty(read result: Result[Str, Str]) -> Str:
    return match result:
        Result.Ok(_) => ""
        Result.Err(err) => err

fn strings_or_empty(read result: Result[List[Str], Str]) -> List[Str]:
    return match result:
        Result.Ok(values) => values
        Result.Err(_) => std.manifest.empty_strings :: :: call

fn strings_err_or_empty(read result: Result[List[Str], Str]) -> Str:
    return match result:
        Result.Ok(_) => ""
        Result.Err(err) => err

fn name_values_or_empty(read result: Result[List[std.manifest.NameValue], Str]) -> List[std.manifest.NameValue]:
    return match result:
        Result.Ok(values) => values
        Result.Err(_) => std.manifest.empty_name_values :: :: call

fn name_values_err_or_empty(read result: Result[List[std.manifest.NameValue], Str]) -> Str:
    return match result:
        Result.Ok(_) => ""
        Result.Err(err) => err

fn name_lists_or_empty(read result: Result[List[std.manifest.NameList], Str]) -> List[std.manifest.NameList]:
    return match result:
        Result.Ok(values) => values
        Result.Err(_) => std.manifest.empty_name_lists :: :: call

fn name_lists_err_or_empty(read result: Result[List[std.manifest.NameList], Str]) -> Str:
    return match result:
        Result.Ok(_) => ""
        Result.Err(err) => err

fn build_entries_or_empty(read result: Result[List[std.manifest.LockBuildEntry], Str]) -> List[std.manifest.LockBuildEntry]:
    return match result:
        Result.Ok(values) => values
        Result.Err(_) => std.manifest.empty_build_entries :: :: call

fn build_entries_err_or_empty(read result: Result[List[std.manifest.LockBuildEntry], Str]) -> Str:
    return match result:
        Result.Ok(_) => ""
        Result.Err(err) => err

fn config_doc_or_empty(read result: Result[std.config.ConfigDoc, Str]) -> std.config.ConfigDoc:
    return match result:
        Result.Ok(doc) => doc
        Result.Err(_) => std.manifest.empty_config_doc :: :: call

fn config_doc_err_or_empty(read result: Result[std.config.ConfigDoc, Str]) -> Str:
    return match result:
        Result.Ok(_) => ""
        Result.Err(err) => err

export fn empty_book_state() -> std.manifest.BookState:
    return std.manifest.BookState :: name = "", kind = "", workspace_member_names = (std.manifest.empty_strings :: :: call) :: call

export fn empty_book_dependency_tables() -> std.manifest.BookDependencyTables:
    return std.manifest.BookDependencyTables :: path_entries = (std.manifest.empty_name_values :: :: call), raw_entries = (std.manifest.empty_name_values :: :: call), source_kind_entries = (std.manifest.empty_name_values :: :: call) :: call

export fn empty_book_manifest() -> std.manifest.BookManifest:
    return std.manifest.BookManifest :: state = (std.manifest.empty_book_state :: :: call), package_version = "", dependency_tables = (std.manifest.empty_book_dependency_tables :: :: call) :: call

export fn empty_lock_metadata() -> std.manifest.LockMetadata:
    return std.manifest.LockMetadata :: version = 0, workspace = "", ordered_members = (std.manifest.empty_strings :: :: call) :: call

export fn empty_lock_member_tables() -> std.manifest.LockMemberTables:
    return std.manifest.LockMemberTables :: package_ids = (std.manifest.empty_strings :: :: call), dependency_binding_entries = (std.manifest.empty_name_values :: :: call), field_entries = (std.manifest.empty_name_values :: :: call) :: call

fn empty_lock_build_manifest_tables() -> std.manifest.LockBuildManifestTables:
    return std.manifest.LockBuildManifestTables :: build_entries = (std.manifest.empty_build_entries :: :: call), native_product_field_entries = (std.manifest.empty_name_values :: :: call), native_product_sidecar_entries = (std.manifest.empty_name_lists :: :: call) :: call

export fn empty_lock_build_tables() -> std.manifest.LockBuildTables:
    return std.manifest.LockBuildTables :: workspace_root = "", workspace_members = (std.manifest.empty_strings :: :: call), manifest_tables = (std.manifest.empty_lock_build_manifest_tables :: :: call) :: call

fn empty_decoded_name_span() -> std.manifest.DecodedNameSpan:
    return std.manifest.DecodedNameSpan :: value = "", next_index = 0 :: call

fn empty_parsed_build_section() -> std.manifest.ParsedBuildSection:
    return std.manifest.ParsedBuildSection :: member = "", target = "" :: call

fn section_has_key(read doc: std.config.ConfigDoc, read section: Str, read key: Str) -> Bool:
    let entries = doc :: section :: entries_in_section
    for entry in entries:
        if entry.key == key:
            return true
    return false

fn dep_entry_raw_value(read doc: std.config.ConfigDoc, read dep_name: Str) -> Result[Str, Str]:
    return doc :: "deps", dep_name, "dependency entry" :: section_required_raw

fn is_quoted_string_value(read raw: Str) -> Bool:
    let value = std.config.trim_ws :: raw :: call
    let n = std.text.len_bytes :: value :: call
    return n >= 2 and (std.text.byte_at :: value, 0 :: call) == 34 and (std.text.byte_at :: value, n - 1 :: call) == 34

fn inline_table_string_field_or_empty(read raw: Str, read field_name: Str) -> Result[Str, Str]:
    let value_result = std.config.parse_inline_table_string_field :: raw, field_name :: call
    if value_result :: :: is_ok:
        return value_result
    let message = std.manifest.string_err_or_empty :: value_result :: call
    if std.text.starts_with :: message, ("missing inline table field `" + field_name + "`") :: call:
        return Result.Ok[Str, Str] :: "" :: call
    return Result.Err[Str, Str] :: message :: call

fn add_name_value(edit out: List[std.manifest.NameValue], read name: Str, read value: Str):
    let pair = std.manifest.NameValue :: name = name, value = value :: call
    out :: pair :: push

fn add_name_list(edit out: List[std.manifest.NameList], read name: Str, read values: List[Str]):
    let pair = std.manifest.NameList :: name = name, values = values :: call
    out :: pair :: push

fn add_string(edit out: List[Str], read value: Str):
    out :: value :: push

fn copy_strings(read values: List[Str]) -> List[Str]:
    let mut out = std.manifest.empty_strings :: :: call
    for value in values:
        out :: value :: push
    return out

fn lookup_name_value(read entries: List[std.manifest.NameValue], read name: Str, read label: Str) -> Result[Str, Str]:
    for entry in entries:
        if entry.name == name:
            return Result.Ok[Str, Str] :: entry.value :: call
    return Result.Err[Str, Str] :: ("missing " + label + " `" + name + "`") :: call

fn lookup_name_list_or_empty(read entries: List[std.manifest.NameList], read name: Str) -> List[Str]:
    for entry in entries:
        if entry.name == name:
            return std.manifest.copy_strings :: entry.values :: call
    return std.manifest.empty_strings :: :: call

fn lookup_optional_name_value_or_empty(read entries: List[std.manifest.NameValue], read name: Str) -> Str:
    for entry in entries:
        if entry.name == name:
            return entry.value
    return ""

fn has_string(read entries: List[Str], read value: Str) -> Bool:
    for entry in entries:
        if entry == value:
            return true
    return false

fn names_from_name_values(read entries: List[std.manifest.NameValue]) -> List[Str]:
    let mut out = std.manifest.empty_strings :: :: call
    for entry in entries:
        std.manifest.add_string :: out, entry.name :: call
    return out

fn dependency_binding_entry_name(read member: Str, read alias: Str) -> Str:
    return member + "|" + alias

fn package_field_entry_name(read package_id: Str, read field_name: Str) -> Str:
    return "package|" + package_id + "|" + field_name

fn native_product_field_entry_name(read package_id: Str, read product_name: Str, read field_name: Str) -> Str:
    return "native_product|" + package_id + "|" + product_name + "|" + field_name

fn lookup_dependency_values_or_empty(read entries: List[std.manifest.NameValue], read member: Str) -> List[Str]:
    let mut out = std.manifest.empty_strings :: :: call
    let prefix = member + "|" 
    for entry in entries:
        if std.text.starts_with :: entry.name, prefix :: call:
            out :: entry.value :: push
    return out

fn collect_native_product_names_or_empty(read entries: List[std.manifest.NameValue], read package_id: Str) -> List[Str]:
    let mut out = std.manifest.empty_strings :: :: call
    let prefix = "native_product|" + package_id + "|"
    let suffix = "|kind"
    for entry in entries:
        if std.text.starts_with :: entry.name, prefix :: call:
            if std.text.ends_with :: entry.name, suffix :: call:
                let prefix_len = std.text.len_bytes :: prefix :: call
                let suffix_len = std.text.len_bytes :: suffix :: call
                let total_len = std.text.len_bytes :: entry.name :: call
                let product_name = std.text.slice_bytes :: entry.name, prefix_len, total_len - suffix_len :: call
                out :: product_name :: push
    return out

fn lookup_build_target_names_or_empty(read entries: List[std.manifest.LockBuildEntry], read member: Str) -> List[Str]:
    let mut out = std.manifest.empty_strings :: :: call
    for entry in entries:
        if entry.address.member == member:
            out :: entry.address.target :: push
    return out

fn decode_quoted_name_component(read text: Str, start: Int) -> Result[std.manifest.DecodedNameSpan, Str]:
    let n = std.text.len_bytes :: text :: call
    if start >= n or (std.text.byte_at :: text, start :: call) != 34:
        return Result.Err[std.manifest.DecodedNameSpan, Str] :: "expected quoted name component" :: call
    let mut out = ""
    let mut i = start + 1
    while i < n:
        let b = std.text.byte_at :: text, i :: call
        if b == 92:
            if i + 1 >= n:
                return Result.Err[std.manifest.DecodedNameSpan, Str] :: "unterminated escape sequence in quoted name component" :: call
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
                                return Result.Err[std.manifest.DecodedNameSpan, Str] :: "unsupported escape sequence in quoted name component" :: call
            i += 2
            continue
        if b == 34:
            let span = std.manifest.DecodedNameSpan :: value = out, next_index = i + 1 :: call
            return Result.Ok[std.manifest.DecodedNameSpan, Str] :: span :: call
        out = out + (std.text.slice_bytes :: text, i, i + 1 :: call)
        i += 1
    return Result.Err[std.manifest.DecodedNameSpan, Str] :: "unterminated quoted name component" :: call

fn parse_two_quoted_section_name(read section: Str, read prefix: Str, read separator_label: Str) -> Result[std.manifest.ParsedBuildSection, Str]:
    if not (std.text.starts_with :: section, prefix :: call):
        return Result.Err[std.manifest.ParsedBuildSection, Str] :: ("expected section prefix `" + prefix + "`") :: call
    let member_result = std.manifest.decode_quoted_name_component :: section, (std.text.len_bytes :: prefix :: call) :: call
    if member_result :: :: is_err:
        return Result.Err[std.manifest.ParsedBuildSection, Str] :: ("invalid first quoted name in " + separator_label) :: call
    let member_span = member_result :: (std.manifest.empty_decoded_name_span :: :: call) :: unwrap_or
    let n = std.text.len_bytes :: section :: call
    if member_span.next_index >= n or (std.text.byte_at :: section, member_span.next_index :: call) != 46:
        return Result.Err[std.manifest.ParsedBuildSection, Str] :: ("expected separator in " + separator_label) :: call
    let target_result = std.manifest.decode_quoted_name_component :: section, member_span.next_index + 1 :: call
    if target_result :: :: is_err:
        return Result.Err[std.manifest.ParsedBuildSection, Str] :: ("invalid second quoted name in " + separator_label) :: call
    let target_span = target_result :: (std.manifest.empty_decoded_name_span :: :: call) :: unwrap_or
    if target_span.next_index != n:
        return Result.Err[std.manifest.ParsedBuildSection, Str] :: ("unexpected suffix in " + separator_label) :: call
    let parsed = std.manifest.ParsedBuildSection :: member = member_span.value, target = target_span.value :: call
    return Result.Ok[std.manifest.ParsedBuildSection, Str] :: parsed :: call

fn parse_build_section_name(read section: Str) -> Result[std.manifest.ParsedBuildSection, Str]:
    return std.manifest.parse_two_quoted_section_name :: section, "builds.", "build section" :: call

fn parse_native_product_section_name(read section: Str) -> Result[std.manifest.ParsedBuildSection, Str]:
    return std.manifest.parse_two_quoted_section_name :: section, "native_products.", "native product section" :: call

fn parse_single_quoted_section_name(read section: Str, read prefix: Str) -> Result[Str, Str]:
    if not (std.text.starts_with :: section, prefix :: call):
        return Result.Err[Str, Str] :: ("expected section prefix `" + prefix + "`") :: call
    let decoded_result = std.manifest.decode_quoted_name_component :: section, (std.text.len_bytes :: prefix :: call) :: call
    if decoded_result :: :: is_err:
        return Result.Err[Str, Str] :: "invalid quoted section name" :: call
    let decoded = decoded_result :: (std.manifest.empty_decoded_name_span :: :: call) :: unwrap_or
    if decoded.next_index != (std.text.len_bytes :: section :: call):
        return Result.Err[Str, Str] :: "unexpected suffix in quoted section name" :: call
    return Result.Ok[Str, Str] :: decoded.value :: call

fn has_named_value(read entries: List[std.manifest.NameValue], read name: Str) -> Bool:
    for entry in entries:
        if entry.name == name:
            return true
    return false

fn collect_build_entries(read doc: std.config.ConfigDoc, read package_ids: List[Str]) -> Result[List[std.manifest.LockBuildEntry], Str]:
    let mut out = std.manifest.empty_build_entries :: :: call
    for section in doc.order:
        if section == "builds":
            continue
        if not (std.text.starts_with :: section, "builds." :: call):
            continue
        let parsed_result = std.manifest.parse_build_section_name :: section :: call
        if parsed_result :: :: is_err:
            return Result.Err[List[std.manifest.LockBuildEntry], Str] :: ("invalid build section `[" + section + "]` in Arcana.lock") :: call
        let parsed = parsed_result :: (std.manifest.empty_parsed_build_section :: :: call) :: unwrap_or
        if not (std.manifest.has_string :: package_ids, parsed.member :: call):
            return Result.Err[List[std.manifest.LockBuildEntry], Str] :: ("build section `[" + section + "]` references unknown member `" + parsed.member + "`") :: call
        let fingerprint_result = doc :: section, "fingerprint", "lock build entry" :: section_required
        if fingerprint_result :: :: is_err:
            return Result.Err[List[std.manifest.LockBuildEntry], Str] :: "missing `fingerprint` in lock build entry" :: call
        let api_fingerprint_result = doc :: section, "api_fingerprint", "lock build entry" :: section_required
        if api_fingerprint_result :: :: is_err:
            return Result.Err[List[std.manifest.LockBuildEntry], Str] :: "missing `api_fingerprint` in lock build entry" :: call
        let artifact_result = doc :: section, "artifact", "lock build entry" :: section_required
        if artifact_result :: :: is_err:
            return Result.Err[List[std.manifest.LockBuildEntry], Str] :: "missing `artifact` in lock build entry" :: call
        let artifact_hash_result = doc :: section, "artifact_hash", "lock build entry" :: section_required
        if artifact_hash_result :: :: is_err:
            return Result.Err[List[std.manifest.LockBuildEntry], Str] :: "missing `artifact_hash` in lock build entry" :: call
        let format_result = doc :: section, "format", "lock build entry" :: section_required
        if format_result :: :: is_err:
            return Result.Err[List[std.manifest.LockBuildEntry], Str] :: "missing `format` in lock build entry" :: call
        let toolchain_result = doc :: section, "toolchain", "lock build entry" :: section_required
        if toolchain_result :: :: is_err:
            return Result.Err[List[std.manifest.LockBuildEntry], Str] :: "missing `toolchain` in lock build entry" :: call
        let address = std.manifest.LockBuildIdentity :: member = parsed.member, target = parsed.target :: call
        let metadata = std.manifest.LockBuildMetadata :: fingerprint = (std.manifest.string_or_empty :: fingerprint_result :: call), api_fingerprint = (std.manifest.string_or_empty :: api_fingerprint_result :: call), artifact_hash = (std.manifest.string_or_empty :: artifact_hash_result :: call) :: call
        let output = std.manifest.LockBuildOutput :: artifact = (std.manifest.string_or_empty :: artifact_result :: call), format = (std.manifest.string_or_empty :: format_result :: call), toolchain = (std.manifest.string_or_empty :: toolchain_result :: call) :: call
        let entry = std.manifest.LockBuildEntry :: address = address, metadata = metadata, output = output :: call
        out :: entry :: push
    for package_id in package_ids:
        let targets = std.manifest.lookup_build_target_names_or_empty :: out, package_id :: call
        if (targets :: :: len) == 0:
            return Result.Err[List[std.manifest.LockBuildEntry], Str] :: ("missing lock build entry for `" + package_id + "`") :: call
    return Result.Ok[List[std.manifest.LockBuildEntry], Str] :: out :: call

fn collect_dep_paths(read doc: std.config.ConfigDoc) -> Result[List[std.manifest.NameValue], Str]:
    let mut out = std.manifest.empty_name_values :: :: call
    let deps = doc :: "deps" :: entries_in_section
    for dep in deps:
        let raw_result = std.manifest.dep_entry_raw_value :: doc, dep.key :: call
        if raw_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: raw_result :: call) :: call
        let raw = std.manifest.string_or_empty :: raw_result :: call
        if std.manifest.is_quoted_string_value :: raw :: call:
            let path_result = std.config.parse_inline_table_string_field :: raw, "path" :: call
            if path_result :: :: is_err:
                return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: path_result :: call) :: call
            std.manifest.add_name_value :: out, dep.key, (std.manifest.string_or_empty :: path_result :: call) :: call
            continue
        let path_result = std.manifest.inline_table_string_field_or_empty :: raw, "path" :: call
        if path_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: path_result :: call) :: call
        let path = std.manifest.string_or_empty :: path_result :: call
        if path != "":
            std.manifest.add_name_value :: out, dep.key, path :: call
    return Result.Ok[List[std.manifest.NameValue], Str] :: out :: call

fn collect_dep_optional_values(read doc: std.config.ConfigDoc, read field_name: Str) -> Result[List[std.manifest.NameValue], Str]:
    let mut out = std.manifest.empty_name_values :: :: call
    let deps = doc :: "deps" :: entries_in_section
    for dep in deps:
        let raw_result = std.manifest.dep_entry_raw_value :: doc, dep.key :: call
        if raw_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: raw_result :: call) :: call
        let raw = std.manifest.string_or_empty :: raw_result :: call
        if std.manifest.is_quoted_string_value :: raw :: call:
            continue
        let value_result = std.manifest.inline_table_string_field_or_empty :: raw, field_name :: call
        if value_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: value_result :: call) :: call
        let value = std.manifest.string_or_empty :: value_result :: call
        if value != "":
            std.manifest.add_name_value :: out, dep.key, value :: call
    return Result.Ok[List[std.manifest.NameValue], Str] :: out :: call

fn collect_dep_raw_values(read doc: std.config.ConfigDoc) -> Result[List[std.manifest.NameValue], Str]:
    let mut out = std.manifest.empty_name_values :: :: call
    let deps = doc :: "deps" :: entries_in_section
    for dep in deps:
        let raw_result = std.manifest.dep_entry_raw_value :: doc, dep.key :: call
        if raw_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: raw_result :: call) :: call
        std.manifest.add_name_value :: out, dep.key, (std.manifest.string_or_empty :: raw_result :: call) :: call
    return Result.Ok[List[std.manifest.NameValue], Str] :: out :: call

fn collect_dep_source_kinds(read doc: std.config.ConfigDoc) -> Result[List[std.manifest.NameValue], Str]:
    let mut out = std.manifest.empty_name_values :: :: call
    let deps = doc :: "deps" :: entries_in_section
    for dep in deps:
        let raw_result = std.manifest.dep_entry_raw_value :: doc, dep.key :: call
        if raw_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: raw_result :: call) :: call
        let raw = std.manifest.string_or_empty :: raw_result :: call
        if std.manifest.is_quoted_string_value :: raw :: call:
            std.manifest.add_name_value :: out, dep.key, "path" :: call
            continue
        let path_result = std.manifest.inline_table_string_field_or_empty :: raw, "path" :: call
        if path_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: path_result :: call) :: call
        if (std.manifest.string_or_empty :: path_result :: call) != "":
            std.manifest.add_name_value :: out, dep.key, "path" :: call
            continue
        let git_result = std.manifest.inline_table_string_field_or_empty :: raw, "git" :: call
        if git_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: git_result :: call) :: call
        if (std.manifest.string_or_empty :: git_result :: call) != "":
            std.manifest.add_name_value :: out, dep.key, "git" :: call
            continue
        let version_result = std.manifest.inline_table_string_field_or_empty :: raw, "version" :: call
        if version_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: version_result :: call) :: call
        if (std.manifest.string_or_empty :: version_result :: call) != "":
            std.manifest.add_name_value :: out, dep.key, "registry" :: call
            continue
        return Result.Err[List[std.manifest.NameValue], Str] :: ("dependency `" + dep.key + "` must set `path`, `version`, or `git`") :: call
    return Result.Ok[List[std.manifest.NameValue], Str] :: out :: call

fn collect_named_values(read doc: std.config.ConfigDoc, read section: Str) -> Result[List[std.manifest.NameValue], Str]:
    let mut out = std.manifest.empty_name_values :: :: call
    let entries = doc :: section :: entries_in_section
    for entry in entries:
        let value_result = doc :: section, entry.key, ("`[" + section + "]` entry") :: section_required
        if value_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: value_result :: call) :: call
        std.manifest.add_name_value :: out, entry.key, (std.manifest.string_or_empty :: value_result :: call) :: call
    return Result.Ok[List[std.manifest.NameValue], Str] :: out :: call

fn collect_named_lists(read doc: std.config.ConfigDoc, read section: Str) -> Result[List[std.manifest.NameList], Str]:
    let mut out = std.manifest.empty_name_lists :: :: call
    let entries = doc :: section :: entries_in_section
    for entry in entries:
        let values_result = doc :: section, entry.key :: section_string_array_or_empty
        if values_result :: :: is_err:
            return Result.Err[List[std.manifest.NameList], Str] :: (std.manifest.strings_err_or_empty :: values_result :: call) :: call
        std.manifest.add_name_list :: out, entry.key, (std.manifest.strings_or_empty :: values_result :: call) :: call
    return Result.Ok[List[std.manifest.NameList], Str] :: out :: call

fn collect_legacy_dependency_bindings(read entries: List[std.manifest.NameList]) -> List[std.manifest.NameValue]:
    let mut out = std.manifest.empty_name_values :: :: call
    for entry in entries:
        for value in entry.values:
            std.manifest.add_name_value :: out, (std.manifest.dependency_binding_entry_name :: entry.name, value :: call), value :: call
    return out

fn collect_legacy_package_field_entries(read entries: List[std.manifest.NameValue], read field_name: Str) -> List[std.manifest.NameValue]:
    let mut out = std.manifest.empty_name_values :: :: call
    for entry in entries:
        std.manifest.add_name_value :: out, (std.manifest.package_field_entry_name :: entry.name, field_name :: call), entry.value :: call
    return out

fn collect_identity_package_field_entries(read package_ids: List[Str], read field_name: Str) -> List[std.manifest.NameValue]:
    let mut out = std.manifest.empty_name_values :: :: call
    for package_id in package_ids:
        std.manifest.add_name_value :: out, (std.manifest.package_field_entry_name :: package_id, field_name :: call), package_id :: call
    return out

fn collect_constant_package_field_entries(read package_ids: List[Str], read field_name: Str, read value: Str) -> List[std.manifest.NameValue]:
    let mut out = std.manifest.empty_name_values :: :: call
    for package_id in package_ids:
        std.manifest.add_name_value :: out, (std.manifest.package_field_entry_name :: package_id, field_name :: call), value :: call
    return out

fn append_name_values(edit out: List[std.manifest.NameValue], read entries: List[std.manifest.NameValue]):
    for entry in entries:
        out :: entry :: push

fn infer_legacy_workspace_root(read path_entries: List[std.manifest.NameValue], read package_ids: List[Str]) -> Str:
    for entry in path_entries:
        if entry.value == ".":
            return entry.name
    for package_id in package_ids:
        return package_id
    return ""

fn infer_legacy_target_for_format(read format: Str) -> Result[Str, Str]:
    if std.text.starts_with :: format, "arcana-aot-windows-exe" :: call:
        return Result.Ok[Str, Str] :: "windows-exe" :: call
    if std.text.starts_with :: format, "arcana-aot-windows-dll" :: call:
        return Result.Ok[Str, Str] :: "windows-dll" :: call
    if std.text.starts_with :: format, "arcana-aot-" :: call:
        return Result.Ok[Str, Str] :: "internal-aot" :: call
    return Result.Err[Str, Str] :: ("unable to infer build target from format `" + format + "`") :: call

fn collect_named_values_if_present(read doc: std.config.ConfigDoc, read section: Str) -> Result[List[std.manifest.NameValue], Str]:
    if doc :: section :: has_section:
        return std.manifest.collect_named_values :: doc, section :: call
    return Result.Ok[List[std.manifest.NameValue], Str] :: (std.manifest.empty_name_values :: :: call) :: call

fn collect_native_product_field_entries_if_present(read doc: std.config.ConfigDoc) -> Result[List[std.manifest.NameValue], Str]:
    if doc :: "native_products" :: has_section:
        return std.manifest.collect_v4_native_product_field_entries :: doc :: call
    return Result.Ok[List[std.manifest.NameValue], Str] :: (std.manifest.empty_name_values :: :: call) :: call

fn collect_native_product_sidecar_entries_if_present(read doc: std.config.ConfigDoc) -> Result[List[std.manifest.NameList], Str]:
    if doc :: "native_products" :: has_section:
        return std.manifest.collect_v4_native_product_sidecar_entries :: doc :: call
    return Result.Ok[List[std.manifest.NameList], Str] :: (std.manifest.empty_name_lists :: :: call) :: call

fn resolve_legacy_target(read target_entries: List[std.manifest.NameValue], read package_id: Str, read format: Str) -> Result[Str, Str]:
    let target = std.manifest.lookup_optional_name_value_or_empty :: target_entries, package_id :: call
    if target != "":
        return Result.Ok[Str, Str] :: target :: call
    return std.manifest.infer_legacy_target_for_format :: format :: call

fn collect_v4_package_ids(read doc: std.config.ConfigDoc) -> Result[List[Str], Str]:
    let mut out = std.manifest.empty_strings :: :: call
    for section in doc.order:
        if section == "packages":
            continue
        if not (std.text.starts_with :: section, "packages." :: call):
            continue
        let id_result = std.manifest.parse_single_quoted_section_name :: section, "packages." :: call
        if id_result :: :: is_err:
            return Result.Err[List[Str], Str] :: ("invalid package section `[" + section + "]` in Arcana.lock") :: call
        let package_id = id_result :: "" :: unwrap_or
        if not (std.manifest.has_string :: out, package_id :: call):
            std.manifest.add_string :: out, package_id :: call
    return Result.Ok[List[Str], Str] :: out :: call

fn collect_v4_package_optional_values(read doc: std.config.ConfigDoc, read field_name: Str, read label: Str) -> Result[List[std.manifest.NameValue], Str]:
    let mut out = std.manifest.empty_name_values :: :: call
    for section in doc.order:
        if section == "packages":
            continue
        if not (std.text.starts_with :: section, "packages." :: call):
            continue
        let id_result = std.manifest.parse_single_quoted_section_name :: section, "packages." :: call
        if id_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: ("invalid package section `[" + section + "]` in Arcana.lock") :: call
        let package_id = id_result :: "" :: unwrap_or
        if not (std.manifest.section_has_key :: doc, section, field_name :: call):
            continue
        let value_result = doc :: section, field_name, label :: section_required
        if value_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: value_result :: call) :: call
        std.manifest.add_name_value :: out, (std.manifest.package_field_entry_name :: package_id, field_name :: call), (std.manifest.string_or_empty :: value_result :: call) :: call
    return Result.Ok[List[std.manifest.NameValue], Str] :: out :: call

fn collect_v4_package_required_values(read doc: std.config.ConfigDoc, read field_name: Str, read label: Str) -> Result[List[std.manifest.NameValue], Str]:
    let mut out = std.manifest.empty_name_values :: :: call
    for section in doc.order:
        if section == "packages":
            continue
        if not (std.text.starts_with :: section, "packages." :: call):
            continue
        let id_result = std.manifest.parse_single_quoted_section_name :: section, "packages." :: call
        if id_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: ("invalid package section `[" + section + "]` in Arcana.lock") :: call
        let package_id = id_result :: "" :: unwrap_or
        if not (std.manifest.section_has_key :: doc, section, field_name :: call):
            return Result.Err[List[std.manifest.NameValue], Str] :: ("missing `" + field_name + "` in lock package `" + package_id + "`") :: call
        let value_result = doc :: section, field_name, label :: section_required
        if value_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: value_result :: call) :: call
        std.manifest.add_name_value :: out, (std.manifest.package_field_entry_name :: package_id, field_name :: call), (std.manifest.string_or_empty :: value_result :: call) :: call
    return Result.Ok[List[std.manifest.NameValue], Str] :: out :: call

fn collect_v4_dependency_bindings(read doc: std.config.ConfigDoc) -> Result[List[std.manifest.NameValue], Str]:
    let mut out = std.manifest.empty_name_values :: :: call
    for section in doc.order:
        if section == "dependencies":
            continue
        if not (std.text.starts_with :: section, "dependencies." :: call):
            continue
        let id_result = std.manifest.parse_single_quoted_section_name :: section, "dependencies." :: call
        if id_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: ("invalid dependency section `[" + section + "]` in Arcana.lock") :: call
        let package_id = id_result :: "" :: unwrap_or
        let entries = doc :: section :: entries_in_section
        for entry in entries:
            let value_result = doc :: section, entry.key, "lock dependency entry" :: section_required
            if value_result :: :: is_err:
                return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: value_result :: call) :: call
            std.manifest.add_name_value :: out, (std.manifest.dependency_binding_entry_name :: package_id, entry.key :: call), (std.manifest.string_or_empty :: value_result :: call) :: call
    return Result.Ok[List[std.manifest.NameValue], Str] :: out :: call

fn collect_v4_native_product_field_entries(read doc: std.config.ConfigDoc) -> Result[List[std.manifest.NameValue], Str]:
    let mut out = std.manifest.empty_name_values :: :: call
    for section in doc.order:
        if section == "native_products":
            continue
        if not (std.text.starts_with :: section, "native_products." :: call):
            continue
        let parsed_result = std.manifest.parse_native_product_section_name :: section :: call
        if parsed_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: ("invalid native product section `[" + section + "]` in Arcana.lock") :: call
        let parsed = parsed_result :: (std.manifest.empty_parsed_build_section :: :: call) :: unwrap_or
        for field_name in ["kind", "role", "producer", "file", "contract"]:
            if not (std.manifest.section_has_key :: doc, section, field_name :: call):
                return Result.Err[List[std.manifest.NameValue], Str] :: ("missing `" + field_name + "` in lock native product `" + parsed.target + "` for `" + parsed.member + "`") :: call
            let value_result = doc :: section, field_name, "lock native product entry" :: section_required
            if value_result :: :: is_err:
                return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: value_result :: call) :: call
            std.manifest.add_name_value :: out, (std.manifest.native_product_field_entry_name :: parsed.member, parsed.target, field_name :: call), (std.manifest.string_or_empty :: value_result :: call) :: call
        if std.manifest.section_has_key :: doc, section, "rust_cdylib_crate" :: call:
            let value_result = doc :: section, "rust_cdylib_crate", "lock native product entry" :: section_required
            if value_result :: :: is_err:
                return Result.Err[List[std.manifest.NameValue], Str] :: (std.manifest.string_err_or_empty :: value_result :: call) :: call
            std.manifest.add_name_value :: out, (std.manifest.native_product_field_entry_name :: parsed.member, parsed.target, "rust_cdylib_crate" :: call), (std.manifest.string_or_empty :: value_result :: call) :: call
    return Result.Ok[List[std.manifest.NameValue], Str] :: out :: call

fn collect_v4_native_product_sidecar_entries(read doc: std.config.ConfigDoc) -> Result[List[std.manifest.NameList], Str]:
    let mut out = std.manifest.empty_name_lists :: :: call
    for section in doc.order:
        if section == "native_products":
            continue
        if not (std.text.starts_with :: section, "native_products." :: call):
            continue
        let parsed_result = std.manifest.parse_native_product_section_name :: section :: call
        if parsed_result :: :: is_err:
            return Result.Err[List[std.manifest.NameList], Str] :: ("invalid native product section `[" + section + "]` in Arcana.lock") :: call
        let parsed = parsed_result :: (std.manifest.empty_parsed_build_section :: :: call) :: unwrap_or
        let values_result = doc :: section, "sidecars" :: section_string_array_or_empty
        if values_result :: :: is_err:
            return Result.Err[List[std.manifest.NameList], Str] :: (std.manifest.strings_err_or_empty :: values_result :: call) :: call
        std.manifest.add_name_list :: out, (std.manifest.native_product_field_entry_name :: parsed.member, parsed.target, "sidecars" :: call), (std.manifest.strings_or_empty :: values_result :: call) :: call
    return Result.Ok[List[std.manifest.NameList], Str] :: out :: call

fn dep_field_from_raw_entries(read raw_entries: List[std.manifest.NameValue], read dep_name: Str, read field_name: Str) -> Result[Str, Str]:
    let raw_result = std.manifest.lookup_name_value :: raw_entries, dep_name, "dependency entry" :: call
    if raw_result :: :: is_err:
        return Result.Err[Str, Str] :: (std.manifest.string_err_or_empty :: raw_result :: call) :: call
    let raw = std.manifest.string_or_empty :: raw_result :: call
    if std.manifest.is_quoted_string_value :: raw :: call:
        return Result.Err[Str, Str] :: ("missing dependency " + field_name + " for `" + dep_name + "`") :: call
    let value_result = std.manifest.inline_table_string_field_or_empty :: raw, field_name :: call
    if value_result :: :: is_err:
        return Result.Err[Str, Str] :: (std.manifest.string_err_or_empty :: value_result :: call) :: call
    let value = std.manifest.string_or_empty :: value_result :: call
    if value == "":
        return Result.Err[Str, Str] :: ("missing dependency " + field_name + " for `" + dep_name + "`") :: call
    return Result.Ok[Str, Str] :: value :: call

impl BookManifest:
    fn workspace_members(read self: BookManifest) -> Result[List[Str], Str]:
        return Result.Ok[List[Str], Str] :: (std.manifest.copy_strings :: self.state.workspace_member_names :: call) :: call

    fn dep_path(read self: BookManifest, dep_name: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.dependency_tables.path_entries, dep_name, "dependency entry" :: call

    fn dep_version(read self: BookManifest, dep_name: Str) -> Result[Str, Str]:
        return std.manifest.dep_field_from_raw_entries :: self.dependency_tables.raw_entries, dep_name, "version" :: call

    fn dep_package(read self: BookManifest, dep_name: Str) -> Result[Str, Str]:
        return std.manifest.dep_field_from_raw_entries :: self.dependency_tables.raw_entries, dep_name, "package" :: call

    fn dep_registry(read self: BookManifest, dep_name: Str) -> Result[Str, Str]:
        return std.manifest.dep_field_from_raw_entries :: self.dependency_tables.raw_entries, dep_name, "registry" :: call

    fn dep_git(read self: BookManifest, dep_name: Str) -> Result[Str, Str]:
        return std.manifest.dep_field_from_raw_entries :: self.dependency_tables.raw_entries, dep_name, "git" :: call

    fn dep_rev(read self: BookManifest, dep_name: Str) -> Result[Str, Str]:
        return std.manifest.dep_field_from_raw_entries :: self.dependency_tables.raw_entries, dep_name, "rev" :: call

    fn dep_tag(read self: BookManifest, dep_name: Str) -> Result[Str, Str]:
        return std.manifest.dep_field_from_raw_entries :: self.dependency_tables.raw_entries, dep_name, "tag" :: call

    fn dep_branch(read self: BookManifest, dep_name: Str) -> Result[Str, Str]:
        return std.manifest.dep_field_from_raw_entries :: self.dependency_tables.raw_entries, dep_name, "branch" :: call

    fn dep_checksum(read self: BookManifest, dep_name: Str) -> Result[Str, Str]:
        return std.manifest.dep_field_from_raw_entries :: self.dependency_tables.raw_entries, dep_name, "checksum" :: call

    fn dep_source_kind(read self: BookManifest, dep_name: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.dependency_tables.source_kind_entries, dep_name, "dependency source-kind entry" :: call

impl LockManifestV1:
    fn order(read self: LockManifestV1) -> Result[List[Str], Str]:
        return Result.Ok[List[Str], Str] :: (std.manifest.copy_strings :: self.metadata.ordered_members :: call) :: call

    fn deps_for(read self: LockManifestV1, member: Str) -> Result[List[Str], Str]:
        return Result.Ok[List[Str], Str] :: (std.manifest.lookup_name_list_or_empty :: self.lookup_tables.dependencies.dependency_lists, member :: call) :: call

    fn path_for(read self: LockManifestV1, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.lookup_tables.dependencies.path_entries, member, "lock path entry" :: call

    fn fingerprint_for(read self: LockManifestV1, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.lookup_tables.dependencies.fingerprint_entries, member, "lock fingerprint entry" :: call

    fn api_fingerprint_for(read self: LockManifestV1, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.lookup_tables.api_fingerprint_entries, member, "lock api fingerprint entry" :: call

    fn artifact_for(read self: LockManifestV1, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.output_tables.artifact_entries, member, "lock artifact entry" :: call

    fn kind_for(read self: LockManifestV1, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.output_tables.kind_entries, member, "lock kind entry" :: call

    fn format_for(read self: LockManifestV1, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.output_tables.format_entries, member, "lock format entry" :: call

impl LockManifestV2:
    fn order(read self: LockManifestV2) -> Result[List[Str], Str]:
        return Result.Ok[List[Str], Str] :: (std.manifest.copy_strings :: self.metadata.ordered_members :: call) :: call

    fn package_ids(read self: LockManifestV2) -> Result[List[Str], Str]:
        return Result.Ok[List[Str], Str] :: (std.manifest.copy_strings :: self.member_tables.package_ids :: call) :: call

    fn workspace_members(read self: LockManifestV2) -> Result[List[Str], Str]:
        return Result.Ok[List[Str], Str] :: (std.manifest.copy_strings :: self.build_tables.workspace_members :: call) :: call

    fn workspace_root(read self: LockManifestV2) -> Result[Str, Str]:
        return Result.Ok[Str, Str] :: self.build_tables.workspace_root :: call

    fn deps_for(read self: LockManifestV2, member: Str) -> Result[List[Str], Str]:
        return Result.Ok[List[Str], Str] :: (std.manifest.lookup_dependency_values_or_empty :: self.member_tables.dependency_binding_entries, member :: call) :: call

    fn dep_for(read self: LockManifestV2, member: Str, alias: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.member_tables.dependency_binding_entries, (std.manifest.dependency_binding_entry_name :: member, alias :: call), "lock dependency binding entry" :: call

    fn path_for(read self: LockManifestV2, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.member_tables.field_entries, (std.manifest.package_field_entry_name :: member, "path" :: call), "lock path entry" :: call

    fn kind_for(read self: LockManifestV2, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.member_tables.field_entries, (std.manifest.package_field_entry_name :: member, "kind" :: call), "lock kind entry" :: call

    fn name_for(read self: LockManifestV2, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.member_tables.field_entries, (std.manifest.package_field_entry_name :: member, "name" :: call), "lock package name entry" :: call

    fn source_kind_for(read self: LockManifestV2, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.member_tables.field_entries, (std.manifest.package_field_entry_name :: member, "source_kind" :: call), "lock package source-kind entry" :: call

    fn version_for(read self: LockManifestV2, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.member_tables.field_entries, (std.manifest.package_field_entry_name :: member, "version" :: call), "lock package version entry" :: call

    fn registry_for(read self: LockManifestV2, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.member_tables.field_entries, (std.manifest.package_field_entry_name :: member, "registry" :: call), "lock package registry entry" :: call

    fn checksum_for(read self: LockManifestV2, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.member_tables.field_entries, (std.manifest.package_field_entry_name :: member, "checksum" :: call), "lock package checksum entry" :: call

    fn git_for(read self: LockManifestV2, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.member_tables.field_entries, (std.manifest.package_field_entry_name :: member, "git" :: call), "lock package git entry" :: call

    fn git_selector_for(read self: LockManifestV2, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.member_tables.field_entries, (std.manifest.package_field_entry_name :: member, "git_selector" :: call), "lock package git selector entry" :: call

    fn native_product_names_for(read self: LockManifestV2, member: Str) -> Result[List[Str], Str]:
        return Result.Ok[List[Str], Str] :: (std.manifest.collect_native_product_names_or_empty :: self.build_tables.manifest_tables.native_product_field_entries, member :: call) :: call

    fn native_product_kind_for(read self: LockManifestV2, member: Str, product_name: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.build_tables.manifest_tables.native_product_field_entries, (std.manifest.native_product_field_entry_name :: member, product_name, "kind" :: call), "lock native product kind entry" :: call

    fn native_product_role_for(read self: LockManifestV2, member: Str, product_name: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.build_tables.manifest_tables.native_product_field_entries, (std.manifest.native_product_field_entry_name :: member, product_name, "role" :: call), "lock native product role entry" :: call

    fn native_product_producer_for(read self: LockManifestV2, member: Str, product_name: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.build_tables.manifest_tables.native_product_field_entries, (std.manifest.native_product_field_entry_name :: member, product_name, "producer" :: call), "lock native product producer entry" :: call

    fn native_product_file_for(read self: LockManifestV2, member: Str, product_name: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.build_tables.manifest_tables.native_product_field_entries, (std.manifest.native_product_field_entry_name :: member, product_name, "file" :: call), "lock native product file entry" :: call

    fn native_product_contract_for(read self: LockManifestV2, member: Str, product_name: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.build_tables.manifest_tables.native_product_field_entries, (std.manifest.native_product_field_entry_name :: member, product_name, "contract" :: call), "lock native product contract entry" :: call

    fn native_product_rust_cdylib_crate_for(read self: LockManifestV2, member: Str, product_name: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.build_tables.manifest_tables.native_product_field_entries, (std.manifest.native_product_field_entry_name :: member, product_name, "rust_cdylib_crate" :: call), "lock native product rust cdylib crate entry" :: call

    fn native_product_sidecars_for(read self: LockManifestV2, member: Str, product_name: Str) -> Result[List[Str], Str]:
        return Result.Ok[List[Str], Str] :: (std.manifest.lookup_name_list_or_empty :: self.build_tables.manifest_tables.native_product_sidecar_entries, (std.manifest.native_product_field_entry_name :: member, product_name, "sidecars" :: call) :: call) :: call

    fn targets_for(read self: LockManifestV2, member: Str) -> Result[List[Str], Str]:
        let targets = std.manifest.lookup_build_target_names_or_empty :: self.build_tables.manifest_tables.build_entries, member :: call
        if (targets :: :: len) == 0:
            return Result.Err[List[Str], Str] :: ("missing lock build entry for `" + member + "`") :: call
        return Result.Ok[List[Str], Str] :: targets :: call

    fn fingerprint_for(read self: LockManifestV2, member: Str, target: Str) -> Result[Str, Str]:
        for entry in self.build_tables.manifest_tables.build_entries:
            if entry.address.member == member and entry.address.target == target:
                return Result.Ok[Str, Str] :: entry.metadata.fingerprint :: call
        return Result.Err[Str, Str] :: ("missing lock fingerprint entry for `" + member + "` target `" + target + "`") :: call

    fn api_fingerprint_for(read self: LockManifestV2, member: Str, target: Str) -> Result[Str, Str]:
        for entry in self.build_tables.manifest_tables.build_entries:
            if entry.address.member == member and entry.address.target == target:
                return Result.Ok[Str, Str] :: entry.metadata.api_fingerprint :: call
        return Result.Err[Str, Str] :: ("missing lock api fingerprint entry for `" + member + "` target `" + target + "`") :: call

    fn artifact_for(read self: LockManifestV2, member: Str, target: Str) -> Result[Str, Str]:
        for entry in self.build_tables.manifest_tables.build_entries:
            if entry.address.member == member and entry.address.target == target:
                return Result.Ok[Str, Str] :: entry.output.artifact :: call
        return Result.Err[Str, Str] :: ("missing lock artifact entry for `" + member + "` target `" + target + "`") :: call

    fn artifact_hash_for(read self: LockManifestV2, member: Str, target: Str) -> Result[Str, Str]:
        for entry in self.build_tables.manifest_tables.build_entries:
            if entry.address.member == member and entry.address.target == target:
                return Result.Ok[Str, Str] :: entry.metadata.artifact_hash :: call
        return Result.Err[Str, Str] :: ("missing lock artifact hash entry for `" + member + "` target `" + target + "`") :: call

    fn format_for(read self: LockManifestV2, member: Str, target: Str) -> Result[Str, Str]:
        for entry in self.build_tables.manifest_tables.build_entries:
            if entry.address.member == member and entry.address.target == target:
                return Result.Ok[Str, Str] :: entry.output.format :: call
        return Result.Err[Str, Str] :: ("missing lock format entry for `" + member + "` target `" + target + "`") :: call

    fn toolchain_for(read self: LockManifestV2, member: Str, target: Str) -> Result[Str, Str]:
        for entry in self.build_tables.manifest_tables.build_entries:
            if entry.address.member == member and entry.address.target == target:
                return Result.Ok[Str, Str] :: entry.output.toolchain :: call
        return Result.Err[Str, Str] :: ("missing lock toolchain entry for `" + member + "` target `" + target + "`") :: call

export fn parse_book(text: Str) -> Result[BookManifest, Str]:
    let doc_result = std.config.parse_document :: text :: call
    if doc_result :: :: is_err:
        return Result.Err[BookManifest, Str] :: (std.manifest.config_doc_err_or_empty :: doc_result :: call) :: call
    let doc = std.manifest.config_doc_or_empty :: doc_result :: call
    if not (doc :: "name" :: root_has_key):
        return Result.Err[BookManifest, Str] :: "missing `name` in book.toml" :: call
    let name_result = doc :: "name", "book field" :: root_required_string
    if name_result :: :: is_err:
        return Result.Err[BookManifest, Str] :: (std.manifest.string_err_or_empty :: name_result :: call) :: call
    let kind_result = doc :: "kind", "app" :: root_string_or
    if kind_result :: :: is_err:
        return Result.Err[BookManifest, Str] :: (std.manifest.string_err_or_empty :: kind_result :: call) :: call
    let version_result = doc :: "version", "" :: root_string_or
    if version_result :: :: is_err:
        return Result.Err[BookManifest, Str] :: (std.manifest.string_err_or_empty :: version_result :: call) :: call
    let members_result = doc :: "workspace", "members" :: section_string_array_or_empty
    if members_result :: :: is_err:
        return Result.Err[BookManifest, Str] :: (std.manifest.strings_err_or_empty :: members_result :: call) :: call
    let dep_paths_result = std.manifest.collect_dep_paths :: doc :: call
    if dep_paths_result :: :: is_err:
        return Result.Err[BookManifest, Str] :: (std.manifest.name_values_err_or_empty :: dep_paths_result :: call) :: call
    let dep_raw_result = std.manifest.collect_dep_raw_values :: doc :: call
    if dep_raw_result :: :: is_err:
        return Result.Err[BookManifest, Str] :: (std.manifest.name_values_err_or_empty :: dep_raw_result :: call) :: call
    let dep_source_kinds_result = std.manifest.collect_dep_source_kinds :: doc :: call
    if dep_source_kinds_result :: :: is_err:
        return Result.Err[BookManifest, Str] :: (std.manifest.name_values_err_or_empty :: dep_source_kinds_result :: call) :: call
    let kind = std.manifest.string_or_empty :: kind_result :: call
    if kind != "app" and kind != "lib":
        return Result.Err[BookManifest, Str] :: ("`kind` must be \"app\" or \"lib\" (found `" + kind + "`)") :: call
    let state = std.manifest.BookState :: name = (std.manifest.string_or_empty :: name_result :: call), kind = kind, workspace_member_names = (std.manifest.strings_or_empty :: members_result :: call) :: call
    let dependency_tables = std.manifest.BookDependencyTables :: path_entries = (std.manifest.name_values_or_empty :: dep_paths_result :: call), raw_entries = (std.manifest.name_values_or_empty :: dep_raw_result :: call), source_kind_entries = (std.manifest.name_values_or_empty :: dep_source_kinds_result :: call) :: call
    let manifest = std.manifest.BookManifest :: state = state, package_version = (std.manifest.string_or_empty :: version_result :: call), dependency_tables = dependency_tables :: call
    return Result.Ok[BookManifest, Str] :: manifest :: call

export fn parse_lock_v1(text: Str) -> Result[LockManifestV1, Str]:
    let doc_result = std.config.parse_document :: text :: call
    if doc_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (std.manifest.config_doc_err_or_empty :: doc_result :: call) :: call
    let doc = std.manifest.config_doc_or_empty :: doc_result :: call
    let version = doc :: "version", 0 :: root_int_or
    if version != 1:
        return Result.Err[LockManifestV1, Str] :: "Arcana.lock version must be 1" :: call
    if not (doc :: "workspace" :: root_has_key):
        return Result.Err[LockManifestV1, Str] :: "missing `workspace` in Arcana.lock" :: call
    let workspace_result = doc :: "workspace", "lock field" :: root_required_string
    if workspace_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (std.manifest.string_err_or_empty :: workspace_result :: call) :: call
    if not (doc :: "paths" :: has_section):
        return Result.Err[LockManifestV1, Str] :: "missing `[paths]` in Arcana.lock" :: call
    if not (doc :: "deps" :: has_section):
        return Result.Err[LockManifestV1, Str] :: "missing `[deps]` in Arcana.lock" :: call
    if not (doc :: "fingerprints" :: has_section):
        return Result.Err[LockManifestV1, Str] :: "missing `[fingerprints]` in Arcana.lock" :: call
    if not (doc :: "api_fingerprints" :: has_section):
        return Result.Err[LockManifestV1, Str] :: "missing `[api_fingerprints]` in Arcana.lock" :: call
    if not (doc :: "artifacts" :: has_section):
        return Result.Err[LockManifestV1, Str] :: "missing `[artifacts]` in Arcana.lock" :: call
    if not (doc :: "kinds" :: has_section):
        return Result.Err[LockManifestV1, Str] :: "missing `[kinds]` in Arcana.lock" :: call
    if not (doc :: "formats" :: has_section):
        return Result.Err[LockManifestV1, Str] :: "missing `[formats]` in Arcana.lock" :: call
    let order_result = doc :: "order" :: root_string_array_or_empty
    if order_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (std.manifest.strings_err_or_empty :: order_result :: call) :: call
    let deps_result = std.manifest.collect_named_lists :: doc, "deps" :: call
    if deps_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (std.manifest.name_lists_err_or_empty :: deps_result :: call) :: call
    let paths_result = std.manifest.collect_named_values :: doc, "paths" :: call
    if paths_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (std.manifest.name_values_err_or_empty :: paths_result :: call) :: call
    let fingerprints_result = std.manifest.collect_named_values :: doc, "fingerprints" :: call
    if fingerprints_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (std.manifest.name_values_err_or_empty :: fingerprints_result :: call) :: call
    let api_fingerprints_result = std.manifest.collect_named_values :: doc, "api_fingerprints" :: call
    if api_fingerprints_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (std.manifest.name_values_err_or_empty :: api_fingerprints_result :: call) :: call
    let artifacts_result = std.manifest.collect_named_values :: doc, "artifacts" :: call
    if artifacts_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (std.manifest.name_values_err_or_empty :: artifacts_result :: call) :: call
    let kinds_result = std.manifest.collect_named_values :: doc, "kinds" :: call
    if kinds_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (std.manifest.name_values_err_or_empty :: kinds_result :: call) :: call
    let formats_result = std.manifest.collect_named_values :: doc, "formats" :: call
    if formats_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (std.manifest.name_values_err_or_empty :: formats_result :: call) :: call
    let metadata = std.manifest.LockMetadata :: version = version, workspace = (std.manifest.string_or_empty :: workspace_result :: call), ordered_members = (std.manifest.strings_or_empty :: order_result :: call) :: call
    let dependency_tables = std.manifest.LockDependencyTables :: dependency_lists = (std.manifest.name_lists_or_empty :: deps_result :: call), path_entries = (std.manifest.name_values_or_empty :: paths_result :: call), fingerprint_entries = (std.manifest.name_values_or_empty :: fingerprints_result :: call) :: call
    let lookup_tables = std.manifest.LockLookupTables :: dependencies = dependency_tables, api_fingerprint_entries = (std.manifest.name_values_or_empty :: api_fingerprints_result :: call) :: call
    let output_tables = std.manifest.LockOutputTables :: artifact_entries = (std.manifest.name_values_or_empty :: artifacts_result :: call), kind_entries = (std.manifest.name_values_or_empty :: kinds_result :: call), format_entries = (std.manifest.name_values_or_empty :: formats_result :: call) :: call
    let manifest = std.manifest.LockManifestV1 :: metadata = metadata, lookup_tables = lookup_tables, output_tables = output_tables :: call
    return Result.Ok[LockManifestV1, Str] :: manifest :: call

export fn parse_lock(text: Str) -> Result[LockManifestV2, Str]:
    let doc_result = std.config.parse_document :: text :: call
    if doc_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.config_doc_err_or_empty :: doc_result :: call) :: call
    let doc = std.manifest.config_doc_or_empty :: doc_result :: call
    let version = doc :: "version", 0 :: root_int_or
    if version == 1:
        return std.manifest.parse_lock_v1_as_v2 :: text :: call
    if version == 2:
        return std.manifest.parse_lock_v2 :: text :: call
    if version == 3:
        return std.manifest.parse_lock_v3 :: text :: call
    if version == 4:
        return std.manifest.parse_lock_v4 :: text :: call
    return Result.Err[LockManifestV2, Str] :: "Arcana.lock version must be 1, 2, 3, or 4" :: call

fn parse_lock_v1_as_v2(text: Str) -> Result[LockManifestV2, Str]:
    let doc_result = std.config.parse_document :: text :: call
    if doc_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.config_doc_err_or_empty :: doc_result :: call) :: call
    let doc = std.manifest.config_doc_or_empty :: doc_result :: call
    let version = doc :: "version", 0 :: root_int_or
    if version != 1:
        return Result.Err[LockManifestV2, Str] :: "Arcana.lock version must be 1" :: call
    if not (doc :: "workspace" :: root_has_key):
        return Result.Err[LockManifestV2, Str] :: "missing `workspace` in Arcana.lock" :: call
    let workspace_result = doc :: "workspace", "lock field" :: root_required_string
    if workspace_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.string_err_or_empty :: workspace_result :: call) :: call
    if not (doc :: "paths" :: has_section):
        return Result.Err[LockManifestV2, Str] :: "missing `[paths]` in Arcana.lock" :: call
    if not (doc :: "deps" :: has_section):
        return Result.Err[LockManifestV2, Str] :: "missing `[deps]` in Arcana.lock" :: call
    if not (doc :: "fingerprints" :: has_section):
        return Result.Err[LockManifestV2, Str] :: "missing `[fingerprints]` in Arcana.lock" :: call
    if not (doc :: "api_fingerprints" :: has_section):
        return Result.Err[LockManifestV2, Str] :: "missing `[api_fingerprints]` in Arcana.lock" :: call
    if not (doc :: "artifacts" :: has_section):
        return Result.Err[LockManifestV2, Str] :: "missing `[artifacts]` in Arcana.lock" :: call
    if not (doc :: "kinds" :: has_section):
        return Result.Err[LockManifestV2, Str] :: "missing `[kinds]` in Arcana.lock" :: call
    if not (doc :: "formats" :: has_section):
        return Result.Err[LockManifestV2, Str] :: "missing `[formats]` in Arcana.lock" :: call
    let order_result = doc :: "order" :: root_string_array_or_empty
    if order_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.strings_err_or_empty :: order_result :: call) :: call
    let deps_result = std.manifest.collect_named_lists :: doc, "deps" :: call
    if deps_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_lists_err_or_empty :: deps_result :: call) :: call
    let paths_result = std.manifest.collect_named_values :: doc, "paths" :: call
    if paths_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: paths_result :: call) :: call
    let fingerprints_result = std.manifest.collect_named_values :: doc, "fingerprints" :: call
    if fingerprints_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: fingerprints_result :: call) :: call
    let api_fingerprints_result = std.manifest.collect_named_values :: doc, "api_fingerprints" :: call
    if api_fingerprints_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: api_fingerprints_result :: call) :: call
    let artifacts_result = std.manifest.collect_named_values :: doc, "artifacts" :: call
    if artifacts_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: artifacts_result :: call) :: call
    let kinds_result = std.manifest.collect_named_values :: doc, "kinds" :: call
    if kinds_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: kinds_result :: call) :: call
    let formats_result = std.manifest.collect_named_values :: doc, "formats" :: call
    if formats_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: formats_result :: call) :: call
    let targets_result = std.manifest.collect_named_values_if_present :: doc, "targets" :: call
    if targets_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: targets_result :: call) :: call
    let artifact_hashes_result = std.manifest.collect_named_values_if_present :: doc, "artifact_hashes" :: call
    if artifact_hashes_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: artifact_hashes_result :: call) :: call
    let toolchain_result = doc :: "toolchain", "" :: root_string_or
    if toolchain_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.string_err_or_empty :: toolchain_result :: call) :: call
    let path_entries = std.manifest.name_values_or_empty :: paths_result :: call
    let package_ids = std.manifest.names_from_name_values :: path_entries :: call
    let mut field_entries = std.manifest.empty_name_values :: :: call
    std.manifest.append_name_values :: field_entries, (std.manifest.collect_legacy_package_field_entries :: path_entries, "path" :: call) :: call
    std.manifest.append_name_values :: field_entries, (std.manifest.collect_legacy_package_field_entries :: (std.manifest.name_values_or_empty :: kinds_result :: call), "kind" :: call) :: call
    std.manifest.append_name_values :: field_entries, (std.manifest.collect_identity_package_field_entries :: package_ids, "name" :: call) :: call
    std.manifest.append_name_values :: field_entries, (std.manifest.collect_constant_package_field_entries :: package_ids, "source_kind", "path" :: call) :: call
    let mut build_entries = std.manifest.empty_build_entries :: :: call
    for package_id in package_ids:
        let fingerprint_result = std.manifest.lookup_name_value :: (std.manifest.name_values_or_empty :: fingerprints_result :: call), package_id, "lock fingerprint entry" :: call
        if fingerprint_result :: :: is_err:
            return Result.Err[LockManifestV2, Str] :: (std.manifest.string_err_or_empty :: fingerprint_result :: call) :: call
        let api_fingerprint_result = std.manifest.lookup_name_value :: (std.manifest.name_values_or_empty :: api_fingerprints_result :: call), package_id, "lock api fingerprint entry" :: call
        if api_fingerprint_result :: :: is_err:
            return Result.Err[LockManifestV2, Str] :: (std.manifest.string_err_or_empty :: api_fingerprint_result :: call) :: call
        let artifact_result = std.manifest.lookup_name_value :: (std.manifest.name_values_or_empty :: artifacts_result :: call), package_id, "lock artifact entry" :: call
        if artifact_result :: :: is_err:
            return Result.Err[LockManifestV2, Str] :: (std.manifest.string_err_or_empty :: artifact_result :: call) :: call
        let format_result = std.manifest.lookup_name_value :: (std.manifest.name_values_or_empty :: formats_result :: call), package_id, "lock format entry" :: call
        if format_result :: :: is_err:
            return Result.Err[LockManifestV2, Str] :: (std.manifest.string_err_or_empty :: format_result :: call) :: call
        let target_result = std.manifest.resolve_legacy_target :: (std.manifest.name_values_or_empty :: targets_result :: call), package_id, (std.manifest.string_or_empty :: format_result :: call) :: call
        if target_result :: :: is_err:
            return Result.Err[LockManifestV2, Str] :: (std.manifest.string_err_or_empty :: target_result :: call) :: call
        let artifact_hash = std.manifest.lookup_optional_name_value_or_empty :: (std.manifest.name_values_or_empty :: artifact_hashes_result :: call), package_id :: call
        let target = std.manifest.string_or_empty :: target_result :: call
        let address = std.manifest.LockBuildIdentity :: member = package_id, target = target :: call
        let metadata = std.manifest.LockBuildMetadata :: fingerprint = (std.manifest.string_or_empty :: fingerprint_result :: call), api_fingerprint = (std.manifest.string_or_empty :: api_fingerprint_result :: call), artifact_hash = artifact_hash :: call
        let output = std.manifest.LockBuildOutput :: artifact = (std.manifest.string_or_empty :: artifact_result :: call), format = (std.manifest.string_or_empty :: format_result :: call), toolchain = (std.manifest.string_or_empty :: toolchain_result :: call) :: call
        let entry = std.manifest.LockBuildEntry :: address = address, metadata = metadata, output = output :: call
        build_entries :: entry :: push
    let metadata = std.manifest.LockMetadata :: version = version, workspace = (std.manifest.string_or_empty :: workspace_result :: call), ordered_members = (std.manifest.strings_or_empty :: order_result :: call) :: call
    let member_tables = std.manifest.LockMemberTables :: package_ids = (std.manifest.names_from_name_values :: path_entries :: call), dependency_binding_entries = (std.manifest.collect_legacy_dependency_bindings :: (std.manifest.name_lists_or_empty :: deps_result :: call) :: call), field_entries = field_entries :: call
    let manifest_tables = std.manifest.LockBuildManifestTables :: build_entries = build_entries, native_product_field_entries = (std.manifest.empty_name_values :: :: call), native_product_sidecar_entries = (std.manifest.empty_name_lists :: :: call) :: call
    let build_tables = std.manifest.LockBuildTables :: workspace_root = (std.manifest.infer_legacy_workspace_root :: path_entries, (std.manifest.names_from_name_values :: path_entries :: call) :: call), workspace_members = (std.manifest.strings_or_empty :: order_result :: call), manifest_tables = manifest_tables :: call
    let manifest = std.manifest.LockManifestV2 :: metadata = metadata, member_tables = member_tables, build_tables = build_tables :: call
    return Result.Ok[LockManifestV2, Str] :: manifest :: call

fn parse_lock_v2_style(text: Str, expected_version: Int) -> Result[LockManifestV2, Str]:
    let doc_result = std.config.parse_document :: text :: call
    if doc_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.config_doc_err_or_empty :: doc_result :: call) :: call
    let doc = std.manifest.config_doc_or_empty :: doc_result :: call
    let version = doc :: "version", 0 :: root_int_or
    if version != expected_version:
        return Result.Err[LockManifestV2, Str] :: ("Arcana.lock version must be " + (std.text.from_int :: expected_version :: call)) :: call
    if not (doc :: "workspace" :: root_has_key):
        return Result.Err[LockManifestV2, Str] :: "missing `workspace` in Arcana.lock" :: call
    let workspace_result = doc :: "workspace", "lock field" :: root_required_string
    if workspace_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.string_err_or_empty :: workspace_result :: call) :: call
    if not (doc :: "paths" :: has_section):
        return Result.Err[LockManifestV2, Str] :: "missing `[paths]` in Arcana.lock" :: call
    if not (doc :: "deps" :: has_section):
        return Result.Err[LockManifestV2, Str] :: "missing `[deps]` in Arcana.lock" :: call
    if not (doc :: "kinds" :: has_section):
        return Result.Err[LockManifestV2, Str] :: "missing `[kinds]` in Arcana.lock" :: call
    if not (doc :: "builds" :: has_section):
        return Result.Err[LockManifestV2, Str] :: "missing `[builds]` in Arcana.lock" :: call
    let order_result = doc :: "order" :: root_string_array_or_empty
    if order_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.strings_err_or_empty :: order_result :: call) :: call
    let deps_result = std.manifest.collect_named_lists :: doc, "deps" :: call
    if deps_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_lists_err_or_empty :: deps_result :: call) :: call
    let paths_result = std.manifest.collect_named_values :: doc, "paths" :: call
    if paths_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: paths_result :: call) :: call
    let kinds_result = std.manifest.collect_named_values :: doc, "kinds" :: call
    if kinds_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: kinds_result :: call) :: call
    let native_product_fields_result = std.manifest.collect_native_product_field_entries_if_present :: doc :: call
    if native_product_fields_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: native_product_fields_result :: call) :: call
    let native_product_sidecars_result = std.manifest.collect_native_product_sidecar_entries_if_present :: doc :: call
    if native_product_sidecars_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_lists_err_or_empty :: native_product_sidecars_result :: call) :: call
    let path_entries = std.manifest.name_values_or_empty :: paths_result :: call
    let package_ids = std.manifest.names_from_name_values :: path_entries :: call
    let mut field_entries = std.manifest.empty_name_values :: :: call
    std.manifest.append_name_values :: field_entries, (std.manifest.collect_legacy_package_field_entries :: path_entries, "path" :: call) :: call
    std.manifest.append_name_values :: field_entries, (std.manifest.collect_legacy_package_field_entries :: (std.manifest.name_values_or_empty :: kinds_result :: call), "kind" :: call) :: call
    std.manifest.append_name_values :: field_entries, (std.manifest.collect_identity_package_field_entries :: package_ids, "name" :: call) :: call
    std.manifest.append_name_values :: field_entries, (std.manifest.collect_constant_package_field_entries :: package_ids, "source_kind", "path" :: call) :: call
    let builds_result = std.manifest.collect_build_entries :: doc, package_ids :: call
    if builds_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.build_entries_err_or_empty :: builds_result :: call) :: call
    let metadata = std.manifest.LockMetadata :: version = version, workspace = (std.manifest.string_or_empty :: workspace_result :: call), ordered_members = (std.manifest.strings_or_empty :: order_result :: call) :: call
    let member_tables = std.manifest.LockMemberTables :: package_ids = (std.manifest.names_from_name_values :: path_entries :: call), dependency_binding_entries = (std.manifest.collect_legacy_dependency_bindings :: (std.manifest.name_lists_or_empty :: deps_result :: call) :: call), field_entries = field_entries :: call
    let manifest_tables = std.manifest.LockBuildManifestTables :: build_entries = (std.manifest.build_entries_or_empty :: builds_result :: call), native_product_field_entries = (std.manifest.name_values_or_empty :: native_product_fields_result :: call), native_product_sidecar_entries = (std.manifest.name_lists_or_empty :: native_product_sidecars_result :: call) :: call
    let build_tables = std.manifest.LockBuildTables :: workspace_root = (std.manifest.infer_legacy_workspace_root :: path_entries, (std.manifest.names_from_name_values :: path_entries :: call) :: call), workspace_members = (std.manifest.strings_or_empty :: order_result :: call), manifest_tables = manifest_tables :: call
    let manifest = std.manifest.LockManifestV2 :: metadata = metadata, member_tables = member_tables, build_tables = build_tables :: call
    return Result.Ok[LockManifestV2, Str] :: manifest :: call

export fn parse_lock_v2(text: Str) -> Result[LockManifestV2, Str]:
    return std.manifest.parse_lock_v2_style :: text, 2 :: call

export fn parse_lock_v3(text: Str) -> Result[LockManifestV2, Str]:
    return std.manifest.parse_lock_v2_style :: text, 3 :: call

export fn parse_lock_v4(text: Str) -> Result[LockManifestV2, Str]:
    let doc_result = std.config.parse_document :: text :: call
    if doc_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.config_doc_err_or_empty :: doc_result :: call) :: call
    let doc = std.manifest.config_doc_or_empty :: doc_result :: call
    let version = doc :: "version", 0 :: root_int_or
    if version != 4:
        return Result.Err[LockManifestV2, Str] :: "Arcana.lock version must be 4" :: call
    if not (doc :: "workspace" :: root_has_key):
        return Result.Err[LockManifestV2, Str] :: "missing `workspace` in Arcana.lock" :: call
    if not (doc :: "workspace_root" :: root_has_key):
        return Result.Err[LockManifestV2, Str] :: "missing `workspace_root` in Arcana.lock" :: call
    if not (doc :: "workspace_members" :: root_has_key):
        return Result.Err[LockManifestV2, Str] :: "missing `workspace_members` in Arcana.lock" :: call
    let workspace_result = doc :: "workspace", "lock field" :: root_required_string
    if workspace_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.string_err_or_empty :: workspace_result :: call) :: call
    let workspace_root_result = doc :: "workspace_root", "lock field" :: root_required_string
    if workspace_root_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.string_err_or_empty :: workspace_root_result :: call) :: call
    let workspace_members_result = doc :: "workspace_members" :: root_string_array_or_empty
    if workspace_members_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.strings_err_or_empty :: workspace_members_result :: call) :: call
    if not (doc :: "packages" :: has_section):
        return Result.Err[LockManifestV2, Str] :: "missing `[packages]` in Arcana.lock" :: call
    if not (doc :: "dependencies" :: has_section):
        return Result.Err[LockManifestV2, Str] :: "missing `[dependencies]` in Arcana.lock" :: call
    if not (doc :: "builds" :: has_section):
        return Result.Err[LockManifestV2, Str] :: "missing `[builds]` in Arcana.lock" :: call
    let order_result = doc :: "order" :: root_string_array_or_empty
    if order_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.strings_err_or_empty :: order_result :: call) :: call
    let package_ids_result = std.manifest.collect_v4_package_ids :: doc :: call
    if package_ids_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.strings_err_or_empty :: package_ids_result :: call) :: call
    let deps_result = std.manifest.collect_v4_dependency_bindings :: doc :: call
    if deps_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: deps_result :: call) :: call
    let names_result = std.manifest.collect_v4_package_required_values :: doc, "name", "lock package name" :: call
    if names_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: names_result :: call) :: call
    let paths_result = std.manifest.collect_v4_package_optional_values :: doc, "path", "lock package path" :: call
    if paths_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: paths_result :: call) :: call
    let kinds_result = std.manifest.collect_v4_package_required_values :: doc, "kind", "lock package kind" :: call
    if kinds_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: kinds_result :: call) :: call
    let source_kinds_result = std.manifest.collect_v4_package_required_values :: doc, "source_kind", "lock package source kind" :: call
    if source_kinds_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: source_kinds_result :: call) :: call
    let versions_result = std.manifest.collect_v4_package_optional_values :: doc, "version", "lock package version" :: call
    if versions_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: versions_result :: call) :: call
    let registries_result = std.manifest.collect_v4_package_optional_values :: doc, "registry", "lock package registry" :: call
    if registries_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: registries_result :: call) :: call
    let checksums_result = std.manifest.collect_v4_package_optional_values :: doc, "checksum", "lock package checksum" :: call
    if checksums_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: checksums_result :: call) :: call
    let git_values_result = std.manifest.collect_v4_package_optional_values :: doc, "git", "lock package git" :: call
    if git_values_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: git_values_result :: call) :: call
    let git_selectors_result = std.manifest.collect_v4_package_optional_values :: doc, "git_selector", "lock package git selector" :: call
    if git_selectors_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: git_selectors_result :: call) :: call
    let native_product_fields_result = std.manifest.collect_v4_native_product_field_entries :: doc :: call
    if native_product_fields_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_values_err_or_empty :: native_product_fields_result :: call) :: call
    let native_product_sidecars_result = std.manifest.collect_v4_native_product_sidecar_entries :: doc :: call
    if native_product_sidecars_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.name_lists_err_or_empty :: native_product_sidecars_result :: call) :: call
    let package_ids = std.manifest.strings_or_empty :: package_ids_result :: call
    let mut field_entries = std.manifest.empty_name_values :: :: call
    for entry in (std.manifest.name_values_or_empty :: names_result :: call):
        field_entries :: entry :: push
    for entry in (std.manifest.name_values_or_empty :: kinds_result :: call):
        field_entries :: entry :: push
    for entry in (std.manifest.name_values_or_empty :: source_kinds_result :: call):
        field_entries :: entry :: push
    for entry in (std.manifest.name_values_or_empty :: paths_result :: call):
        field_entries :: entry :: push
    for entry in (std.manifest.name_values_or_empty :: versions_result :: call):
        field_entries :: entry :: push
    for entry in (std.manifest.name_values_or_empty :: registries_result :: call):
        field_entries :: entry :: push
    for entry in (std.manifest.name_values_or_empty :: checksums_result :: call):
        field_entries :: entry :: push
    for entry in (std.manifest.name_values_or_empty :: git_values_result :: call):
        field_entries :: entry :: push
    for entry in (std.manifest.name_values_or_empty :: git_selectors_result :: call):
        field_entries :: entry :: push
    let builds_result = std.manifest.collect_build_entries :: doc, package_ids :: call
    if builds_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (std.manifest.build_entries_err_or_empty :: builds_result :: call) :: call
    let metadata = std.manifest.LockMetadata :: version = version, workspace = (std.manifest.string_or_empty :: workspace_result :: call), ordered_members = (std.manifest.strings_or_empty :: order_result :: call) :: call
    let member_tables = std.manifest.LockMemberTables :: package_ids = package_ids, dependency_binding_entries = (std.manifest.name_values_or_empty :: deps_result :: call), field_entries = field_entries :: call
    let manifest_tables = std.manifest.LockBuildManifestTables :: build_entries = (std.manifest.build_entries_or_empty :: builds_result :: call), native_product_field_entries = (std.manifest.name_values_or_empty :: native_product_fields_result :: call), native_product_sidecar_entries = (std.manifest.name_lists_or_empty :: native_product_sidecars_result :: call) :: call
    let build_tables = std.manifest.LockBuildTables :: workspace_root = (std.manifest.string_or_empty :: workspace_root_result :: call), workspace_members = (std.manifest.strings_or_empty :: workspace_members_result :: call), manifest_tables = manifest_tables :: call
    let manifest = std.manifest.LockManifestV2 :: metadata = metadata, member_tables = member_tables, build_tables = build_tables :: call
    return Result.Ok[LockManifestV2, Str] :: manifest :: call
