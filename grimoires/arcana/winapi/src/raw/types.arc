export shackle type BOOL = i32
export shackle type I32 = i32
export shackle type U32 = u32
export shackle type I64 = i64
export shackle type U64 = u64
export shackle type BYTE = u8
export shackle type WORD = u16
export shackle type DWORD = u32
export shackle type UINT = u32
export shackle type ULONG = u32
export shackle type UINT32 = u32
export shackle type UINT64 = u64
export shackle type LONG = i32
export shackle type HRESULT = i32
export shackle type FLOAT = f32
export shackle type DOUBLE = f64
export shackle type WCHAR = u16
export shackle type LONG_PTR = isize
export shackle type ULONG_PTR = usize
export shackle type SIZE_T = usize
export shackle type WPARAM = usize
export shackle type LPARAM = isize
export shackle type LRESULT = isize
export shackle type ATOM = u16
export shackle type REFERENCE_TIME = i64
export shackle type HANDLE = *mut c_void
export shackle type HMODULE = HANDLE
export shackle type HINSTANCE = HMODULE
export shackle type HWND = HANDLE
export shackle type HMENU = HANDLE
export shackle type HICON = HANDLE
export shackle type HCURSOR = HANDLE
export shackle type HBRUSH = HANDLE
export shackle type HMONITOR = HANDLE
export shackle type HDC = HANDLE
export shackle type HGDIOBJ = HANDLE
export shackle type HBITMAP = HANDLE
export shackle type HDROP = HANDLE
export shackle type HIMC = HANDLE
export shackle type LPVOID = *mut c_void
export shackle type LPCVOID = *const c_void
export shackle type LPCSTR = *const i8
export shackle type LPWSTR = *mut WCHAR
export shackle type LPCWSTR = *const WCHAR
export shackle type PWSTR = *mut WCHAR
export shackle type PCWSTR = *const WCHAR
export shackle type PHMODULE = *mut arcana_winapi.raw.types.HMODULE
export shackle type PHANDLE = *mut arcana_winapi.raw.types.HANDLE
export shackle type PDWORD = *mut arcana_winapi.raw.types.DWORD
export shackle type PUINT = *mut arcana_winapi.raw.types.UINT
export shackle type PGUID = *mut arcana_winapi.raw.types.GUID
export shackle type PIXAUDIO2 = *mut arcana_winapi.raw.types.IXAudio2
export shackle type PPROPVARIANT = *mut arcana_winapi.raw.types.PROPVARIANT
export shackle type PPVOID = *mut *mut c_void
export shackle type REFGUID = *const arcana_winapi.raw.types.GUID
export shackle type REFIID = *const arcana_winapi.raw.types.GUID
export shackle type LPCGUID = *const arcana_winapi.raw.types.GUID
export shackle type RAW_WNDPROC = Option<unsafe extern "system" fn(arcana_winapi.raw.types.HWND, arcana_winapi.raw.types.UINT, arcana_winapi.raw.types.WPARAM, arcana_winapi.raw.types.LPARAM) -> arcana_winapi.raw.types.LRESULT>
export shackle type PCWNDCLASSW = *const arcana_winapi.raw.types.WNDCLASSW
export shackle type LPMSG = *mut arcana_winapi.raw.types.MSG
export shackle type PCMSG = *const arcana_winapi.raw.types.MSG
export shackle type PCREATESTRUCTW = *const arcana_winapi.raw.types.CREATESTRUCTW
export shackle type LPRECT = *mut arcana_winapi.raw.types.RECT
export shackle type PCRECT = *const arcana_winapi.raw.types.RECT
export shackle type LPMINMAXINFO = *mut arcana_winapi.raw.types.MINMAXINFO
export shackle type PCBITMAPINFO = *const arcana_winapi.raw.types.BITMAPINFO
export shackle type PMONITORINFO = *mut arcana_winapi.raw.types.MONITORINFO
export shackle type PMONITORINFOEXW = *mut arcana_winapi.raw.types.MONITORINFOEXW
export shackle type LPBITMAPINFO = *mut arcana_winapi.raw.types.BITMAPINFO
export shackle type LPWAVEFORMATEX = *mut arcana_winapi.raw.types.WAVEFORMATEX
export shackle type LPCWAVEFORMATEX = *const arcana_winapi.raw.types.WAVEFORMATEX
export shackle type PCD2D1_FACTORY_OPTIONS = *const arcana_winapi.raw.types.D2D1_FACTORY_OPTIONS
export shackle type LPCOMPOSITIONFORM = *const arcana_winapi.raw.types.COMPOSITIONFORM

