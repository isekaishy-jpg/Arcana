intrinsic fn text_len_bytes(text: Str) -> Int = HostTextLenBytes
intrinsic fn text_byte_at(text: Str, index: Int) -> Int = HostTextByteAt
intrinsic fn text_slice_bytes(text: Str, start: Int, end: Int) -> Str = HostTextSliceBytes
intrinsic fn text_starts_with(text: Str, prefix: Str) -> Bool = HostTextStartsWith
intrinsic fn text_ends_with(text: Str, suffix: Str) -> Bool = HostTextEndsWith
intrinsic fn text_split_lines(text: Str) -> List[Str] = HostTextSplitLines
intrinsic fn text_from_int(value: Int) -> Str = HostTextFromInt

intrinsic fn bytes_from_str_utf8(text: Str) -> Array[Int] = HostBytesFromStrUtf8
intrinsic fn bytes_to_str_utf8(read bytes: Array[Int]) -> Str = HostBytesToStrUtf8
intrinsic fn bytes_len(read bytes: Array[Int]) -> Int = HostBytesLen
intrinsic fn bytes_at(read bytes: Array[Int], index: Int) -> Int = HostBytesAt
intrinsic fn bytes_slice(read bytes: Array[Int], start: Int, end: Int) -> Array[Int] = HostBytesSlice
