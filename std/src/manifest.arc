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

export record BookManifest:
    state: std.manifest.BookState
    dependency_paths: List[std.manifest.NameValue]

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
    dependency_lists: List[std.manifest.NameList]
    path_entries: List[std.manifest.NameValue]
    kind_entries: List[std.manifest.NameValue]

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

export record LockBuildTables:
    build_entries: List[std.manifest.LockBuildEntry]

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

fn empty_decoded_name_span() -> std.manifest.DecodedNameSpan:
    return std.manifest.DecodedNameSpan :: value = "", next_index = 0 :: call

fn empty_parsed_build_section() -> std.manifest.ParsedBuildSection:
    return std.manifest.ParsedBuildSection :: member = "", target = "" :: call

fn add_name_value(edit out: List[std.manifest.NameValue], read name: Str, read value: Str):
    let pair = std.manifest.NameValue :: name = name, value = value :: call
    out :: pair :: push

fn add_name_list(edit out: List[std.manifest.NameList], read name: Str, read values: List[Str]):
    let pair = std.manifest.NameList :: name = name, values = values :: call
    out :: pair :: push

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

fn parse_build_section_name(read section: Str) -> Result[std.manifest.ParsedBuildSection, Str]:
    let prefix = "builds."
    if not (std.text.starts_with :: section, prefix :: call):
        return Result.Err[std.manifest.ParsedBuildSection, Str] :: "expected `[builds.\"member\".\"target\"]` section" :: call
    let member_result = std.manifest.decode_quoted_name_component :: section, (std.text.len_bytes :: prefix :: call) :: call
    if member_result :: :: is_err:
        return Result.Err[std.manifest.ParsedBuildSection, Str] :: "invalid build member name" :: call
    let member_span = member_result :: (std.manifest.empty_decoded_name_span :: :: call) :: unwrap_or
    let n = std.text.len_bytes :: section :: call
    if member_span.next_index >= n or (std.text.byte_at :: section, member_span.next_index :: call) != 46:
        return Result.Err[std.manifest.ParsedBuildSection, Str] :: "expected target separator in build section" :: call
    let target_result = std.manifest.decode_quoted_name_component :: section, member_span.next_index + 1 :: call
    if target_result :: :: is_err:
        return Result.Err[std.manifest.ParsedBuildSection, Str] :: "invalid build target name" :: call
    let target_span = target_result :: (std.manifest.empty_decoded_name_span :: :: call) :: unwrap_or
    if target_span.next_index != n:
        return Result.Err[std.manifest.ParsedBuildSection, Str] :: "unexpected suffix in build section" :: call
    let parsed = std.manifest.ParsedBuildSection :: member = member_span.value, target = target_span.value :: call
    return Result.Ok[std.manifest.ParsedBuildSection, Str] :: parsed :: call

fn has_named_value(read entries: List[std.manifest.NameValue], read name: Str) -> Bool:
    for entry in entries:
        if entry.name == name:
            return true
    return false

fn collect_build_entries(read doc: std.config.ConfigDoc, read path_entries: List[std.manifest.NameValue]) -> Result[List[std.manifest.LockBuildEntry], Str]:
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
        if not (std.manifest.has_named_value :: path_entries, parsed.member :: call):
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
        let metadata = std.manifest.LockBuildMetadata :: fingerprint = (fingerprint_result :: "" :: unwrap_or), api_fingerprint = (api_fingerprint_result :: "" :: unwrap_or), artifact_hash = (artifact_hash_result :: "" :: unwrap_or) :: call
        let output = std.manifest.LockBuildOutput :: artifact = (artifact_result :: "" :: unwrap_or), format = (format_result :: "" :: unwrap_or), toolchain = (toolchain_result :: "" :: unwrap_or) :: call
        let entry = std.manifest.LockBuildEntry :: address = address, metadata = metadata, output = output :: call
        out :: entry :: push
    for path_entry in path_entries:
        let targets = std.manifest.lookup_build_target_names_or_empty :: out, path_entry.name :: call
        if (targets :: :: len) == 0:
            return Result.Err[List[std.manifest.LockBuildEntry], Str] :: ("missing lock build entry for `" + path_entry.name + "`") :: call
    return Result.Ok[List[std.manifest.LockBuildEntry], Str] :: out :: call

fn collect_dep_paths(read doc: std.config.ConfigDoc) -> Result[List[std.manifest.NameValue], Str]:
    let mut out = std.manifest.empty_name_values :: :: call
    let deps = doc :: "deps" :: entries_in_section
    for dep in deps:
        let path_result = doc :: "deps", (dep.key, "path"), "dependency entry" :: section_inline_table_string_field
        if path_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: (path_result :: "" :: unwrap_or) :: call
        std.manifest.add_name_value :: out, dep.key, (path_result :: "" :: unwrap_or) :: call
    return Result.Ok[List[std.manifest.NameValue], Str] :: out :: call

