export native fn take_last_error() -> Str = helpers.clipboard.take_last_error
export native fn read_text_raw() -> Str = helpers.clipboard.read_text_raw
export native fn write_text_raw(text: Str) -> Bool = helpers.clipboard.write_text_raw
export native fn read_bytes_raw() -> Bytes = helpers.clipboard.read_bytes_raw
export native fn write_bytes_raw(read bytes: Bytes) -> Bool = helpers.clipboard.write_bytes_raw
