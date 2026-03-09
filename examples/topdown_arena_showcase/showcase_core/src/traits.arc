export trait SceneMetric[T]:
    type Item
    fn base(read self: T) -> Int
    fn weighted(read self: T, weight: Int) -> Int:
        return weight

impl SceneMetric[Int] for Int:
    type Item = Int
    fn base(read self: Int) -> Int:
        return self

export fn score_metric(raw: Int, weight: Int) -> Int:
    return raw * weight
