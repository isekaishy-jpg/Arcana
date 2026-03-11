import std.io

use std.io as io

export fn seed(v: Int) -> Int:
    return v + 3

#stage[pure=true, deterministic=true, thread=any, authority=local, rollback_safe=true]
export fn seed0() -> Int:
    return 17

#stage[pure=true, deterministic=true, thread=any, authority=local, rollback_safe=true]
export fn plus7(v: Int) -> Int:
    return v + 7

#stage[pure=true, deterministic=true, thread=any, authority=local, rollback_safe=true]
export fn mul2(v: Int) -> Int:
    return v * 2

#stage[pure=true, deterministic=true, thread=any, authority=local, rollback_safe=true]
export fn cap500(v: Int) -> Int:
    if v > 500:
        return 500
    return v

#stage[pure=true, deterministic=true, thread=any, authority=local, rollback_safe=true]
export fn echo(v: Int) -> Int:
    return v

export fn score_formula(seed: Int) -> Int:
    return forward :=> showcase_core.chains.seed with (seed) => showcase_core.chains.plus7 => showcase_core.chains.mul2 => showcase_core.chains.cap500

export fn run_scene5_chains(seed_value: Int):
    #chain[phase=update, deterministic=true, thread=worker, authority=local, rollback_safe=true]
    forward :=> showcase_core.chains.seed0 => showcase_core.chains.plus7 => showcase_core.chains.mul2 => showcase_core.chains.cap500 => showcase_core.chains.echo
    #chain[phase=update, deterministic=true, thread=worker, authority=local, rollback_safe=true]
    collect :=> showcase_core.chains.seed0 => showcase_core.chains.plus7 => showcase_core.chains.mul2 => showcase_core.chains.cap500
    #chain[phase=update, deterministic=true, thread=worker, authority=local, rollback_safe=true]
    plan :=> showcase_core.chains.seed0 => showcase_core.chains.plus7 => showcase_core.chains.mul2 => showcase_core.chains.cap500

    showcase_core.chains.seed :: seed_value :: call
        #chain[phase=update, deterministic=true, thread=worker, authority=local, rollback_safe=true]
        forward :=> showcase_core.chains.plus7 => showcase_core.chains.mul2 => showcase_core.chains.cap500
        #chain[phase=update, deterministic=true, thread=worker, authority=local, rollback_safe=true]
        plan :=> showcase_core.chains.plus7 => showcase_core.chains.mul2
