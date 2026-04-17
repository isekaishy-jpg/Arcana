// GENERATED FILE. DO NOT EDIT BY HAND.
// Source of truth: grimoires/arcana/winapi/generation/types.toml
// Projection config: grimoires/arcana/winapi/generation/projection.toml
// Source authority: Pinned Windows SDK metadata snapshot
// Metadata authority: Windows.Win32.winmd Microsoft.Windows.SDK.Win32Metadata 63.0.31 sha256:97D24CF1A9DC3E50782BBF1DBA0952BF6A025FA583D8B3AE6C5EF713B463C869
// Parity target: windows-sys; pinned metadata wins on disagreement.

export shackle type BOOL = i32

export shackle type I32 = i32

export shackle type I16 = i16

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

export shackle type HGLOBAL = HANDLE

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

export shackle type FARPROC = LPVOID

export shackle type LPCVOID = *const c_void

export shackle type LPCSTR = *const i8

export shackle type PSTR = *mut i8

export shackle type PCSTR = *const i8

export shackle type LPWSTR = *mut WCHAR

export shackle type LPCWSTR = *const WCHAR

export shackle type PWSTR = *mut WCHAR

export shackle type PCWSTR = *const WCHAR

export shackle type PHMODULE = *mut HMODULE

export shackle type PHANDLE = *mut HANDLE

export shackle type PDWORD = *mut DWORD

export shackle type PUINT = *mut UINT

export shackle type PGUID = *mut GUID

export shackle type REFCLSID = *const GUID

export shackle type PIXAUDIO2 = *mut IXAudio2

export shackle type PPROPVARIANT = *mut PROPVARIANT

export shackle type PPVOID = *mut *mut c_void

export shackle type LPUNKNOWN = *mut c_void

export shackle type REFGUID = *const GUID

export shackle type REFIID = *const GUID

export shackle type LPCGUID = *const GUID

export shackle type RAW_WNDPROC = Option<unsafe extern "system" fn(arcana_winapi.raw.types.HWND, arcana_winapi.raw.types.UINT, arcana_winapi.raw.types.WPARAM, arcana_winapi.raw.types.LPARAM) -> arcana_winapi.raw.types.LRESULT>

export shackle type PCWNDCLASSW = *const WNDCLASSW

export shackle type PWNDCLASSW = *mut WNDCLASSW

export shackle type LPMSG = *mut MSG

export shackle type PCMSG = *const MSG

export shackle type PCREATESTRUCTW = *const CREATESTRUCTW

export shackle type LPRECT = *mut RECT

export shackle type PCRECT = *const RECT

export shackle type LPMINMAXINFO = *mut MINMAXINFO

export shackle type PCBITMAPINFO = *const BITMAPINFO

export shackle type PMONITORINFO = *mut MONITORINFO

export shackle type PMONITORINFOEXW = *mut MONITORINFOEXW

export shackle type LPBITMAPINFO = *mut BITMAPINFO

export shackle type LPWAVEFORMATEX = *mut WAVEFORMATEX

export shackle type LPCWAVEFORMATEX = *const WAVEFORMATEX

export shackle type PCD2D1_FACTORY_OPTIONS = *const D2D1_FACTORY_OPTIONS

export shackle type LPCOMPOSITIONFORM = *const COMPOSITIONFORM

export shackle type PCOMPOSITIONFORM = *mut COMPOSITIONFORM

export shackle type PD2D1_FACTORY_OPTIONS = *mut D2D1_FACTORY_OPTIONS

export shackle type PSECURITY_ATTRIBUTES = *mut SECURITY_ATTRIBUTES

export shackle type SECURITY_ATTRIBUTES = c_void

export shackle type PATOM = *mut ATOM

export shackle type PCATOM = *const ATOM

export shackle type PAUDCLNT_SHAREMODE = *mut AUDCLNT_SHAREMODE

export shackle type PCAUDCLNT_SHAREMODE = *const AUDCLNT_SHAREMODE

export shackle type PAUDIOCLIENT_PROPERTIES = *mut AUDIOCLIENT_PROPERTIES

export shackle type PCAUDIOCLIENT_PROPERTIES = *const AUDIOCLIENT_PROPERTIES

export shackle type PAUDIO_STREAM_CATEGORY = *mut AUDIO_STREAM_CATEGORY

export shackle type PCAUDIO_STREAM_CATEGORY = *const AUDIO_STREAM_CATEGORY

export shackle type PBITMAPINFO = *mut BITMAPINFO

export shackle type PBITMAPINFOHEADER = *mut BITMAPINFOHEADER

export shackle type PCBITMAPINFOHEADER = *const BITMAPINFOHEADER

export shackle type PBOOL = *mut BOOL

export shackle type PCBOOL = *const BOOL

export shackle type PBYTE = *mut BYTE

export shackle type PCBYTE = *const BYTE

export shackle type PCCOMPOSITIONFORM = *const COMPOSITIONFORM

export shackle type PCCREATESTRUCTW = *const CREATESTRUCTW

export shackle type PD2D1_FACTORY_TYPE = *mut D2D1_FACTORY_TYPE

export shackle type PCD2D1_FACTORY_TYPE = *const D2D1_FACTORY_TYPE

export shackle type PD3D12_COMMAND_LIST_TYPE = *mut D3D12_COMMAND_LIST_TYPE

export shackle type PCD3D12_COMMAND_LIST_TYPE = *const D3D12_COMMAND_LIST_TYPE

export shackle type PD3D12_COMMAND_QUEUE_DESC = *mut D3D12_COMMAND_QUEUE_DESC

export shackle type PCD3D12_COMMAND_QUEUE_DESC = *const D3D12_COMMAND_QUEUE_DESC

export shackle type PD3D12_COMMAND_QUEUE_FLAGS = *mut D3D12_COMMAND_QUEUE_FLAGS

export shackle type PCD3D12_COMMAND_QUEUE_FLAGS = *const D3D12_COMMAND_QUEUE_FLAGS

export shackle type PD3D12_FENCE_FLAGS = *mut D3D12_FENCE_FLAGS

export shackle type PCD3D12_FENCE_FLAGS = *const D3D12_FENCE_FLAGS

export shackle type PDOUBLE = *mut DOUBLE

export shackle type PCDOUBLE = *const DOUBLE

export shackle type PCDWORD = *const DWORD

export shackle type PDWRITE_FACTORY_TYPE = *mut DWRITE_FACTORY_TYPE

export shackle type PCDWRITE_FACTORY_TYPE = *const DWRITE_FACTORY_TYPE

export shackle type PDXGI_ADAPTER_DESC1 = *mut DXGI_ADAPTER_DESC1

export shackle type PCDXGI_ADAPTER_DESC1 = *const DXGI_ADAPTER_DESC1

export shackle type PDXGI_RATIONAL = *mut DXGI_RATIONAL

export shackle type PCDXGI_RATIONAL = *const DXGI_RATIONAL

export shackle type PDXGI_SAMPLE_DESC = *mut DXGI_SAMPLE_DESC

export shackle type PCDXGI_SAMPLE_DESC = *const DXGI_SAMPLE_DESC

export shackle type PDXGI_SWAP_CHAIN_DESC1 = *mut DXGI_SWAP_CHAIN_DESC1

export shackle type PCDXGI_SWAP_CHAIN_DESC1 = *const DXGI_SWAP_CHAIN_DESC1

export shackle type PEDataFlow = *mut EDataFlow

export shackle type PCEDataFlow = *const EDataFlow

export shackle type PERole = *mut ERole

export shackle type PCERole = *const ERole

export shackle type PFARPROC = *mut FARPROC

export shackle type PCFARPROC = *const FARPROC

export shackle type PFLOAT = *mut FLOAT

export shackle type PCFLOAT = *const FLOAT

export shackle type PCGUID = *const GUID

export shackle type PCHANDLE = *const HANDLE

export shackle type PHBITMAP = *mut HBITMAP

export shackle type PCHBITMAP = *const HBITMAP

export shackle type PHBRUSH = *mut HBRUSH

export shackle type PCHBRUSH = *const HBRUSH

export shackle type PHCURSOR = *mut HCURSOR

export shackle type PCHCURSOR = *const HCURSOR

export shackle type PHDC = *mut HDC

export shackle type PCHDC = *const HDC

export shackle type PHDROP = *mut HDROP

export shackle type PCHDROP = *const HDROP

export shackle type PHGDIOBJ = *mut HGDIOBJ

export shackle type PCHGDIOBJ = *const HGDIOBJ

export shackle type PHGLOBAL = *mut HGLOBAL

export shackle type PCHGLOBAL = *const HGLOBAL

export shackle type PHICON = *mut HICON

export shackle type PCHICON = *const HICON

export shackle type PHIMC = *mut HIMC

export shackle type PCHIMC = *const HIMC

export shackle type PHINSTANCE = *mut HINSTANCE

export shackle type PCHINSTANCE = *const HINSTANCE

export shackle type PHMENU = *mut HMENU

