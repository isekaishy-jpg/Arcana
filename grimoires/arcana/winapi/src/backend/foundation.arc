export native fn current_module() -> arcana_winapi.raw.types.HMODULE = foundation.current_module
export native fn module_is_null(read module: arcana_winapi.raw.types.HMODULE) -> Bool = foundation.module_is_null
export native fn module_path(read module: arcana_winapi.raw.types.HMODULE) -> Str = foundation.module_path
export native fn utf16_len(read text: Str) -> Int = foundation.utf16_len
export native fn fail_sample(read message: Str) -> Int = foundation.fail_sample
