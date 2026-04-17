// GENERATED FILE. DO NOT EDIT BY HAND.
// Source of truth: grimoires/arcana/winapi/generation/imports.toml
// Projection config: grimoires/arcana/winapi/generation/projection.toml
// Source authority: Pinned Windows SDK metadata snapshot
// Metadata authority: Windows.Win32.winmd Microsoft.Windows.SDK.Win32Metadata 63.0.31 sha256:97D24CF1A9DC3E50782BBF1DBA0952BF6A025FA583D8B3AE6C5EF713B463C869
// Parity target: windows-sys; pinned metadata wins on disagreement.

export shackle import fn DwmAttachMilContent(hwnd: arcana_winapi.raw.types.HWND) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmAttachMilContent
export shackle import fn DwmDefWindowProc(h_wnd: arcana_winapi.raw.types.HWND, msg: arcana_winapi.raw.types.U32, w_param: arcana_winapi.raw.types.WPARAM, l_param: arcana_winapi.raw.types.LPARAM, pl_result: arcana_winapi.raw.types.PLRESULT) -> arcana_winapi.raw.types.BOOL = dwmapi.DwmDefWindowProc
export shackle import fn DwmDetachMilContent(hwnd: arcana_winapi.raw.types.HWND) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmDetachMilContent
export shackle import fn DwmEnableBlurBehindWindow(h_wnd: arcana_winapi.raw.types.HWND, p_blur_behind: arcana_winapi.raw.types.PLPVOID) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmEnableBlurBehindWindow
export shackle import fn DwmEnableComposition(u_composition_action: arcana_winapi.raw.types.U32) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmEnableComposition
export shackle import fn DwmEnableMMCSS(f_enable_mmcss: arcana_winapi.raw.types.BOOL) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmEnableMMCSS
export shackle import fn DwmExtendFrameIntoClientArea(h_wnd: arcana_winapi.raw.types.HWND, p_mar_inset: arcana_winapi.raw.types.PLPVOID) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmExtendFrameIntoClientArea
export shackle import fn DwmFlush() -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmFlush
export shackle import fn DwmGetColorizationColor(pcr_colorization: arcana_winapi.raw.types.PUINT, pf_opaque_blend: arcana_winapi.raw.types.PBOOL) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmGetColorizationColor
export shackle import fn DwmGetCompositionTimingInfo(hwnd: arcana_winapi.raw.types.HWND, p_timing_info: arcana_winapi.raw.types.PLPVOID) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmGetCompositionTimingInfo
export shackle import fn DwmGetGraphicsStreamClient(u_index: arcana_winapi.raw.types.U32, p_client_uuid: arcana_winapi.raw.types.PGUID) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmGetGraphicsStreamClient
export shackle import fn DwmGetGraphicsStreamTransformHint(u_index: arcana_winapi.raw.types.U32, p_transform: arcana_winapi.raw.types.PLPVOID) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmGetGraphicsStreamTransformHint
export shackle import fn DwmGetTransportAttributes(pf_is_remoting: arcana_winapi.raw.types.PBOOL, pf_is_connected: arcana_winapi.raw.types.PBOOL, p_dw_generation: arcana_winapi.raw.types.PUINT) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmGetTransportAttributes
export shackle import fn DwmGetUnmetTabRequirements(app_window: arcana_winapi.raw.types.HWND, value: arcana_winapi.raw.types.PI32) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmGetUnmetTabRequirements
export shackle import fn DwmGetWindowAttribute(hwnd: arcana_winapi.raw.types.HWND, dw_attribute: arcana_winapi.raw.types.U32, pv_attribute: arcana_winapi.raw.types.HANDLE, cb_attribute: arcana_winapi.raw.types.U32) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmGetWindowAttribute
export shackle import fn DwmInvalidateIconicBitmaps(hwnd: arcana_winapi.raw.types.HWND) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmInvalidateIconicBitmaps
export shackle import fn DwmIsCompositionEnabled(pf_enabled: arcana_winapi.raw.types.PBOOL) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmIsCompositionEnabled
export shackle import fn DwmModifyPreviousDxFrameDuration(hwnd: arcana_winapi.raw.types.HWND, c_refreshes: arcana_winapi.raw.types.I32, f_relative: arcana_winapi.raw.types.BOOL) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmModifyPreviousDxFrameDuration
export shackle import fn DwmQueryThumbnailSourceSize(h_thumbnail: arcana_winapi.raw.types.LONG_PTR, p_size: arcana_winapi.raw.types.PSIZE) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmQueryThumbnailSourceSize
export shackle import fn DwmRegisterThumbnail(hwnd_destination: arcana_winapi.raw.types.HWND, hwnd_source: arcana_winapi.raw.types.HWND, ph_thumbnail_id: arcana_winapi.raw.types.PLONG_PTR) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmRegisterThumbnail
export shackle import fn DwmRenderGesture(gt: arcana_winapi.raw.types.I32, c_contacts: arcana_winapi.raw.types.U32, pdw_pointer_id: arcana_winapi.raw.types.PUINT, p_points: arcana_winapi.raw.types.PPOINT) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmRenderGesture
export shackle import fn DwmSetDxFrameDuration(hwnd: arcana_winapi.raw.types.HWND, c_refreshes: arcana_winapi.raw.types.I32) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmSetDxFrameDuration
export shackle import fn DwmSetIconicLivePreviewBitmap(hwnd: arcana_winapi.raw.types.HWND, hbmp: arcana_winapi.raw.types.HBITMAP, ppt_client: arcana_winapi.raw.types.PPOINT, dw_sitflags: arcana_winapi.raw.types.U32) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmSetIconicLivePreviewBitmap
export shackle import fn DwmSetIconicThumbnail(hwnd: arcana_winapi.raw.types.HWND, hbmp: arcana_winapi.raw.types.HBITMAP, dw_sitflags: arcana_winapi.raw.types.U32) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmSetIconicThumbnail
export shackle import fn DwmSetPresentParameters(hwnd: arcana_winapi.raw.types.HWND, p_present_params: arcana_winapi.raw.types.PLPVOID) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmSetPresentParameters
export shackle import fn DwmSetWindowAttribute(hwnd: arcana_winapi.raw.types.HWND, dw_attribute: arcana_winapi.raw.types.U32, pv_attribute: arcana_winapi.raw.types.HANDLE, cb_attribute: arcana_winapi.raw.types.U32) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmSetWindowAttribute
export shackle import fn DwmShowContact(dw_pointer_id: arcana_winapi.raw.types.U32, e_show_contact: arcana_winapi.raw.types.U32) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmShowContact
export shackle import fn DwmTetherContact(dw_pointer_id: arcana_winapi.raw.types.U32, f_enable: arcana_winapi.raw.types.BOOL, pt_tether: arcana_winapi.raw.types.POINT) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmTetherContact
export shackle import fn DwmTransitionOwnedWindow(hwnd: arcana_winapi.raw.types.HWND, target: arcana_winapi.raw.types.I32) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmTransitionOwnedWindow
export shackle import fn DwmUnregisterThumbnail(h_thumbnail_id: arcana_winapi.raw.types.LONG_PTR) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmUnregisterThumbnail
export shackle import fn DwmUpdateThumbnailProperties(h_thumbnail_id: arcana_winapi.raw.types.LONG_PTR, ptn_properties: arcana_winapi.raw.types.PLPVOID) -> arcana_winapi.raw.types.HRESULT = dwmapi.DwmUpdateThumbnailProperties