export shackle type PCHMENU = *const HMENU

export shackle type PCHMODULE = *const HMODULE

export shackle type PHMONITOR = *mut HMONITOR

export shackle type PCHMONITOR = *const HMONITOR

export shackle type PHRESULT = *mut HRESULT

export shackle type PCHRESULT = *const HRESULT

export shackle type PHWND = *mut HWND

export shackle type PCHWND = *const HWND

export shackle type PI16 = *mut I16

export shackle type PCI16 = *const I16

export shackle type PI32 = *mut I32

export shackle type PCI32 = *const I32

export shackle type PI64 = *mut I64

export shackle type PCI64 = *const I64

export shackle type PIAudioCaptureClient = *mut IAudioCaptureClient

export shackle type PCIAudioCaptureClient = *const IAudioCaptureClient

export shackle type PIAudioClient = *mut IAudioClient

export shackle type PCIAudioClient = *const IAudioClient

export shackle type PIAudioClient2 = *mut IAudioClient2

export shackle type PCIAudioClient2 = *const IAudioClient2

export shackle type PIAudioClient3 = *mut IAudioClient3

export shackle type PCIAudioClient3 = *const IAudioClient3

export shackle type PIAudioClock2 = *mut IAudioClock2

export shackle type PCIAudioClock2 = *const IAudioClock2

export shackle type PIAudioEndpointVolume = *mut IAudioEndpointVolume

export shackle type PCIAudioEndpointVolume = *const IAudioEndpointVolume

export shackle type PIAudioRenderClient = *mut IAudioRenderClient

export shackle type PCIAudioRenderClient = *const IAudioRenderClient

export shackle type PIAudioSessionControl2 = *mut IAudioSessionControl2

export shackle type PCIAudioSessionControl2 = *const IAudioSessionControl2

export shackle type PIAudioSessionManager2 = *mut IAudioSessionManager2

export shackle type PCIAudioSessionManager2 = *const IAudioSessionManager2

export shackle type PID2D1Factory1 = *mut ID2D1Factory1

export shackle type PCID2D1Factory1 = *const ID2D1Factory1

export shackle type PID3D12CommandAllocator = *mut ID3D12CommandAllocator

export shackle type PCID3D12CommandAllocator = *const ID3D12CommandAllocator

export shackle type PID3D12CommandQueue = *mut ID3D12CommandQueue

export shackle type PCID3D12CommandQueue = *const ID3D12CommandQueue

export shackle type PID3D12Device = *mut ID3D12Device

export shackle type PCID3D12Device = *const ID3D12Device

export shackle type PID3D12Fence = *mut ID3D12Fence

export shackle type PCID3D12Fence = *const ID3D12Fence

export shackle type PID3D12GraphicsCommandList = *mut ID3D12GraphicsCommandList

export shackle type PCID3D12GraphicsCommandList = *const ID3D12GraphicsCommandList

export shackle type PIDWriteFactory = *mut IDWriteFactory

export shackle type PCIDWriteFactory = *const IDWriteFactory

export shackle type PIDWriteFontCollection = *mut IDWriteFontCollection

export shackle type PCIDWriteFontCollection = *const IDWriteFontCollection

export shackle type PIDWriteTextFormat = *mut IDWriteTextFormat

export shackle type PCIDWriteTextFormat = *const IDWriteTextFormat

export shackle type PIDWriteTextLayout = *mut IDWriteTextLayout

export shackle type PCIDWriteTextLayout = *const IDWriteTextLayout

export shackle type PIDXGIAdapter1 = *mut IDXGIAdapter1

export shackle type PCIDXGIAdapter1 = *const IDXGIAdapter1

export shackle type PIDXGIFactory4 = *mut IDXGIFactory4

export shackle type PCIDXGIFactory4 = *const IDXGIFactory4

export shackle type PIDXGISwapChain1 = *mut IDXGISwapChain1

export shackle type PCIDXGISwapChain1 = *const IDXGISwapChain1

export shackle type PIMMDevice = *mut IMMDevice

export shackle type PCIMMDevice = *const IMMDevice

export shackle type PIMMDeviceCollection = *mut IMMDeviceCollection

export shackle type PCIMMDeviceCollection = *const IMMDeviceCollection

export shackle type PIMMDeviceEnumerator = *mut IMMDeviceEnumerator

export shackle type PCIMMDeviceEnumerator = *const IMMDeviceEnumerator

export shackle type PIMMNotificationClient = *mut IMMNotificationClient

export shackle type PCIMMNotificationClient = *const IMMNotificationClient

export shackle type PIUnknown = *mut IUnknown

export shackle type PCIUnknown = *const IUnknown

export shackle type PIWICImagingFactory = *mut IWICImagingFactory

export shackle type PCIWICImagingFactory = *const IWICImagingFactory

export shackle type PIXAudio2 = *mut IXAudio2

export shackle type PCIXAudio2 = *const IXAudio2

export shackle type PIXAudio2EngineCallback = *mut IXAudio2EngineCallback

export shackle type PCIXAudio2EngineCallback = *const IXAudio2EngineCallback

export shackle type PIXAudio2MasteringVoice = *mut IXAudio2MasteringVoice

export shackle type PCIXAudio2MasteringVoice = *const IXAudio2MasteringVoice

export shackle type PIXAudio2SourceVoice = *mut IXAudio2SourceVoice

export shackle type PCIXAudio2SourceVoice = *const IXAudio2SourceVoice

export shackle type PIXAudio2SubmixVoice = *mut IXAudio2SubmixVoice

export shackle type PCIXAudio2SubmixVoice = *const IXAudio2SubmixVoice

export shackle type PIXAudio2VoiceCallback = *mut IXAudio2VoiceCallback

export shackle type PCIXAudio2VoiceCallback = *const IXAudio2VoiceCallback

export shackle type PLONG = *mut LONG

export shackle type PCLONG = *const LONG

export shackle type PLONG_PTR = *mut LONG_PTR

export shackle type PCLONG_PTR = *const LONG_PTR

export shackle type PLPARAM = *mut LPARAM

export shackle type PCLPARAM = *const LPARAM

export shackle type PLPBITMAPINFO = *mut LPBITMAPINFO

export shackle type PCLPBITMAPINFO = *const LPBITMAPINFO

export shackle type PLPCGUID = *mut LPCGUID

export shackle type PCLPCGUID = *const LPCGUID

export shackle type PLPCOMPOSITIONFORM = *mut LPCOMPOSITIONFORM

export shackle type PCLPCOMPOSITIONFORM = *const LPCOMPOSITIONFORM

export shackle type PLPCSTR = *mut LPCSTR

export shackle type PCLPCSTR = *const LPCSTR

export shackle type PLPCVOID = *mut LPCVOID

export shackle type PCLPCVOID = *const LPCVOID

export shackle type PLPCWAVEFORMATEX = *mut LPCWAVEFORMATEX

export shackle type PCLPCWAVEFORMATEX = *const LPCWAVEFORMATEX

export shackle type PLPCWSTR = *mut LPCWSTR

export shackle type PCLPCWSTR = *const LPCWSTR

export shackle type PLPMINMAXINFO = *mut LPMINMAXINFO

export shackle type PCLPMINMAXINFO = *const LPMINMAXINFO

export shackle type PLPMSG = *mut LPMSG

export shackle type PCLPMSG = *const LPMSG

export shackle type PLPRECT = *mut LPRECT

export shackle type PCLPRECT = *const LPRECT

export shackle type PLPUNKNOWN = *mut LPUNKNOWN

export shackle type PCLPUNKNOWN = *const LPUNKNOWN

export shackle type PLPVOID = *mut LPVOID

export shackle type PCLPVOID = *const LPVOID

export shackle type PLPWAVEFORMATEX = *mut LPWAVEFORMATEX

export shackle type PCLPWAVEFORMATEX = *const LPWAVEFORMATEX

export shackle type PLPWSTR = *mut LPWSTR

export shackle type PCLPWSTR = *const LPWSTR

export shackle type PLRESULT = *mut LRESULT

export shackle type PCLRESULT = *const LRESULT

export shackle type PLUID = *mut LUID

export shackle type PCLUID = *const LUID

export shackle type PMINMAXINFO = *mut MINMAXINFO

export shackle type PCMINMAXINFO = *const MINMAXINFO

export shackle type PCMONITORINFO = *const MONITORINFO

export shackle type PCMONITORINFOEXW = *const MONITORINFOEXW

export shackle type PMSG = *mut MSG

export shackle type PPCBITMAPINFO = *mut PCBITMAPINFO

export shackle type PCPCBITMAPINFO = *const PCBITMAPINFO

export shackle type PPCD2D1_FACTORY_OPTIONS = *mut PCD2D1_FACTORY_OPTIONS

export shackle type PCPCD2D1_FACTORY_OPTIONS = *const PCD2D1_FACTORY_OPTIONS

export shackle type PPCMSG = *mut PCMSG

export shackle type PCPCMSG = *const PCMSG

