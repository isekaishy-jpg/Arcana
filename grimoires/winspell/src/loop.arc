import winspell.window
import winspell.draw

export record FrameConfig:
    clear: Int

export fn default_frame_config() -> FrameConfig:
    return winspell.loop.FrameConfig :: clear = 0 :: call

export fn begin_frame(edit win: Window, read cfg: FrameConfig):
    winspell.draw.fill :: win, cfg.clear :: call

export fn end_frame(edit win: Window):
    winspell.draw.present :: win :: call

export fn should_run(read win: Window) -> Bool:
    return winspell.window.alive :: win :: call
