import std.audio
import std.result
import spell_audio.types
use std.result.Result
use std.audio.AudioBuffer

export fn load_wav(path: Str) -> Result[AudioBuffer, Str]:
    return std.audio.buffer_load_wav :: path :: call

export fn frames(read clip: AudioBuffer) -> Int:
    return std.audio.buffer_frames :: clip :: call

export fn channels(read clip: AudioBuffer) -> Int:
    return std.audio.buffer_channels :: clip :: call

export fn sample_rate_hz(read clip: AudioBuffer) -> Int:
    return std.audio.buffer_sample_rate_hz :: clip :: call

export fn info(read clip: AudioBuffer) -> spell_audio.types.ClipInfo:
    return spell_audio.types.ClipInfo :: frames = (spell_audio.clip.frames :: clip :: call), channels = (spell_audio.clip.channels :: clip :: call), sample_rate_hz = (spell_audio.clip.sample_rate_hz :: clip :: call) :: call
