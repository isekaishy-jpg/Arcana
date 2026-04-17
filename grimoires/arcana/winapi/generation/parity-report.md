# arcana_winapi raw parity report

Source authority: Pinned Windows SDK metadata snapshot
Metadata authority: Windows.Win32.winmd Microsoft.Windows.SDK.Win32Metadata 63.0.31 sha256:97D24CF1A9DC3E50782BBF1DBA0952BF6A025FA583D8B3AE6C5EF713B463C869
Parity target: windows-sys; pinned metadata wins on disagreement.
Upstream parity source: windows-sys under cargo registry windows-sys-0.61.2
Projection config: grimoires/arcana/winapi/generation/projection.toml

## callbacks
kind: callbacks
- projected callbacks: 5

## constants
kind: constants
- metadata-backed constants: 46

## gdi32
kind: imports
- strategy: namespace-driven broad discovery
- binding library: gdi32
- matched import libraries: gdi32
- namespace prefixes: Windows.Win32.Graphics.Gdi
- metadata candidates: 300
- emitted declarations: 300
- windows-sys parity scope: compared
- windows-sys parity: exact symbol parity across 300 matched declarations
- excluded by config: 0
- skipped by skiplist: 0

## dwmapi
kind: imports
- strategy: namespace-driven broad discovery
- binding library: dwmapi
- matched import libraries: dwmapi
- namespace prefixes: Windows.Win32.Graphics.Dwm
- metadata candidates: 31
- emitted declarations: 31
- windows-sys parity scope: compared
- windows-sys parity: exact symbol parity across 31 matched declarations
- excluded by config: 0
- skipped by skiplist: 0

## shcore
kind: imports
- strategy: namespace-driven broad discovery
- binding library: shcore
- matched import libraries: api-ms-win-shcore-scaling-l1-1-1, shcore
- namespace prefixes: Windows.Win32.UI.HiDpi
- metadata candidates: 3
- emitted declarations: 3
- windows-sys parity scope: compared
- windows-sys parity: exact symbol parity across 3 matched declarations
- excluded by config: 0
- skipped by skiplist: 0

## shell32
kind: imports
- strategy: namespace-driven broad discovery
- binding library: shell32
- matched import libraries: shell32
- namespace prefixes: Windows.Win32.UI.Shell
- metadata candidates: 244
- emitted declarations: 243
- windows-sys parity scope: compared
- windows-sys parity: exact symbol parity across 243 matched declarations
- excluded by config: 0
- skipped by skiplist: 1
- skipped metadata ids: Windows.Win32.UI.Shell.FileIconInit

## imm32
kind: imports
- strategy: namespace-driven broad discovery
- binding library: imm32
- matched import libraries: imm32
- namespace prefixes: Windows.Win32.UI.Input.Ime
- metadata candidates: 82
- emitted declarations: 82
- windows-sys parity scope: compared
- windows-sys parity: exact symbol parity across 82 matched declarations
- excluded by config: 0
- skipped by skiplist: 0

## kernel32
kind: imports
- strategy: namespace-driven broad discovery
- binding library: kernel32
- matched import libraries: kernel32
- namespace prefixes: Windows.Win32.Foundation, Windows.Win32.System.LibraryLoader, Windows.Win32.System.Memory, Windows.Win32.System.Threading
- metadata candidates: 411
- emitted declarations: 405
- windows-sys parity scope: compared
- windows-sys parity: exact symbol parity across 405 matched declarations
- excluded by config: 6
- skipped by skiplist: 0
- excluded symbols: GetMachineTypeAttributes, GetNumaNodeProcessorMask2, GetProcessDefaultCpuSetMasks, GetThreadSelectedCpuSetMasks, SetProcessDefaultCpuSetMasks, SetThreadSelectedCpuSetMasks

