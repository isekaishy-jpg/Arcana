import std.audio
import std.result
import spell_audio.types
use std.result.Result
use std.audio.AudioDevice

export fn default_output() -> Result[AudioDevice, Str]:
    return std.audio.default_output :: :: call

export fn default_output_config() -> spell_audio.types.OutputConfig:
    return spell_audio.types.OutputConfig :: gain_milli = 1000 :: call

export fn configure(edit device: AudioDevice, read cfg: spell_audio.types.OutputConfig):
    std.audio.output_set_gain_milli :: device, cfg.gain_milli :: call

export fn set_gain_milli(edit device: AudioDevice, milli: Int):
    std.audio.output_set_gain_milli :: device, milli :: call
