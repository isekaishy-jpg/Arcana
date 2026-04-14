export opaque type Window as move, boundary_unsafe
export opaque type FrameInput as move, boundary_unsafe
export opaque type Session as move, boundary_unsafe
export opaque type WakeHandle as copy, boundary_unsafe

lang window_handle = Window
lang app_frame_handle = FrameInput
lang app_session_handle = Session
lang wake_handle = WakeHandle
