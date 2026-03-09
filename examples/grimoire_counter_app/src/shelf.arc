import types
import runtime
use types.Counter
use runtime.run_counter

fn main() -> Int:
    let mut state = types.Counter :: value = 0, limit = 3 :: call
    run_counter :: state :: call
    return 0
