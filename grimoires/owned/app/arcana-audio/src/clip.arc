import std.audio
import std.result
import arcana_audio.types
use std.result.Result
use std.audio.AudioBuffer

export fn load_wav(path: Str) -> Result[AudioBuffer, Str]:
    return std.audio.buffer_load_wav :: path :: call

export fn info(read clip: AudioBuffer) -> arcana_audio.types.ClipInfo:
    return arcana_audio.types.ClipInfo :: frames = (std.audio.buffer_frames :: clip :: call), channels = (std.audio.buffer_channels :: clip :: call), sample_rate_hz = (std.audio.buffer_sample_rate_hz :: clip :: call) :: call
