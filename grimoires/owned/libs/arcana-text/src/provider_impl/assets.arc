import arcana_text.monaspace
import std.fs
import std.package
import std.path
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

fn monaspace_release() -> Str:
    return "v1.400"

fn monaspace_relative_path(read family: arcana_text.monaspace.MonaspaceFamily, read form: arcana_text.monaspace.MonaspaceForm) -> Str:
    let folder = monaspace_folder :: family :: call
    let stem = monaspace_file_stem :: family :: call
    let prefix = "monaspace/" + (monaspace_release :: :: call) + "/"
    return match form:
        arcana_text.monaspace.MonaspaceForm.Static => prefix + "Static Fonts/" + folder + "/" + stem + "-Regular.otf"
        arcana_text.monaspace.MonaspaceForm.Frozen => prefix + "Frozen Fonts/" + folder + "/" + stem + "Frozen-Regular.ttf"
        arcana_text.monaspace.MonaspaceForm.Nerd => prefix + "NerdFonts/" + folder + "/" + stem + "NF-Regular.otf"
        _ => prefix + "Variable Fonts/" + folder + "/" + folder + " Var.ttf"

fn package_root() -> Result[Str, Str]:
    return std.package.asset_root :: :: call

fn resolve(relative_path: Str) -> Result[Str, Str]:
    return match (package_root :: :: call):
        Result.Ok(root) => Result.Ok[Str, Str] :: (std.path.join :: root, relative_path :: call) :: call
        Result.Err(err) => Result.Err[Str, Str] :: err :: call

fn load_utf8(path: Str) -> Result[Str, Str]:
    return std.fs.read_text :: path :: call

fn split_lines(text: Str) -> List[Str]:
    return std.text.split_lines :: text :: call

fn monaspace_source_path(read family: arcana_text.monaspace.MonaspaceFamily, read form: arcana_text.monaspace.MonaspaceForm) -> Result[Str, Str]:
    return resolve :: (monaspace_relative_path :: family, form :: call) :: call
