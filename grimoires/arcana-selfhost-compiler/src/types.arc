export record Span:
    path: Str
    start: (Int, Int)
    end: (Int, Int)

export record Diag:
    meta: (Str, Str)
    loc: (Str, (Int, Int))
    tail: ((Int, Int), Str)

export record Artifact:
    left: (Str, Str)
    right: (Str, (Str, Int))

export record BuildEvent:
    member: Str
    status: Str
    artifact_path: Str

export record ParsedDiag:
    loc: (Str, (Int, Int))
    meta: (Str, Str)
    message: Str

export record ParsedBuildEvent:
    member: Str
    status: Str
    hash: Str

export record DiagState:
    counts: (Int, Int)
    checksum: Int
