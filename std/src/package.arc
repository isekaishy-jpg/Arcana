import std.kernel.package
import std.result
use std.result.Result

export fn asset_root() -> Result[Str, Str]:
    return std.kernel.package.package_current_asset_root :: :: call
