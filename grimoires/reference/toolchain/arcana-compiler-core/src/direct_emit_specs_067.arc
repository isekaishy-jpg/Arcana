export fn spec_text_for_fingerprint(source_fingerprint: Str) -> Str:
    if source_fingerprint == "fold_625033434":
        let mut spec = "kind=lib"
        spec += "\nbytecode_version=29"
        spec += "\nstd_abi=std-abi-v1"
        spec += "\nexport=shared_seed|0|0|||Int"
        spec += "\nkind=module"
        spec += "\nversion=29"
        spec += "\nfunction=shared_seed|0|0||0||0"
        spec += "\ncode=0|7|0"
        spec += "\ncode=20|0|0"
        spec += "\nendfn"
        return spec
    return ""