export shackle type DWRITE_FACTORY_TYPE = U32:
    Shared = 0
    Isolated = 1

export shackle type D2D1_FACTORY_TYPE = U32:
    SingleThreaded = 0
    MultiThreaded = 1

export shackle type EDataFlow = U32:
    Render = 0
    Capture = 1
    All = 2

export shackle type ERole = U32:
    Console = 0
    Multimedia = 1
    Communications = 2

export shackle type AUDCLNT_SHAREMODE = U32:
    Shared = 0
    Exclusive = 1

export shackle type D3D12_COMMAND_LIST_TYPE = U32:
    Direct = 0
    Bundle = 1
    Compute = 2
    Copy = 3

export shackle type D3D12_COMMAND_QUEUE_FLAGS = U32:
    None = 0
    DisableGpuTimeout = 1

export shackle type D3D12_FENCE_FLAGS = U32:
    None = 0
    Shared = 1
    SharedCrossAdapter = 2

export shackle type GUID_DATA4 = [U8; 8]
export shackle type WCHAR32 = [WCHAR; 32]
export shackle type WCHAR128 = [WCHAR; 128]

export shackle struct GUID:
    data1: U32
    data2: U16
    data3: U16
    data4: GUID_DATA4

export shackle struct LUID:
    low_part: U32
    high_part: I32

export shackle struct POINT:
    x: I32
    y: I32

export shackle struct SIZE:
    cx: I32
    cy: I32

export shackle struct RECT:
    left: I32
    top: I32
    right: I32
    bottom: I32

export shackle struct MINMAXINFO:
    ptReserved: POINT
    ptMaxSize: POINT
    ptMaxPosition: POINT
    ptMinTrackSize: POINT
    ptMaxTrackSize: POINT

export shackle struct COMPOSITIONFORM:
    dwStyle: DWORD
    ptCurrentPos: POINT
    rcArea: RECT

export shackle struct MSG:
    hwnd: HWND
    message: UINT
    wParam: WPARAM
    lParam: LPARAM
    time: DWORD
    pt: POINT
    lPrivate: DWORD

export shackle struct WNDCLASSW:
    style: UINT
    lpfnWndProc: RAW_WNDPROC
    cbClsExtra: I32
    cbWndExtra: I32
    hInstance: HINSTANCE
    hIcon: HICON
    hCursor: HCURSOR
    hbrBackground: HBRUSH
    lpszMenuName: LPCWSTR
    lpszClassName: LPCWSTR

export shackle struct CREATESTRUCTW:
    lpCreateParams: LPVOID
    hInstance: HINSTANCE
    hMenu: HMENU
    hwndParent: HWND
    cy: I32
    cx: I32
    y: I32
    x: I32
    style: LONG
    lpszName: LPCWSTR
    lpszClass: LPCWSTR
    dwExStyle: DWORD

export shackle struct MONITORINFO:
    cbSize: DWORD
    rcMonitor: RECT
    rcWork: RECT
    dwFlags: DWORD

export shackle struct MONITORINFOEXW:
    cbSize: DWORD
    rcMonitor: RECT
    rcWork: RECT
    dwFlags: DWORD
    szDevice: WCHAR32

export shackle struct RGBQUAD:
    rgbBlue: BYTE
    rgbGreen: BYTE
    rgbRed: BYTE
    rgbReserved: BYTE

export shackle type RGBQUAD1 = [RGBQUAD; 1]

export shackle struct BITMAPINFOHEADER:
    biSize: DWORD
    biWidth: LONG
    biHeight: LONG
    biPlanes: WORD
    biBitCount: WORD
    biCompression: DWORD
    biSizeImage: DWORD
    biXPelsPerMeter: LONG
    biYPelsPerMeter: LONG
    biClrUsed: DWORD
    biClrImportant: DWORD

export shackle struct BITMAPINFO:
    bmiHeader: BITMAPINFOHEADER
    bmiColors: RGBQUAD1

export shackle struct PROPERTYKEY:
    fmtid: GUID
    pid: DWORD

export shackle union PROPVARIANT_VALUE:
    llVal: I64
    ulVal: U64
    pwszVal: LPWSTR
    punkVal: LPVOID

export shackle struct PROPVARIANT:
    vt: WORD
    wReserved1: WORD
    wReserved2: WORD
    wReserved3: WORD
    value: PROPVARIANT_VALUE

export shackle struct DXGI_RATIONAL:
    Numerator: UINT
    Denominator: UINT

export shackle struct DXGI_SAMPLE_DESC:
    Count: UINT
    Quality: UINT

