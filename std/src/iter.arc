export trait Iterator[I]:
    type Item
    fn next(edit self: I) -> (Bool, std.iter.Iterator[I].Item)

export record RangeIter:
    cur: Int
    end: Int

export fn range(start: Int, end: Int) -> RangeIter:
    return std.iter.RangeIter :: cur = start, end = end :: call

impl std.iter.Iterator[std.iter.RangeIter] for std.iter.RangeIter:
    type Item = Int
    fn next(edit self: std.iter.RangeIter) -> (Bool, Int):
        if self.cur < self.end:
            let v = self.cur
            self.cur = self.cur + 1
            return (true, v)
        return (false, 0)

export fn count[I, where std.iter.Iterator[I]](edit it: I) -> Int:
    let mut n = 0
    while true:
        let step = it :: :: next
        if not step.0:
            return n
        n = n + 1
    return n