export shackle type PPCOMPOSITIONFORM = *mut PCOMPOSITIONFORM

export shackle type PCPCOMPOSITIONFORM = *const PCOMPOSITIONFORM

export shackle type PPCREATESTRUCTW = *mut PCREATESTRUCTW

export shackle type PCPCREATESTRUCTW = *const PCREATESTRUCTW

export shackle type PPCRECT = *mut PCRECT

export shackle type PCPCRECT = *const PCRECT

export shackle type PPCSTR = *mut PCSTR

export shackle type PCPCSTR = *const PCSTR

export shackle type PPCWNDCLASSW = *mut PCWNDCLASSW

export shackle type PCPCWNDCLASSW = *const PCWNDCLASSW

export shackle type PPCWSTR = *mut PCWSTR

export shackle type PCPCWSTR = *const PCWSTR

export shackle type PPD2D1_FACTORY_OPTIONS = *mut PD2D1_FACTORY_OPTIONS

export shackle type PCPD2D1_FACTORY_OPTIONS = *const PD2D1_FACTORY_OPTIONS

export shackle type PPDWORD = *mut PDWORD

export shackle type PCPDWORD = *const PDWORD

export shackle type PPGUID = *mut PGUID

export shackle type PCPGUID = *const PGUID

export shackle type PPHANDLE = *mut PHANDLE

export shackle type PCPHANDLE = *const PHANDLE

export shackle type PPHMODULE = *mut PHMODULE

export shackle type PCPHMODULE = *const PHMODULE

export shackle type PPIXAUDIO2 = *mut PIXAUDIO2

export shackle type PCPIXAUDIO2 = *const PIXAUDIO2

export shackle type PPMONITORINFO = *mut PMONITORINFO

export shackle type PCPMONITORINFO = *const PMONITORINFO

export shackle type PPMONITORINFOEXW = *mut PMONITORINFOEXW

export shackle type PCPMONITORINFOEXW = *const PMONITORINFOEXW

export shackle type PPOINT = *mut POINT

export shackle type PCPOINT = *const POINT

export shackle type PPPROPVARIANT = *mut PPROPVARIANT

export shackle type PCPPROPVARIANT = *const PPROPVARIANT

export shackle type PPPVOID = *mut PPVOID

export shackle type PCPPVOID = *const PPVOID

export shackle type PPROPERTYKEY = *mut PROPERTYKEY

export shackle type PCPROPERTYKEY = *const PROPERTYKEY

export shackle type PCPROPVARIANT = *const PROPVARIANT

export shackle type PPROPVARIANT_VALUE = *mut PROPVARIANT_VALUE

export shackle type PCPROPVARIANT_VALUE = *const PROPVARIANT_VALUE

export shackle type PPSECURITY_ATTRIBUTES = *mut PSECURITY_ATTRIBUTES

export shackle type PCPSECURITY_ATTRIBUTES = *const PSECURITY_ATTRIBUTES

export shackle type PPSTR = *mut PSTR

export shackle type PCPSTR = *const PSTR

export shackle type PPUINT = *mut PUINT

export shackle type PCPUINT = *const PUINT

export shackle type PPWNDCLASSW = *mut PWNDCLASSW

export shackle type PCPWNDCLASSW = *const PWNDCLASSW

export shackle type PPWSTR = *mut PWSTR

export shackle type PCPWSTR = *const PWSTR

export shackle type PRECT = *mut RECT

export shackle type PREFCLSID = *mut REFCLSID

export shackle type PCREFCLSID = *const REFCLSID

export shackle type PREFERENCE_TIME = *mut REFERENCE_TIME

export shackle type PCREFERENCE_TIME = *const REFERENCE_TIME

export shackle type PREFGUID = *mut REFGUID

export shackle type PCREFGUID = *const REFGUID

export shackle type PREFIID = *mut REFIID

export shackle type PCREFIID = *const REFIID

export shackle type PRGBQUAD = *mut RGBQUAD

export shackle type PCRGBQUAD = *const RGBQUAD

export shackle type PCSECURITY_ATTRIBUTES = *const SECURITY_ATTRIBUTES

export shackle type PSIZE = *mut SIZE

export shackle type PCSIZE = *const SIZE

export shackle type PSIZE_T = *mut SIZE_T

export shackle type PCSIZE_T = *const SIZE_T

export shackle type PU32 = *mut U32

export shackle type PCU32 = *const U32

export shackle type PU64 = *mut U64

export shackle type PCU64 = *const U64

export shackle type PCUINT = *const UINT

export shackle type PUINT32 = *mut UINT32

export shackle type PCUINT32 = *const UINT32

export shackle type PUINT64 = *mut UINT64

export shackle type PCUINT64 = *const UINT64

export shackle type PULONG = *mut ULONG

export shackle type PCULONG = *const ULONG

export shackle type PULONG_PTR = *mut ULONG_PTR

export shackle type PCULONG_PTR = *const ULONG_PTR

export shackle type PWAVEFORMATEX = *mut WAVEFORMATEX

export shackle type PCWAVEFORMATEX = *const WAVEFORMATEX

export shackle type PWAVEFORMATEXTENSIBLE = *mut WAVEFORMATEXTENSIBLE

export shackle type PCWAVEFORMATEXTENSIBLE = *const WAVEFORMATEXTENSIBLE

export shackle type PWAVEFORMATEXTENSIBLE_SAMPLES = *mut WAVEFORMATEXTENSIBLE_SAMPLES

export shackle type PCWAVEFORMATEXTENSIBLE_SAMPLES = *const WAVEFORMATEXTENSIBLE_SAMPLES

export shackle type PWCHAR = *mut WCHAR

export shackle type PCWCHAR = *const WCHAR

export shackle type PWORD = *mut WORD

export shackle type PCWORD = *const WORD

export shackle type PWPARAM = *mut WPARAM

export shackle type PCWPARAM = *const WPARAM

export shackle type GUID_DATA4 = [U8; 8]

export shackle type WCHAR32 = [WCHAR; 32]

export shackle type WCHAR128 = [WCHAR; 128]

export shackle type RGBQUAD1 = [RGBQUAD; 1]

export shackle struct GUID:
    data1: U32
    data2: U16
    data3: U16
    data4: GUID_DATA4

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

export shackle type DWRITE_FACTORY_TYPE = I32:
    DWRITE_FACTORY_TYPE_SHARED = 0
    DWRITE_FACTORY_TYPE_ISOLATED = 1

export shackle type D2D1_FACTORY_TYPE = I32:
    D2D1_FACTORY_TYPE_SINGLE_THREADED = 0
    D2D1_FACTORY_TYPE_MULTI_THREADED = 1

export shackle type EDataFlow = I32:
    eRender = 0
    eCapture = 1
    eAll = 2
    EDataFlow_enum_count = 3

export shackle type ERole = I32:
    eConsole = 0
    eMultimedia = 1
    eCommunications = 2
    ERole_enum_count = 3

export shackle type AUDCLNT_SHAREMODE = I32:
    AUDCLNT_SHAREMODE_SHARED = 0
    AUDCLNT_SHAREMODE_EXCLUSIVE = 1

export shackle type AUDIO_STREAM_CATEGORY = I32:
    AudioCategory_Other = 0
    AudioCategory_ForegroundOnlyMedia = 1
    AudioCategory_Communications = 3
    AudioCategory_Alerts = 4
    AudioCategory_SoundEffects = 5
    AudioCategory_GameEffects = 6
    AudioCategory_GameMedia = 7
    AudioCategory_GameChat = 8
    AudioCategory_Speech = 9
    AudioCategory_Movie = 10
    AudioCategory_Media = 11
    AudioCategory_FarFieldSpeech = 12
    AudioCategory_UniformSpeech = 13
    AudioCategory_VoiceTyping = 14

export shackle type D3D12_COMMAND_LIST_TYPE = I32:
    D3D12_COMMAND_LIST_TYPE_DIRECT = 0
    D3D12_COMMAND_LIST_TYPE_BUNDLE = 1
    D3D12_COMMAND_LIST_TYPE_COMPUTE = 2
    D3D12_COMMAND_LIST_TYPE_COPY = 3
    D3D12_COMMAND_LIST_TYPE_VIDEO_DECODE = 4
    D3D12_COMMAND_LIST_TYPE_VIDEO_PROCESS = 5
    D3D12_COMMAND_LIST_TYPE_VIDEO_ENCODE = 6
    D3D12_COMMAND_LIST_TYPE_NONE = -1

export shackle type D3D12_COMMAND_QUEUE_FLAGS = I32:
    D3D12_COMMAND_QUEUE_FLAG_NONE = 0
    D3D12_COMMAND_QUEUE_FLAG_DISABLE_GPU_TIMEOUT = 1

export shackle type D3D12_FENCE_FLAGS = I32:
    D3D12_FENCE_FLAG_NONE = 0
    D3D12_FENCE_FLAG_SHARED = 1
    D3D12_FENCE_FLAG_SHARED_CROSS_ADAPTER = 2
    D3D12_FENCE_FLAG_NON_MONITORED = 4

