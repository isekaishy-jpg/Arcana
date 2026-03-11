import std.result
use std.result.Result

intrinsic fn path_cwd() -> Str = HostPathCwd
intrinsic fn path_join(a: Str, b: Str) -> Str = HostPathJoin
intrinsic fn path_normalize(path: Str) -> Str = HostPathNormalize
intrinsic fn path_parent(path: Str) -> Str = HostPathParent
intrinsic fn path_file_name(path: Str) -> Str = HostPathFileName
intrinsic fn path_ext(path: Str) -> Str = HostPathExt
intrinsic fn path_is_absolute(path: Str) -> Bool = HostPathIsAbsolute
intrinsic fn path_stem(path: Str) -> Result[Str, Str] = HostPathStemTry
intrinsic fn path_with_ext(path: Str, ext: Str) -> Str = HostPathWithExt
intrinsic fn path_relative_to(path: Str, base: Str) -> Result[Str, Str] = HostPathRelativeToTry
intrinsic fn path_canonicalize(path: Str) -> Result[Str, Str] = HostPathCanonicalizeTry
intrinsic fn path_strip_prefix(path: Str, prefix: Str) -> Result[Str, Str] = HostPathStripPrefixTry