## ole32
kind: imports
- strategy: namespace-driven broad discovery
- binding library: ole32
- matched import libraries: ole32
- namespace prefixes: Windows.Win32.System.Com, Windows.Win32.System.Ole
- symbol prefixes: Co, Ole
- metadata candidates: 307
- emitted declarations: 146
- windows-sys parity scope: compared
- windows-sys parity: exact symbol parity across 146 matched declarations
- windows-sys classified divergences: 6
- excluded by config: 161
- skipped by skiplist: 0
- excluded symbols: BindMoniker, CLIPFORMAT_UserFree, CLIPFORMAT_UserFree64, CLIPFORMAT_UserMarshal, CLIPFORMAT_UserMarshal64, CLIPFORMAT_UserSize, CLIPFORMAT_UserSize64, CLIPFORMAT_UserUnmarshal, CLIPFORMAT_UserUnmarshal64, CLSIDFromProgID, CLSIDFromProgIDEx, CLSIDFromString, CoRegisterDeviceCatalog, CoRevokeDeviceCatalog, CreateAntiMoniker, CreateBindCtx, CreateClassMoniker, CreateDataAdviseHolder, CreateDataCache, CreateFileMoniker, CreateGenericComposite, CreateILockBytesOnHGlobal, CreateItemMoniker, CreateObjrefMoniker, CreateOleAdviseHolder, CreatePointerMoniker, CreateStdProgressIndicator, CreateStreamOnHGlobal, DcomChannelSetHResult, DoDragDrop, FmtIdToPropStgName, FreePropVariantArray, GetClassFile, GetConvertStg, GetHGlobalFromILockBytes, GetHGlobalFromStream, GetRunningObjectTable, HACCEL_UserFree, HACCEL_UserFree64, HACCEL_UserMarshal, HACCEL_UserMarshal64, HACCEL_UserSize, HACCEL_UserSize64, HACCEL_UserUnmarshal, HACCEL_UserUnmarshal64, HBITMAP_UserFree, HBITMAP_UserFree64, HBITMAP_UserMarshal, HBITMAP_UserMarshal64, HBITMAP_UserSize, HBITMAP_UserSize64, HBITMAP_UserUnmarshal, HBITMAP_UserUnmarshal64, HDC_UserFree, HDC_UserFree64, HDC_UserMarshal, HDC_UserMarshal64, HDC_UserSize, HDC_UserSize64, HDC_UserUnmarshal, HDC_UserUnmarshal64, HGLOBAL_UserFree, HGLOBAL_UserFree64, HGLOBAL_UserMarshal, HGLOBAL_UserMarshal64, HGLOBAL_UserSize, HGLOBAL_UserSize64, HGLOBAL_UserUnmarshal, HGLOBAL_UserUnmarshal64, HICON_UserFree, HICON_UserFree64, HICON_UserMarshal, HICON_UserMarshal64, HICON_UserSize, HICON_UserSize64, HICON_UserUnmarshal, HICON_UserUnmarshal64, HMENU_UserFree, HMENU_UserFree64, HMENU_UserMarshal, HMENU_UserMarshal64, HMENU_UserSize, HMENU_UserSize64, HMENU_UserUnmarshal, HMENU_UserUnmarshal64, HPALETTE_UserFree, HPALETTE_UserFree64, HPALETTE_UserMarshal, HPALETTE_UserMarshal64, HPALETTE_UserSize, HPALETTE_UserSize64, HPALETTE_UserUnmarshal, HPALETTE_UserUnmarshal64, HRGN_UserFree, HRGN_UserMarshal, HRGN_UserSize, HRGN_UserUnmarshal, HWND_UserFree, HWND_UserFree64, HWND_UserMarshal, HWND_UserMarshal64, HWND_UserSize, HWND_UserSize64, HWND_UserUnmarshal, HWND_UserUnmarshal64, IIDFromString, IsAccelerator, MkParseDisplayName, MonikerCommonPrefixWith, MonikerRelativePathTo, ProgIDFromCLSID, PropStgNameToFmtId, PropVariantClear, PropVariantCopy, ReadClassStg, ReadClassStm, ReadFmtUserTypeStg, RegisterDragDrop, ReleaseStgMedium, RevokeDragDrop, SNB_UserFree, SNB_UserFree64, SNB_UserMarshal, SNB_UserMarshal64, SNB_UserSize, SNB_UserSize64, SNB_UserUnmarshal, SNB_UserUnmarshal64, STGMEDIUM_UserFree, STGMEDIUM_UserFree64, STGMEDIUM_UserMarshal, STGMEDIUM_UserMarshal64, STGMEDIUM_UserSize, STGMEDIUM_UserSize64, STGMEDIUM_UserUnmarshal, STGMEDIUM_UserUnmarshal64, SetConvertStg, StgConvertPropertyToVariant, StgConvertVariantToProperty, StgCreateDocfile, StgCreateDocfileOnILockBytes, StgCreatePropSetStg, StgCreatePropStg, StgCreateStorageEx, StgGetIFillLockBytesOnFile, StgGetIFillLockBytesOnILockBytes, StgIsStorageFile, StgIsStorageILockBytes, StgOpenAsyncDocfileOnIFillLockBytes, StgOpenPropStg, StgOpenStorage, StgOpenStorageEx, StgOpenStorageOnILockBytes, StgPropertyLengthAsVariant, StgSetTimes, StringFromCLSID, StringFromGUID2, StringFromIID, WriteClassStg, WriteClassStm, WriteFmtUserTypeStg