export shackle struct LUID:
    LowPart: U32
    HighPart: I32

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
    dwStyle: U32
    ptCurrentPos: POINT
    rcArea: RECT

export shackle struct MSG:
    hwnd: HWND
    message: U32
    wParam: WPARAM
    lParam: LPARAM
    time: U32
    pt: POINT

export shackle struct WNDCLASSW:
    style: U32
    lpfnWndProc: RAW_WNDPROC
    cbClsExtra: I32
    cbWndExtra: I32
    hInstance: HINSTANCE
    hIcon: HICON
    hCursor: HCURSOR
    hbrBackground: HBRUSH
    lpszMenuName: PWSTR
    lpszClassName: PWSTR

export shackle struct CREATESTRUCTW:
    lpCreateParams: HANDLE
    hInstance: HINSTANCE
    hMenu: HMENU
    hwndParent: HWND
    cy: I32
    cx: I32
    y: I32
    x: I32
    style: I32
    lpszName: PWSTR
    lpszClass: PWSTR
    dwExStyle: U32

export shackle struct MONITORINFO:
    cbSize: U32
    rcMonitor: RECT
    rcWork: RECT
    dwFlags: U32

export shackle struct MONITORINFOEXW:
    monitorInfo: MONITORINFO
    szDevice: WCHAR32

export shackle struct RGBQUAD:
    rgbBlue: BYTE
    rgbGreen: BYTE
    rgbRed: BYTE
    rgbReserved: BYTE

export shackle struct BITMAPINFOHEADER:
    biSize: U32
    biWidth: I32
    biHeight: I32
    biPlanes: WORD
    biBitCount: WORD
    biCompression: U32
    biSizeImage: U32
    biXPelsPerMeter: I32
    biYPelsPerMeter: I32
    biClrUsed: U32
    biClrImportant: U32

export shackle struct BITMAPINFO:
    bmiHeader: BITMAPINFOHEADER
    bmiColors: RGBQUAD1

export shackle struct PROPERTYKEY:
    fmtid: GUID
    pid: U32

export shackle struct DXGI_RATIONAL:
    Numerator: U32
    Denominator: U32

export shackle struct DXGI_SAMPLE_DESC:
    Count: U32
    Quality: U32

export shackle struct DXGI_ADAPTER_DESC1:
    Description: WCHAR128
    VendorId: U32
    DeviceId: U32
    SubSysId: U32
    Revision: U32
    DedicatedVideoMemory: ULONG_PTR
    DedicatedSystemMemory: ULONG_PTR
    SharedSystemMemory: ULONG_PTR
    AdapterLuid: LUID
    Flags: U32

export shackle struct DXGI_SWAP_CHAIN_DESC1:
    Width: U32
    Height: U32
    Format: I32
    Stereo: BOOL
    SampleDesc: DXGI_SAMPLE_DESC
    BufferUsage: U32
    BufferCount: U32
    Scaling: I32
    SwapEffect: I32
    AlphaMode: I32
    Flags: U32

export shackle struct D3D12_COMMAND_QUEUE_DESC:
    Type: D3D12_COMMAND_LIST_TYPE
    Priority: I32
    Flags: D3D12_COMMAND_QUEUE_FLAGS
    NodeMask: U32

export shackle struct D2D1_FACTORY_OPTIONS:
    debugLevel: I32

export shackle struct WAVEFORMATEX:
    wFormatTag: WORD
    nChannels: WORD
    nSamplesPerSec: U32
    nAvgBytesPerSec: U32
    nBlockAlign: WORD
    wBitsPerSample: WORD
    cbSize: WORD

export shackle struct WAVEFORMATEXTENSIBLE:
    Format: WAVEFORMATEX
    Samples: WAVEFORMATEXTENSIBLE_SAMPLES
    dwChannelMask: U32
    SubFormat: GUID

export shackle struct AUDIOCLIENT_PROPERTIES:
    cbSize: U32
    bIsOffload: BOOL
    eCategory: AUDIO_STREAM_CATEGORY
    Options: I32

export shackle union WAVEFORMATEXTENSIBLE_SAMPLES:
    wValidBitsPerSample: WORD
    wSamplesPerBlock: WORD
    wReserved: WORD

export shackle struct IUnknownVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32

export shackle type IUnknown = *mut c_void

export shackle struct IDXGIAdapter1VTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    SetPrivateData: unsafe extern "system" fn(*mut c_void, PGUID, U32, HANDLE) -> HRESULT
    SetPrivateDataInterface: unsafe extern "system" fn(*mut c_void, PGUID, IUnknown) -> HRESULT
    GetPrivateData: unsafe extern "system" fn(*mut c_void, PGUID, PUINT, HANDLE) -> HRESULT
    GetParent: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    EnumOutputs: unsafe extern "system" fn(*mut c_void, U32, PLPVOID) -> HRESULT
    GetDesc: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    CheckInterfaceSupport: unsafe extern "system" fn(*mut c_void, PGUID, PI64) -> HRESULT
    GetDesc1: unsafe extern "system" fn(*mut c_void, PDXGI_ADAPTER_DESC1) -> HRESULT

export shackle type IDXGIAdapter1 = *mut c_void

export shackle struct IDXGIFactory4VTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    SetPrivateData: unsafe extern "system" fn(*mut c_void, PGUID, U32, HANDLE) -> HRESULT
    SetPrivateDataInterface: unsafe extern "system" fn(*mut c_void, PGUID, IUnknown) -> HRESULT
    GetPrivateData: unsafe extern "system" fn(*mut c_void, PGUID, PUINT, HANDLE) -> HRESULT
    GetParent: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    EnumAdapters: unsafe extern "system" fn(*mut c_void, U32, PLPVOID) -> HRESULT
    MakeWindowAssociation: unsafe extern "system" fn(*mut c_void, HWND, U32) -> HRESULT
    GetWindowAssociation: unsafe extern "system" fn(*mut c_void, PHWND) -> HRESULT
    CreateSwapChain: unsafe extern "system" fn(*mut c_void, IUnknown, PLPVOID, PLPVOID) -> HRESULT
    CreateSoftwareAdapter: unsafe extern "system" fn(*mut c_void, HMODULE, PLPVOID) -> HRESULT
    EnumAdapters1: unsafe extern "system" fn(*mut c_void, U32, PIDXGIAdapter1) -> HRESULT
    IsCurrent: unsafe extern "system" fn(*mut c_void) -> BOOL
    IsWindowedStereoEnabled: unsafe extern "system" fn(*mut c_void) -> BOOL
    CreateSwapChainForHwnd: unsafe extern "system" fn(*mut c_void, IUnknown, HWND, PDXGI_SWAP_CHAIN_DESC1, PLPVOID, LPVOID, PIDXGISwapChain1) -> HRESULT
    CreateSwapChainForCoreWindow: unsafe extern "system" fn(*mut c_void, IUnknown, IUnknown, PDXGI_SWAP_CHAIN_DESC1, LPVOID, PIDXGISwapChain1) -> HRESULT
    GetSharedResourceAdapterLuid: unsafe extern "system" fn(*mut c_void, HANDLE, PLUID) -> HRESULT
    RegisterStereoStatusWindow: unsafe extern "system" fn(*mut c_void, HWND, U32, PUINT) -> HRESULT
    RegisterStereoStatusEvent: unsafe extern "system" fn(*mut c_void, HANDLE, PUINT) -> HRESULT
    UnregisterStereoStatus: unsafe extern "system" fn(*mut c_void, U32)
    RegisterOcclusionStatusWindow: unsafe extern "system" fn(*mut c_void, HWND, U32, PUINT) -> HRESULT
    RegisterOcclusionStatusEvent: unsafe extern "system" fn(*mut c_void, HANDLE, PUINT) -> HRESULT
    UnregisterOcclusionStatus: unsafe extern "system" fn(*mut c_void, U32)
    CreateSwapChainForComposition: unsafe extern "system" fn(*mut c_void, IUnknown, PDXGI_SWAP_CHAIN_DESC1, LPVOID, PIDXGISwapChain1) -> HRESULT
    GetCreationFlags: unsafe extern "system" fn(*mut c_void) -> U32
    EnumAdapterByLuid: unsafe extern "system" fn(*mut c_void, LUID, PGUID, HANDLE) -> HRESULT
    EnumWarpAdapter: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT

export shackle type IDXGIFactory4 = *mut c_void

export shackle type IDXGISwapChain1 = *mut c_void