fn collect_named_values(read doc: std.config.ConfigDoc, read section: Str) -> Result[List[std.manifest.NameValue], Str]:
    let mut out = std.manifest.empty_name_values :: :: call
    let entries = doc :: section :: entries_in_section
    for entry in entries:
        let value_result = doc :: section, entry.key, ("`[" + section + "]` entry") :: section_required
        if value_result :: :: is_err:
            return Result.Err[List[std.manifest.NameValue], Str] :: (value_result :: "" :: unwrap_or) :: call
        std.manifest.add_name_value :: out, entry.key, (value_result :: "" :: unwrap_or) :: call
    return Result.Ok[List[std.manifest.NameValue], Str] :: out :: call

fn collect_named_lists(read doc: std.config.ConfigDoc, read section: Str) -> Result[List[std.manifest.NameList], Str]:
    let mut out = std.manifest.empty_name_lists :: :: call
    let entries = doc :: section :: entries_in_section
    for entry in entries:
        let values_result = doc :: section, entry.key :: section_string_array_or_empty
        if values_result :: :: is_err:
            return Result.Err[List[std.manifest.NameList], Str] :: (values_result :: "" :: unwrap_or) :: call
        std.manifest.add_name_list :: out, entry.key, (values_result :: (std.manifest.empty_strings :: :: call) :: unwrap_or) :: call
    return Result.Ok[List[std.manifest.NameList], Str] :: out :: call

