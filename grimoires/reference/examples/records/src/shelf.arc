import std.io
use std.io as io
record Mage:
    name: Str
    mana: Int

fn spend(edit m: Mage):
    m.mana = m.mana - 1

fn main() -> Int:
    let mut m = Mage :: name = "Aster", mana = 3 :: call
    spend :: m :: call
    io.print[Int] :: m.mana :: call
    return 0






