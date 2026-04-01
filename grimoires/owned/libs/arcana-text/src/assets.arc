import arcana_text.monaspace
import arcana_text.provider_impl.assets
import std.fs
import std.result
import std.text
use std.result.Result

fn monaspace_folder(read family: arcana_text.monaspace.MonaspaceFamily) -> Str:
    return match family:
        arcana_text.monaspace.MonaspaceFamily.Argon => "Monaspace Argon"
        arcana_text.monaspace.MonaspaceFamily.Xenon => "Monaspace Xenon"
        arcana_text.monaspace.MonaspaceFamily.Radon => "Monaspace Radon"
        arcana_text.monaspace.MonaspaceFamily.Krypton => "Monaspace Krypton"
        _ => "Monaspace Neon"

fn monaspace_file_stem(read family: arcana_text.monaspace.MonaspaceFamily) -> Str:
    return match family:
        arcana_text.monaspace.MonaspaceFamily.Argon => "MonaspaceArgon"
        arcana_text.monaspace.MonaspaceFamily.Xenon => "MonaspaceXenon"
        arcana_text.monaspace.MonaspaceFamily.Radon => "MonaspaceRadon"
        arcana_text.monaspace.MonaspaceFamily.Krypton => "MonaspaceKrypton"
        _ => "MonaspaceNeon"

fn monaspace_relative_path(read family: arcana_text.monaspace.MonaspaceFamily, read form: arcana_text.monaspace.MonaspaceForm) -> Str:
    let folder = arcana_text.assets.monaspace_folder :: family :: call
    let stem = arcana_text.assets.monaspace_file_stem :: family :: call
    let prefix = "monaspace/" + (arcana_text.assets.monaspace_release :: :: call) + "/"
    return match form:
        arcana_text.monaspace.MonaspaceForm.Static => prefix + "Static Fonts/" + folder + "/" + stem + "-Regular.otf"
        arcana_text.monaspace.MonaspaceForm.Frozen => prefix + "Frozen Fonts/" + folder + "/" + stem + "Frozen-Regular.ttf"
        arcana_text.monaspace.MonaspaceForm.Nerd => prefix + "NerdFonts/" + folder + "/" + stem + "NF-Regular.otf"
        _ => prefix + "Variable Fonts/" + folder + "/" + folder + " Var.ttf"

export fn monaspace_release() -> Str:
    return "v1.400"

export fn package_root() -> Result[Str, Str]:
    return arcana_text.provider_impl.assets.package_root :: :: call

export fn resolve(relative_path: Str) -> Result[Str, Str]:
    return arcana_text.provider_impl.assets.resolve :: relative_path :: call

export fn monaspace_source_path(read family: arcana_text.monaspace.MonaspaceFamily, read form: arcana_text.monaspace.MonaspaceForm) -> Result[Str, Str]:
    return arcana_text.provider_impl.assets.monaspace_source_path :: family, form :: call

export fn load_utf8(path: Str) -> Result[Str, Str]:
    return arcana_text.provider_impl.assets.load_utf8 :: path :: call

export fn split_lines(text: Str) -> List[Str]:
    return arcana_text.provider_impl.assets.split_lines :: text :: call
