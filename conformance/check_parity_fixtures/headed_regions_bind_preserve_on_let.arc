enum Result[T, E]:
    Ok(T)
    Err(E)

fn main() -> Int:
    bind -return 0
        let value = Result.Err[Int, Str] :: "no" :: call -preserve
    return 0
