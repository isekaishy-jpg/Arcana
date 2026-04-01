import std.result
use std.result.Result

intrinsic fn package_current_asset_root() -> Result[Str, Str] = PackageCurrentAssetRootTry
