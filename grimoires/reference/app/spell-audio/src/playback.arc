import std.audio
import std.result
import spell_audio.clip
use std.result.Result
use std.audio.AudioBuffer
use std.audio.AudioDevice
use std.audio.AudioPlayback

export fn play_once(edit device: AudioDevice, read clip: AudioBuffer) -> Result[AudioPlayback, Str]:
    return std.audio.play_buffer :: device, clip :: call

export fn play_wav(edit device: AudioDevice, path: Str) -> Result[AudioPlayback, Str]:
    let clip = spell_audio.clip.load_wav :: path :: call
    return match clip:
        Result.Ok(value) => std.audio.play_buffer :: device, value :: call
        Result.Err(err) => Result.Err[AudioPlayback, Str] :: err :: call

export fn stop(take playback: AudioPlayback):
    playback :: :: stop

export fn playing(read playback: AudioPlayback) -> Bool:
    return playback :: :: playing
