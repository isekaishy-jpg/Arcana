obj Counter:
    value: Int
    fn init(read self: Self):
        return

create Session [Counter] scope-exit:
    done: when false hold [Counter]

fn main() -> Int:
    return 0