## combase
kind: imports
- strategy: namespace-driven broad discovery
- binding library: ole32
- matched import libraries: ole32
- namespace prefixes: Windows.Win32.System.Com
- symbol prefixes: CLSID, IID, ProgID, StringFrom
- metadata candidates: 245
- emitted declarations: 8
- windows-sys parity scope: compared
- windows-sys parity: exact symbol parity across 8 matched declarations
- excluded by config: 237
- skipped by skiplist: 0
- excluded symbols: BindMoniker, CLIPFORMAT_UserFree, CLIPFORMAT_UserFree64, CLIPFORMAT_UserMarshal, CLIPFORMAT_UserMarshal64, CLIPFORMAT_UserSize, CLIPFORMAT_UserSize64, CLIPFORMAT_UserUnmarshal, CLIPFORMAT_UserUnmarshal64, CoAddRefServerProcess, CoAllowSetForegroundWindow, CoAllowUnmarshalerCLSID, CoBuildVersion, CoCancelCall, CoCopyProxy, CoCreateFreeThreadedMarshaler, CoCreateGuid, CoCreateInstance, CoCreateInstanceEx, CoCreateInstanceFromApp, CoDecrementMTAUsage, CoDisableCallCancellation, CoDisconnectContext, CoDisconnectObject, CoDosDateTimeToFileTime, CoEnableCallCancellation, CoFileTimeNow, CoFileTimeToDosDateTime, CoFreeAllLibraries, CoFreeLibrary, CoFreeUnusedLibraries, CoFreeUnusedLibrariesEx, CoGetApartmentType, CoGetCallContext, CoGetCallerTID, CoGetCancelObject, CoGetClassObject, CoGetContextToken, CoGetCurrentLogicalThreadId, CoGetCurrentProcess, CoGetInstanceFromFile, CoGetInstanceFromIStorage, CoGetInterceptor, CoGetInterceptorFromTypeInfo, CoGetInterfaceAndReleaseStream, CoGetMalloc, CoGetMarshalSizeMax, CoGetObject, CoGetObjectContext, CoGetPSClsid, CoGetStandardMarshal, CoGetStdMarshalEx, CoGetSystemSecurityPermissions, CoGetTreatAsClass, CoImpersonateClient, CoIncrementMTAUsage, CoInitialize, CoInitializeEx, CoInitializeSecurity, CoInstall, CoInvalidateRemoteMachineBindings, CoIsHandlerConnected, CoIsOle1Class, CoLoadLibrary, CoLockObjectExternal, CoMarshalHresult, CoMarshalInterThreadInterfaceInStream, CoMarshalInterface, CoQueryAuthenticationServices, CoQueryClientBlanket, CoQueryProxyBlanket, CoRegisterActivationFilter, CoRegisterChannelHook, CoRegisterClassObject, CoRegisterDeviceCatalog, CoRegisterInitializeSpy, CoRegisterMallocSpy, CoRegisterPSClsid, CoRegisterSurrogate, CoReleaseMarshalData, CoReleaseServerProcess, CoResumeClassObjects, CoRevertToSelf, CoRevokeClassObject, CoRevokeDeviceCatalog, CoRevokeInitializeSpy, CoRevokeMallocSpy, CoSetCancelObject, CoSetProxyBlanket, CoSuspendClassObjects, CoSwitchCallContext, CoTaskMemAlloc, CoTaskMemFree, CoTaskMemRealloc, CoTestCancel, CoTreatAsClass, CoUninitialize, CoUnmarshalHresult, CoUnmarshalInterface, CoWaitForMultipleHandles, CoWaitForMultipleObjects, CreateAntiMoniker, CreateBindCtx, CreateClassMoniker, CreateDataAdviseHolder, CreateDataCache, CreateFileMoniker, CreateGenericComposite, CreateILockBytesOnHGlobal, CreateItemMoniker, CreateObjrefMoniker, CreatePointerMoniker, CreateStdProgressIndicator, CreateStreamOnHGlobal, DcomChannelSetHResult, FmtIdToPropStgName, FreePropVariantArray, GetClassFile, GetConvertStg, GetHGlobalFromILockBytes, GetHGlobalFromStream, GetRunningObjectTable, HACCEL_UserFree, HACCEL_UserFree64, HACCEL_UserMarshal, HACCEL_UserMarshal64, HACCEL_UserSize, HACCEL_UserSize64, HACCEL_UserUnmarshal, HACCEL_UserUnmarshal64, HBITMAP_UserFree, HBITMAP_UserFree64, HBITMAP_UserMarshal, HBITMAP_UserMarshal64, HBITMAP_UserSize, HBITMAP_UserSize64, HBITMAP_UserUnmarshal, HBITMAP_UserUnmarshal64, HDC_UserFree, HDC_UserFree64, HDC_UserMarshal, HDC_UserMarshal64, HDC_UserSize, HDC_UserSize64, HDC_UserUnmarshal, HDC_UserUnmarshal64, HGLOBAL_UserFree, HGLOBAL_UserFree64, HGLOBAL_UserMarshal, HGLOBAL_UserMarshal64, HGLOBAL_UserSize, HGLOBAL_UserSize64, HGLOBAL_UserUnmarshal, HGLOBAL_UserUnmarshal64, HICON_UserFree, HICON_UserFree64, HICON_UserMarshal, HICON_UserMarshal64, HICON_UserSize, HICON_UserSize64, HICON_UserUnmarshal, HICON_UserUnmarshal64, HMENU_UserFree, HMENU_UserFree64, HMENU_UserMarshal, HMENU_UserMarshal64, HMENU_UserSize, HMENU_UserSize64, HMENU_UserUnmarshal, HMENU_UserUnmarshal64, HPALETTE_UserFree, HPALETTE_UserFree64, HPALETTE_UserMarshal, HPALETTE_UserMarshal64, HPALETTE_UserSize, HPALETTE_UserSize64, HPALETTE_UserUnmarshal, HPALETTE_UserUnmarshal64, HWND_UserFree, HWND_UserFree64, HWND_UserMarshal, HWND_UserMarshal64, HWND_UserSize, HWND_UserSize64, HWND_UserUnmarshal, HWND_UserUnmarshal64, MkParseDisplayName, MonikerCommonPrefixWith, MonikerRelativePathTo, OleConvertIStorageToOLESTREAM, OleConvertIStorageToOLESTREAMEx, OleConvertOLESTREAMToIStorage, OleConvertOLESTREAMToIStorageEx, PropStgNameToFmtId, PropVariantClear, PropVariantCopy, ReadClassStg, ReadClassStm, ReadFmtUserTypeStg, SNB_UserFree, SNB_UserFree64, SNB_UserMarshal, SNB_UserMarshal64, SNB_UserSize, SNB_UserSize64, SNB_UserUnmarshal, SNB_UserUnmarshal64, STGMEDIUM_UserFree, STGMEDIUM_UserFree64, STGMEDIUM_UserMarshal, STGMEDIUM_UserMarshal64, STGMEDIUM_UserSize, STGMEDIUM_UserSize64, STGMEDIUM_UserUnmarshal, STGMEDIUM_UserUnmarshal64, SetConvertStg, StgConvertPropertyToVariant, StgConvertVariantToProperty, StgCreateDocfile, StgCreateDocfileOnILockBytes, StgCreatePropSetStg, StgCreatePropStg, StgCreateStorageEx, StgGetIFillLockBytesOnFile, StgGetIFillLockBytesOnILockBytes, StgIsStorageFile, StgIsStorageILockBytes, StgOpenAsyncDocfileOnIFillLockBytes, StgOpenPropStg, StgOpenStorage, StgOpenStorageEx, StgOpenStorageOnILockBytes, StgPropertyLengthAsVariant, StgSetTimes, WriteClassStg, WriteClassStm, WriteFmtUserTypeStg

