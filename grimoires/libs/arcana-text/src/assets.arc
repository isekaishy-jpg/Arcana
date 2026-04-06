import std.fs
import std.package
import std.path

export fn root() -> Str:
    let runtime_root = (std.package.asset_root :: :: call) :: "." :: fallback
    if std.fs.is_dir :: (std.path.join :: runtime_root, "monaspace" :: call) :: call:
        return runtime_root
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
