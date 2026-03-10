import std.kernel.audio

export fn default_output() -> AudioDevice:
    return std.kernel.audio.default_output :: :: call

export fn buffer_load_wav(path: Str) -> AudioBuffer:
    return std.kernel.audio.buffer_load_wav :: path :: call

export fn buffer_frames(read buffer: AudioBuffer) -> Int:
    return std.kernel.audio.buffer_frames :: buffer :: call

export fn buffer_channels(read buffer: AudioBuffer) -> Int:
    return std.kernel.audio.buffer_channels :: buffer :: call

export fn buffer_sample_rate_hz(read buffer: AudioBuffer) -> Int:
    return std.kernel.audio.buffer_sample_rate_hz :: buffer :: call

export fn play_buffer(read device: AudioDevice, read buffer: AudioBuffer) -> AudioPlayback:
    return std.kernel.audio.play_buffer :: device, buffer :: call

export fn output_set_gain_milli(read device: AudioDevice, milli: Int):
    std.kernel.audio.output_set_gain_milli :: device, milli :: call

impl AudioPlayback:
    fn stop(read self: AudioPlayback):
        std.kernel.audio.playback_stop :: self :: call

    fn playing(read self: AudioPlayback) -> Bool:
        return std.kernel.audio.playback_playing :: self :: call
