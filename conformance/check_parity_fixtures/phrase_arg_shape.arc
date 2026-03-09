fn f(a: Int, b: Int, c: Int, d: Int) -> Int:
    return a + b + c + d

fn main() -> Int:
    // CHECK_FIXTURE_PHRASE_ARG_SHAPE
    return f :: 1, 2, 3, 4 :: call
