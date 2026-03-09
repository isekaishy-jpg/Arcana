intrinsic fn task_done[T](read t: Task[T]) -> Bool = ConcurrentTaskDone
intrinsic fn task_join[T](read t: Task[T]) -> T = ConcurrentTaskJoin
intrinsic fn thread_done[T](read h: Thread[T]) -> Bool = ConcurrentThreadDone
intrinsic fn thread_join[T](read h: Thread[T]) -> T = ConcurrentThreadJoin
intrinsic fn thread_id() -> Int = ConcurrentThreadId
intrinsic fn sleep(ms: Int) = ConcurrentSleep
intrinsic fn behavior_step(phase: Str) -> Int = ConcurrentBehaviorStep

intrinsic fn channel_new[T](capacity: Int) -> Channel[T] = ConcurrentChannelNew
intrinsic fn channel_send[T](read ch: Channel[T], take value: T) = ConcurrentChannelSend
intrinsic fn channel_recv[T](read ch: Channel[T]) -> T = ConcurrentChannelRecv

intrinsic fn mutex_new[T](take value: T) -> Mutex[T] = ConcurrentMutexNew
intrinsic fn mutex_take[T](read m: Mutex[T]) -> T = ConcurrentMutexTake
intrinsic fn mutex_put[T](read m: Mutex[T], take value: T) = ConcurrentMutexPut

intrinsic fn atomic_int_new(value: Int) -> AtomicInt = ConcurrentAtomicIntNew
intrinsic fn atomic_int_load(read a: AtomicInt) -> Int = ConcurrentAtomicIntLoad
intrinsic fn atomic_int_store(read a: AtomicInt, value: Int) = ConcurrentAtomicIntStore
intrinsic fn atomic_int_add(read a: AtomicInt, delta: Int) -> Int = ConcurrentAtomicIntAdd
intrinsic fn atomic_int_sub(read a: AtomicInt, delta: Int) -> Int = ConcurrentAtomicIntSub
intrinsic fn atomic_int_swap(read a: AtomicInt, value: Int) -> Int = ConcurrentAtomicIntSwap

intrinsic fn atomic_bool_new(value: Bool) -> AtomicBool = ConcurrentAtomicBoolNew
intrinsic fn atomic_bool_load(read a: AtomicBool) -> Bool = ConcurrentAtomicBoolLoad
intrinsic fn atomic_bool_store(read a: AtomicBool, value: Bool) = ConcurrentAtomicBoolStore
intrinsic fn atomic_bool_swap(read a: AtomicBool, value: Bool) -> Bool = ConcurrentAtomicBoolSwap
