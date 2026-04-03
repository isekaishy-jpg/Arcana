import std.collections.array
import std.collections.list
import std.memory

export record Reader:
    view: std.memory.ByteView
    cursor: Int

export record Writer:
    values: List[Int]

export trait BinaryReadable[T]:
    fn read_from(edit reader: std.binary.Reader) -> T

export trait ByteSink[T]:
    fn write_to(read value: T, edit writer: std.binary.Writer)

export fn from_array(read values: Array[Int]) -> std.binary.Reader:
    return std.binary.Reader :: view = (std.memory.bytes_view :: values, 0, (values :: :: len) :: call), cursor = 0 :: call

export fn from_view(read view: std.memory.ByteView) -> std.binary.Reader:
    return std.binary.Reader :: view = view, cursor = 0 :: call

impl Reader:
    fn len(read self: Reader) -> Int:
        return self.view :: :: len

    fn remaining(read self: Reader) -> Int:
        return (self.view :: :: len) - self.cursor

    fn seek(edit self: Reader, offset: Int):
        self.cursor = offset

    fn skip(edit self: Reader, amount: Int):
        self.cursor += amount

    fn subview(edit self: Reader, len: Int) -> std.memory.ByteView:
        let start = self.cursor
        let end = start + len
        self.cursor = end
        return self.view :: start, end :: subview

    fn read_u8(edit self: Reader) -> Int:
        let value = self.view :: self.cursor :: at
        self.cursor += 1
        return value

    fn read_u16_be(edit self: Reader) -> Int:
        let a = self :: :: read_u8
        let b = self :: :: read_u8
        return (a << 8) | b

    fn read_u32_be(edit self: Reader) -> Int:
        let a = self :: :: read_u8
        let b = self :: :: read_u8
        let c = self :: :: read_u8
        let d = self :: :: read_u8
        return (a << 24) | (b << 16) | (c << 8) | d

    fn read_i16_be(edit self: Reader) -> Int:
        let value = self :: :: read_u16_be
        if value >= 32768:
            return value - 65536
        return value

    fn read_i32_be(edit self: Reader) -> Int:
        let value = self :: :: read_u32_be
        if value >= 2147483648:
            return value - 4294967296
        return value

impl Writer:
    fn len(read self: Writer) -> Int:
        return self.values :: :: len

    fn push_u8(edit self: Writer, value: Int):
        self.values :: value :: push

    fn push_u16_be(edit self: Writer, value: Int):
        self.values :: ((value shr 8) & 255) :: push
        self.values :: (value & 255) :: push

    fn push_u32_be(edit self: Writer, value: Int):
        self.values :: ((value shr 24) & 255) :: push
        self.values :: ((value shr 16) & 255) :: push
        self.values :: ((value shr 8) & 255) :: push
        self.values :: (value & 255) :: push

    fn into_array(read self: Writer) -> Array[Int]:
        let mut out = std.collections.list.new[Int] :: :: call
        for value in self.values:
            out :: value :: push
        return std.collections.array.from_list[Int] :: out :: call

export fn writer() -> std.binary.Writer:
    return std.binary.Writer :: values = (std.collections.list.new[Int] :: :: call) :: call
