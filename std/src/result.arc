export enum Result[T, E]:
    Ok(T)
    Err(E)

impl[T, E] Result[T, E]:
    fn is_ok(read self: Result[T, E]) -> Bool:
        return match self:
            Result.Ok(_) => true
            Result.Err(_) => false

    fn is_err(read self: Result[T, E]) -> Bool:
        return not (self :: :: is_ok)

    fn unwrap_or(read self: Result[T, E], take fallback: T) -> T:
        return match self:
            Result.Ok(value) => value
            Result.Err(_) => fallback