export shackle struct DXGI_ADAPTER_DESC1:
    Description: WCHAR128
    VendorId: UINT
    DeviceId: UINT
    SubSysId: UINT
    Revision: UINT
    DedicatedVideoMemory: SIZE_T
    DedicatedSystemMemory: SIZE_T
    SharedSystemMemory: SIZE_T
    AdapterLuid: LUID
    Flags: UINT

export shackle struct DXGI_SWAP_CHAIN_DESC1:
    Width: UINT
    Height: UINT
    Format: UINT
    Stereo: BOOL
    SampleDesc: DXGI_SAMPLE_DESC
    BufferUsage: UINT
    BufferCount: UINT
    Scaling: UINT
    SwapEffect: UINT
    AlphaMode: UINT
    Flags: UINT

export shackle struct D3D12_COMMAND_QUEUE_DESC:
    Type: D3D12_COMMAND_LIST_TYPE
    Priority: I32
    Flags: D3D12_COMMAND_QUEUE_FLAGS
    NodeMask: UINT

export shackle struct D2D1_FACTORY_OPTIONS:
    debugLevel: U32

export shackle struct WAVEFORMATEX:
    wFormatTag: WORD
    nChannels: WORD
    nSamplesPerSec: DWORD
    nAvgBytesPerSec: DWORD
    nBlockAlign: WORD
    wBitsPerSample: WORD
    cbSize: WORD

export shackle union WAVEFORMATEXTENSIBLE_SAMPLES:
    wValidBitsPerSample: WORD
    wSamplesPerBlock: WORD
    wReserved: WORD

export shackle struct WAVEFORMATEXTENSIBLE:
    Format: WAVEFORMATEX
    Samples: WAVEFORMATEXTENSIBLE_SAMPLES
    dwChannelMask: DWORD
    SubFormat: GUID

export shackle struct IUnknownVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG

export shackle type IUnknown = *mut c_void

export shackle struct IDXGIAdapter1VTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG
    SetPrivateData: usize
    SetPrivateDataInterface: usize
    GetPrivateData: usize
    GetParent: usize
    EnumOutputs: usize
    GetDesc: usize
    CheckInterfaceSupport: usize
    GetDesc1: unsafe extern "system" fn(*mut c_void, *mut DXGI_ADAPTER_DESC1) -> HRESULT

export shackle type IDXGIAdapter1 = *mut c_void

export shackle struct IDXGIFactory4VTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG
    SetPrivateData: usize
    SetPrivateDataInterface: usize
    GetPrivateData: usize
    GetParent: usize
    EnumAdapters: usize
    MakeWindowAssociation: usize
    GetWindowAssociation: usize
    CreateSwapChain: usize
    CreateSoftwareAdapter: usize
    EnumAdapters1: unsafe extern "system" fn(*mut c_void, UINT, *mut IDXGIAdapter1) -> HRESULT
    IsCurrent: unsafe extern "system" fn(*mut c_void) -> BOOL
    IsWindowedStereoEnabled: usize
    CreateSwapChainForHwnd: unsafe extern "system" fn(*mut c_void, *mut c_void, HWND, *const DXGI_SWAP_CHAIN_DESC1, *const c_void, *mut c_void, *mut *mut c_void) -> HRESULT
    CreateSwapChainForCoreWindow: usize
    GetSharedResourceAdapterLuid: usize
    RegisterStereoStatusWindow: usize
    RegisterStereoStatusEvent: usize
    UnregisterStereoStatus: usize
    RegisterOcclusionStatusWindow: usize
    RegisterOcclusionStatusEvent: usize
    UnregisterOcclusionStatus: usize
    CreateSwapChainForComposition: usize
    GetCreationFlags: usize
    EnumAdapterByLuid: usize
    EnumWarpAdapter: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT

export shackle type IDXGIFactory4 = *mut c_void
export shackle type IDXGISwapChain1 = *mut c_void

