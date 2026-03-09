export fn mix(prev: Int, delta: Int, salt: Int) -> Int:
    let mut out = prev * 131
    out += delta * 17
    out += salt * 7
    out = out % 1000000007
    if out < 0:
        out = -out
    return out
