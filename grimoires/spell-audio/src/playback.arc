import std.audio
import spell_audio.clip

export fn play_once(read device: AudioDevice, read clip: AudioBuffer) -> AudioPlayback:
    return std.audio.play_buffer :: device, clip :: call

export fn play_wav(read device: AudioDevice, path: Str) -> AudioPlayback:
    let clip = spell_audio.clip.load_wav :: path :: call
    return std.audio.play_buffer :: device, clip :: call

export fn stop(read playback: AudioPlayback):
    playback :: :: stop

export fn playing(read playback: AudioPlayback) -> Bool:
    return playback :: :: playing
