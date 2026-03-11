import std.kernel.error
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
    let pair = std.kernel.path.path_stem_try :: path :: call
    if pair.0:
        return Result.Ok[Str, Str] :: pair.1 :: call
    return Result.Err[Str, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn with_ext(path: Str, ext: Str) -> Str:
    return std.kernel.path.path_with_ext :: path, ext :: call

export fn relative_to(path: Str, base: Str) -> Result[Str, Str]:
    let pair = std.kernel.path.path_relative_to_try :: path, base :: call
    if pair.0:
        return Result.Ok[Str, Str] :: pair.1 :: call
    return Result.Err[Str, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn canonicalize(path: Str) -> Result[Str, Str]:
    let pair = std.kernel.path.path_canonicalize_try :: path :: call
    if pair.0:
        return Result.Ok[Str, Str] :: pair.1 :: call
    return Result.Err[Str, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn strip_prefix(path: Str, prefix: Str) -> Result[Str, Str]:
    let pair = std.kernel.path.path_strip_prefix_try :: path, prefix :: call
    if pair.0:
        return Result.Ok[Str, Str] :: pair.1 :: call
    return Result.Err[Str, Str] :: (std.kernel.error.last_error_take :: :: call) :: call
