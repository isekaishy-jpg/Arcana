export record Paint:
    color: Int

export record RectSpec:
    pos: (Int, Int)
    size: (Int, Int)
    color: Int

export record LineSpec:
    start: (Int, Int)
    end: (Int, Int)
    color: Int

export record CircleFillSpec:
    center: (Int, Int)
    radius: Int
    color: Int

export record SpriteSpec:
    pos: (Int, Int)

export record SpriteScaledSpec:
    pos: (Int, Int)
    size: (Int, Int)
