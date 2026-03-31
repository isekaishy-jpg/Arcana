record Inner:
    value: Int

record Outer:
    inner: Inner

fn main() -> Int:
    let built = construct yield Outer -return 0
        inner = construct yield Inner -return 0
            value = 1
    return 0
