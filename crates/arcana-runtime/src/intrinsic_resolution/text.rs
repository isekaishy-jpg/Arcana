use crate::runtime_intrinsics::RuntimeIntrinsic;

pub(super) fn resolve_path(parts: &[&str]) -> Option<RuntimeIntrinsic> {
    match parts {
        ["std", "text", "len_bytes"] | ["std", "kernel", "text", "text_len_bytes"] => {
            Some(RuntimeIntrinsic::TextLenBytes)
        }
        ["std", "text", "byte_at"] | ["std", "kernel", "text", "text_byte_at"] => {
            Some(RuntimeIntrinsic::TextByteAt)
        }
        ["std", "text", "slice_bytes"] | ["std", "kernel", "text", "text_slice_bytes"] => {
            Some(RuntimeIntrinsic::TextSliceBytes)
        }
        ["std", "text", "starts_with"] | ["std", "kernel", "text", "text_starts_with"] => {
            Some(RuntimeIntrinsic::TextStartsWith)
        }
        ["std", "text", "ends_with"] | ["std", "kernel", "text", "text_ends_with"] => {
            Some(RuntimeIntrinsic::TextEndsWith)
        }
        ["std", "text", "find"] => Some(RuntimeIntrinsic::TextFind),
        ["std", "text", "contains"] => Some(RuntimeIntrinsic::TextContains),
        ["std", "text", "trim_start"] => Some(RuntimeIntrinsic::TextTrimStart),
        ["std", "text", "trim_end"] => Some(RuntimeIntrinsic::TextTrimEnd),
        ["std", "text", "trim"] => Some(RuntimeIntrinsic::TextTrim),
        ["std", "text", "split"] => Some(RuntimeIntrinsic::TextSplit),
        ["std", "text", "join"] => Some(RuntimeIntrinsic::TextJoin),
        ["std", "text", "repeat"] => Some(RuntimeIntrinsic::TextRepeat),
        ["std", "text", "split_lines"] | ["std", "kernel", "text", "text_split_lines"] => {
            Some(RuntimeIntrinsic::TextSplitLines)
        }
        ["std", "text", "from_int"] | ["std", "kernel", "text", "text_from_int"] => {
            Some(RuntimeIntrinsic::TextFromInt)
        }
        ["std", "text", "to_int"] => Some(RuntimeIntrinsic::TextToIntTry),
        ["std", "kernel", "text", "bytes_from_str_utf8"] => {
            Some(RuntimeIntrinsic::BytesFromStrUtf8)
        }
        ["std", "kernel", "text", "bytes_to_str_utf8"] => Some(RuntimeIntrinsic::BytesToStrUtf8),
        ["std", "kernel", "text", "bytes_len"] => Some(RuntimeIntrinsic::BytesLen),
        ["std", "kernel", "text", "bytes_at"] => Some(RuntimeIntrinsic::BytesAt),
        ["std", "kernel", "text", "bytes_slice"] => Some(RuntimeIntrinsic::BytesSlice),
        ["std", "kernel", "text", "bytes_sha256_hex"] => Some(RuntimeIntrinsic::BytesSha256Hex),
        ["std", "kernel", "text", "bytes_thaw"] => Some(RuntimeIntrinsic::BytesThaw),
        ["std", "kernel", "text", "byte_buffer_new"] => Some(RuntimeIntrinsic::ByteBufferNew),
        ["std", "kernel", "text", "byte_buffer_len"] => Some(RuntimeIntrinsic::ByteBufferLen),
        ["std", "kernel", "text", "byte_buffer_at"] => Some(RuntimeIntrinsic::ByteBufferAt),
        ["std", "kernel", "text", "byte_buffer_set"] => Some(RuntimeIntrinsic::ByteBufferSet),
        ["std", "kernel", "text", "byte_buffer_push"] => Some(RuntimeIntrinsic::ByteBufferPush),
        ["std", "kernel", "text", "byte_buffer_freeze"] => Some(RuntimeIntrinsic::ByteBufferFreeze),
        ["std", "kernel", "text", "utf16_from_str"] => Some(RuntimeIntrinsic::Utf16FromStr),
        ["std", "kernel", "text", "utf16_to_str"] => Some(RuntimeIntrinsic::Utf16ToStr),
        ["std", "kernel", "text", "utf16_len"] => Some(RuntimeIntrinsic::Utf16Len),
        ["std", "kernel", "text", "utf16_at"] => Some(RuntimeIntrinsic::Utf16At),
        ["std", "kernel", "text", "utf16_slice"] => Some(RuntimeIntrinsic::Utf16Slice),
        ["std", "kernel", "text", "utf16_thaw"] => Some(RuntimeIntrinsic::Utf16Thaw),
        ["std", "kernel", "text", "utf16_buffer_new"] => Some(RuntimeIntrinsic::Utf16BufferNew),
        ["std", "kernel", "text", "utf16_buffer_len"] => Some(RuntimeIntrinsic::Utf16BufferLen),
        ["std", "kernel", "text", "utf16_buffer_at"] => Some(RuntimeIntrinsic::Utf16BufferAt),
        ["std", "kernel", "text", "utf16_buffer_set"] => Some(RuntimeIntrinsic::Utf16BufferSet),
        ["std", "kernel", "text", "utf16_buffer_push"] => Some(RuntimeIntrinsic::Utf16BufferPush),
        ["std", "kernel", "text", "utf16_buffer_freeze"] => {
            Some(RuntimeIntrinsic::Utf16BufferFreeze)
        }
        _ => None,
    }
}

