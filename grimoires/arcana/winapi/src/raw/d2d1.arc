// GENERATED FILE. DO NOT EDIT BY HAND.
// Source of truth: grimoires/arcana/winapi/generation/imports.toml
// Projection config: grimoires/arcana/winapi/generation/projection.toml
// Source authority: Pinned Windows SDK metadata snapshot
// Metadata authority: Windows.Win32.winmd Microsoft.Windows.SDK.Win32Metadata 63.0.31 sha256:97D24CF1A9DC3E50782BBF1DBA0952BF6A025FA583D8B3AE6C5EF713B463C869
// Parity target: windows-sys; pinned metadata wins on disagreement.

export shackle import fn D2D1ComputeMaximumScaleFactor(matrix: arcana_winapi.raw.types.PLPVOID) -> arcana_winapi.raw.types.FLOAT = d2d1.D2D1ComputeMaximumScaleFactor
export shackle import fn D2D1ConvertColorSpace(source_color_space: arcana_winapi.raw.types.I32, destination_color_space: arcana_winapi.raw.types.I32, color: arcana_winapi.raw.types.PLPVOID) -> arcana_winapi.raw.types.LPVOID = d2d1.D2D1ConvertColorSpace
export shackle import fn D2D1CreateDevice(dxgi_device: arcana_winapi.raw.types.LPVOID, creation_properties: arcana_winapi.raw.types.PLPVOID, d2d_device: arcana_winapi.raw.types.PLPVOID) -> arcana_winapi.raw.types.HRESULT = d2d1.D2D1CreateDevice
export shackle import fn D2D1CreateDeviceContext(dxgi_surface: arcana_winapi.raw.types.LPVOID, creation_properties: arcana_winapi.raw.types.PLPVOID, d2d_device_context: arcana_winapi.raw.types.PLPVOID) -> arcana_winapi.raw.types.HRESULT = d2d1.D2D1CreateDeviceContext
export shackle import fn D2D1CreateFactory(factory_type: arcana_winapi.raw.types.D2D1_FACTORY_TYPE, riid: arcana_winapi.raw.types.PGUID, p_factory_options: arcana_winapi.raw.types.PD2D1_FACTORY_OPTIONS, pp_ifactory: arcana_winapi.raw.types.HANDLE) -> arcana_winapi.raw.types.HRESULT = d2d1.D2D1CreateFactory
export shackle import fn D2D1GetGradientMeshInteriorPointsFromCoonsPatch(p_point0: arcana_winapi.raw.types.PLPVOID, p_point1: arcana_winapi.raw.types.PLPVOID, p_point2: arcana_winapi.raw.types.PLPVOID, p_point3: arcana_winapi.raw.types.PLPVOID, p_point4: arcana_winapi.raw.types.PLPVOID, p_point5: arcana_winapi.raw.types.PLPVOID, p_point6: arcana_winapi.raw.types.PLPVOID, p_point7: arcana_winapi.raw.types.PLPVOID, p_point8: arcana_winapi.raw.types.PLPVOID, p_point9: arcana_winapi.raw.types.PLPVOID, p_point10: arcana_winapi.raw.types.PLPVOID, p_point11: arcana_winapi.raw.types.PLPVOID, p_tensor_point11: arcana_winapi.raw.types.PLPVOID, p_tensor_point12: arcana_winapi.raw.types.PLPVOID, p_tensor_point21: arcana_winapi.raw.types.PLPVOID, p_tensor_point22: arcana_winapi.raw.types.PLPVOID) = d2d1.D2D1GetGradientMeshInteriorPointsFromCoonsPatch
export shackle import fn D2D1InvertMatrix(matrix: arcana_winapi.raw.types.PLPVOID) -> arcana_winapi.raw.types.BOOL = d2d1.D2D1InvertMatrix
export shackle import fn D2D1IsMatrixInvertible(matrix: arcana_winapi.raw.types.PLPVOID) -> arcana_winapi.raw.types.BOOL = d2d1.D2D1IsMatrixInvertible
export shackle import fn D2D1MakeRotateMatrix(angle: arcana_winapi.raw.types.FLOAT, center: arcana_winapi.raw.types.LPVOID, matrix: arcana_winapi.raw.types.PLPVOID) = d2d1.D2D1MakeRotateMatrix
export shackle import fn D2D1MakeSkewMatrix(angle_x: arcana_winapi.raw.types.FLOAT, angle_y: arcana_winapi.raw.types.FLOAT, center: arcana_winapi.raw.types.LPVOID, matrix: arcana_winapi.raw.types.PLPVOID) = d2d1.D2D1MakeSkewMatrix
export shackle import fn D2D1SinCos(angle: arcana_winapi.raw.types.FLOAT, s: arcana_winapi.raw.types.PFLOAT, c: arcana_winapi.raw.types.PFLOAT) = d2d1.D2D1SinCos
export shackle import fn D2D1Tan(angle: arcana_winapi.raw.types.FLOAT) -> arcana_winapi.raw.types.FLOAT = d2d1.D2D1Tan
export shackle import fn D2D1Vec3Length(x: arcana_winapi.raw.types.FLOAT, y: arcana_winapi.raw.types.FLOAT, z: arcana_winapi.raw.types.FLOAT) -> arcana_winapi.raw.types.FLOAT = d2d1.D2D1Vec3Length
