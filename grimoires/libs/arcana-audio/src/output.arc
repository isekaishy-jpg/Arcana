import std.result
import arcana_audio.types
import arcana_winapi.helpers.audio
use std.result.Result
use arcana_winapi.audio_handles.AudioDevice

export fn default_output() -> Result[AudioDevice, Str]:
    return arcana_winapi.helpers.audio.default_output :: :: call

export fn close(take device: AudioDevice) -> Result[Unit, Str]:
    return arcana_winapi.helpers.audio.output_close :: device :: call

export fn default_output_config() -> arcana_audio.types.OutputConfig:
    return arcana_audio.types.OutputConfig :: gain_milli = 1000 :: call

export fn configure(edit device: AudioDevice, read cfg: arcana_audio.types.OutputConfig):
    arcana_winapi.helpers.audio.output_set_gain_milli :: device, cfg.gain_milli :: call

export fn set_gain_milli(edit device: AudioDevice, milli: Int):
    arcana_winapi.helpers.audio.output_set_gain_milli :: device, milli :: call

export fn sample_rate_hz(read device: AudioDevice) -> Int:
    return arcana_winapi.helpers.audio.output_sample_rate_hz :: device :: call

export fn channels(read device: AudioDevice) -> Int:
    return arcana_winapi.helpers.audio.output_channels :: device :: call

