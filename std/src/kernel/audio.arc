import std.result
use std.audio.AudioBuffer
use std.audio.AudioDevice
use std.audio.AudioPlayback
use std.result.Result

intrinsic fn default_output() -> Result[AudioDevice, Str] = AudioDefaultOutputTry
intrinsic fn output_close(take device: AudioDevice) -> Result[Unit, Str] = AudioOutputClose
intrinsic fn output_sample_rate_hz(read device: AudioDevice) -> Int = AudioOutputSampleRateHz
intrinsic fn output_channels(read device: AudioDevice) -> Int = AudioOutputChannels
intrinsic fn buffer_load_wav(path: Str) -> Result[AudioBuffer, Str] = AudioBufferLoadWavTry
intrinsic fn buffer_frames(read buffer: AudioBuffer) -> Int = AudioBufferFrames
intrinsic fn buffer_channels(read buffer: AudioBuffer) -> Int = AudioBufferChannels
intrinsic fn buffer_sample_rate_hz(read buffer: AudioBuffer) -> Int = AudioBufferSampleRateHz
intrinsic fn play_buffer(edit device: AudioDevice, read buffer: AudioBuffer) -> Result[AudioPlayback, Str] = AudioPlayBufferTry
intrinsic fn output_set_gain_milli(edit device: AudioDevice, milli: Int) = AudioOutputSetGainMilli
intrinsic fn playback_stop(take playback: AudioPlayback) -> Result[Unit, Str] = AudioPlaybackStop
intrinsic fn playback_pause(edit playback: AudioPlayback) = AudioPlaybackPause
intrinsic fn playback_resume(edit playback: AudioPlayback) = AudioPlaybackResume
intrinsic fn playback_playing(read playback: AudioPlayback) -> Bool = AudioPlaybackPlaying
intrinsic fn playback_paused(read playback: AudioPlayback) -> Bool = AudioPlaybackPaused
intrinsic fn playback_finished(read playback: AudioPlayback) -> Bool = AudioPlaybackFinished
intrinsic fn playback_set_gain_milli(edit playback: AudioPlayback, milli: Int) = AudioPlaybackSetGainMilli
intrinsic fn playback_set_looping(edit playback: AudioPlayback, looping: Bool) = AudioPlaybackSetLooping
intrinsic fn playback_looping(read playback: AudioPlayback) -> Bool = AudioPlaybackLooping
intrinsic fn playback_position_frames(read playback: AudioPlayback) -> Int = AudioPlaybackPositionFrames
