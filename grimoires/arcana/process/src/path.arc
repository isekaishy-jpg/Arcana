import std.result
use std.result.Result

// `arcana_process.path` is runtime-owned host-core surface.
export fn cwd() -> Str:
    return arcana_process.path.cwd :: :: call

export fn join(a: Str, b: Str) -> Str:
    return arcana_process.path.join :: a, b :: call

export fn normalize(path: Str) -> Str:
    return arcana_process.path.normalize :: path :: call

export fn parent(path: Str) -> Str:
    return arcana_process.path.parent :: path :: call

export fn file_name(path: Str) -> Str:
    return arcana_process.path.file_name :: path :: call

export fn ext(path: Str) -> Str:
    return arcana_process.path.ext :: path :: call

export fn is_absolute(path: Str) -> Bool:
    return arcana_process.path.is_absolute :: path :: call

export fn stem(path: Str) -> Result[Str, Str]:
    return arcana_process.path.stem :: path :: call

export fn with_ext(path: Str, ext: Str) -> Str:
    return arcana_process.path.with_ext :: path, ext :: call

export fn relative_to(path: Str, base: Str) -> Result[Str, Str]:
    return arcana_process.path.relative_to :: path, base :: call

export fn canonicalize(path: Str) -> Result[Str, Str]:
    return arcana_process.path.canonicalize :: path :: call

export fn strip_prefix(path: Str, prefix: Str) -> Result[Str, Str]:
    return arcana_process.path.strip_prefix :: path, prefix :: call
