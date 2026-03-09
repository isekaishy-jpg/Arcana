lang result = Result

enum Result[T, E]:
    Ok(T)
    Err(E)

fn inner() -> Result[Int, Str]:
    return Result.Ok[Int, Str] :: 5 :: call

fn plus_one() -> Result[Int, Str]:
    let tmp = inner :: :: call
    let v = tmp :: :: ?
    return Result.Ok[Int, Str] :: v + 1 :: call

fn main() -> Int:
    let r = plus_one :: :: call
    return match r:
        Result.Ok(v) => v
        Result.Err(_) => 0
