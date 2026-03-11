record Box[T]:
    value: T

fn wrap[T](x: T) -> Box[T]:
    return Box[T] :: value = x :: call

fn main() -> Int:
    let b = wrap[Int] :: 41 :: call
    return b.value + 1





