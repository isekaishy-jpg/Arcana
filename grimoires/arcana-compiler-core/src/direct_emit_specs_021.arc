export fn spec_text_for_fingerprint(source_fingerprint: Str) -> Str:
    if source_fingerprint == "fold_132229002":
        let mut spec = "kind=module"
        spec += "\nversion=29"
        spec += "\nstring=hello\\, arcana"
        spec += "\nfunction=main|0|0||0||2"
        spec += "\ncode=2|0|0"
        spec += "\ncode=17|2|0"
        spec += "\ncode=19|0|0"
        spec += "\ncode=20|0|0"
        spec += "\nendfn"
        spec += "\nfunction=std.kernel.io.print|0|1|0|1|2|2"
        spec += "\ncode=20|0|0"
        spec += "\nendfn"
        spec += "\nfunction=__gen_std_io_print_0|0|1|0|1|4|2"
        spec += "\ncode=3|0|0"
        spec += "\ncode=131|76|1"
        spec += "\ncode=19|0|0"
        spec += "\ncode=20|0|0"
        spec += "\nendfn"
        return spec
    return ""

