import std.concurrent

use std.concurrent as concurrent

async fn async_boost(seed: Int) -> Int:
    return seed + 9

export fn tick(seed: Int) -> Int:
    let ch = concurrent.channel[Int] :: 4 :: call
    ch :: seed :: send
    let recv = ch :: :: recv

    let atom = concurrent.atomic_int :: 0 :: call
    let _old = atom :: recv :: add
    let cur = atom :: :: load

    let t = weave async_boost :: recv :: call
    let boosted = t :: :: join
    return cur + boosted
