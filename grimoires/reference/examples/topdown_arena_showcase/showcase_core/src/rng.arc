export fn next(state: Int) -> (Int, Int):
    let mut base = state % 1000000007
    if base < 0:
        base = -base
    let mut n = base * 48271
    n += 1
    n = n % 2147483647
    if n < 0:
        n = -n
    return (n, (n % 100000))
