fn cleanup(value: Int):
    return

fn main(seed: Int) -> Int:
    let local = seed
    local += 1
    return local
-cleanup[target = local, handler = cleanup]
