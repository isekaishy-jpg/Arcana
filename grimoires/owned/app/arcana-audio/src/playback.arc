import std.audio
import std.result
import arcana_audio.clip
use std.result.Result

export fn play(read device: AudioDevice, read clip: AudioBuffer) -> Result[AudioPlayback, Str]:
    return std.audio.play_buffer :: device, clip :: call

export fn play_wav(read device: AudioDevice, path: Str) -> Result[AudioPlayback, Str]:
    let clip = arcana_audio.clip.load_wav :: path :: call
    return match clip:
        Result.Ok(value) => std.audio.play_buffer :: device, value :: call
        Result.Err(err) => Result.Err[AudioPlayback, Str] :: err :: call

export fn stop(take playback: AudioPlayback) -> Result[Unit, Str]:
    return playback :: :: stop

export fn pause(read playback: AudioPlayback):
    playback :: :: pause

export fn resume(read playback: AudioPlayback):
    playback :: :: resume

export fn playing(read playback: AudioPlayback) -> Bool:
    return playback :: :: playing

export fn paused(read playback: AudioPlayback) -> Bool:
    return playback :: :: paused

export fn finished(read playback: AudioPlayback) -> Bool:
    return playback :: :: finished

export fn set_gain_milli(read playback: AudioPlayback, milli: Int):
    playback :: milli :: set_gain_milli

export fn set_looping(read playback: AudioPlayback, looping: Bool):
    playback :: looping :: set_looping

export fn looping(read playback: AudioPlayback) -> Bool:
    return playback :: :: looping

export fn position_frames(read playback: AudioPlayback) -> Int:
    return playback :: :: position_frames
