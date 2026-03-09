export record Pos:
    line: Int
    column: Int

export record Span:
    path: Str
    start: Pos
    end: Pos

export record DiagMeta:
    code: Str
    severity: Str

export record Diag:
    meta: DiagMeta
    span: Span
    message: Str

export record Token:
    kind: Str
    lexeme: Str
    span: Span
