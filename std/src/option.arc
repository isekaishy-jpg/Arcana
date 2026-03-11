export enum Option[T]:
    None
    Some(T)

impl[T] Option[T]:
    fn is_some(read self: Option[T]) -> Bool:
        return match self:
            Option.Some(_) => true
            Option.None => false

    fn is_none(read self: Option[T]) -> Bool:
        return not (self :: :: is_some)

    fn unwrap_or(read self: Option[T], take fallback: T) -> T:
        return match self:
            Option.Some(value) => value
            Option.None => fallback
