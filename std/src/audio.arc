import std.kernel.audio
import std.result
use std.result.Result

export opaque type AudioDevice as move, boundary_unsafe
export opaque type AudioBuffer as move, boundary_unsafe
export opaque type AudioPlayback as move, boundary_unsafe

lang audio_device_handle = AudioDevice
lang audio_buffer_handle = AudioBuffer
lang audio_playback_handle = AudioPlayback

export fn default_output() -> Result[AudioDevice, Str]:
    return std.kernel.audio.default_output :: :: call

export fn output_close(take device: AudioDevice) -> Result[Unit, Str]:
    return std.kernel.audio.output_close :: device :: call

export fn output_sample_rate_hz(read device: AudioDevice) -> Int:
    return std.kernel.audio.output_sample_rate_hz :: device :: call

export fn output_channels(read device: AudioDevice) -> Int:
    return std.kernel.audio.output_channels :: device :: call

export fn buffer_load_wav(path: Str) -> Result[AudioBuffer, Str]:
    return std.kernel.audio.buffer_load_wav :: path :: call

export fn buffer_frames(read buffer: AudioBuffer) -> Int:
    return std.kernel.audio.buffer_frames :: buffer :: call

export fn buffer_channels(read buffer: AudioBuffer) -> Int:
    return std.kernel.audio.buffer_channels :: buffer :: call

export fn buffer_sample_rate_hz(read buffer: AudioBuffer) -> Int:
    return std.kernel.audio.buffer_sample_rate_hz :: buffer :: call

export fn play_buffer(edit device: AudioDevice, read buffer: AudioBuffer) -> Result[AudioPlayback, Str]:
    return std.kernel.audio.play_buffer :: device, buffer :: call

export fn output_set_gain_milli(edit device: AudioDevice, milli: Int):
    std.kernel.audio.output_set_gain_milli :: device, milli :: call

impl AudioPlayback:
    fn stop(take self: AudioPlayback) -> Result[Unit, Str]:
        return std.kernel.audio.playback_stop :: self :: call

    fn pause(edit self: AudioPlayback):
        std.kernel.audio.playback_pause :: self :: call

    fn resume(edit self: AudioPlayback):
        std.kernel.audio.playback_resume :: self :: call

    fn playing(read self: AudioPlayback) -> Bool:
        return std.kernel.audio.playback_playing :: self :: call

    fn paused(read self: AudioPlayback) -> Bool:
        return std.kernel.audio.playback_paused :: self :: call

    fn finished(read self: AudioPlayback) -> Bool:
        return std.kernel.audio.playback_finished :: self :: call

    fn set_gain_milli(edit self: AudioPlayback, milli: Int):
        std.kernel.audio.playback_set_gain_milli :: self, milli :: call

    fn set_looping(edit self: AudioPlayback, looping: Bool):
        std.kernel.audio.playback_set_looping :: self, looping :: call

    fn looping(read self: AudioPlayback) -> Bool:
        return std.kernel.audio.playback_looping :: self :: call

    fn position_frames(read self: AudioPlayback) -> Int:
        return std.kernel.audio.playback_position_frames :: self :: call
