import std.cleanup
import std.result
import arcana_audio.clip
import arcana_winapi.helpers.audio
use std.result.Result
use arcana_winapi.audio_handles.AudioBuffer
use arcana_winapi.audio_handles.AudioDevice
use arcana_winapi.audio_handles.AudioPlayback

export fn play(edit device: AudioDevice, read clip: AudioBuffer) -> Result[AudioPlayback, Str]:
    return arcana_winapi.helpers.audio.play_buffer :: device, clip :: call

export fn play_wav(edit device: AudioDevice, path: Str) -> Result[AudioPlayback, Str]:
    let clip = arcana_audio.clip.load_wav :: path :: call
    return match clip:
        Result.Ok(value) => arcana_winapi.helpers.audio.play_buffer :: device, value :: call
        Result.Err(err) => Result.Err[AudioPlayback, Str] :: err :: call

export fn stop(take playback: AudioPlayback) -> Result[Unit, Str]:
    return playback :: :: stop

export fn pause(edit playback: AudioPlayback):
    playback :: :: pause

export fn resume(edit playback: AudioPlayback):
    playback :: :: resume

export fn playing(read playback: AudioPlayback) -> Bool:
    return playback :: :: playing

export fn paused(read playback: AudioPlayback) -> Bool:
    return playback :: :: paused

export fn finished(read playback: AudioPlayback) -> Bool:
    return playback :: :: finished

export fn set_gain_milli(edit playback: AudioPlayback, milli: Int):
    playback :: milli :: set_gain_milli

export fn set_looping(edit playback: AudioPlayback, looping: Bool):
    playback :: looping :: set_looping

export fn looping(read playback: AudioPlayback) -> Bool:
    return playback :: :: looping

export fn position_frames(read playback: AudioPlayback) -> Int:
    return playback :: :: position_frames

impl std.cleanup.Cleanup[arcana_winapi.audio_handles.AudioDevice] for arcana_winapi.audio_handles.AudioDevice:
    fn cleanup(take self: arcana_winapi.audio_handles.AudioDevice) -> Result[Unit, Str]:
        return arcana_audio.output.close :: self :: call

impl std.cleanup.Cleanup[arcana_winapi.audio_handles.AudioPlayback] for arcana_winapi.audio_handles.AudioPlayback:
    fn cleanup(take self: arcana_winapi.audio_handles.AudioPlayback) -> Result[Unit, Str]:
        return self :: :: stop

impl AudioPlayback:
    fn stop(take self: AudioPlayback) -> Result[Unit, Str]:
        return arcana_winapi.helpers.audio.playback_stop :: self :: call

    fn pause(edit self: AudioPlayback):
        arcana_winapi.helpers.audio.playback_pause :: self :: call

    fn resume(edit self: AudioPlayback):
        arcana_winapi.helpers.audio.playback_resume :: self :: call

    fn playing(read self: AudioPlayback) -> Bool:
        return arcana_winapi.helpers.audio.playback_playing :: self :: call

    fn paused(read self: AudioPlayback) -> Bool:
        return arcana_winapi.helpers.audio.playback_paused :: self :: call

    fn finished(read self: AudioPlayback) -> Bool:
        return arcana_winapi.helpers.audio.playback_finished :: self :: call

    fn set_gain_milli(edit self: AudioPlayback, milli: Int):
        arcana_winapi.helpers.audio.playback_set_gain_milli :: self, milli :: call

    fn set_looping(edit self: AudioPlayback, looping: Bool):
        arcana_winapi.helpers.audio.playback_set_looping :: self, looping :: call

    fn looping(read self: AudioPlayback) -> Bool:
        return arcana_winapi.helpers.audio.playback_looping :: self :: call

    fn position_frames(read self: AudioPlayback) -> Int:
        return arcana_winapi.helpers.audio.playback_position_frames :: self :: call