## dxgi
kind: imports
- strategy: namespace-driven broad discovery
- binding library: dxgi
- matched import libraries: dxgi
- namespace prefixes: Windows.Win32.Graphics.Dxgi
- metadata candidates: 6
- emitted declarations: 5
- windows-sys parity scope: skipped
- windows-sys parity skip reason: The pinned windows-sys 0.61.2 source tree does not publish a Windows.Win32.Graphics.Dxgi module subtree to compare against.
- excluded by config: 1
- skipped by skiplist: 0
- excluded symbols: DXGIDisableVBlankVirtualization

## d3d12
kind: imports
- strategy: namespace-driven broad discovery
- binding library: d3d12
- matched import libraries: d3d12
- namespace prefixes: Windows.Win32.Graphics.Direct3D12
- metadata candidates: 8
- emitted declarations: 8
- windows-sys parity scope: skipped
- windows-sys parity skip reason: The pinned windows-sys 0.61.2 source tree does not publish a Windows.Win32.Graphics.Direct3D12 module subtree to compare against.
- excluded by config: 0
- skipped by skiplist: 0

## dwrite
kind: imports
- strategy: namespace-driven broad discovery
- binding library: dwrite
- matched import libraries: dwrite
- namespace prefixes: Windows.Win32.Graphics.DirectWrite
- metadata candidates: 1
- emitted declarations: 1
- windows-sys parity scope: skipped
- windows-sys parity skip reason: The pinned windows-sys 0.61.2 source tree does not publish a Windows.Win32.Graphics.DirectWrite module subtree to compare against.
- excluded by config: 0
- skipped by skiplist: 0

