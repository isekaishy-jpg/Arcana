import spell_audio.output
import spell_audio.clip
import spell_audio.playback
import std.io

fn main() -> Int:
    let mut device = spell_audio.output.default_output :: :: call
    let cfg = spell_audio.output.default_output_config :: :: call
    spell_audio.output.configure :: device, cfg :: call

    let clip = spell_audio.clip.load_wav :: "examples/assets/audio_smoke.wav" :: call
    let info = spell_audio.clip.info :: clip :: call
    info.frames :: :: std.io.print
    info.channels :: :: std.io.print
    info.sample_rate_hz :: :: std.io.print

    let playback = spell_audio.playback.play_once :: device, clip :: call
    let active = spell_audio.playback.playing :: playback :: call
    active :: :: std.io.print
    spell_audio.playback.stop :: playback :: call
    return 0
