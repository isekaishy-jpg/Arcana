export native fn last_error() -> arcana_winapi.raw.types.DWORD = helpers.errors.last_error
export native fn hresult_succeeded(read code: arcana_winapi.raw.types.HRESULT) -> Bool = helpers.errors.hresult_succeeded
export native fn hresult_failed(read code: arcana_winapi.raw.types.HRESULT) -> Bool = helpers.errors.hresult_failed
