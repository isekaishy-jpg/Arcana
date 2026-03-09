import std.io
use std.io as io
fn list_len_i(read xs: List[Int]) -> Int:
    let mut n = 0
    for _v in xs:
        n += 1
    return n

record Counter:
    value: Int

impl Counter:
    fn inc(edit self: Counter):
        self.value += 1
    fn get(read self: Counter) -> Int:
        return self.value

impl List[Int]:
    fn sum(read self: List[Int]) -> Int:
        let mut total = 0
        for v in self:
            total += v
        return total
    fn bump_all(edit self: List[Int], by: Int):
        let mut i = 0
        let n = list_len_i :: self :: call
        while i < n:
            self[i] += by
            i += 1
    fn set_last(edit self: List[Int], value: Int):
        let n = list_len_i :: self :: call
        if n > 0:
            self[n - 1] = value

fn main() -> Int:
    let mut c = Counter :: value = 1 :: call
    c :: :: inc
    io.print[Int] :: c :: :: get :: call
    let mut xs = [1, 2]
    xs :: 3 :: bump_all
    xs :: 42 :: set_last
    io.print[Int] :: xs :: :: sum :: call
    return 0






