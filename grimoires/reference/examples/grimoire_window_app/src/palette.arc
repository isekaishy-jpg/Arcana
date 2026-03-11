import winspell.draw

export fn bg_color() -> Int:
    return winspell.draw.rgb :: 16, 18, 24 :: call

export fn title_color() -> Int:
    return winspell.draw.rgb :: 255, 255, 255 :: call

export fn accent_a() -> Int:
    return winspell.draw.rgb :: 60, 120, 220 :: call

export fn accent_b() -> Int:
    return winspell.draw.rgb :: 210, 90, 110 :: call