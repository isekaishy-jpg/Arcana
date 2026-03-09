import std.io
#allow[deprecated_use]
use std.io as io

#inline
fn helper() -> Int:
    return 1

#cold
fn slow() -> Int:
    return 2

#test
fn smoke() -> Int:
    return 0

fn main() -> Int:
    return helper()
