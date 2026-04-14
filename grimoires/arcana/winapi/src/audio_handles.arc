export opaque type AudioDevice as move, boundary_unsafe
export opaque type AudioBuffer as move, boundary_unsafe
export opaque type AudioPlayback as move, boundary_unsafe

lang audio_device_handle = AudioDevice
lang audio_buffer_handle = AudioBuffer
lang audio_playback_handle = AudioPlayback
