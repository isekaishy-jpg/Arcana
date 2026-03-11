export fn spec_text_for_fingerprint(source_fingerprint: Str) -> Str:
    if source_fingerprint == "fold_212910946":
        let mut spec = "kind=module"
        spec += "\nversion=29"
        spec += "\nfunction=main|0|0||0||0"
        spec += "\ncode=0|0|0"
        spec += "\ncode=20|0|0"
        spec += "\nendfn"
        return spec
    return ""

