import std.io
use std.io as io
record State:
    name: Str
    flags: Int

fn main() -> Int:
    let mut s = "Arc"
    s += "ana"
    let banner = s + "!"
    io.print[Str] :: banner :: call

    let mut x = 10
    x += 2
    x *= 3
    x /= 2
    x %= 7
    x |= 8
    x &= 14
    x ^= 3
    x <<= 2
    x shr= 1
    io.print[Int] :: x :: call

    let mut st = State :: name = "Mage", flags = 1 :: call
    st.name += "!"
    st.flags |= 4
    st.flags <<= 1
    io.print[Str] :: st.name :: call
    io.print[Int] :: st.flags :: call

    return 0






