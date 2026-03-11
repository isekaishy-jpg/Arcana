import std.kernel.audio
import std.kernel.error
import std.result
use std.result.Result

export fn default_output() -> Result[AudioDevice, Str]:
    let pair = std.kernel.audio.default_output_try :: :: call
    if pair.0:
        return Result.Ok[AudioDevice, Str] :: pair.1 :: call
    return Result.Err[AudioDevice, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn output_close(read device: AudioDevice):
    std.kernel.audio.output_close :: device :: call

export fn output_sample_rate_hz(read device: AudioDevice) -> Int:
    return std.kernel.audio.output_sample_rate_hz :: device :: call

export fn output_channels(read device: AudioDevice) -> Int:
    return std.kernel.audio.output_channels :: device :: call

export fn buffer_load_wav(path: Str) -> Result[AudioBuffer, Str]:
    let pair = std.kernel.audio.buffer_load_wav_try :: path :: call
    if pair.0:
        return Result.Ok[AudioBuffer, Str] :: pair.1 :: call
    return Result.Err[AudioBuffer, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn buffer_frames(read buffer: AudioBuffer) -> Int:
    return std.kernel.audio.buffer_frames :: buffer :: call

export fn buffer_channels(read buffer: AudioBuffer) -> Int:
    return std.kernel.audio.buffer_channels :: buffer :: call

export fn buffer_sample_rate_hz(read buffer: AudioBuffer) -> Int:
    return std.kernel.audio.buffer_sample_rate_hz :: buffer :: call

export fn play_buffer(read device: AudioDevice, read buffer: AudioBuffer) -> Result[AudioPlayback, Str]:
    let pair = std.kernel.audio.play_buffer_try :: device, buffer :: call
    if pair.0:
        return Result.Ok[AudioPlayback, Str] :: pair.1 :: call
    return Result.Err[AudioPlayback, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn output_set_gain_milli(read device: AudioDevice, milli: Int):
    std.kernel.audio.output_set_gain_milli :: device, milli :: call

impl AudioPlayback:
    fn stop(read self: AudioPlayback):
        std.kernel.audio.playback_stop :: self :: call

    fn pause(read self: AudioPlayback):
        std.kernel.audio.playback_pause :: self :: call

    fn resume(read self: AudioPlayback):
        std.kernel.audio.playback_resume :: self :: call

    fn playing(read self: AudioPlayback) -> Bool:
        return std.kernel.audio.playback_playing :: self :: call

    fn paused(read self: AudioPlayback) -> Bool:
        return std.kernel.audio.playback_paused :: self :: call

    fn finished(read self: AudioPlayback) -> Bool:
        return std.kernel.audio.playback_finished :: self :: call

    fn set_gain_milli(read self: AudioPlayback, milli: Int):
        std.kernel.audio.playback_set_gain_milli :: self, milli :: call

    fn set_looping(read self: AudioPlayback, looping: Bool):
        std.kernel.audio.playback_set_looping :: self, looping :: call

    fn looping(read self: AudioPlayback) -> Bool:
        return std.kernel.audio.playback_looping :: self :: call

    fn position_frames(read self: AudioPlayback) -> Int:
        return std.kernel.audio.playback_position_frames :: self :: call
