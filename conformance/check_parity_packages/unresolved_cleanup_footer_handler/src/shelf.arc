fn main() -> Int:
    while true:
        let local = 1
        break
    -cleanup[target = local, handler = missing.cleanup]
    return 0
