import arcana_frontend.summary
import std.args

fn main() -> Int:
    let mut target = "."
    let argc = std.args.count :: :: call
    if argc > 0:
        target = std.args.get :: 0 :: call
    let exit = arcana_frontend.summary.run :: target :: call
    return exit
