export record Outcome[T]:
    ok: Bool
    value: T
    message: Str

export record ByteCursor:
    bytes: Array[Int]
    pos: Int
    error: Str

export record ByteCursorState:
    pos: Int
    failed: Bool

export record DecodedInstr:
    meta: (Str, Bool)
    tail: (Str, (Str, Int))

export record DecodedFunction:
    name: Str
    meta: (Bool, (Int, Int))
    code: Array[DecodedInstr]

export record DecodedModule:
    counts: (Int, (Int, Int))
    functions: Array[DecodedFunction]

export record BytecodeRecordType:
    name: Str
    fields: Array[Str]

export record BytecodeFunctionSig:
    params: Array[Int]
    ret: Int

export record BytecodeInstr:
    tag: Int
    a: Int
    b: Int

export record BytecodeFunction:
    name: Str
    meta: (Bool, (Int, Array[Int]))
    tail: (Int, Array[BytecodeInstr])

export record BytecodeBehaviorContract:
    meta: (Bool, Int)
    tags: (Int, (Int, Int))
    flags: (Bool, Bool)

export record BytecodeAccessSets:
    reads: Array[Int]
    writes: Array[Int]
    excludes: Array[Int]

export record BytecodeBehavior:
    name: Str
    meta: (Int, (Int, Int))
    tail: (Array[Str], (BytecodeBehaviorContract, BytecodeAccessSets))

export record BytecodeModuleHead:
    version: Int
    strings: Array[Str]
    records: Array[BytecodeRecordType]

export record BytecodeModuleTail:
    function_sigs: Array[BytecodeFunctionSig]
    functions: Array[BytecodeFunction]
    behaviors: Array[BytecodeBehavior]

export record BytecodeModule:
    head: BytecodeModuleHead
    tail: BytecodeModuleTail

export record BytecodeDepFingerprint:
    dep: Str
    fingerprint: Str

export record BytecodeLibExport:
    name: Str
    meta: (Int, (Bool, Array[Int]))
    tail: (Array[Str], Str)

export record BytecodeLibMeta:
    format_version: Int
    bytecode_version: Int
    std_abi: Str

export record BytecodeLibArtifact:
    meta: BytecodeLibMeta
    exports: Array[BytecodeLibExport]
    tail: (Array[BytecodeDepFingerprint], BytecodeModule)

export record IrPhiInput:
    pred_block: Int
    value_id: Int

export record IrPhi:
    output: Int
    inputs: Array[IrPhiInput]

export record IrInstr:
    meta: (Bool, Int)
    tail: (Str, Str)

export record IrBlock:
    id: Int
    content: (Array[IrPhi], Array[IrInstr])
    term: (Str, (Int, (Int, Int)))

export record IrFunction:
    name: Str
    meta: (Bool, (Int, Int))
    tail: (Int, (Int, Array[IrBlock]))

export record IrModule:
    counts: (Int, (Int, Int))
    functions: Array[IrFunction]