export shackle struct ID3D12DeviceVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    GetPrivateData: unsafe extern "system" fn(*mut c_void, PGUID, PUINT, HANDLE) -> HRESULT
    SetPrivateData: unsafe extern "system" fn(*mut c_void, PGUID, U32, HANDLE) -> HRESULT
    SetPrivateDataInterface: unsafe extern "system" fn(*mut c_void, PGUID, IUnknown) -> HRESULT
    SetName: unsafe extern "system" fn(*mut c_void, PWSTR) -> HRESULT
    GetNodeCount: unsafe extern "system" fn(*mut c_void) -> U32
    CreateCommandQueue: unsafe extern "system" fn(*mut c_void, PD3D12_COMMAND_QUEUE_DESC, PGUID, HANDLE) -> HRESULT
    CreateCommandAllocator: unsafe extern "system" fn(*mut c_void, D3D12_COMMAND_LIST_TYPE, PGUID, HANDLE) -> HRESULT
    CreateGraphicsPipelineState: unsafe extern "system" fn(*mut c_void, PLPVOID, PGUID, HANDLE) -> HRESULT
    CreateComputePipelineState: unsafe extern "system" fn(*mut c_void, PLPVOID, PGUID, HANDLE) -> HRESULT
    CreateCommandList: unsafe extern "system" fn(*mut c_void, U32, D3D12_COMMAND_LIST_TYPE, ID3D12CommandAllocator, LPVOID, PGUID, HANDLE) -> HRESULT
    CheckFeatureSupport: unsafe extern "system" fn(*mut c_void, I32, HANDLE, U32) -> HRESULT
    CreateDescriptorHeap: unsafe extern "system" fn(*mut c_void, PLPVOID, PGUID, HANDLE) -> HRESULT
    GetDescriptorHandleIncrementSize: unsafe extern "system" fn(*mut c_void, I32) -> U32
    CreateRootSignature: unsafe extern "system" fn(*mut c_void, U32, HANDLE, ULONG_PTR, PGUID, HANDLE) -> HRESULT
    CreateConstantBufferView: unsafe extern "system" fn(*mut c_void, PLPVOID, LPVOID)
    CreateShaderResourceView: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID, LPVOID)
    CreateUnorderedAccessView: unsafe extern "system" fn(*mut c_void, LPVOID, LPVOID, PLPVOID, LPVOID)
    CreateRenderTargetView: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID, LPVOID)
    CreateDepthStencilView: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID, LPVOID)
    CreateSampler: unsafe extern "system" fn(*mut c_void, PLPVOID, LPVOID)
    CopyDescriptors: unsafe extern "system" fn(*mut c_void, U32, PLPVOID, PUINT, U32, PLPVOID, PUINT, I32)
    CopyDescriptorsSimple: unsafe extern "system" fn(*mut c_void, U32, LPVOID, LPVOID, I32)
    GetResourceAllocationInfo: unsafe extern "system" fn(*mut c_void, U32, U32, PLPVOID) -> LPVOID
    GetCustomHeapProperties: unsafe extern "system" fn(*mut c_void, U32, I32) -> LPVOID
    CreateCommittedResource: unsafe extern "system" fn(*mut c_void, PLPVOID, I32, PLPVOID, I32, PLPVOID, PGUID, HANDLE) -> HRESULT
    CreateHeap: unsafe extern "system" fn(*mut c_void, PLPVOID, PGUID, HANDLE) -> HRESULT
    CreatePlacedResource: unsafe extern "system" fn(*mut c_void, LPVOID, U64, PLPVOID, I32, PLPVOID, PGUID, HANDLE) -> HRESULT
    CreateReservedResource: unsafe extern "system" fn(*mut c_void, PLPVOID, I32, PLPVOID, PGUID, HANDLE) -> HRESULT
    CreateSharedHandle: unsafe extern "system" fn(*mut c_void, LPVOID, PSECURITY_ATTRIBUTES, U32, PWSTR, PHANDLE) -> HRESULT
    OpenSharedHandle: unsafe extern "system" fn(*mut c_void, HANDLE, PGUID, HANDLE) -> HRESULT
    OpenSharedHandleByName: unsafe extern "system" fn(*mut c_void, PWSTR, U32, PHANDLE) -> HRESULT
    MakeResident: unsafe extern "system" fn(*mut c_void, U32, PLPVOID) -> HRESULT
    Evict: unsafe extern "system" fn(*mut c_void, U32, PLPVOID) -> HRESULT
    CreateFence: unsafe extern "system" fn(*mut c_void, U64, D3D12_FENCE_FLAGS, PGUID, HANDLE) -> HRESULT
    GetDeviceRemovedReason: unsafe extern "system" fn(*mut c_void) -> HRESULT
    GetCopyableFootprints: unsafe extern "system" fn(*mut c_void, PLPVOID, U32, U32, U64, PLPVOID, PUINT, PU64, PU64)
    CreateQueryHeap: unsafe extern "system" fn(*mut c_void, PLPVOID, PGUID, HANDLE) -> HRESULT
    SetStablePowerState: unsafe extern "system" fn(*mut c_void, BOOL) -> HRESULT
    CreateCommandSignature: unsafe extern "system" fn(*mut c_void, PLPVOID, LPVOID, PGUID, HANDLE) -> HRESULT
    GetResourceTiling: unsafe extern "system" fn(*mut c_void, LPVOID, PUINT, PLPVOID, PLPVOID, PUINT, U32, PLPVOID)
    GetAdapterLuid: unsafe extern "system" fn(*mut c_void) -> LUID

export shackle type ID3D12Device = *mut c_void

export shackle type ID3D12CommandQueue = *mut c_void

export shackle type ID3D12CommandAllocator = *mut c_void

export shackle type ID3D12GraphicsCommandList = *mut c_void

export shackle type ID3D12Fence = *mut c_void

export shackle struct IDWriteFactoryVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    GetSystemFontCollection: unsafe extern "system" fn(*mut c_void, PIDWriteFontCollection, BOOL) -> HRESULT
    CreateCustomFontCollection: unsafe extern "system" fn(*mut c_void, LPVOID, HANDLE, U32, PIDWriteFontCollection) -> HRESULT
    RegisterFontCollectionLoader: unsafe extern "system" fn(*mut c_void, LPVOID) -> HRESULT
    UnregisterFontCollectionLoader: unsafe extern "system" fn(*mut c_void, LPVOID) -> HRESULT
    CreateFontFileReference: unsafe extern "system" fn(*mut c_void, PWSTR, PLPVOID, PLPVOID) -> HRESULT
    CreateCustomFontFileReference: unsafe extern "system" fn(*mut c_void, HANDLE, U32, LPVOID, PLPVOID) -> HRESULT
    CreateFontFace: unsafe extern "system" fn(*mut c_void, I32, U32, PLPVOID, U32, I32, PLPVOID) -> HRESULT
    CreateRenderingParams: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    CreateMonitorRenderingParams: unsafe extern "system" fn(*mut c_void, HMONITOR, PLPVOID) -> HRESULT
    CreateCustomRenderingParams: unsafe extern "system" fn(*mut c_void, FLOAT, FLOAT, FLOAT, I32, I32, PLPVOID) -> HRESULT
    RegisterFontFileLoader: unsafe extern "system" fn(*mut c_void, LPVOID) -> HRESULT
    UnregisterFontFileLoader: unsafe extern "system" fn(*mut c_void, LPVOID) -> HRESULT
    CreateTextFormat: unsafe extern "system" fn(*mut c_void, PWSTR, IDWriteFontCollection, I32, I32, I32, FLOAT, PWSTR, PIDWriteTextFormat) -> HRESULT
    CreateTypography: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    GetGdiInterop: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    CreateTextLayout: unsafe extern "system" fn(*mut c_void, PWSTR, U32, IDWriteTextFormat, FLOAT, FLOAT, PIDWriteTextLayout) -> HRESULT
    CreateGdiCompatibleTextLayout: unsafe extern "system" fn(*mut c_void, PWSTR, U32, IDWriteTextFormat, FLOAT, FLOAT, FLOAT, PLPVOID, BOOL, PIDWriteTextLayout) -> HRESULT
    CreateEllipsisTrimmingSign: unsafe extern "system" fn(*mut c_void, IDWriteTextFormat, PLPVOID) -> HRESULT
    CreateTextAnalyzer: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    CreateNumberSubstitution: unsafe extern "system" fn(*mut c_void, I32, PWSTR, BOOL, PLPVOID) -> HRESULT
    CreateGlyphRunAnalysis: unsafe extern "system" fn(*mut c_void, PLPVOID, FLOAT, PLPVOID, I32, I32, FLOAT, FLOAT, PLPVOID) -> HRESULT

export shackle type IDWriteFactory = *mut c_void

export shackle struct IDWriteFontCollectionVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    GetFontFamilyCount: unsafe extern "system" fn(*mut c_void) -> U32
    GetFontFamily: unsafe extern "system" fn(*mut c_void, U32, PLPVOID) -> HRESULT
    FindFamilyName: unsafe extern "system" fn(*mut c_void, PWSTR, PUINT, PBOOL) -> HRESULT
    GetFontFromFontFace: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID) -> HRESULT

export shackle type IDWriteFontCollection = *mut c_void

export shackle type IDWriteTextFormat = *mut c_void

export shackle type IDWriteTextLayout = *mut c_void

