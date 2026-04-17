// GENERATED FILE. DO NOT EDIT BY HAND.
// Source of truth: grimoires/arcana/winapi/generation/imports.toml
// Projection config: grimoires/arcana/winapi/generation/projection.toml
// Source authority: Pinned Windows SDK metadata snapshot
// Metadata authority: Windows.Win32.winmd Microsoft.Windows.SDK.Win32Metadata 63.0.31 sha256:97D24CF1A9DC3E50782BBF1DBA0952BF6A025FA583D8B3AE6C5EF713B463C869
// Parity target: windows-sys; pinned metadata wins on disagreement.

export shackle import fn AvQuerySystemResponsiveness(avrt_handle: arcana_winapi.raw.types.HANDLE, system_responsiveness_value: arcana_winapi.raw.types.PUINT) -> arcana_winapi.raw.types.BOOL = avrt.AvQuerySystemResponsiveness
export shackle import fn AvRevertMmThreadCharacteristics(avrt_handle: arcana_winapi.raw.types.HANDLE) -> arcana_winapi.raw.types.BOOL = avrt.AvRevertMmThreadCharacteristics
export shackle import fn AvRtCreateThreadOrderingGroup(context: arcana_winapi.raw.types.PHANDLE, period: arcana_winapi.raw.types.PI64, thread_ordering_guid: arcana_winapi.raw.types.PGUID, timeout: arcana_winapi.raw.types.PI64) -> arcana_winapi.raw.types.BOOL = avrt.AvRtCreateThreadOrderingGroup
export shackle import fn AvRtCreateThreadOrderingGroupExA(context: arcana_winapi.raw.types.PHANDLE, period: arcana_winapi.raw.types.PI64, thread_ordering_guid: arcana_winapi.raw.types.PGUID, timeout: arcana_winapi.raw.types.PI64, task_name: arcana_winapi.raw.types.PSTR) -> arcana_winapi.raw.types.BOOL = avrt.AvRtCreateThreadOrderingGroupExA
export shackle import fn AvRtCreateThreadOrderingGroupExW(context: arcana_winapi.raw.types.PHANDLE, period: arcana_winapi.raw.types.PI64, thread_ordering_guid: arcana_winapi.raw.types.PGUID, timeout: arcana_winapi.raw.types.PI64, task_name: arcana_winapi.raw.types.PWSTR) -> arcana_winapi.raw.types.BOOL = avrt.AvRtCreateThreadOrderingGroupExW
export shackle import fn AvRtDeleteThreadOrderingGroup(context: arcana_winapi.raw.types.HANDLE) -> arcana_winapi.raw.types.BOOL = avrt.AvRtDeleteThreadOrderingGroup
export shackle import fn AvRtJoinThreadOrderingGroup(context: arcana_winapi.raw.types.PHANDLE, thread_ordering_guid: arcana_winapi.raw.types.PGUID, before_arg: arcana_winapi.raw.types.BOOL) -> arcana_winapi.raw.types.BOOL = avrt.AvRtJoinThreadOrderingGroup
export shackle import fn AvRtLeaveThreadOrderingGroup(context: arcana_winapi.raw.types.HANDLE) -> arcana_winapi.raw.types.BOOL = avrt.AvRtLeaveThreadOrderingGroup
export shackle import fn AvRtWaitOnThreadOrderingGroup(context: arcana_winapi.raw.types.HANDLE) -> arcana_winapi.raw.types.BOOL = avrt.AvRtWaitOnThreadOrderingGroup
export shackle import fn AvSetMmMaxThreadCharacteristicsA(first_task: arcana_winapi.raw.types.PSTR, second_task: arcana_winapi.raw.types.PSTR, task_index: arcana_winapi.raw.types.PUINT) -> arcana_winapi.raw.types.HANDLE = avrt.AvSetMmMaxThreadCharacteristicsA
export shackle import fn AvSetMmMaxThreadCharacteristicsW(first_task: arcana_winapi.raw.types.PWSTR, second_task: arcana_winapi.raw.types.PWSTR, task_index: arcana_winapi.raw.types.PUINT) -> arcana_winapi.raw.types.HANDLE = avrt.AvSetMmMaxThreadCharacteristicsW
export shackle import fn AvSetMmThreadCharacteristicsA(task_name: arcana_winapi.raw.types.PSTR, task_index: arcana_winapi.raw.types.PUINT) -> arcana_winapi.raw.types.HANDLE = avrt.AvSetMmThreadCharacteristicsA
export shackle import fn AvSetMmThreadCharacteristicsW(task_name: arcana_winapi.raw.types.PWSTR, task_index: arcana_winapi.raw.types.PUINT) -> arcana_winapi.raw.types.HANDLE = avrt.AvSetMmThreadCharacteristicsW
export shackle import fn AvSetMmThreadPriority(avrt_handle: arcana_winapi.raw.types.HANDLE, priority: arcana_winapi.raw.types.I32) -> arcana_winapi.raw.types.BOOL = avrt.AvSetMmThreadPriority
