export native fn initialize_multithreaded() -> arcana_winapi.raw.types.HRESULT = helpers.com.initialize_multithreaded
export native fn initialize_apartment_threaded() -> arcana_winapi.raw.types.HRESULT = helpers.com.initialize_apartment_threaded
export native fn uninitialize() = helpers.com.uninitialize
export native fn guid_to_text(read guid: arcana_winapi.raw.types.GUID) -> Str = helpers.com.guid_to_text
export native fn make_property_key(read fmtid: arcana_winapi.raw.types.GUID, read pid: arcana_winapi.raw.types.DWORD) -> arcana_winapi.raw.types.PROPERTYKEY = helpers.com.make_property_key
export native fn property_key_pid(read key: arcana_winapi.raw.types.PROPERTYKEY) -> arcana_winapi.raw.types.DWORD = helpers.com.property_key_pid
