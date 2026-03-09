import std.text

export fn parse_written_lock_line(line: Str) -> (Bool, Str):
    if not (std.text.starts_with :: line, "wrote " :: call):
        return (false, "")
    let n = std.text.len_bytes :: line :: call
    let path = std.text.slice_bytes :: line, 6, n :: call
    return (true, path)
