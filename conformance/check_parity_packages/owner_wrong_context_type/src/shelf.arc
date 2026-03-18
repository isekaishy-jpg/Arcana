obj SessionCtx:
    base: Int

obj Counter:
    value: Int
    fn init(edit self: Self, read ctx: SessionCtx):
        self.value = ctx.base

create Session [Counter] scope-exit:
    done: when false hold [Counter]

Session
Counter
fn main() -> Int:
    Session :: 1 :: call
    return 0
