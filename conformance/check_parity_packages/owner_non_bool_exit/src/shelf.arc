obj Counter:
    value: Int

create Session [Counter] scope-exit:
    exit when 1

fn main() -> Int:
    return 0