## d2d1
kind: imports
- strategy: namespace-driven broad discovery
- binding library: d2d1
- matched import libraries: d2d1
- namespace prefixes: Windows.Win32.Graphics.Direct2D
- metadata candidates: 13
- emitted declarations: 13
- windows-sys parity scope: skipped
- windows-sys parity skip reason: The pinned windows-sys 0.61.2 source tree does not publish a Windows.Win32.Graphics.Direct2D module subtree to compare against.
- excluded by config: 0
- skipped by skiplist: 0

## wic
kind: imports
- strategy: namespace-driven broad discovery
- binding library: windowscodecs
- matched import libraries: windowscodecs
- namespace prefixes: Windows.Win32.Graphics.Imaging
- metadata candidates: 9
- emitted declarations: 9
- windows-sys parity scope: skipped
- windows-sys parity skip reason: The pinned windows-sys 0.61.2 source tree does not publish a Windows.Win32.Graphics.Imaging module subtree to compare against.
- excluded by config: 0
- skipped by skiplist: 0

## mmdeviceapi
kind: constants
- metadata-backed constants: 5

## audioclient
kind: constants
- metadata-backed constants: 13

## audiopolicy
kind: constants
- metadata-backed constants: 10

## endpointvolume
kind: constants
- metadata-backed constants: 3

## avrt
kind: imports
- strategy: namespace-driven broad discovery
- binding library: avrt
- matched import libraries: avrt
- namespace prefixes: Windows.Win32.System.Threading
- symbol prefixes: Av
- metadata candidates: 14
- emitted declarations: 14
- windows-sys parity scope: compared
- windows-sys parity: exact symbol parity across 14 matched declarations
- excluded by config: 0
- skipped by skiplist: 0

## mmreg
kind: constants
- metadata-backed constants: 8

## ksmedia
kind: constants
- metadata-backed constants: 8

## propsys
kind: imports
- strategy: namespace-driven broad discovery
- binding library: propsys
- matched import libraries: propsys
- namespace prefixes: Windows.Win32.UI.Shell.PropertiesSystem
- metadata candidates: 65
- emitted declarations: 65
- windows-sys parity scope: compared
- windows-sys parity: exact symbol parity across 65 matched declarations
- excluded by config: 0
- skipped by skiplist: 0

## xaudio2
kind: imports
- strategy: namespace-driven broad discovery
- binding library: xaudio2
- matched import libraries: xaudio2, xaudio2_9, xaudio2_8
- namespace prefixes: Windows.Win32.Media.Audio.XAudio2
- symbol prefixes: XAudio2
- metadata candidates: 4
- emitted declarations: 1
- windows-sys parity scope: skipped
- windows-sys parity skip reason: The pinned windows-sys 0.61.2 source tree does not publish a Windows.Win32.Media.Audio.XAudio2 module subtree to compare against.
- excluded by config: 3
- skipped by skiplist: 0
- excluded symbols: CreateAudioReverb, CreateAudioVolumeMeter, CreateFX

## x3daudio
kind: raw-shim
- exception `x3daudio_initialize_dynload` for `X3DAudioInitialize` via shackle-private

## types
kind: types
- aliases: 85
- array aliases: 4
- manual records: 3
- projected metadata types: 33
- nested projected records: 1
- interface projections: 35

## user32
kind: imports
- strategy: namespace-driven broad discovery
- binding library: user32
- matched import libraries: user32
- namespace prefixes: Windows.Win32.Graphics.Gdi, Windows.Win32.System.DataExchange, Windows.Win32.UI.HiDpi, Windows.Win32.UI.WindowsAndMessaging
- metadata candidates: 549
- emitted declarations: 547
- windows-sys parity scope: compared
- windows-sys parity: exact symbol parity across 547 matched declarations
- excluded by config: 2
- skipped by skiplist: 0
- excluded symbols: RegisterForTooltipDismissNotification, SetAdditionalForegroundBoostProcesses
