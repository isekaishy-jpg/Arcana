use std.result.Result

export trait Cleanup[T]:
    fn cleanup(take self: T) -> Result[Unit, Str]

lang cleanup_contract = std.cleanup.Cleanup
