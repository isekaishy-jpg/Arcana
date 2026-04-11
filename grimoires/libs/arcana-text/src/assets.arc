import std.fs
import std.package
import std.path
import std.result

fn monaspace_root_at(path: Str) -> Str:
    if std.fs.is_dir :: (std.path.join :: path, "monaspace" :: call) :: call:
        return path
    return ""

fn first_child_with_monaspace(path: Str) -> Str:
    if not (std.fs.is_dir :: path :: call):
        return ""
    let listed = std.fs.list_dir :: path :: call
    return match listed:
        std.result.Result.Ok(value) => first_child_with_monaspace_ready :: path, value :: call
        std.result.Result.Err(_) => ""

fn first_child_with_monaspace_ready(path: Str, read entries: List[Str]) -> Str:
    for entry in entries:
        if (monaspace_root_at :: entry :: call) != "":
            return entry
        let nested = std.path.join :: path, entry :: call
        if (monaspace_root_at :: nested :: call) != "":
            return nested
    return ""

export fn root() -> Str:
    let runtime_root = (std.package.asset_root :: :: call) :: "." :: fallback
    let direct_runtime_root = monaspace_root_at :: runtime_root :: call
    if direct_runtime_root != "":
        return direct_runtime_root
    let packaged_runtime_root = first_child_with_monaspace :: runtime_root :: call
    if packaged_runtime_root != "":
        return packaged_runtime_root
    let package_assets_root = std.path.join :: runtime_root, "package-assets" :: call
    let packaged_assets_root = first_child_with_monaspace :: package_assets_root :: call
    if packaged_assets_root != "":
        return packaged_assets_root
    let cwd = std.path.cwd :: :: call
    let workspace_candidate = std.path.join :: cwd, "../../grimoires/libs/arcana-text/assets" :: call
    if std.fs.is_dir :: workspace_candidate :: call:
        return workspace_candidate
    let repo_candidate = std.path.join :: cwd, "grimoires/libs/arcana-text/assets" :: call
    if std.fs.is_dir :: repo_candidate :: call:
        return repo_candidate
    let parent_candidate = std.path.join :: cwd, "../grimoires/libs/arcana-text/assets" :: call
    if std.fs.is_dir :: parent_candidate :: call:
        return parent_candidate
    return runtime_root

export fn monaspace_root() -> Str:
    return std.path.join :: (arcana_text.assets.root :: :: call), "monaspace" :: call

export fn monaspace_version_root() -> Str:
    return std.path.join :: (arcana_text.assets.monaspace_root :: :: call), "v1.400" :: call

export fn monaspace_variable_root() -> Str:
    return std.path.join :: (arcana_text.assets.monaspace_version_root :: :: call), "Variable Fonts" :: call
