import std.result
import arcana_audio.types
import arcana_winapi.helpers.audio
use std.result.Result
use arcana_winapi.audio_handles.AudioBuffer

export fn load_wav(path: Str) -> Result[AudioBuffer, Str]:
    return arcana_winapi.helpers.audio.buffer_load_wav :: path :: call

export fn info(read clip: AudioBuffer) -> arcana_audio.types.ClipInfo:
    return arcana_audio.types.ClipInfo :: frames = (arcana_winapi.helpers.audio.buffer_frames :: clip :: call), channels = (arcana_winapi.helpers.audio.buffer_channels :: clip :: call), sample_rate_hz = (arcana_winapi.helpers.audio.buffer_sample_rate_hz :: clip :: call) :: call

