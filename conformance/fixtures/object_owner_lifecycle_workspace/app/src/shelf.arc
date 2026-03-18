obj SessionCtx:
    base: Int

obj Counter:
    value: Int
    fn init(edit self: Self, read ctx: SessionCtx):
        self.value = ctx.base
    fn resume(edit self: Self, read ctx: SessionCtx):
        self.value += ctx.base

create Session [Counter] scope-exit:
    done: when Counter.value == 3 hold [Counter]

Session
Counter
fn main() -> Int:
    let start = SessionCtx :: base = 1 :: call
    Session :: start :: call
    Counter.value
    Counter.value = 3
    let resume_ctx = SessionCtx :: base = 2 :: call
    let resumed = Session :: resume_ctx :: call
    return resumed.Counter.value
