import std.kernel.path
import std.result
use std.result.Result

export fn cwd() -> Str:
    return std.kernel.path.path_cwd :: :: call

export fn join(a: Str, b: Str) -> Str:
    return std.kernel.path.path_join :: a, b :: call

export fn normalize(path: Str) -> Str:
    return std.kernel.path.path_normalize :: path :: call

export fn parent(path: Str) -> Str:
    return std.kernel.path.path_parent :: path :: call

export fn file_name(path: Str) -> Str:
    return std.kernel.path.path_file_name :: path :: call

export fn ext(path: Str) -> Str:
    return std.kernel.path.path_ext :: path :: call

export fn is_absolute(path: Str) -> Bool:
    return std.kernel.path.path_is_absolute :: path :: call

export fn stem(path: Str) -> Result[Str, Str]:
    return std.kernel.path.path_stem :: path :: call

export fn with_ext(path: Str, ext: Str) -> Str:
    return std.kernel.path.path_with_ext :: path, ext :: call

export fn relative_to(path: Str, base: Str) -> Result[Str, Str]:
    return std.kernel.path.path_relative_to :: path, base :: call

export fn canonicalize(path: Str) -> Result[Str, Str]:
    return std.kernel.path.path_canonicalize :: path :: call

export fn strip_prefix(path: Str, prefix: Str) -> Result[Str, Str]:
    return std.kernel.path.path_strip_prefix :: path, prefix :: call
