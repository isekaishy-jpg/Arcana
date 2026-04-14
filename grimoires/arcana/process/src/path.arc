import arcana_winapi.helpers.process
import std.result
use std.result.Result

export fn cwd() -> Str:
    return arcana_winapi.helpers.process.path_cwd :: :: call

export fn join(a: Str, b: Str) -> Str:
    return arcana_winapi.helpers.process.path_join :: a, b :: call

export fn normalize(path: Str) -> Str:
    return arcana_winapi.helpers.process.path_normalize :: path :: call

export fn parent(path: Str) -> Str:
    return arcana_winapi.helpers.process.path_parent :: path :: call

export fn file_name(path: Str) -> Str:
    return arcana_winapi.helpers.process.path_file_name :: path :: call

export fn ext(path: Str) -> Str:
    return arcana_winapi.helpers.process.path_ext :: path :: call

export fn is_absolute(path: Str) -> Bool:
    return arcana_winapi.helpers.process.path_is_absolute :: path :: call

export fn stem(path: Str) -> Result[Str, Str]:
    return arcana_winapi.helpers.process.path_stem :: path :: call

export fn with_ext(path: Str, ext: Str) -> Str:
    return arcana_winapi.helpers.process.path_with_ext :: path, ext :: call

export fn relative_to(path: Str, base: Str) -> Result[Str, Str]:
    return arcana_winapi.helpers.process.path_relative_to :: path, base :: call

export fn canonicalize(path: Str) -> Result[Str, Str]:
    return arcana_winapi.helpers.process.path_canonicalize :: path :: call

export fn strip_prefix(path: Str, prefix: Str) -> Result[Str, Str]:
    return arcana_winapi.helpers.process.path_strip_prefix :: path, prefix :: call
