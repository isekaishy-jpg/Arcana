export record Payload:
    inner: types.WorkerHandle

record WorkerHandle:
    task: Task[Int]
