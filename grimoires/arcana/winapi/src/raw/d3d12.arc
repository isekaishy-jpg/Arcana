// GENERATED FILE. DO NOT EDIT BY HAND.
// Source of truth: grimoires/arcana/winapi/generation/imports.toml
// Projection config: grimoires/arcana/winapi/generation/projection.toml
// Source authority: Pinned Windows SDK metadata snapshot
// Metadata authority: Windows.Win32.winmd Microsoft.Windows.SDK.Win32Metadata 63.0.31 sha256:97D24CF1A9DC3E50782BBF1DBA0952BF6A025FA583D8B3AE6C5EF713B463C869
// Parity target: windows-sys; pinned metadata wins on disagreement.

export shackle import fn D3D12CreateDevice(p_adapter: arcana_winapi.raw.types.IUnknown, minimum_feature_level: arcana_winapi.raw.types.I32, riid: arcana_winapi.raw.types.PGUID, pp_device: arcana_winapi.raw.types.HANDLE) -> arcana_winapi.raw.types.HRESULT = d3d12.D3D12CreateDevice
export shackle import fn D3D12CreateRootSignatureDeserializer(p_src_data: arcana_winapi.raw.types.HANDLE, src_data_size_in_bytes: arcana_winapi.raw.types.ULONG_PTR, p_root_signature_deserializer_interface: arcana_winapi.raw.types.PGUID, pp_root_signature_deserializer: arcana_winapi.raw.types.HANDLE) -> arcana_winapi.raw.types.HRESULT = d3d12.D3D12CreateRootSignatureDeserializer
export shackle import fn D3D12CreateVersionedRootSignatureDeserializer(p_src_data: arcana_winapi.raw.types.HANDLE, src_data_size_in_bytes: arcana_winapi.raw.types.ULONG_PTR, p_root_signature_deserializer_interface: arcana_winapi.raw.types.PGUID, pp_root_signature_deserializer: arcana_winapi.raw.types.HANDLE) -> arcana_winapi.raw.types.HRESULT = d3d12.D3D12CreateVersionedRootSignatureDeserializer
export shackle import fn D3D12EnableExperimentalFeatures(num_features: arcana_winapi.raw.types.U32, p_iids: arcana_winapi.raw.types.PGUID, p_configuration_structs: arcana_winapi.raw.types.HANDLE, p_configuration_struct_sizes: arcana_winapi.raw.types.PUINT) -> arcana_winapi.raw.types.HRESULT = d3d12.D3D12EnableExperimentalFeatures
export shackle import fn D3D12GetDebugInterface(riid: arcana_winapi.raw.types.PGUID, ppv_debug: arcana_winapi.raw.types.HANDLE) -> arcana_winapi.raw.types.HRESULT = d3d12.D3D12GetDebugInterface
export shackle import fn D3D12GetInterface(rclsid: arcana_winapi.raw.types.PGUID, riid: arcana_winapi.raw.types.PGUID, ppv_debug: arcana_winapi.raw.types.HANDLE) -> arcana_winapi.raw.types.HRESULT = d3d12.D3D12GetInterface
export shackle import fn D3D12SerializeRootSignature(p_root_signature: arcana_winapi.raw.types.PLPVOID, version: arcana_winapi.raw.types.I32, pp_blob: arcana_winapi.raw.types.PLPVOID, pp_error_blob: arcana_winapi.raw.types.PLPVOID) -> arcana_winapi.raw.types.HRESULT = d3d12.D3D12SerializeRootSignature
export shackle import fn D3D12SerializeVersionedRootSignature(p_root_signature: arcana_winapi.raw.types.PLPVOID, pp_blob: arcana_winapi.raw.types.PLPVOID, pp_error_blob: arcana_winapi.raw.types.PLPVOID) -> arcana_winapi.raw.types.HRESULT = d3d12.D3D12SerializeVersionedRootSignature
