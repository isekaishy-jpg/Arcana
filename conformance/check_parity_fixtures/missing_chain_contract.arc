fn seed() -> Int:
    return 0

fn step(v: Int) -> Int:
    return v

behavior[fixed] fn tick():
    // CHECK_FIXTURE_MISSING_CHAIN_CONTRACT
    forward :=> seed => step

fn main() -> Int:
    return 0
