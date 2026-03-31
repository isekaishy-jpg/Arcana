enum Option[T]:
    Some(T)
    None

fn main() -> Int:
    let mut value = "x"
    bind -return 0
        value = Option.Some[Int] :: 1 :: call -preserve
    return 0