pub(super) fn resolve_impl(intrinsic_impl: &str) -> Option<RuntimeIntrinsic> {
    match intrinsic_impl {
        "HostTextLenBytes" => Some(RuntimeIntrinsic::TextLenBytes),
        "HostTextByteAt" => Some(RuntimeIntrinsic::TextByteAt),
        "HostTextSliceBytes" => Some(RuntimeIntrinsic::TextSliceBytes),
        "HostTextStartsWith" => Some(RuntimeIntrinsic::TextStartsWith),
        "HostTextEndsWith" => Some(RuntimeIntrinsic::TextEndsWith),
        "HostTextSplitLines" => Some(RuntimeIntrinsic::TextSplitLines),
        "HostTextFromInt" => Some(RuntimeIntrinsic::TextFromInt),
        "HostBytesFromStrUtf8" => Some(RuntimeIntrinsic::BytesFromStrUtf8),
        "HostBytesToStrUtf8" => Some(RuntimeIntrinsic::BytesToStrUtf8),
        "HostBytesLen" => Some(RuntimeIntrinsic::BytesLen),
        "HostBytesAt" => Some(RuntimeIntrinsic::BytesAt),
        "HostBytesSlice" => Some(RuntimeIntrinsic::BytesSlice),
        "HostBytesSha256Hex" => Some(RuntimeIntrinsic::BytesSha256Hex),
        "HostBytesThaw" => Some(RuntimeIntrinsic::BytesThaw),
        "HostByteBufferNew" => Some(RuntimeIntrinsic::ByteBufferNew),
        "HostByteBufferLen" => Some(RuntimeIntrinsic::ByteBufferLen),
        "HostByteBufferAt" => Some(RuntimeIntrinsic::ByteBufferAt),
        "HostByteBufferSet" => Some(RuntimeIntrinsic::ByteBufferSet),
        "HostByteBufferPush" => Some(RuntimeIntrinsic::ByteBufferPush),
        "HostByteBufferFreeze" => Some(RuntimeIntrinsic::ByteBufferFreeze),
        "HostUtf16FromStr" => Some(RuntimeIntrinsic::Utf16FromStr),
        "HostUtf16ToStr" => Some(RuntimeIntrinsic::Utf16ToStr),
        "HostUtf16Len" => Some(RuntimeIntrinsic::Utf16Len),
        "HostUtf16At" => Some(RuntimeIntrinsic::Utf16At),
        "HostUtf16Slice" => Some(RuntimeIntrinsic::Utf16Slice),
        "HostUtf16Thaw" => Some(RuntimeIntrinsic::Utf16Thaw),
        "HostUtf16BufferNew" => Some(RuntimeIntrinsic::Utf16BufferNew),
        "HostUtf16BufferLen" => Some(RuntimeIntrinsic::Utf16BufferLen),
        "HostUtf16BufferAt" => Some(RuntimeIntrinsic::Utf16BufferAt),
        "HostUtf16BufferSet" => Some(RuntimeIntrinsic::Utf16BufferSet),
        "HostUtf16BufferPush" => Some(RuntimeIntrinsic::Utf16BufferPush),
        "HostUtf16BufferFreeze" => Some(RuntimeIntrinsic::Utf16BufferFreeze),
        _ => None,
    }
}
