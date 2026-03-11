import std.collections.list
import std.config
import std.result
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

fn empty_name_values() -> List[std.manifest.NameValue]:
    return std.collections.list.new[std.manifest.NameValue] :: :: call

fn empty_name_lists() -> List[std.manifest.NameList]:
    return std.collections.list.new[std.manifest.NameList] :: :: call

fn empty_strings() -> List[Str]:
    return std.collections.list.new[Str] :: :: call

fn empty_config_doc() -> std.config.ConfigDoc:
    return std.config.empty_document :: :: call

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