export shackle struct ID2D1Factory1VTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    ReloadSystemMetrics: unsafe extern "system" fn(*mut c_void) -> HRESULT
    GetDesktopDpi: unsafe extern "system" fn(*mut c_void, PFLOAT, PFLOAT)
    CreateRectangleGeometry: unsafe extern "system" fn(*mut c_void, PLPVOID, PLPVOID) -> HRESULT
    CreateRoundedRectangleGeometry: unsafe extern "system" fn(*mut c_void, PLPVOID, PLPVOID) -> HRESULT
    CreateEllipseGeometry: unsafe extern "system" fn(*mut c_void, PLPVOID, PLPVOID) -> HRESULT
    CreateGeometryGroup: unsafe extern "system" fn(*mut c_void, I32, PLPVOID, U32, PLPVOID) -> HRESULT
    CreateTransformedGeometry: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID, PLPVOID) -> HRESULT
    CreatePathGeometry: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    CreateStrokeStyle: unsafe extern "system" fn(*mut c_void, PLPVOID, PFLOAT, U32, PLPVOID) -> HRESULT
    CreateDrawingStateBlock: unsafe extern "system" fn(*mut c_void, PLPVOID, LPVOID, PLPVOID) -> HRESULT
    CreateWicBitmapRenderTarget: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID, PLPVOID) -> HRESULT
    CreateHwndRenderTarget: unsafe extern "system" fn(*mut c_void, PLPVOID, PLPVOID, PLPVOID) -> HRESULT
    CreateDxgiSurfaceRenderTarget: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID, PLPVOID) -> HRESULT
    CreateDCRenderTarget: unsafe extern "system" fn(*mut c_void, PLPVOID, PLPVOID) -> HRESULT
    CreateDevice: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID) -> HRESULT
    CreateStrokeStyle_2: unsafe extern "system" fn(*mut c_void, PLPVOID, PFLOAT, U32, PLPVOID) -> HRESULT
    CreatePathGeometry_2: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    CreateDrawingStateBlock_2: unsafe extern "system" fn(*mut c_void, PLPVOID, LPVOID, PLPVOID) -> HRESULT
    CreateGdiMetafile: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID) -> HRESULT
    RegisterEffectFromStream: unsafe extern "system" fn(*mut c_void, PGUID, LPVOID, PLPVOID, U32, LPVOID) -> HRESULT
    RegisterEffectFromString: unsafe extern "system" fn(*mut c_void, PGUID, PWSTR, PLPVOID, U32, LPVOID) -> HRESULT
    UnregisterEffect: unsafe extern "system" fn(*mut c_void, PGUID) -> HRESULT
    GetRegisteredEffects: unsafe extern "system" fn(*mut c_void, PGUID, U32, PUINT, PUINT) -> HRESULT
    GetEffectProperties: unsafe extern "system" fn(*mut c_void, PGUID, PLPVOID) -> HRESULT

export shackle type ID2D1Factory1 = *mut c_void

export shackle struct IWICImagingFactoryVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    CreateDecoderFromFilename: unsafe extern "system" fn(*mut c_void, PWSTR, PGUID, U32, I32, PLPVOID) -> HRESULT
    CreateDecoderFromStream: unsafe extern "system" fn(*mut c_void, LPVOID, PGUID, I32, PLPVOID) -> HRESULT
    CreateDecoderFromFileHandle: unsafe extern "system" fn(*mut c_void, ULONG_PTR, PGUID, I32, PLPVOID) -> HRESULT
    CreateComponentInfo: unsafe extern "system" fn(*mut c_void, PGUID, PLPVOID) -> HRESULT
    CreateDecoder: unsafe extern "system" fn(*mut c_void, PGUID, PGUID, PLPVOID) -> HRESULT
    CreateEncoder: unsafe extern "system" fn(*mut c_void, PGUID, PGUID, PLPVOID) -> HRESULT
    CreatePalette: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    CreateFormatConverter: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    CreateBitmapScaler: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    CreateBitmapClipper: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    CreateBitmapFlipRotator: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    CreateStream: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    CreateColorContext: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    CreateColorTransformer: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    CreateBitmap: unsafe extern "system" fn(*mut c_void, U32, U32, PGUID, I32, PLPVOID) -> HRESULT
    CreateBitmapFromSource: unsafe extern "system" fn(*mut c_void, LPVOID, I32, PLPVOID) -> HRESULT
    CreateBitmapFromSourceRect: unsafe extern "system" fn(*mut c_void, LPVOID, U32, U32, U32, U32, PLPVOID) -> HRESULT
    CreateBitmapFromMemory: unsafe extern "system" fn(*mut c_void, U32, U32, PGUID, U32, U32, PBYTE, PLPVOID) -> HRESULT
    CreateBitmapFromHBITMAP: unsafe extern "system" fn(*mut c_void, HBITMAP, HANDLE, I32, PLPVOID) -> HRESULT
    CreateBitmapFromHICON: unsafe extern "system" fn(*mut c_void, HICON, PLPVOID) -> HRESULT
    CreateComponentEnumerator: unsafe extern "system" fn(*mut c_void, U32, U32, PLPVOID) -> HRESULT
    CreateFastMetadataEncoderFromDecoder: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID) -> HRESULT
    CreateFastMetadataEncoderFromFrameDecode: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID) -> HRESULT
    CreateQueryWriter: unsafe extern "system" fn(*mut c_void, PGUID, PGUID, PLPVOID) -> HRESULT
    CreateQueryWriterFromReader: unsafe extern "system" fn(*mut c_void, LPVOID, PGUID, PLPVOID) -> HRESULT

export shackle type IWICImagingFactory = *mut c_void

export shackle struct IMMDeviceEnumeratorVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    EnumAudioEndpoints: unsafe extern "system" fn(*mut c_void, EDataFlow, U32, PIMMDeviceCollection) -> HRESULT
    GetDefaultAudioEndpoint: unsafe extern "system" fn(*mut c_void, EDataFlow, ERole, PIMMDevice) -> HRESULT
    GetDevice: unsafe extern "system" fn(*mut c_void, PWSTR, PIMMDevice) -> HRESULT
    RegisterEndpointNotificationCallback: unsafe extern "system" fn(*mut c_void, IMMNotificationClient) -> HRESULT
    UnregisterEndpointNotificationCallback: unsafe extern "system" fn(*mut c_void, IMMNotificationClient) -> HRESULT

export shackle type IMMDeviceEnumerator = *mut c_void

export shackle struct IMMDeviceCollectionVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    GetCount: unsafe extern "system" fn(*mut c_void, PUINT) -> HRESULT
    Item: unsafe extern "system" fn(*mut c_void, U32, PIMMDevice) -> HRESULT

export shackle type IMMDeviceCollection = *mut c_void

export shackle struct IMMDeviceVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    Activate: unsafe extern "system" fn(*mut c_void, PGUID, U32, PPROPVARIANT, HANDLE) -> HRESULT
    OpenPropertyStore: unsafe extern "system" fn(*mut c_void, U32, PLPVOID) -> HRESULT
    GetId: unsafe extern "system" fn(*mut c_void, PPWSTR) -> HRESULT
    GetState: unsafe extern "system" fn(*mut c_void, PUINT) -> HRESULT

export shackle type IMMDevice = *mut c_void

export shackle struct IAudioClientVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    Initialize: unsafe extern "system" fn(*mut c_void, AUDCLNT_SHAREMODE, U32, I64, I64, LPWAVEFORMATEX, PGUID) -> HRESULT
    GetBufferSize: unsafe extern "system" fn(*mut c_void, PUINT) -> HRESULT
    GetStreamLatency: unsafe extern "system" fn(*mut c_void, PI64) -> HRESULT
    GetCurrentPadding: unsafe extern "system" fn(*mut c_void, PUINT) -> HRESULT
    IsFormatSupported: unsafe extern "system" fn(*mut c_void, AUDCLNT_SHAREMODE, LPWAVEFORMATEX, LPWAVEFORMATEX) -> HRESULT
    GetMixFormat: unsafe extern "system" fn(*mut c_void, LPWAVEFORMATEX) -> HRESULT
    GetDevicePeriod: unsafe extern "system" fn(*mut c_void, PI64, PI64) -> HRESULT
    Start: unsafe extern "system" fn(*mut c_void) -> HRESULT
    Stop: unsafe extern "system" fn(*mut c_void) -> HRESULT
    Reset: unsafe extern "system" fn(*mut c_void) -> HRESULT
    SetEventHandle: unsafe extern "system" fn(*mut c_void, HANDLE) -> HRESULT
    GetService: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT

export shackle type IAudioClient = *mut c_void