impl BookManifest:
    fn workspace_members(read self: BookManifest) -> Result[List[Str], Str]:
        return Result.Ok[List[Str], Str] :: (std.manifest.copy_strings :: self.state.workspace_member_names :: call) :: call

    fn dep_path(read self: BookManifest, dep_name: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.dependency_paths, dep_name, "dependency entry" :: call

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

    fn deps_for(read self: LockManifestV2, member: Str) -> Result[List[Str], Str]:
        return Result.Ok[List[Str], Str] :: (std.manifest.lookup_name_list_or_empty :: self.member_tables.dependency_lists, member :: call) :: call

    fn path_for(read self: LockManifestV2, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.member_tables.path_entries, member, "lock path entry" :: call

    fn kind_for(read self: LockManifestV2, member: Str) -> Result[Str, Str]:
        return std.manifest.lookup_name_value :: self.member_tables.kind_entries, member, "lock kind entry" :: call

    fn targets_for(read self: LockManifestV2, member: Str) -> Result[List[Str], Str]:
        let targets = std.manifest.lookup_build_target_names_or_empty :: self.build_tables.build_entries, member :: call
        if (targets :: :: len) == 0:
            return Result.Err[List[Str], Str] :: ("missing lock build entry for `" + member + "`") :: call
        return Result.Ok[List[Str], Str] :: targets :: call

    fn fingerprint_for(read self: LockManifestV2, member: Str, target: Str) -> Result[Str, Str]:
        for entry in self.build_tables.build_entries:
            if entry.address.member == member and entry.address.target == target:
                return Result.Ok[Str, Str] :: entry.metadata.fingerprint :: call
        return Result.Err[Str, Str] :: ("missing lock fingerprint entry for `" + member + "` target `" + target + "`") :: call

    fn api_fingerprint_for(read self: LockManifestV2, member: Str, target: Str) -> Result[Str, Str]:
        for entry in self.build_tables.build_entries:
            if entry.address.member == member and entry.address.target == target:
                return Result.Ok[Str, Str] :: entry.metadata.api_fingerprint :: call
        return Result.Err[Str, Str] :: ("missing lock api fingerprint entry for `" + member + "` target `" + target + "`") :: call

    fn artifact_for(read self: LockManifestV2, member: Str, target: Str) -> Result[Str, Str]:
        for entry in self.build_tables.build_entries:
            if entry.address.member == member and entry.address.target == target:
                return Result.Ok[Str, Str] :: entry.output.artifact :: call
        return Result.Err[Str, Str] :: ("missing lock artifact entry for `" + member + "` target `" + target + "`") :: call

    fn artifact_hash_for(read self: LockManifestV2, member: Str, target: Str) -> Result[Str, Str]:
        for entry in self.build_tables.build_entries:
            if entry.address.member == member and entry.address.target == target:
                return Result.Ok[Str, Str] :: entry.metadata.artifact_hash :: call
        return Result.Err[Str, Str] :: ("missing lock artifact hash entry for `" + member + "` target `" + target + "`") :: call

    fn format_for(read self: LockManifestV2, member: Str, target: Str) -> Result[Str, Str]:
        for entry in self.build_tables.build_entries:
            if entry.address.member == member and entry.address.target == target:
                return Result.Ok[Str, Str] :: entry.output.format :: call
        return Result.Err[Str, Str] :: ("missing lock format entry for `" + member + "` target `" + target + "`") :: call

    fn toolchain_for(read self: LockManifestV2, member: Str, target: Str) -> Result[Str, Str]:
        for entry in self.build_tables.build_entries:
            if entry.address.member == member and entry.address.target == target:
                return Result.Ok[Str, Str] :: entry.output.toolchain :: call
        return Result.Err[Str, Str] :: ("missing lock toolchain entry for `" + member + "` target `" + target + "`") :: call

export fn parse_book(text: Str) -> Result[BookManifest, Str]:
    let doc_result = std.config.parse_document :: text :: call
    if doc_result :: :: is_err:
        return Result.Err[BookManifest, Str] :: (doc_result :: "" :: unwrap_or) :: call
    let doc = doc_result :: (std.manifest.empty_config_doc :: :: call) :: unwrap_or
    if not (doc :: "name" :: root_has_key):
        return Result.Err[BookManifest, Str] :: "missing `name` in book.toml" :: call
    let name_result = doc :: "name", "book field" :: root_required_string
    if name_result :: :: is_err:
        return Result.Err[BookManifest, Str] :: (name_result :: "" :: unwrap_or) :: call
    let kind_result = doc :: "kind", "app" :: root_string_or
    if kind_result :: :: is_err:
        return Result.Err[BookManifest, Str] :: (kind_result :: "" :: unwrap_or) :: call
    let members_result = doc :: "workspace", "members" :: section_string_array_or_empty
    if members_result :: :: is_err:
        return Result.Err[BookManifest, Str] :: (members_result :: "" :: unwrap_or) :: call
    let deps_result = std.manifest.collect_dep_paths :: doc :: call
    if deps_result :: :: is_err:
        return Result.Err[BookManifest, Str] :: (deps_result :: "" :: unwrap_or) :: call
    let kind = kind_result :: "" :: unwrap_or
    if kind != "app" and kind != "lib":
        return Result.Err[BookManifest, Str] :: ("`kind` must be \"app\" or \"lib\" (found `" + kind + "`)") :: call
    let state = std.manifest.BookState :: name = (name_result :: "" :: unwrap_or), kind = kind, workspace_member_names = (members_result :: (std.manifest.empty_strings :: :: call) :: unwrap_or) :: call
    let manifest = std.manifest.BookManifest :: state = state, dependency_paths = (deps_result :: (std.manifest.empty_name_values :: :: call) :: unwrap_or) :: call
    return Result.Ok[BookManifest, Str] :: manifest :: call

export fn parse_lock_v1(text: Str) -> Result[LockManifestV1, Str]:
    let doc_result = std.config.parse_document :: text :: call
    if doc_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (doc_result :: "" :: unwrap_or) :: call
    let doc = doc_result :: (std.manifest.empty_config_doc :: :: call) :: unwrap_or
    let version = doc :: "version", 0 :: root_int_or
    if version != 1:
        return Result.Err[LockManifestV1, Str] :: "Arcana.lock version must be 1" :: call
    if not (doc :: "workspace" :: root_has_key):
        return Result.Err[LockManifestV1, Str] :: "missing `workspace` in Arcana.lock" :: call
    let workspace_result = doc :: "workspace", "lock field" :: root_required_string
    if workspace_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (workspace_result :: "" :: unwrap_or) :: call
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
        return Result.Err[LockManifestV1, Str] :: (order_result :: "" :: unwrap_or) :: call
    let deps_result = std.manifest.collect_named_lists :: doc, "deps" :: call
    if deps_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (deps_result :: "" :: unwrap_or) :: call
    let paths_result = std.manifest.collect_named_values :: doc, "paths" :: call
    if paths_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (paths_result :: "" :: unwrap_or) :: call
    let fingerprints_result = std.manifest.collect_named_values :: doc, "fingerprints" :: call
    if fingerprints_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (fingerprints_result :: "" :: unwrap_or) :: call
    let api_fingerprints_result = std.manifest.collect_named_values :: doc, "api_fingerprints" :: call
    if api_fingerprints_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (api_fingerprints_result :: "" :: unwrap_or) :: call
    let artifacts_result = std.manifest.collect_named_values :: doc, "artifacts" :: call
    if artifacts_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (artifacts_result :: "" :: unwrap_or) :: call
    let kinds_result = std.manifest.collect_named_values :: doc, "kinds" :: call
    if kinds_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (kinds_result :: "" :: unwrap_or) :: call
    let formats_result = std.manifest.collect_named_values :: doc, "formats" :: call
    if formats_result :: :: is_err:
        return Result.Err[LockManifestV1, Str] :: (formats_result :: "" :: unwrap_or) :: call
    let metadata = std.manifest.LockMetadata :: version = version, workspace = (workspace_result :: "" :: unwrap_or), ordered_members = (order_result :: (std.manifest.empty_strings :: :: call) :: unwrap_or) :: call
    let dependency_tables = std.manifest.LockDependencyTables :: dependency_lists = (deps_result :: (std.manifest.empty_name_lists :: :: call) :: unwrap_or), path_entries = (paths_result :: (std.manifest.empty_name_values :: :: call) :: unwrap_or), fingerprint_entries = (fingerprints_result :: (std.manifest.empty_name_values :: :: call) :: unwrap_or) :: call
    let lookup_tables = std.manifest.LockLookupTables :: dependencies = dependency_tables, api_fingerprint_entries = (api_fingerprints_result :: (std.manifest.empty_name_values :: :: call) :: unwrap_or) :: call
    let output_tables = std.manifest.LockOutputTables :: artifact_entries = (artifacts_result :: (std.manifest.empty_name_values :: :: call) :: unwrap_or), kind_entries = (kinds_result :: (std.manifest.empty_name_values :: :: call) :: unwrap_or), format_entries = (formats_result :: (std.manifest.empty_name_values :: :: call) :: unwrap_or) :: call
    let manifest = std.manifest.LockManifestV1 :: metadata = metadata, lookup_tables = lookup_tables, output_tables = output_tables :: call
    return Result.Ok[LockManifestV1, Str] :: manifest :: call

export fn parse_lock(text: Str) -> Result[LockManifestV2, Str]:
    return std.manifest.parse_lock_v2 :: text :: call

export fn parse_lock_v2(text: Str) -> Result[LockManifestV2, Str]:
    let doc_result = std.config.parse_document :: text :: call
    if doc_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (doc_result :: "" :: unwrap_or) :: call
    let doc = doc_result :: (std.manifest.empty_config_doc :: :: call) :: unwrap_or
    let version = doc :: "version", 0 :: root_int_or
    if version != 2:
        return Result.Err[LockManifestV2, Str] :: "Arcana.lock version must be 2" :: call
    if not (doc :: "workspace" :: root_has_key):
        return Result.Err[LockManifestV2, Str] :: "missing `workspace` in Arcana.lock" :: call
    let workspace_result = doc :: "workspace", "lock field" :: root_required_string
    if workspace_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (workspace_result :: "" :: unwrap_or) :: call
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
        return Result.Err[LockManifestV2, Str] :: (order_result :: "" :: unwrap_or) :: call
    let deps_result = std.manifest.collect_named_lists :: doc, "deps" :: call
    if deps_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (deps_result :: "" :: unwrap_or) :: call
    let paths_result = std.manifest.collect_named_values :: doc, "paths" :: call
    if paths_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (paths_result :: "" :: unwrap_or) :: call
    let kinds_result = std.manifest.collect_named_values :: doc, "kinds" :: call
    if kinds_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: (kinds_result :: "" :: unwrap_or) :: call
    let builds_result = std.manifest.collect_build_entries :: doc, (paths_result :: (std.manifest.empty_name_values :: :: call) :: unwrap_or) :: call
    if builds_result :: :: is_err:
        return Result.Err[LockManifestV2, Str] :: "invalid `[builds]` entries in Arcana.lock" :: call
    let metadata = std.manifest.LockMetadata :: version = version, workspace = (workspace_result :: "" :: unwrap_or), ordered_members = (order_result :: (std.manifest.empty_strings :: :: call) :: unwrap_or) :: call
    let member_tables = std.manifest.LockMemberTables :: dependency_lists = (deps_result :: (std.manifest.empty_name_lists :: :: call) :: unwrap_or), path_entries = (paths_result :: (std.manifest.empty_name_values :: :: call) :: unwrap_or), kind_entries = (kinds_result :: (std.manifest.empty_name_values :: :: call) :: unwrap_or) :: call
    let build_tables = std.manifest.LockBuildTables :: build_entries = (builds_result :: (std.manifest.empty_build_entries :: :: call) :: unwrap_or) :: call
    let manifest = std.manifest.LockManifestV2 :: metadata = metadata, member_tables = member_tables, build_tables = build_tables :: call
    return Result.Ok[LockManifestV2, Str] :: manifest :: call
