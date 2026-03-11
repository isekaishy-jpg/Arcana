enum Token:
    IntLit(Int)
    Plus
    Minus

fn pick[T](flag: Bool, a: T, b: T) -> T:
    if flag:
        return a
    return b

fn score(t: Token) -> Int:
    return match t:
        Token.Plus | Token.Minus => 1
        Token.IntLit(v) => v

fn main() -> Int:
    let a = pick[Int] :: true, 3, 9 :: call
    let t = Token.Minus :: :: call
    let b = score :: t :: call
    return a + b
