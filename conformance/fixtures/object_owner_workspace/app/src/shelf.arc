obj Counter:
    value: Int

create Session [Counter] scope-exit:
    done: when Counter.value >= 10 retain [Counter]

Session
Counter
fn main() -> Int:
    let active = Session :: :: call
    Counter.value = 9
    Counter.value += 1
    let resumed = Session :: :: call
    return resumed.Counter.value