export shackle struct IAudioClient2VTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    Initialize: unsafe extern "system" fn(*mut c_void, AUDCLNT_SHAREMODE, U32, I64, I64, LPWAVEFORMATEX, PGUID) -> HRESULT
    GetBufferSize: unsafe extern "system" fn(*mut c_void, PUINT) -> HRESULT
    GetStreamLatency: unsafe extern "system" fn(*mut c_void, PI64) -> HRESULT
    GetCurrentPadding: unsafe extern "system" fn(*mut c_void, PUINT) -> HRESULT
    IsFormatSupported: unsafe extern "system" fn(*mut c_void, AUDCLNT_SHAREMODE, LPWAVEFORMATEX, LPWAVEFORMATEX) -> HRESULT
    GetMixFormat: unsafe extern "system" fn(*mut c_void, LPWAVEFORMATEX) -> HRESULT
    GetDevicePeriod: unsafe extern "system" fn(*mut c_void, PI64, PI64) -> HRESULT
    Start: unsafe extern "system" fn(*mut c_void) -> HRESULT
    Stop: unsafe extern "system" fn(*mut c_void) -> HRESULT
    Reset: unsafe extern "system" fn(*mut c_void) -> HRESULT
    SetEventHandle: unsafe extern "system" fn(*mut c_void, HANDLE) -> HRESULT
    GetService: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    IsOffloadCapable: unsafe extern "system" fn(*mut c_void, AUDIO_STREAM_CATEGORY, PBOOL) -> HRESULT
    SetClientProperties: unsafe extern "system" fn(*mut c_void, PAUDIOCLIENT_PROPERTIES) -> HRESULT
    GetBufferSizeLimits: unsafe extern "system" fn(*mut c_void, LPWAVEFORMATEX, BOOL, PI64, PI64) -> HRESULT

export shackle type IAudioClient2 = *mut c_void

export shackle struct IAudioClient3VTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    Initialize: unsafe extern "system" fn(*mut c_void, AUDCLNT_SHAREMODE, U32, I64, I64, LPWAVEFORMATEX, PGUID) -> HRESULT
    GetBufferSize: unsafe extern "system" fn(*mut c_void, PUINT) -> HRESULT
    GetStreamLatency: unsafe extern "system" fn(*mut c_void, PI64) -> HRESULT
    GetCurrentPadding: unsafe extern "system" fn(*mut c_void, PUINT) -> HRESULT
    IsFormatSupported: unsafe extern "system" fn(*mut c_void, AUDCLNT_SHAREMODE, LPWAVEFORMATEX, LPWAVEFORMATEX) -> HRESULT
    GetMixFormat: unsafe extern "system" fn(*mut c_void, LPWAVEFORMATEX) -> HRESULT
    GetDevicePeriod: unsafe extern "system" fn(*mut c_void, PI64, PI64) -> HRESULT
    Start: unsafe extern "system" fn(*mut c_void) -> HRESULT
    Stop: unsafe extern "system" fn(*mut c_void) -> HRESULT
    Reset: unsafe extern "system" fn(*mut c_void) -> HRESULT
    SetEventHandle: unsafe extern "system" fn(*mut c_void, HANDLE) -> HRESULT
    GetService: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    IsOffloadCapable: unsafe extern "system" fn(*mut c_void, AUDIO_STREAM_CATEGORY, PBOOL) -> HRESULT
    SetClientProperties: unsafe extern "system" fn(*mut c_void, PAUDIOCLIENT_PROPERTIES) -> HRESULT
    GetBufferSizeLimits: unsafe extern "system" fn(*mut c_void, LPWAVEFORMATEX, BOOL, PI64, PI64) -> HRESULT
    GetSharedModeEnginePeriod: unsafe extern "system" fn(*mut c_void, LPWAVEFORMATEX, PUINT, PUINT, PUINT, PUINT) -> HRESULT
    GetCurrentSharedModeEnginePeriod: unsafe extern "system" fn(*mut c_void, LPWAVEFORMATEX, PUINT) -> HRESULT
    InitializeSharedAudioStream: unsafe extern "system" fn(*mut c_void, U32, U32, LPWAVEFORMATEX, PGUID) -> HRESULT

export shackle type IAudioClient3 = *mut c_void

export shackle type IAudioRenderClient = *mut c_void

export shackle struct IAudioCaptureClientVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    GetBuffer: unsafe extern "system" fn(*mut c_void, PBYTE, PUINT, PUINT, PU64, PU64) -> HRESULT
    ReleaseBuffer: unsafe extern "system" fn(*mut c_void, U32) -> HRESULT
    GetNextPacketSize: unsafe extern "system" fn(*mut c_void, PUINT) -> HRESULT

export shackle type IAudioCaptureClient = *mut c_void

export shackle struct IAudioClock2VTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    GetDevicePosition: unsafe extern "system" fn(*mut c_void, PU64, PU64) -> HRESULT

export shackle type IAudioClock2 = *mut c_void

export shackle struct IAudioSessionControl2VTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    GetState: unsafe extern "system" fn(*mut c_void, PI32) -> HRESULT
    GetDisplayName: unsafe extern "system" fn(*mut c_void, PPWSTR) -> HRESULT
    SetDisplayName: unsafe extern "system" fn(*mut c_void, PWSTR, PGUID) -> HRESULT
    GetIconPath: unsafe extern "system" fn(*mut c_void, PPWSTR) -> HRESULT
    SetIconPath: unsafe extern "system" fn(*mut c_void, PWSTR, PGUID) -> HRESULT
    GetGroupingParam: unsafe extern "system" fn(*mut c_void, PGUID) -> HRESULT
    SetGroupingParam: unsafe extern "system" fn(*mut c_void, PGUID, PGUID) -> HRESULT
    RegisterAudioSessionNotification: unsafe extern "system" fn(*mut c_void, LPVOID) -> HRESULT
    UnregisterAudioSessionNotification: unsafe extern "system" fn(*mut c_void, LPVOID) -> HRESULT
    GetSessionIdentifier: unsafe extern "system" fn(*mut c_void, PPWSTR) -> HRESULT
    GetSessionInstanceIdentifier: unsafe extern "system" fn(*mut c_void, PPWSTR) -> HRESULT
    GetProcessId: unsafe extern "system" fn(*mut c_void, PUINT) -> HRESULT
    IsSystemSoundsSession: unsafe extern "system" fn(*mut c_void) -> HRESULT
    SetDuckingPreference: unsafe extern "system" fn(*mut c_void, BOOL) -> HRESULT

export shackle type IAudioSessionControl2 = *mut c_void

export shackle struct IAudioSessionManager2VTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    GetAudioSessionControl: unsafe extern "system" fn(*mut c_void, PGUID, U32, PLPVOID) -> HRESULT
    GetSimpleAudioVolume: unsafe extern "system" fn(*mut c_void, PGUID, U32, PLPVOID) -> HRESULT
    GetSessionEnumerator: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    RegisterSessionNotification: unsafe extern "system" fn(*mut c_void, LPVOID) -> HRESULT
    UnregisterSessionNotification: unsafe extern "system" fn(*mut c_void, LPVOID) -> HRESULT
    RegisterDuckNotification: unsafe extern "system" fn(*mut c_void, PWSTR, LPVOID) -> HRESULT
    UnregisterDuckNotification: unsafe extern "system" fn(*mut c_void, LPVOID) -> HRESULT

export shackle type IAudioSessionManager2 = *mut c_void

export shackle struct IAudioEndpointVolumeVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    RegisterControlChangeNotify: unsafe extern "system" fn(*mut c_void, LPVOID) -> HRESULT
    UnregisterControlChangeNotify: unsafe extern "system" fn(*mut c_void, LPVOID) -> HRESULT
    GetChannelCount: unsafe extern "system" fn(*mut c_void, PUINT) -> HRESULT
    SetMasterVolumeLevel: unsafe extern "system" fn(*mut c_void, FLOAT, PGUID) -> HRESULT
    SetMasterVolumeLevelScalar: unsafe extern "system" fn(*mut c_void, FLOAT, PGUID) -> HRESULT
    GetMasterVolumeLevel: unsafe extern "system" fn(*mut c_void, PFLOAT) -> HRESULT
    GetMasterVolumeLevelScalar: unsafe extern "system" fn(*mut c_void, PFLOAT) -> HRESULT
    SetChannelVolumeLevel: unsafe extern "system" fn(*mut c_void, U32, FLOAT, PGUID) -> HRESULT
    SetChannelVolumeLevelScalar: unsafe extern "system" fn(*mut c_void, U32, FLOAT, PGUID) -> HRESULT
    GetChannelVolumeLevel: unsafe extern "system" fn(*mut c_void, U32, PFLOAT) -> HRESULT
    GetChannelVolumeLevelScalar: unsafe extern "system" fn(*mut c_void, U32, PFLOAT) -> HRESULT
    SetMute: unsafe extern "system" fn(*mut c_void, BOOL, PGUID) -> HRESULT
    GetMute: unsafe extern "system" fn(*mut c_void, PBOOL) -> HRESULT
    GetVolumeStepInfo: unsafe extern "system" fn(*mut c_void, PUINT, PUINT) -> HRESULT
    VolumeStepUp: unsafe extern "system" fn(*mut c_void, PGUID) -> HRESULT
    VolumeStepDown: unsafe extern "system" fn(*mut c_void, PGUID) -> HRESULT
    QueryHardwareSupport: unsafe extern "system" fn(*mut c_void, PUINT) -> HRESULT
    GetVolumeRange: unsafe extern "system" fn(*mut c_void, PFLOAT, PFLOAT, PFLOAT) -> HRESULT

