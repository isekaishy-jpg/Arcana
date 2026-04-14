import std.result
use arcana_winapi.audio_handles.AudioBuffer
use arcana_winapi.audio_handles.AudioDevice
use arcana_winapi.audio_handles.AudioPlayback
use std.result.Result

native fn take_last_error() -> Str = helpers.audio.take_last_error
native fn default_output_raw() -> AudioDevice = helpers.audio.default_output
native fn output_close_raw(take device: AudioDevice) -> Bool = helpers.audio.output_close
native fn buffer_load_wav_raw(path: Str) -> AudioBuffer = helpers.audio.buffer_load_wav
native fn play_buffer_raw(edit device: AudioDevice, read buffer: AudioBuffer) -> AudioPlayback = helpers.audio.play_buffer
native fn playback_stop_raw(take playback: AudioPlayback) -> Bool = helpers.audio.playback_stop

fn result_handle[T](take value: T) -> Result[T, Str]:
    let err = take_last_error :: :: call
    if err == "":
        return Result.Ok[T, Str] :: value :: call
    return Result.Err[T, Str] :: err :: call

fn result_unit(ok: Bool) -> Result[Unit, Str]:
    if ok:
        return Result.Ok[Unit, Str] :: :: call
    return Result.Err[Unit, Str] :: (take_last_error :: :: call) :: call

export native fn render_device_count() -> Int = helpers.audio.render_device_count
export native fn bootstrap_wasapi_default_render() -> Bool = helpers.audio.bootstrap_wasapi_default_render
export native fn bootstrap_wasapi_render_client() -> Bool = helpers.audio.bootstrap_wasapi_render_client
export native fn bootstrap_endpoint_volume() -> Bool = helpers.audio.bootstrap_endpoint_volume
export native fn bootstrap_session_policy_game_effects() -> Bool = helpers.audio.bootstrap_session_policy_game_effects
export native fn register_pro_audio_thread() -> Bool = helpers.audio.register_pro_audio_thread
export native fn bootstrap_xaudio2() -> Bool = helpers.audio.bootstrap_xaudio2
export native fn bootstrap_x3daudio() -> Bool = helpers.audio.bootstrap_x3daudio

export fn default_output() -> Result[AudioDevice, Str]:
    return result_handle[AudioDevice] :: (default_output_raw :: :: call) :: call

export fn output_close(take device: AudioDevice) -> Result[Unit, Str]:
    return result_unit :: (output_close_raw :: device :: call) :: call

export native fn output_sample_rate_hz(read device: AudioDevice) -> Int = helpers.audio.output_sample_rate_hz
export native fn output_channels(read device: AudioDevice) -> Int = helpers.audio.output_channels

export fn buffer_load_wav(path: Str) -> Result[AudioBuffer, Str]:
    return result_handle[AudioBuffer] :: (buffer_load_wav_raw :: path :: call) :: call

export native fn buffer_frames(read buffer: AudioBuffer) -> Int = helpers.audio.buffer_frames
export native fn buffer_channels(read buffer: AudioBuffer) -> Int = helpers.audio.buffer_channels
export native fn buffer_sample_rate_hz(read buffer: AudioBuffer) -> Int = helpers.audio.buffer_sample_rate_hz

export fn play_buffer(edit device: AudioDevice, read buffer: AudioBuffer) -> Result[AudioPlayback, Str]:
    return result_handle[AudioPlayback] :: (play_buffer_raw :: device, buffer :: call) :: call

export native fn output_set_gain_milli(edit device: AudioDevice, milli: Int) = helpers.audio.output_set_gain_milli

export fn playback_stop(take playback: AudioPlayback) -> Result[Unit, Str]:
    return result_unit :: (playback_stop_raw :: playback :: call) :: call

export native fn playback_pause(edit playback: AudioPlayback) = helpers.audio.playback_pause
export native fn playback_resume(edit playback: AudioPlayback) = helpers.audio.playback_resume
export native fn playback_playing(read playback: AudioPlayback) -> Bool = helpers.audio.playback_playing
export native fn playback_paused(read playback: AudioPlayback) -> Bool = helpers.audio.playback_paused
export native fn playback_finished(read playback: AudioPlayback) -> Bool = helpers.audio.playback_finished
export native fn playback_set_gain_milli(edit playback: AudioPlayback, milli: Int) = helpers.audio.playback_set_gain_milli
export native fn playback_set_looping(edit playback: AudioPlayback, looping: Bool) = helpers.audio.playback_set_looping
export native fn playback_looping(read playback: AudioPlayback) -> Bool = helpers.audio.playback_looping
export native fn playback_position_frames(read playback: AudioPlayback) -> Int = helpers.audio.playback_position_frames