export shackle struct ID3D12DeviceVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG
    GetPrivateData: usize
    SetPrivateData: usize
    SetPrivateDataInterface: usize
    SetName: usize
    GetNodeCount: unsafe extern "system" fn(*mut c_void) -> UINT
    CreateCommandQueue: unsafe extern "system" fn(*mut c_void, *const D3D12_COMMAND_QUEUE_DESC, REFIID, *mut *mut c_void) -> HRESULT
    CreateCommandAllocator: unsafe extern "system" fn(*mut c_void, D3D12_COMMAND_LIST_TYPE, REFIID, *mut *mut c_void) -> HRESULT
    CreateGraphicsPipelineState: usize
    CreateComputePipelineState: usize
    CreateCommandList: unsafe extern "system" fn(*mut c_void, UINT, D3D12_COMMAND_LIST_TYPE, *mut c_void, *mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    Reserved0: usize
    Reserved1: usize
    Reserved2: usize
    Reserved3: usize
    Reserved4: usize
    Reserved5: usize
    Reserved6: usize
    Reserved7: usize
    Reserved8: usize
    Reserved9: usize
    Reserved10: usize
    Reserved11: usize
    Reserved12: usize
    Reserved13: usize
    Reserved14: usize
    Reserved15: usize
    Reserved16: usize
    Reserved17: usize
    Reserved18: usize
    Reserved19: usize
    Reserved20: usize
    Reserved21: usize
    Reserved22: usize
    CreateFence: unsafe extern "system" fn(*mut c_void, UINT64, D3D12_FENCE_FLAGS, REFIID, *mut *mut c_void) -> HRESULT

export shackle type ID3D12Device = *mut c_void
export shackle type ID3D12CommandQueue = *mut c_void
export shackle type ID3D12CommandAllocator = *mut c_void
export shackle type ID3D12GraphicsCommandList = *mut c_void
export shackle type ID3D12Fence = *mut c_void

export shackle struct IDWriteFactoryVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG
    GetSystemFontCollection: unsafe extern "system" fn(*mut c_void, *mut *mut c_void, BOOL) -> HRESULT
    CreateCustomFontCollection: usize
    RegisterFontCollectionLoader: usize
    UnregisterFontCollectionLoader: usize
    CreateFontFileReference: usize
    CreateCustomFontFileReference: usize
    CreateFontFace: usize
    CreateRenderingParams: usize
    CreateMonitorRenderingParams: usize
    CreateCustomRenderingParams: usize
    RegisterFontFileLoader: usize
    UnregisterFontFileLoader: usize
    CreateTextFormat: unsafe extern "system" fn(*mut c_void, LPCWSTR, *mut c_void, U32, U32, U32, FLOAT, LPCWSTR, *mut *mut c_void) -> HRESULT
    CreateTypography: usize
    GetGdiInterop: usize
    CreateTextLayout: unsafe extern "system" fn(*mut c_void, LPCWSTR, U32, *mut c_void, FLOAT, FLOAT, *mut *mut c_void) -> HRESULT

export shackle type IDWriteFactory = *mut c_void

export shackle struct IDWriteFontCollectionVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG
    GetFontFamilyCount: unsafe extern "system" fn(*mut c_void) -> U32
    GetFontFamily: usize
    FindFamilyName: usize
    GetFontFromFontFace: usize

export shackle type IDWriteFontCollection = *mut c_void
export shackle type IDWriteTextFormat = *mut c_void
export shackle type IDWriteTextLayout = *mut c_void

export shackle struct ID2D1Factory1VTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG

export shackle type ID2D1Factory1 = *mut c_void

export shackle struct IWICImagingFactoryVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG

export shackle type IWICImagingFactory = *mut c_void

export shackle struct IMMDeviceEnumeratorVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG
    EnumAudioEndpoints: unsafe extern "system" fn(*mut c_void, EDataFlow, DWORD, *mut *mut c_void) -> HRESULT
    GetDefaultAudioEndpoint: unsafe extern "system" fn(*mut c_void, EDataFlow, ERole, *mut *mut c_void) -> HRESULT
    GetDevice: usize
    RegisterEndpointNotificationCallback: usize
    UnregisterEndpointNotificationCallback: usize

export shackle type IMMDeviceEnumerator = *mut c_void

export shackle struct IMMDeviceCollectionVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG
    GetCount: unsafe extern "system" fn(*mut c_void, *mut UINT) -> HRESULT
    Item: usize

export shackle type IMMDeviceCollection = *mut c_void

export shackle struct IMMDeviceVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG
    Activate: unsafe extern "system" fn(*mut c_void, REFIID, DWORD, *mut PROPVARIANT, *mut LPVOID) -> HRESULT
    OpenPropertyStore: usize
    GetId: usize
    GetState: usize

export shackle type IMMDevice = *mut c_void

export shackle struct IAudioClientVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG
    Initialize: unsafe extern "system" fn(*mut c_void, AUDCLNT_SHAREMODE, DWORD, REFERENCE_TIME, REFERENCE_TIME, *const WAVEFORMATEX, LPCGUID) -> HRESULT
    GetBufferSize: usize
    GetStreamLatency: usize
    GetCurrentPadding: usize
    IsFormatSupported: unsafe extern "system" fn(*mut c_void, AUDCLNT_SHAREMODE, *const WAVEFORMATEX, *mut *mut WAVEFORMATEX) -> HRESULT
    GetMixFormat: unsafe extern "system" fn(*mut c_void, *mut *mut WAVEFORMATEX) -> HRESULT
    GetDevicePeriod: usize
    Start: usize
    Stop: usize
    Reset: usize
    SetEventHandle: usize
    GetService: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT

export shackle type IAudioClient = *mut c_void
export shackle struct AUDIOCLIENT_PROPERTIES:
    cbSize: UINT32
    bIsOffload: BOOL
    eCategory: U32
    Options: DWORD

export shackle struct IAudioClient2VTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG
    Initialize: unsafe extern "system" fn(*mut c_void, AUDCLNT_SHAREMODE, DWORD, REFERENCE_TIME, REFERENCE_TIME, *const WAVEFORMATEX, LPCGUID) -> HRESULT
    GetBufferSize: usize
    GetStreamLatency: usize
    GetCurrentPadding: usize
    IsFormatSupported: unsafe extern "system" fn(*mut c_void, AUDCLNT_SHAREMODE, *const WAVEFORMATEX, *mut *mut WAVEFORMATEX) -> HRESULT
    GetMixFormat: unsafe extern "system" fn(*mut c_void, *mut *mut WAVEFORMATEX) -> HRESULT
    GetDevicePeriod: usize
    Start: usize
    Stop: usize
    Reset: usize
    SetEventHandle: usize
    GetService: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    IsOffloadCapable: unsafe extern "system" fn(*mut c_void, U32, *mut BOOL) -> HRESULT
    SetClientProperties: unsafe extern "system" fn(*mut c_void, *const AUDIOCLIENT_PROPERTIES) -> HRESULT
    GetBufferSizeLimits: unsafe extern "system" fn(*mut c_void, *const WAVEFORMATEX, BOOL, *mut REFERENCE_TIME, *mut REFERENCE_TIME) -> HRESULT

export shackle type IAudioClient2 = *mut c_void
export shackle type IAudioRenderClient = *mut c_void

export shackle struct IAudioEndpointVolumeVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG
    RegisterControlChangeNotify: usize
    UnregisterControlChangeNotify: usize
    GetChannelCount: usize
    SetMasterVolumeLevel: usize
    SetMasterVolumeLevelScalar: usize
    GetMasterVolumeLevel: usize
    GetMasterVolumeLevelScalar: unsafe extern "system" fn(*mut c_void, *mut FLOAT) -> HRESULT
    SetChannelVolumeLevel: usize
    SetChannelVolumeLevelScalar: usize
    GetChannelVolumeLevel: usize
    GetChannelVolumeLevelScalar: usize
    SetMute: usize
    GetMute: usize
    GetVolumeStepInfo: usize
    VolumeStepUp: usize
    VolumeStepDown: usize
    QueryHardwareSupport: usize
    GetVolumeRange: usize

export shackle type IAudioEndpointVolume = *mut c_void

export shackle struct IXAudio2VTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, REFIID, *mut *mut c_void) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> ULONG
    Release: unsafe extern "system" fn(*mut c_void) -> ULONG
    RegisterForCallbacks: usize
    UnregisterForCallbacks: usize
    CreateSourceVoice: usize
    CreateSubmixVoice: usize
    CreateMasteringVoice: unsafe extern "system" fn(*mut c_void, *mut *mut c_void, UINT32, UINT32, UINT32, LPCWSTR, LPCVOID, U32) -> HRESULT
    StartEngine: unsafe extern "system" fn(*mut c_void) -> HRESULT
    StopEngine: unsafe extern "system" fn(*mut c_void)
    CommitChanges: usize
    GetPerformanceData: usize
    SetDebugConfiguration: usize

export shackle type IXAudio2 = *mut c_void

export shackle struct IXAudio2VoiceVTable:
    Reserved0: usize
    Reserved1: usize
    Reserved2: usize
    Reserved3: usize
    Reserved4: usize
    Reserved5: usize
    Reserved6: usize
    Reserved7: usize
    Reserved8: usize
    Reserved9: usize
    Reserved10: usize
    Reserved11: usize
    Reserved12: usize
    Reserved13: usize
    Reserved14: usize
    Reserved15: usize
    Reserved16: usize
    DestroyVoice: unsafe extern "system" fn(*mut c_void)

export shackle type IXAudio2MasteringVoice = *mut c_void