export shackle type IAudioEndpointVolume = *mut c_void

export shackle struct IMMNotificationClientVTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    OnDeviceStateChanged: unsafe extern "system" fn(*mut c_void, PWSTR, U32) -> HRESULT
    OnDeviceAdded: unsafe extern "system" fn(*mut c_void, PWSTR) -> HRESULT
    OnDeviceRemoved: unsafe extern "system" fn(*mut c_void, PWSTR) -> HRESULT
    OnDefaultDeviceChanged: unsafe extern "system" fn(*mut c_void, EDataFlow, ERole, PWSTR) -> HRESULT
    OnPropertyValueChanged: unsafe extern "system" fn(*mut c_void, PWSTR, PROPERTYKEY) -> HRESULT

export shackle type IMMNotificationClient = *mut c_void

export shackle struct IXAudio2VTable:
    QueryInterface: unsafe extern "system" fn(*mut c_void, PGUID, HANDLE) -> HRESULT
    AddRef: unsafe extern "system" fn(*mut c_void) -> U32
    Release: unsafe extern "system" fn(*mut c_void) -> U32
    RegisterForCallbacks: unsafe extern "system" fn(*mut c_void, IXAudio2EngineCallback) -> HRESULT
    UnregisterForCallbacks: unsafe extern "system" fn(*mut c_void, IXAudio2EngineCallback)
    CreateSourceVoice: unsafe extern "system" fn(*mut c_void, PIXAudio2SourceVoice, LPWAVEFORMATEX, U32, FLOAT, IXAudio2VoiceCallback, PLPVOID, PLPVOID) -> HRESULT
    CreateSubmixVoice: unsafe extern "system" fn(*mut c_void, PIXAudio2SubmixVoice, U32, U32, U32, U32, PLPVOID, PLPVOID) -> HRESULT
    CreateMasteringVoice: unsafe extern "system" fn(*mut c_void, PIXAudio2MasteringVoice, U32, U32, U32, PWSTR, PLPVOID, AUDIO_STREAM_CATEGORY) -> HRESULT
    StartEngine: unsafe extern "system" fn(*mut c_void) -> HRESULT
    StopEngine: unsafe extern "system" fn(*mut c_void)
    CommitChanges: unsafe extern "system" fn(*mut c_void, U32) -> HRESULT
    GetPerformanceData: unsafe extern "system" fn(*mut c_void, PLPVOID)
    SetDebugConfiguration: unsafe extern "system" fn(*mut c_void, PLPVOID, HANDLE)

export shackle type IXAudio2 = *mut c_void

export shackle struct IXAudio2EngineCallbackVTable:
    OnProcessingPassStart: unsafe extern "system" fn(*mut c_void)
    OnProcessingPassEnd: unsafe extern "system" fn(*mut c_void)
    OnCriticalError: unsafe extern "system" fn(*mut c_void, HRESULT)

export shackle type IXAudio2EngineCallback = *mut c_void

export shackle type IXAudio2MasteringVoice = *mut c_void

export shackle struct IXAudio2VoiceVTable:
    GetVoiceDetails: unsafe extern "system" fn(*mut c_void, PLPVOID)
    SetOutputVoices: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    SetEffectChain: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    EnableEffect: unsafe extern "system" fn(*mut c_void, U32, U32) -> HRESULT
    DisableEffect: unsafe extern "system" fn(*mut c_void, U32, U32) -> HRESULT
    GetEffectState: unsafe extern "system" fn(*mut c_void, U32, PBOOL)
    SetEffectParameters: unsafe extern "system" fn(*mut c_void, U32, HANDLE, U32, U32) -> HRESULT
    GetEffectParameters: unsafe extern "system" fn(*mut c_void, U32, HANDLE, U32) -> HRESULT
    SetFilterParameters: unsafe extern "system" fn(*mut c_void, PLPVOID, U32) -> HRESULT
    GetFilterParameters: unsafe extern "system" fn(*mut c_void, PLPVOID)
    SetOutputFilterParameters: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID, U32) -> HRESULT
    GetOutputFilterParameters: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID)
    SetVolume: unsafe extern "system" fn(*mut c_void, FLOAT, U32) -> HRESULT
    GetVolume: unsafe extern "system" fn(*mut c_void, PFLOAT)
    SetChannelVolumes: unsafe extern "system" fn(*mut c_void, U32, PFLOAT, U32) -> HRESULT
    GetChannelVolumes: unsafe extern "system" fn(*mut c_void, U32, PFLOAT)
    SetOutputMatrix: unsafe extern "system" fn(*mut c_void, LPVOID, U32, U32, PFLOAT, U32) -> HRESULT
    GetOutputMatrix: unsafe extern "system" fn(*mut c_void, LPVOID, U32, U32, PFLOAT)
    DestroyVoice: unsafe extern "system" fn(*mut c_void)

export shackle struct IXAudio2VoiceCallbackVTable:
    OnVoiceProcessingPassStart: unsafe extern "system" fn(*mut c_void, U32)
    OnVoiceProcessingPassEnd: unsafe extern "system" fn(*mut c_void)
    OnStreamEnd: unsafe extern "system" fn(*mut c_void)
    OnBufferStart: unsafe extern "system" fn(*mut c_void, HANDLE)
    OnBufferEnd: unsafe extern "system" fn(*mut c_void, HANDLE)
    OnLoopEnd: unsafe extern "system" fn(*mut c_void, HANDLE)
    OnVoiceError: unsafe extern "system" fn(*mut c_void, HANDLE, HRESULT)

export shackle type IXAudio2VoiceCallback = *mut c_void

export shackle struct IXAudio2SourceVoiceVTable:
    GetVoiceDetails: unsafe extern "system" fn(*mut c_void, PLPVOID)
    SetOutputVoices: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    SetEffectChain: unsafe extern "system" fn(*mut c_void, PLPVOID) -> HRESULT
    EnableEffect: unsafe extern "system" fn(*mut c_void, U32, U32) -> HRESULT
    DisableEffect: unsafe extern "system" fn(*mut c_void, U32, U32) -> HRESULT
    GetEffectState: unsafe extern "system" fn(*mut c_void, U32, PBOOL)
    SetEffectParameters: unsafe extern "system" fn(*mut c_void, U32, HANDLE, U32, U32) -> HRESULT
    GetEffectParameters: unsafe extern "system" fn(*mut c_void, U32, HANDLE, U32) -> HRESULT
    SetFilterParameters: unsafe extern "system" fn(*mut c_void, PLPVOID, U32) -> HRESULT
    GetFilterParameters: unsafe extern "system" fn(*mut c_void, PLPVOID)
    SetOutputFilterParameters: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID, U32) -> HRESULT
    GetOutputFilterParameters: unsafe extern "system" fn(*mut c_void, LPVOID, PLPVOID)
    SetVolume: unsafe extern "system" fn(*mut c_void, FLOAT, U32) -> HRESULT
    GetVolume: unsafe extern "system" fn(*mut c_void, PFLOAT)
    SetChannelVolumes: unsafe extern "system" fn(*mut c_void, U32, PFLOAT, U32) -> HRESULT
    GetChannelVolumes: unsafe extern "system" fn(*mut c_void, U32, PFLOAT)
    SetOutputMatrix: unsafe extern "system" fn(*mut c_void, LPVOID, U32, U32, PFLOAT, U32) -> HRESULT
    GetOutputMatrix: unsafe extern "system" fn(*mut c_void, LPVOID, U32, U32, PFLOAT)
    DestroyVoice: unsafe extern "system" fn(*mut c_void)
    Start: unsafe extern "system" fn(*mut c_void, U32, U32) -> HRESULT
    Stop: unsafe extern "system" fn(*mut c_void, U32, U32) -> HRESULT
    SubmitSourceBuffer: unsafe extern "system" fn(*mut c_void, PLPVOID, PLPVOID) -> HRESULT
    FlushSourceBuffers: unsafe extern "system" fn(*mut c_void) -> HRESULT
    Discontinuity: unsafe extern "system" fn(*mut c_void) -> HRESULT
    ExitLoop: unsafe extern "system" fn(*mut c_void, U32) -> HRESULT
    GetState: unsafe extern "system" fn(*mut c_void, PLPVOID, U32)
    SetFrequencyRatio: unsafe extern "system" fn(*mut c_void, FLOAT, U32) -> HRESULT
    GetFrequencyRatio: unsafe extern "system" fn(*mut c_void, PFLOAT)
    SetSourceSampleRate: unsafe extern "system" fn(*mut c_void, U32) -> HRESULT

export shackle type IXAudio2SourceVoice = *mut c_void

export shackle type IXAudio2SubmixVoice = *mut c_void
