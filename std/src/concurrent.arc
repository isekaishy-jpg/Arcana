import std.kernel.concurrency

export fn channel[T](capacity: Int) -> Channel[T]:
    return std.kernel.concurrency.channel_new[T] :: capacity :: call

export fn mutex[T](take value: T) -> Mutex[T]:
    return std.kernel.concurrency.mutex_new[T] :: value :: call

export fn atomic_int(value: Int) -> AtomicInt:
    return std.kernel.concurrency.atomic_int_new :: value :: call

export fn atomic_bool(value: Bool) -> AtomicBool:
    return std.kernel.concurrency.atomic_bool_new :: value :: call

export fn sleep(ms: Int):
    std.kernel.concurrency.sleep :: ms :: call

export fn thread_id() -> Int:
    return std.kernel.concurrency.thread_id :: :: call

impl[T] Task[T]:
    fn done(read self: Task[T]) -> Bool:
        return std.kernel.concurrency.task_done[T] :: self :: call

    fn join(read self: Task[T]) -> T:
        return std.kernel.concurrency.task_join[T] :: self :: call

impl[T] Thread[T]:
    fn done(read self: Thread[T]) -> Bool:
        return std.kernel.concurrency.thread_done[T] :: self :: call

    fn join(read self: Thread[T]) -> T:
        return std.kernel.concurrency.thread_join[T] :: self :: call

impl[T] Channel[T]:
    fn send(read self: Channel[T], take value: T):
        std.kernel.concurrency.channel_send[T] :: self, value :: call

    fn recv(read self: Channel[T]) -> T:
        return std.kernel.concurrency.channel_recv[T] :: self :: call

impl[T] Mutex[T]:
    fn pull(read self: Mutex[T]) -> T:
        return std.kernel.concurrency.mutex_take[T] :: self :: call

    fn put(read self: Mutex[T], take value: T):
        std.kernel.concurrency.mutex_put[T] :: self, value :: call

impl AtomicInt:
    fn load(read self: AtomicInt) -> Int:
        return std.kernel.concurrency.atomic_int_load :: self :: call

    fn store(read self: AtomicInt, value: Int):
        std.kernel.concurrency.atomic_int_store :: self, value :: call

    fn add(read self: AtomicInt, delta: Int) -> Int:
        return std.kernel.concurrency.atomic_int_add :: self, delta :: call

    fn sub(read self: AtomicInt, delta: Int) -> Int:
        return std.kernel.concurrency.atomic_int_sub :: self, delta :: call

    fn swap(read self: AtomicInt, value: Int) -> Int:
        return std.kernel.concurrency.atomic_int_swap :: self, value :: call

impl AtomicBool:
    fn load(read self: AtomicBool) -> Bool:
        return std.kernel.concurrency.atomic_bool_load :: self :: call

    fn store(read self: AtomicBool, value: Bool):
        std.kernel.concurrency.atomic_bool_store :: self, value :: call

    fn swap(read self: AtomicBool, value: Bool) -> Bool:
        return std.kernel.concurrency.atomic_bool_swap :: self, value :: call
