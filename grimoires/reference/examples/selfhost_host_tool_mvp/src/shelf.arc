import std.args
import std.fs
import std.io
import std.path
import fs_support
import tokenizer
import report

fn fold_checksum(acc: Int, delta: Int) -> Int:
    return (acc + delta) % 2147483647

fn main() -> Int:
    let mut root = std.path.cwd :: :: call
    let mut report_name = "host_tool_report.txt"
    let arg_count = std.args.count :: :: call
    if arg_count > 0:
        root = std.args.get :: 0 :: call
    if arg_count > 1:
        report_name = std.args.get :: 1 :: call

    if not (report.write_header_ok :: root, report_name :: call):
        return 1
    let mut pending = [root]
    let mut files = 0
    let mut checksum = 0
    while (pending :: :: len) > 0:
        let path_value = pending :: :: pop
        if std.fs.is_dir :: path_value :: call:
            let mut entries = fs_support.list_dir_or_empty :: path_value :: call
            while (entries :: :: len) > 0:
                let entry = entries :: :: pop
                pending :: entry :: push
            continue

        if (std.path.ext :: path_value :: call) != "arc":
            continue

        let text = fs_support.read_text_or :: path_value, "" :: call
        let stats = tokenizer.tokenize_subset :: text :: call
        path_value :: :: std.io.print
        stats.0 :: :: std.io.print
        stats.1 :: :: std.io.print
        files += 1
        checksum = fold_checksum :: checksum, stats.1 :: call

    "FILES" :: :: std.io.print
    files :: :: std.io.print
    "FINAL" :: :: std.io.print
    checksum :: :: std.io.print
    return 0
