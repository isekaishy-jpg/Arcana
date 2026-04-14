import arcana_text.buffer
import arcana_text.fonts
import arcana_text.shape.glyphs
import arcana_text.shape.pipeline
import arcana_text.shape.styles
import arcana_text.shape.tokens
import arcana_text.types
import std.text
import std.collections.list
import arcana_process.fs
import std.option
import arcana_process.path
import std.text
use std.option.Option

fn layout_probe_flag_path() -> Str:
    return arcana_process.path.join :: (arcana_process.path.join :: (arcana_process.path.cwd :: :: call), "scratch" :: call), "enable_text_fonts_probe" :: call

fn layout_probe_log_path() -> Str:
    return arcana_process.path.join :: (arcana_process.path.join :: (arcana_process.path.cwd :: :: call), "scratch" :: call), "text_layout_probe.log" :: call

fn layout_probe_enabled() -> Bool:
    return arcana_process.fs.is_file :: (layout_probe_flag_path :: :: call) :: call

fn layout_probe_append(line: Str):
    if not (layout_probe_enabled :: :: call):
        return
    let _ = arcana_process.fs.mkdir_all :: (arcana_process.path.parent :: (layout_probe_log_path :: :: call) :: call) :: call
    let opened = arcana_process.fs.stream_open_write :: (layout_probe_log_path :: :: call), true :: call
    return match opened:
        std.result.Result.Ok(value) => layout_probe_append_ready :: value, line :: call
        std.result.Result.Err(_) => 0

fn layout_probe_append_ready(take value: arcana_winapi.process_handles.FileStream, line: Str):
    let mut stream = value
    let bytes = std.text.bytes_from_str_utf8 :: (line + "\n") :: call
    let _ = arcana_process.fs.stream_write :: stream, bytes :: call
    let _ = arcana_process.fs.stream_close :: stream :: call

record LayoutScratch:
    value: Int

record LayoutPassContext:
    buffer_version: Int
    config: arcana_text.types.LayoutConfig
    font_count: Int

record LineGlyphPiece:
    glyph: arcana_text.types.LayoutGlyph
    run: arcana_text.types.LayoutRun
    run_id: Int

record WorkingLine:
    start: Int
    end: Int
    width: Int
    height: Int
    baseline: Int
    justify: Bool
    text: Str
    glyphs: List[arcana_text.types.LayoutGlyph]
    runs: List[arcana_text.types.LayoutRun]

record LayoutStageState:
    default_style: arcana_text.types.SpanStyle
    config: arcana_text.types.LayoutConfig
    lines: List[arcana_text.types.SnapshotLine]
    runs: List[arcana_text.types.LayoutRun]
    glyphs: List[arcana_text.types.LayoutGlyph]
    unresolved: List[arcana_text.types.UnresolvedGlyph]
    fonts_used: List[arcana_text.types.FontMatch]
    width: Int
    height: Int
    signature: Int
    next_top: Int
    default_line_height: Int
    default_baseline: Int
    line: arcana_text.layout.WorkingLine
    stopped: Bool

export obj LayoutSnapshot:
    source_version: Int
    size: (Int, Int)
    signature: Int
    lines: List[arcana_text.types.SnapshotLine]
    runs: List[arcana_text.types.LayoutRun]
    glyphs: List[arcana_text.types.LayoutGlyph]
    unresolved: List[arcana_text.types.UnresolvedGlyph]
    fonts_used: List[arcana_text.types.FontMatch]

obj LayoutPassState:
    done: Bool
    buffer_version: Int
    font_count: Int
    config: arcana_text.types.LayoutConfig
    fn init(edit self: Self, read ctx: LayoutPassContext):
        self.done = false
        self.buffer_version = ctx.buffer_version
        self.font_count = ctx.font_count
        self.config = ctx.config
    fn resume(edit self: Self, read ctx: LayoutPassContext):
        self.done = false
        self.buffer_version = ctx.buffer_version
        self.font_count = ctx.font_count
        self.config = ctx.config

create LayoutPass [LayoutPassState] context: LayoutPassContext scope-exit:
    done: when LayoutPassState.done

Memory temp:layout_scratch -alloc
    capacity = 64
    reset_on = owner_exit

fn max_int(a: Int, b: Int) -> Int:
    if a >= b:
        return a
    return b

fn empty_runs() -> List[arcana_text.types.LayoutRun]:
    return std.collections.list.empty[arcana_text.types.LayoutRun] :: :: call

fn empty_glyphs() -> List[arcana_text.types.LayoutGlyph]:
    return std.collections.list.empty[arcana_text.types.LayoutGlyph] :: :: call

fn empty_lines() -> List[arcana_text.types.SnapshotLine]:
    return std.collections.list.empty[arcana_text.types.SnapshotLine] :: :: call

fn copy_run(read run: arcana_text.types.LayoutRun) -> arcana_text.types.LayoutRun:
    return run

fn copy_runs(read values: List[arcana_text.types.LayoutRun]) -> List[arcana_text.types.LayoutRun]:
    let mut out = arcana_text.layout.empty_runs :: :: call
    out :: values :: extend_list
    return out

fn copy_glyphs(read values: List[arcana_text.types.LayoutGlyph]) -> List[arcana_text.types.LayoutGlyph]:
    let mut out = arcana_text.layout.empty_glyphs :: :: call
    out :: values :: extend_list
    return out

fn copy_lines(read values: List[arcana_text.types.SnapshotLine]) -> List[arcana_text.types.SnapshotLine]:
    let mut out = arcana_text.layout.empty_lines :: :: call
    out :: values :: extend_list
    return out

fn copy_unresolved(read values: List[arcana_text.types.UnresolvedGlyph]) -> List[arcana_text.types.UnresolvedGlyph]:
    let mut out = arcana_text.shape.types.empty_unresolved :: :: call
    out :: values :: extend_list
    return out

fn copy_matches(read values: List[arcana_text.types.FontMatch]) -> List[arcana_text.types.FontMatch]:
    let mut out = arcana_text.shape.types.empty_matches :: :: call
    out :: values :: extend_list
    return out

fn copy_layout_glyph(read glyph: arcana_text.types.LayoutGlyph) -> arcana_text.types.LayoutGlyph:
    return glyph

fn empty_line_pieces() -> List[arcana_text.layout.LineGlyphPiece]:
    return std.collections.list.empty[arcana_text.layout.LineGlyphPiece] :: :: call

fn copy_run_template(read run: arcana_text.types.LayoutRun) -> arcana_text.types.LayoutRun:
    let mut next = run
    next.position = (0, 0)
    next.size = (0, 0)
    next.baseline = 0
    next.glyphs = arcana_text.layout.empty_glyphs :: :: call
    next.range = arcana_text.types.TextRange :: start = run.range.start, end = run.range.start :: call
    return next

fn collect_line_pieces(read line: arcana_text.layout.WorkingLine) -> List[arcana_text.layout.LineGlyphPiece]:
    let mut out = arcana_text.layout.empty_line_pieces :: :: call
    let mut run_id = 0
    for run in line.runs:
        let template = arcana_text.layout.copy_run_template :: run :: call
        for glyph in run.glyphs:
            out :: (arcana_text.layout.LineGlyphPiece :: glyph = (arcana_text.layout.copy_layout_glyph :: glyph :: call), run = template, run_id = run_id :: call) :: push
        run_id += 1
    return out

fn line_piece_width(read piece: arcana_text.layout.LineGlyphPiece) -> Int:
    if arcana_text.layout.layout_glyph_is_vertical :: piece.glyph :: call:
        return piece.glyph.size.0
    return piece.glyph.advance

fn line_piece_range_width(read pieces: List[arcana_text.layout.LineGlyphPiece], start: Int, end: Int) -> Int:
    let mut width = 0
    let mut index = start
    while index < end:
        width += arcana_text.layout.line_piece_width :: (pieces)[index] :: call
        index += 1
    return width

fn prefix_piece_count_within_width(read pieces: List[arcana_text.layout.LineGlyphPiece], limit: Int) -> Int:
    if limit <= 0:
        return 0
    let total = pieces :: :: len
    let mut index = 0
    let mut width = 0
    while index < total:
        let next = arcana_text.layout.line_piece_width :: (pieces)[index] :: call
        if width + next > limit:
            return index
        width += next
        index += 1
    return total

fn suffix_piece_start_within_width(read pieces: List[arcana_text.layout.LineGlyphPiece], limit: Int, minimum_start: Int) -> Int:
    if limit <= 0:
        return pieces :: :: len
    let mut start = pieces :: :: len
    let mut width = 0
    while start > minimum_start:
        let next = arcana_text.layout.line_piece_width :: (pieces)[start - 1] :: call
        if width + next > limit:
            return start
        width += next
        start -= 1
    return start

fn collect_piece_slice(read pieces: List[arcana_text.layout.LineGlyphPiece], start: Int, end: Int) -> List[arcana_text.layout.LineGlyphPiece]:
    let mut out = arcana_text.layout.empty_line_pieces :: :: call
    let mut index = start
    while index < end:
        out :: (pieces)[index] :: push
        index += 1
    return out

fn prefix_seam(read pieces: List[arcana_text.layout.LineGlyphPiece], prefix_count: Int, fallback: Int) -> Int:
    if prefix_count <= 0:
        return fallback
    return (pieces)[prefix_count - 1].glyph.cluster_range.end

fn suffix_seam(read pieces: List[arcana_text.layout.LineGlyphPiece], suffix_start: Int, fallback: Int) -> Int:
    if suffix_start >= (pieces :: :: len):
        return fallback
    return (pieces)[suffix_start].glyph.cluster_range.start

fn ellipsis_direction(read pieces: List[arcana_text.layout.LineGlyphPiece], prefix_count: Int, suffix_start: Int) -> arcana_text.types.TextDirection:
    if prefix_count > 0:
        return (pieces)[prefix_count - 1].run.direction
    if suffix_start < (pieces :: :: len):
        return (pieces)[suffix_start].run.direction
    return arcana_text.types.TextDirection.LeftToRight :: :: call

fn ellipsis_level(read pieces: List[arcana_text.layout.LineGlyphPiece], prefix_count: Int, suffix_start: Int) -> Int:
    if prefix_count > 0:
        return (pieces)[prefix_count - 1].run.bidi_level
    if suffix_start < (pieces :: :: len):
        return (pieces)[suffix_start].run.bidi_level
    return 0

fn first_strong_direction(read runs: List[arcana_text.types.LayoutRun]) -> arcana_text.types.TextDirection:
    for run in runs:
        if run.direction == (arcana_text.types.TextDirection.RightToLeft :: :: call):
            return run.direction
        if run.direction == (arcana_text.types.TextDirection.LeftToRight :: :: call):
            return run.direction
    return arcana_text.types.TextDirection.LeftToRight :: :: call

fn append_runs_reversed(edit out: List[arcana_text.types.LayoutRun], read values: List[arcana_text.types.LayoutRun]):
    let mut copy = arcana_text.layout.empty_runs :: :: call
    copy :: values :: extend_list
    while not (copy :: :: is_empty):
        out :: (copy :: :: pop) :: push

fn run_at(read values: List[arcana_text.types.LayoutRun], target: Int) -> arcana_text.types.LayoutRun:
    let mut index = 0
    for value in values:
        if index == target:
            return arcana_text.layout.copy_run :: value :: call
        index += 1
    let mut run = arcana_text.types.LayoutRun :: range = (arcana_text.types.TextRange :: start = 0, end = 0 :: call), position = (0, 0), size = (0, 0) :: call
    run.kind = arcana_text.types.ShapedRunKind.Text :: :: call
    run.direction = arcana_text.types.TextDirection.LeftToRight :: :: call
    run.script = arcana_text.types.ScriptClass.Common :: :: call
    run.bidi_level = 0
    run.language_tag = ""
    run.color = 0
    run.baseline = 0
    run.font_size = 0
    run.line_height_milli = 0
    run.underline = arcana_text.types.UnderlineStyle.None :: :: call
    run.underline_color_enabled = false
    run.underline_color = 0
    run.strikethrough_enabled = false
    run.strikethrough_color_enabled = false
    run.strikethrough_color = 0
    run.overline_enabled = false
    run.overline_color_enabled = false
    run.overline_color = 0
    run.family = ""
    run.face_id = arcana_text.types.FontFaceId :: source_index = -1, face_index = -1 :: call
    run.glyphs = arcana_text.layout.empty_glyphs :: :: call
    run.placeholder = Option.None[arcana_text.types.PlaceholderSpec] :: :: call
    return run

fn reverse_run_segment(read values: List[arcana_text.types.LayoutRun], start: Int, end: Int) -> List[arcana_text.types.LayoutRun]:
    let mut out = arcana_text.layout.empty_runs :: :: call
    let total = values :: :: len
    let mut index = 0
    while index < start:
        out :: (arcana_text.layout.run_at :: values, index :: call) :: push
        index += 1
    let mut reverse = end - 1
    while reverse >= start:
        out :: (arcana_text.layout.run_at :: values, reverse :: call) :: push
        reverse -= 1
    index = end
    while index < total:
        out :: (arcana_text.layout.run_at :: values, index :: call) :: push
        index += 1
    return out

fn max_bidi_level(read runs: List[arcana_text.types.LayoutRun]) -> Int:
    let mut level = 0
    for run in runs:
        if run.bidi_level > level:
            level = run.bidi_level
    return level

fn lowest_odd_bidi_level(read runs: List[arcana_text.types.LayoutRun]) -> Int:
    let mut found = false
    let mut level = 0
    for run in runs:
        if (run.bidi_level % 2) == 1:
            if not found or run.bidi_level < level:
                level = run.bidi_level
                found = true
    if found:
        return level
    return 1

fn visual_runs(read runs: List[arcana_text.types.LayoutRun]) -> List[arcana_text.types.LayoutRun]:
    let mut out = arcana_text.layout.copy_runs :: runs :: call
    let mut level = arcana_text.layout.max_bidi_level :: out :: call
    let stop = arcana_text.layout.lowest_odd_bidi_level :: out :: call
    while level >= stop:
        let total = out :: :: len
        let mut index = 0
        while index < total:
            while index < total and ((out)[index].bidi_level < level):
                index += 1
            let start = index
            while index < total and ((out)[index].bidi_level >= level):
                index += 1
            if index - start > 1:
                out = arcana_text.layout.reverse_run_segment :: out, start, index :: call
        level -= 1
    return out

fn working_line(start: Int) -> arcana_text.layout.WorkingLine:
    let mut line = arcana_text.layout.WorkingLine :: start = start, end = start, width = 0 :: call
    line.height = 0
    line.baseline = 0
    line.justify = false
    line.text = ""
    line.glyphs = arcana_text.layout.empty_glyphs :: :: call
    line.runs = arcana_text.layout.empty_runs :: :: call
    return line

fn seed_state(read shaped: arcana_text.shape.types.ShapeSnapshot, read config: arcana_text.types.LayoutConfig) -> arcana_text.layout.LayoutStageState:
    let mut state = arcana_text.layout.LayoutStageState :: default_style = shaped.default_style, config = config :: call
    state.lines = std.collections.list.empty[arcana_text.types.SnapshotLine] :: :: call
    state.runs = arcana_text.layout.empty_runs :: :: call
    state.glyphs = arcana_text.layout.empty_glyphs :: :: call
    state.unresolved = shaped.unresolved
    state.fonts_used = shaped.fonts_used
    state.width = 0
    state.height = 0
    state.signature = shaped.signature
    state.next_top = 0
    state.default_line_height = shaped.default_line_height
    state.default_baseline = shaped.default_baseline
    state.line = arcana_text.layout.working_line :: 0 :: call
    state.stopped = false
    return state

fn seed_state_from_buffer(edit fonts: arcana_text.fonts.FontSystem, read buffer: arcana_text.buffer.TextBuffer, read config: arcana_text.types.LayoutConfig) -> arcana_text.layout.LayoutStageState:
    let default_style = arcana_text.shape.styles.span_style_from_text_style :: buffer.style :: call
    let first_visible = arcana_text.shape.tokens.first_visible_char :: buffer.text :: call
    let matched = fonts :: buffer.style, first_visible :: resolve_style_char
    let line_height = fonts :: matched, buffer.style :: line_height
    let baseline = fonts :: matched, buffer.style :: baseline
    let mut state = arcana_text.layout.LayoutStageState :: default_style = default_style, config = config :: call
    state.lines = std.collections.list.empty[arcana_text.types.SnapshotLine] :: :: call
    state.runs = arcana_text.layout.empty_runs :: :: call
    state.glyphs = arcana_text.layout.empty_glyphs :: :: call
    state.unresolved = arcana_text.shape.types.empty_unresolved :: :: call
    state.fonts_used = arcana_text.shape.types.empty_matches :: :: call
    state.width = 0
    state.height = 0
    state.signature = 23
    state.next_top = 0
    state.default_line_height = arcana_text.shape.types.max_int :: line_height, (buffer.style.size + 6) :: call
    state.default_baseline = arcana_text.shape.types.max_int :: baseline, buffer.style.size :: call
    state.config.max_lines = arcana_text.layout.effective_max_lines :: config, state.default_line_height :: call
    state.line = arcana_text.layout.working_line :: 0 :: call
    state.stopped = false
    return state

fn bool_code(value: Bool) -> Int:
    return match value:
        true => 1
        false => 0

fn align_code(read value: arcana_text.types.TextAlign) -> Int:
    return match value:
        arcana_text.types.TextAlign.Center => 2
        arcana_text.types.TextAlign.Right => 3
        arcana_text.types.TextAlign.Justified => 4
        arcana_text.types.TextAlign.End => 5
        _ => 1

fn wrap_code(read value: arcana_text.types.TextWrap) -> Int:
    return match value:
        arcana_text.types.TextWrap.NoWrap => 2
        arcana_text.types.TextWrap.Glyph => 3
        arcana_text.types.TextWrap.WordOrGlyph => 4
        _ => 1

fn ellipsize_mode_code(read value: arcana_text.types.EllipsizeMode) -> Int:
    return match value:
        arcana_text.types.EllipsizeMode.Start => 2
        arcana_text.types.EllipsizeMode.Middle => 3
        arcana_text.types.EllipsizeMode.End => 4
        _ => 1

fn ellipsize_limit_code(read value: arcana_text.types.EllipsizeHeightLimit) -> Int:
    let mut code = match value.kind:
        arcana_text.types.EllipsizeLimitKind.Lines => 2
        arcana_text.types.EllipsizeLimitKind.Height => 3
        _ => 1
    return arcana_text.shape.types.mix_signature :: code, value.value :: call

fn hinting_code(read value: arcana_text.types.Hinting) -> Int:
    return match value:
        arcana_text.types.Hinting.Enabled => 2
        _ => 1

fn effective_max_lines(read config: arcana_text.types.LayoutConfig, default_line_height: Int) -> Int:
    if config.ellipsize_limit.kind == (arcana_text.types.EllipsizeLimitKind.Lines :: :: call) and config.ellipsize_limit.value > 0:
        return config.ellipsize_limit.value
    if config.ellipsize_limit.kind == (arcana_text.types.EllipsizeLimitKind.Height :: :: call) and config.ellipsize_limit.value > 0:
        let safe_line_height = arcana_text.layout.max_int :: default_line_height, 1 :: call
        let lines = config.ellipsize_limit.value / safe_line_height
        return arcana_text.layout.max_int :: lines, 1 :: call
    return config.max_lines

fn clamp_max_lines(limit: Int, remaining_lines: Int) -> Int:
    if remaining_lines <= 0:
        return limit
    if limit <= 0 or limit > remaining_lines:
        return remaining_lines
    return limit

fn direction_code(read value: arcana_text.types.TextDirection) -> Int:
    return match value:
        arcana_text.types.TextDirection.RightToLeft => 2
        _ => 1

fn run_kind_code(read value: arcana_text.types.ShapedRunKind) -> Int:
    return match value:
        arcana_text.types.ShapedRunKind.Placeholder => 2
        _ => 1

fn underline_code(read value: arcana_text.types.UnderlineStyle) -> Int:
    return match value:
        arcana_text.types.UnderlineStyle.Single => 2
        arcana_text.types.UnderlineStyle.Double => 3
        _ => 1

fn script_code(read value: arcana_text.types.ScriptClass) -> Int:
    return match value:
        arcana_text.types.ScriptClass.Common => 2
        arcana_text.types.ScriptClass.Latin => 3
        arcana_text.types.ScriptClass.Cyrillic => 4
        arcana_text.types.ScriptClass.Arabic => 5
        arcana_text.types.ScriptClass.Hebrew => 6
        arcana_text.types.ScriptClass.Han => 7
        arcana_text.types.ScriptClass.Hangul => 8
        arcana_text.types.ScriptClass.Devanagari => 9
        arcana_text.types.ScriptClass.Adlam => 10
        arcana_text.types.ScriptClass.Bengali => 11
        arcana_text.types.ScriptClass.Bopomofo => 12
        arcana_text.types.ScriptClass.CanadianAboriginal => 13
        arcana_text.types.ScriptClass.Chakma => 14
        arcana_text.types.ScriptClass.Cherokee => 15
        arcana_text.types.ScriptClass.Ethiopic => 16
        arcana_text.types.ScriptClass.Gujarati => 17
        arcana_text.types.ScriptClass.Gurmukhi => 18
        arcana_text.types.ScriptClass.Hiragana => 19
        arcana_text.types.ScriptClass.Katakana => 20
        arcana_text.types.ScriptClass.Javanese => 21
        arcana_text.types.ScriptClass.Kannada => 22
        arcana_text.types.ScriptClass.Khmer => 23
        arcana_text.types.ScriptClass.Lao => 24
        arcana_text.types.ScriptClass.Malayalam => 25
        arcana_text.types.ScriptClass.Mongolian => 26
        arcana_text.types.ScriptClass.Myanmar => 27
        arcana_text.types.ScriptClass.Oriya => 28
        arcana_text.types.ScriptClass.Sinhala => 29
        arcana_text.types.ScriptClass.Tamil => 30
        arcana_text.types.ScriptClass.Telugu => 31
        arcana_text.types.ScriptClass.Thaana => 32
        arcana_text.types.ScriptClass.Thai => 33
        arcana_text.types.ScriptClass.Tibetan => 34
        arcana_text.types.ScriptClass.Tifinagh => 35
        arcana_text.types.ScriptClass.Vai => 36
        arcana_text.types.ScriptClass.Yi => 37
        _ => 1

fn prepared_layout_line_key(read state: arcana_text.layout.LayoutStageState, read prepared_runs: List[arcana_text.types.PreparedRun], remaining_lines: Int) -> Str:
    let mut signature = 61
    signature = arcana_text.shape.types.mix_signature :: signature, state.config.max_width :: call
    signature = arcana_text.shape.types.mix_signature :: signature, state.config.tab_width :: call
    signature = arcana_text.shape.types.mix_signature :: signature, state.default_line_height :: call
    signature = arcana_text.shape.types.mix_signature :: signature, state.default_baseline :: call
    signature = arcana_text.shape.types.mix_signature :: signature, remaining_lines :: call
    signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.align_code :: state.config.align :: call) :: call
    signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.wrap_code :: state.config.wrap :: call) :: call
    signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.ellipsize_mode_code :: state.config.ellipsize_mode :: call) :: call
    signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.ellipsize_limit_code :: state.config.ellipsize_limit :: call) :: call
    signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.hinting_code :: state.config.hinting :: call) :: call
    signature = arcana_text.shape.types.mix_signature_text :: signature, state.config.ellipsis :: call
    for prepared in prepared_runs:
        let run = prepared.run
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.run_kind_code :: run.kind :: call) :: call
        signature = arcana_text.shape.types.mix_signature :: signature, run.range.start :: call
        signature = arcana_text.shape.types.mix_signature :: signature, run.range.end :: call
        signature = arcana_text.shape.types.mix_signature :: signature, run.width :: call
        signature = arcana_text.shape.types.mix_signature :: signature, run.match.id.source_index :: call
        signature = arcana_text.shape.types.mix_signature :: signature, run.match.id.face_index :: call
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.direction_code :: run.direction :: call) :: call
        signature = arcana_text.shape.types.mix_signature :: signature, run.bidi_level :: call
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.script_code :: run.script :: call) :: call
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.bool_code :: run.whitespace :: call) :: call
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.bool_code :: run.hard_break :: call) :: call
        signature = arcana_text.shape.types.mix_signature :: signature, run.style.color :: call
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.bool_code :: run.style.background_enabled :: call) :: call
        signature = arcana_text.shape.types.mix_signature :: signature, run.style.background_color :: call
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.bool_code :: run.style.underline_color_enabled :: call) :: call
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.underline_code :: run.style.underline :: call) :: call
        signature = arcana_text.shape.types.mix_signature :: signature, run.style.underline_color :: call
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.bool_code :: run.style.strikethrough_enabled :: call) :: call
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.bool_code :: run.style.strikethrough_color_enabled :: call) :: call
        signature = arcana_text.shape.types.mix_signature :: signature, run.style.strikethrough_color :: call
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.bool_code :: run.style.overline_enabled :: call) :: call
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.layout.bool_code :: run.style.overline_color_enabled :: call) :: call
        signature = arcana_text.shape.types.mix_signature :: signature, run.style.overline_color :: call
        signature = arcana_text.shape.types.mix_signature :: signature, run.style.size :: call
        signature = arcana_text.shape.types.mix_signature :: signature, run.style.letter_spacing :: call
        signature = arcana_text.shape.types.mix_signature :: signature, run.style.line_height :: call
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.types.feature_signature :: run.style.features :: call) :: call
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.types.axis_signature :: run.style.axes :: call) :: call
        signature = arcana_text.shape.types.mix_signature_text :: signature, run.text :: call
        signature = arcana_text.shape.types.mix_signature_text :: signature, run.language_tag :: call
        for family in run.style.families:
            signature = arcana_text.shape.types.mix_signature_text :: signature, family :: call
        for glyph in run.glyphs:
            signature = arcana_text.shape.types.mix_signature :: signature, glyph.glyph_index :: call
            signature = arcana_text.shape.types.mix_signature :: signature, glyph.advance :: call
            signature = arcana_text.shape.types.mix_signature :: signature, glyph.x_advance :: call
            signature = arcana_text.shape.types.mix_signature :: signature, glyph.y_advance :: call
            signature = arcana_text.shape.types.mix_signature :: signature, glyph.range.start :: call
            signature = arcana_text.shape.types.mix_signature :: signature, glyph.range.end :: call
            signature = arcana_text.shape.types.mix_signature :: signature, glyph.offset.0 :: call
            signature = arcana_text.shape.types.mix_signature :: signature, glyph.offset.1 :: call
            signature = arcana_text.shape.types.mix_signature_text :: signature, glyph.glyph :: call
    return (std.text.from_int :: signature :: call) + ":" + (std.text.from_int :: (prepared_runs :: :: len) :: call)

fn line_end_for_prepared_runs(read prepared_runs: List[arcana_text.types.PreparedRun]) -> Int:
    let mut end = 0
    for prepared in prepared_runs:
        if prepared.run.range.end > end:
            end = prepared.run.range.end
    return end

fn seed_line_state(read base: arcana_text.layout.LayoutStageState, line_start: Int, remaining_lines: Int) -> arcana_text.layout.LayoutStageState:
    let mut config = base.config
    if config.max_lines > 0:
        config.max_lines = remaining_lines
    let mut state = arcana_text.layout.LayoutStageState :: default_style = base.default_style, config = config :: call
    state.lines = arcana_text.layout.empty_lines :: :: call
    state.runs = arcana_text.layout.empty_runs :: :: call
    state.glyphs = arcana_text.layout.empty_glyphs :: :: call
    state.unresolved = arcana_text.shape.types.empty_unresolved :: :: call
    state.fonts_used = arcana_text.shape.types.empty_matches :: :: call
    state.width = 0
    state.height = 0
    state.signature = 23
    state.next_top = 0
    state.default_line_height = base.default_line_height
    state.default_baseline = base.default_baseline
    state.config.max_lines = arcana_text.layout.clamp_max_lines :: (arcana_text.layout.effective_max_lines :: config, state.default_line_height :: call), remaining_lines :: call
    state.line = arcana_text.layout.working_line :: line_start :: call
    state.stopped = false
    return state

fn prepared_layout_line(read state: arcana_text.layout.LayoutStageState, start: Int, end: Int) -> arcana_text.types.PreparedLayoutLine:
    let mut out = arcana_text.types.PreparedLayoutLine :: start = start, end = end, size = (state.width, state.height) :: call
    out.signature = state.signature
    out.stopped = state.stopped
    out.lines = arcana_text.layout.copy_lines :: state.lines :: call
    out.runs = arcana_text.layout.copy_runs :: state.runs :: call
    out.glyphs = arcana_text.layout.copy_glyphs :: state.glyphs :: call
    out.unresolved = arcana_text.layout.copy_unresolved :: state.unresolved :: call
    out.fonts_used = arcana_text.layout.copy_matches :: state.fonts_used :: call
    return out

fn prepare_layout_line(edit fonts: arcana_text.fonts.FontSystem, read payload: (arcana_text.layout.LayoutStageState, List[arcana_text.types.PreparedRun], Int)) -> arcana_text.types.PreparedLayoutLine:
    let base = payload.0
    let prepared_runs = payload.1
    let remaining_lines = payload.2
    let line_start = match prepared_runs :: :: is_empty:
        true => 0
        false => prepared_runs[0].run.range.start
    let mut state = arcana_text.layout.seed_line_state :: base, line_start, remaining_lines :: call
    let line_end = arcana_text.layout.line_end_for_prepared_runs :: prepared_runs :: call
    for prepared in prepared_runs:
        arcana_text.layout.append_prepared_run :: fonts, state, prepared :: call
        if state.stopped:
            break
    if (state.lines :: :: is_empty) or not (state.line.glyphs :: :: is_empty) or state.line.start < line_end:
        arcana_text.layout.finalize_line :: state :: call
    return arcana_text.layout.prepared_layout_line :: state, line_start, state.line.start :: call

fn record_prepared_run(edit state: arcana_text.layout.LayoutStageState, read prepared: arcana_text.types.PreparedRun):
    let run = prepared.run
    state.signature = arcana_text.shape.types.mix_signature :: state.signature, run.range.start :: call
    state.signature = arcana_text.shape.types.mix_signature :: state.signature, run.range.end :: call
    state.signature = arcana_text.shape.types.mix_signature :: state.signature, run.width :: call
    if run.whitespace:
        state.signature = arcana_text.shape.types.mix_signature :: state.signature, 3 :: call
    if run.hard_break:
        state.signature = arcana_text.shape.types.mix_signature :: state.signature, 7 :: call
    state.signature = arcana_text.shape.types.mix_signature_text :: state.signature, run.text :: call
    for glyph in run.glyphs:
        state.signature = arcana_text.shape.types.mix_signature :: state.signature, glyph.glyph_index :: call
        state.signature = arcana_text.shape.types.mix_signature :: state.signature, glyph.advance :: call
        state.signature = arcana_text.shape.types.mix_signature_text :: state.signature, glyph.glyph :: call
    arcana_text.shape.types.push_unique_match :: state.fonts_used, run.match :: call
    state.unresolved :: prepared.unresolved :: extend_list

fn append_prepared_run(edit fonts: arcana_text.fonts.FontSystem, edit state: arcana_text.layout.LayoutStageState, read prepared: arcana_text.types.PreparedRun):
    arcana_text.layout.record_prepared_run :: state, prepared :: call
    let run = prepared.run
    arcana_text.layout.process_run :: fonts, state, run :: call

fn line_start_x(read state: arcana_text.layout.LayoutStageState, line_width: Int) -> Int:
    if state.config.max_width <= 0:
        return 0
    let remaining = arcana_text.layout.max_int :: (state.config.max_width - line_width), 0 :: call
    let rtl = (arcana_text.layout.first_strong_direction :: state.line.runs :: call) == (arcana_text.types.TextDirection.RightToLeft :: :: call)
    return match state.config.align:
        arcana_text.types.TextAlign.Left => match rtl:
            true => remaining
            false => 0
        arcana_text.types.TextAlign.Center => remaining / 2
        arcana_text.types.TextAlign.Right => match rtl:
            true => 0
            false => remaining
        arcana_text.types.TextAlign.End => remaining
        _ => 0

fn trim_line_text(edit state: arcana_text.layout.LayoutStageState, count: Int):
    if count <= 0:
        return
    let total = std.text.len_bytes :: state.line.text :: call
    let mut next_end = total - count
    if next_end < 0:
        next_end = 0
    state.line.text = std.text.slice_bytes :: state.line.text, 0, next_end :: call

fn pop_glyph(edit state: arcana_text.layout.LayoutStageState):
    if state.line.glyphs :: :: is_empty:
        return
    let removed = state.line.glyphs :: :: pop
    state.line.width -= removed.advance
    if state.line.width < 0:
        state.line.width = 0
    state.line.end = removed.range.start
    arcana_text.layout.trim_line_text :: state, (std.text.len_bytes :: removed.glyph :: call) :: call
    if not (state.line.runs :: :: is_empty):
        let mut last_run = state.line.runs :: :: pop
        if not (last_run.glyphs :: :: is_empty):
            let _ = last_run.glyphs :: :: pop
        if not (last_run.glyphs :: :: is_empty):
            let mut run_height = 0
            let mut run_end = last_run.range.start
            let mut run_width = 0
            for glyph in last_run.glyphs:
                if glyph.size.1 > run_height:
                    run_height = glyph.size.1
                run_end = glyph.range.end
                run_width += glyph.advance
            last_run.range = arcana_text.types.TextRange :: start = last_run.range.start, end = run_end :: call
            last_run.size = (run_width, run_height)
            state.line.runs :: last_run :: push

fn wrap_prefers_glyph(read config: arcana_text.types.LayoutConfig, run_width: Int) -> Bool:
    if config.wrap == (arcana_text.types.TextWrap.Glyph :: :: call):
        return true
    if config.wrap == (arcana_text.types.TextWrap.WordOrGlyph :: :: call) and config.max_width > 0 and run_width > config.max_width:
        return true
    return false

fn layout_needs_wrap(read state: arcana_text.layout.LayoutStageState, advance: Int) -> Bool:
    if state.config.wrap == (arcana_text.types.TextWrap.NoWrap :: :: call):
        return false
    return state.config.max_width > 0 and not (state.line.glyphs :: :: is_empty) and state.line.width + advance > state.config.max_width

fn empty_layout_run_from_shaped(edit fonts: arcana_text.fonts.FontSystem, edit state: arcana_text.layout.LayoutStageState, read shaped: arcana_text.types.ShapedRun) -> arcana_text.types.LayoutRun:
    let mut run = arcana_text.types.LayoutRun :: range = (arcana_text.types.TextRange :: start = shaped.range.start, end = shaped.range.start :: call), position = (state.line.width, 0), size = (0, 0) :: call
    run.kind = shaped.kind
    run.direction = shaped.direction
    run.script = shaped.script
    run.bidi_level = shaped.bidi_level
    run.language_tag = shaped.language_tag
    run.color = shaped.style.color
    run.baseline = 0
    run.font_size = shaped.style.size
    if shaped.style.line_height > 0 and shaped.style.size > 0:
        run.line_height_milli = (shaped.style.line_height * 1000) / shaped.style.size
    else:
        run.line_height_milli = 1000
    run.underline = shaped.style.underline
    run.underline_color_enabled = shaped.style.underline_color_enabled
    run.underline_color = shaped.style.underline_color
    run.strikethrough_enabled = shaped.style.strikethrough_enabled
    run.strikethrough_color_enabled = shaped.style.strikethrough_color_enabled
    run.strikethrough_color = shaped.style.strikethrough_color
    run.overline_enabled = shaped.style.overline_enabled
    run.overline_color_enabled = shaped.style.overline_color_enabled
    run.overline_color = shaped.style.overline_color
    run.family = arcana_text.fonts.match_family_or_label :: fonts, shaped.match :: call
    run.face_id = shaped.match.id
    run.glyphs = arcana_text.layout.empty_glyphs :: :: call
    run.placeholder = shaped.placeholder
    return run

fn append_wrapped_layout_run(edit fonts: arcana_text.fonts.FontSystem, edit state: arcana_text.layout.LayoutStageState, read shaped: arcana_text.types.ShapedRun):
    let vertical_run = arcana_text.layout.shaped_run_is_vertical :: shaped :: call
    if vertical_run:
        arcana_text.layout.append_layout_run :: fonts, state, shaped :: call
        return
    let line_cap_reached = state.config.max_lines > 0 and (state.lines :: :: len) >= state.config.max_lines - 1
    let mut run = arcana_text.layout.empty_layout_run_from_shaped :: fonts, state, shaped :: call
    for glyph in shaped.glyphs:
        let glyph_width = arcana_text.layout.shaped_display_advance :: state.line.width, glyph, state.config :: call
        let glyph_wraps = arcana_text.layout.layout_needs_wrap :: state, glyph_width :: call
        if line_cap_reached and glyph_wraps:
            arcana_text.layout.append_ellipsis :: fonts, state :: call
            return
        if glyph_wraps:
            state.line.justify = state.config.align == (arcana_text.types.TextAlign.Justified :: :: call)
            if not (run.glyphs :: :: is_empty):
                state.line.runs :: run :: push
            arcana_text.layout.finalize_line :: state :: call
            run = arcana_text.layout.empty_layout_run_from_shaped :: fonts, state, shaped :: call
        arcana_text.layout.append_shaped_glyph :: state, run, (glyph, shaped.style, false) :: call
    if not (run.glyphs :: :: is_empty):
        state.line.runs :: run :: push

fn justifyable_glyph_count(read runs: List[arcana_text.types.LayoutRun]) -> Int:
    let mut count = 0
    for run in runs:
        for glyph in run.glyphs:
            if glyph.glyph == " " or glyph.glyph == "\t":
                count += 1
    return count

fn justify_extra_for_slot(extra: Int, slot_count: Int, slot_index: Int) -> Int:
    if extra <= 0 or slot_count <= 0:
        return 0
    let base = extra / slot_count
    let remainder = extra % slot_count
    if slot_index < remainder:
        return base + 1
    return base

fn tab_cell_width(read shaped: arcana_text.types.ShapedGlyph) -> Int:
    let from_glyph = arcana_text.layout.max_int :: shaped.x_advance, shaped.advance :: call
    let fallback = arcana_text.layout.max_int :: (shaped.font_size / 2), 1 :: call
    return arcana_text.layout.max_int :: from_glyph, fallback :: call

fn hinting_enabled(read config: arcana_text.types.LayoutConfig) -> Bool:
    return config.hinting == (arcana_text.types.Hinting.Enabled :: :: call)

fn shaped_primary_advance(read shaped: arcana_text.types.ShapedGlyph, read config: arcana_text.types.LayoutConfig) -> Int:
    if arcana_text.layout.hinting_enabled :: config :: call:
        if shaped.y_advance > 0 and shaped.x_advance == 0:
            return shaped.y_advance
        if shaped.x_advance > 0:
            return shaped.x_advance
    return shaped.advance

fn layout_primary_advance(read glyph: arcana_text.types.LayoutGlyph, read config: arcana_text.types.LayoutConfig) -> Int:
    if arcana_text.layout.hinting_enabled :: config :: call:
        if glyph.y_advance > 0 and glyph.x_advance == 0:
            return glyph.y_advance
        if glyph.x_advance > 0:
            return glyph.x_advance
    return glyph.advance

fn shaped_run_is_vertical(read run: arcana_text.types.ShapedRun) -> Bool:
    for glyph in run.glyphs:
        if glyph.y_advance > 0 and glyph.x_advance == 0:
            return true
        if glyph.x_advance > 0:
            return false
    return false

fn layout_run_is_vertical(read run: arcana_text.types.LayoutRun) -> Bool:
    for glyph in run.glyphs:
        if glyph.y_advance > 0 and glyph.x_advance == 0:
            return true
        if glyph.x_advance > 0:
            return false
    return false

fn vertical_glyph_width(read shaped: arcana_text.types.ShapedGlyph) -> Int:
    let ink = arcana_text.layout.max_int :: shaped.ink_size.0, 0 :: call
    let fallback = arcana_text.layout.max_int :: (shaped.font_size / 2), 1 :: call
    return arcana_text.layout.max_int :: ink, fallback :: call

fn shaped_display_advance(progress: Int, read shaped: arcana_text.types.ShapedGlyph, read config: arcana_text.types.LayoutConfig) -> Int:
    if shaped.glyph != "\t":
        return arcana_text.layout.shaped_primary_advance :: shaped, config :: call
    let tab_width = arcana_text.layout.max_int :: config.tab_width, 1 :: call
    let stop = (arcana_text.layout.tab_cell_width :: shaped :: call) * tab_width
    if stop <= 0:
        return shaped.advance
    let used = progress % stop
    if used == 0:
        return stop
    return stop - used

fn run_display_width(read state: arcana_text.layout.LayoutStageState, read run: arcana_text.types.ShapedRun) -> Int:
    if arcana_text.layout.shaped_run_is_vertical :: run :: call:
        let mut width = 0
        let mut progress = 0
        for glyph in run.glyphs:
            let glyph_width = arcana_text.layout.vertical_glyph_width :: glyph :: call
            if glyph_width > width:
                width = glyph_width
            progress += arcana_text.layout.shaped_display_advance :: progress, glyph, state.config :: call
        return width
    let mut total = 0
    for glyph in run.glyphs:
        total += arcana_text.layout.shaped_display_advance :: state.line.width + total, glyph, state.config :: call
    return total

fn append_shaped_glyph(edit state: arcana_text.layout.LayoutStageState, edit run: arcana_text.types.LayoutRun, read payload: (arcana_text.types.ShapedGlyph, arcana_text.types.SpanStyle, Bool)):
    layout_probe_append :: ("append_shaped_glyph:start glyph=" + payload.0.glyph + " index=" + (std.text.from_int :: payload.0.glyph_index :: call)) :: call
    let shaped = payload.0
    let style = payload.1
    let vertical = payload.2
    let progress = match vertical:
        true => run.size.1
        false => state.line.width
    let display_advance = arcana_text.layout.shaped_display_advance :: progress, shaped, state.config :: call
    if shaped.line_height > state.line.height:
        state.line.height = shaped.line_height
    if shaped.baseline > state.line.baseline:
        state.line.baseline = shaped.baseline
    let glyph_position = match vertical:
        true => (run.position.0, run.size.1)
        false => (state.line.width, 0)
    let mut glyph = arcana_text.types.LayoutGlyph :: glyph = shaped.glyph, range = shaped.range, position = glyph_position :: call
    glyph.cluster_range = shaped.cluster_range
    glyph.size = match vertical:
        true => ((arcana_text.layout.vertical_glyph_width :: shaped :: call), display_advance)
        false => (display_advance, shaped.line_height)
    glyph.advance = display_advance
    glyph.x_advance = match vertical:
        true => 0
        false => display_advance
    glyph.y_advance = match vertical:
        true => display_advance
        false => shaped.y_advance
    glyph.offset = shaped.offset
    glyph.color = style.color
    glyph.background_enabled = style.background_enabled
    glyph.background_color = style.background_color
    glyph.family = shaped.family
    glyph.face_id = shaped.face_id
    glyph.glyph_index = shaped.glyph_index
    glyph.line_index = 0
    glyph.direction = run.direction
    glyph.baseline = shaped.baseline
    glyph.font_size = shaped.font_size
    glyph.line_height_milli = shaped.line_height_milli
    glyph.weight = shaped.weight
    glyph.width_milli = shaped.width_milli
    glyph.slant_milli = shaped.slant_milli
    glyph.feature_signature = shaped.feature_signature
    glyph.axis_signature = shaped.axis_signature
    glyph.ink_offset = shaped.ink_offset
    glyph.ink_size = shaped.ink_size
    glyph.caret_stop_before = shaped.caret_stop_before
    glyph.caret_stop_after = shaped.caret_stop_after
    glyph.empty = shaped.empty
    let mut glyph_for_run = record yield arcana_text.types.LayoutGlyph from glyph -return 0
        glyph = glyph.glyph
    state.line.glyphs :: glyph :: push
    run.glyphs :: glyph_for_run :: push
    state.line.text = state.line.text + shaped.glyph
    state.line.end = shaped.range.end
    if vertical:
        let run_width = arcana_text.layout.max_int :: run.size.0, glyph.size.0 :: call
        run.size = (run_width, run.size.1 + display_advance)
        if run.size.1 > state.line.height:
            state.line.height = run.size.1
    else:
        state.line.width += display_advance
        let run_height = arcana_text.layout.max_int :: run.size.1, shaped.line_height :: call
        run.size = (run.size.0 + display_advance, run_height)
    layout_probe_append :: ("append_shaped_glyph:done width=" + (std.text.from_int :: state.line.width :: call)) :: call

fn append_layout_run(edit fonts: arcana_text.fonts.FontSystem, edit state: arcana_text.layout.LayoutStageState, read shaped: arcana_text.types.ShapedRun):
    layout_probe_append :: ("append_layout_run:start start=" + (std.text.from_int :: shaped.range.start :: call) + " end=" + (std.text.from_int :: shaped.range.end :: call) + " glyphs=" + (std.text.from_int :: (shaped.glyphs :: :: len) :: call)) :: call
    if shaped.hard_break:
        let line_cap_reached = state.config.max_lines > 0 and (state.lines :: :: len) >= state.config.max_lines - 1
        if line_cap_reached:
            state.stopped = true
            return
        if state.line.end < shaped.range.start:
            state.line.end = shaped.range.start
        arcana_text.layout.finalize_line :: state :: call
        state.line.start = shaped.range.end
        state.line.end = shaped.range.end
        return
    if shaped.whitespace and (state.line.glyphs :: :: is_empty):
        state.line.start = shaped.range.start
        return
    let line_cap_reached = state.config.max_lines > 0 and (state.lines :: :: len) >= state.config.max_lines - 1
    let display_width = arcana_text.layout.run_display_width :: state, shaped :: call
    let shaped_wraps = arcana_text.layout.layout_needs_wrap :: state, display_width :: call
    if line_cap_reached and shaped_wraps:
        state.stopped = true
        return
    let vertical_run = arcana_text.layout.shaped_run_is_vertical :: shaped :: call
    if (arcana_text.layout.wrap_prefers_glyph :: state.config, display_width :: call) and not vertical_run:
        arcana_text.layout.append_wrapped_layout_run :: fonts, state, shaped :: call
        layout_probe_append :: ("append_layout_run:wrapped line_runs=" + (std.text.from_int :: (state.line.runs :: :: len) :: call)) :: call
        return
    if shaped_wraps:
        state.line.justify = state.config.align == (arcana_text.types.TextAlign.Justified :: :: call)
        arcana_text.layout.finalize_line :: state :: call
    let mut run = arcana_text.layout.empty_layout_run_from_shaped :: fonts, state, shaped :: call
    run.range = shaped.range
    for glyph in shaped.glyphs:
        layout_probe_append :: ("append_layout_run:glyph index=" + (std.text.from_int :: glyph.glyph_index :: call)) :: call
        arcana_text.layout.append_shaped_glyph :: state, run, (glyph, shaped.style, vertical_run) :: call
    if run.glyphs :: :: is_empty:
        return
    if vertical_run:
        state.line.width += run.size.0
        if run.size.1 > state.line.height:
            state.line.height = run.size.1
    state.line.runs :: run :: push
    layout_probe_append :: ("append_layout_run:done line_runs=" + (std.text.from_int :: (state.line.runs :: :: len) :: call)) :: call

fn layout_glyph_is_vertical(read glyph: arcana_text.types.LayoutGlyph) -> Bool:
    return glyph.y_advance > 0 and glyph.x_advance == 0

fn append_piece_glyph(edit state: arcana_text.layout.LayoutStageState, edit run: arcana_text.types.LayoutRun, read source: arcana_text.types.LayoutGlyph):
    let vertical = arcana_text.layout.layout_glyph_is_vertical :: source :: call
    let display_advance = match vertical:
        true => arcana_text.layout.layout_primary_advance :: source, state.config :: call
        false => arcana_text.layout.layout_primary_advance :: source, state.config :: call
    let glyph_position = match vertical:
        true => (run.position.0, run.size.1)
        false => (state.line.width, 0)
    let first_glyph = run.glyphs :: :: is_empty
    let mut glyph = source
    glyph.position = glyph_position
    glyph.line_index = 0
    state.line.glyphs :: glyph :: push
    run.glyphs :: glyph :: push
    if first_glyph:
        run.range = arcana_text.types.TextRange :: start = source.range.start, end = source.range.end :: call
    else:
        run.range = arcana_text.types.TextRange :: start = run.range.start, end = source.range.end :: call
    state.line.text = state.line.text + source.glyph
    if source.baseline > state.line.baseline:
        state.line.baseline = source.baseline
    if vertical:
        let run_width = arcana_text.layout.max_int :: run.size.0, source.size.0 :: call
        run.size = (run_width, run.size.1 + display_advance)
        if run.size.1 > state.line.height:
            state.line.height = run.size.1
    else:
        state.line.width += display_advance
        let run_height = arcana_text.layout.max_int :: run.size.1, source.size.1 :: call
        run.size = (run.size.0 + display_advance, run_height)
        if source.size.1 > state.line.height:
            state.line.height = source.size.1

fn rebuild_line_from_pieces(edit state: arcana_text.layout.LayoutStageState, read payload: (List[arcana_text.layout.LineGlyphPiece], Int, Int)):
    let pieces = payload.0
    let original_start = payload.1
    let original_end = payload.2
    state.line = arcana_text.layout.working_line :: original_start :: call
    state.line.justify = false
    let mut active_id = -1
    let mut active_run = arcana_text.layout.run_at :: (arcana_text.layout.empty_runs :: :: call), 0 :: call
    let mut have_active = false
    for piece in pieces:
        if (not have_active) or piece.run_id != active_id:
            if have_active and not (active_run.glyphs :: :: is_empty):
                state.line.runs :: active_run :: push
            active_run = arcana_text.layout.copy_run_template :: piece.run :: call
            active_run.position = (state.line.width, 0)
            active_id = piece.run_id
            have_active = true
        arcana_text.layout.append_piece_glyph :: state, active_run, piece.glyph :: call
    if have_active and not (active_run.glyphs :: :: is_empty):
        state.line.runs :: active_run :: push
    state.line.start = original_start
    state.line.end = original_end

fn ellipsis_pieces(edit fonts: arcana_text.fonts.FontSystem, read payload: (arcana_text.layout.LayoutStageState, Int, arcana_text.types.TextDirection, Int, Int)) -> List[arcana_text.layout.LineGlyphPiece]:
    let state = payload.0
    let seam = payload.1
    let direction = payload.2
    let bidi_level = payload.3
    let run_id = payload.4
    let mut out = arcana_text.layout.empty_line_pieces :: :: call
    if state.config.ellipsis == "":
        return out
    let mut shaped = arcana_text.shape.glyphs.shape_inline :: fonts, state.default_style, (state.config.ellipsis, seam) :: call
    shaped.direction = direction
    shaped.bidi_level = bidi_level
    shaped.range = arcana_text.types.TextRange :: start = seam, end = seam :: call
    let mut temp = arcana_text.layout.seed_line_state :: state, seam, 1 :: call
    let vertical = arcana_text.layout.shaped_run_is_vertical :: shaped :: call
    let mut run = arcana_text.layout.empty_layout_run_from_shaped :: fonts, temp, shaped :: call
    run.direction = direction
    run.bidi_level = bidi_level
    run.range = arcana_text.types.TextRange :: start = seam, end = seam :: call
    for glyph_item in shaped.glyphs:
        let mut glyph = glyph_item
        glyph.range = arcana_text.types.TextRange :: start = seam, end = seam :: call
        glyph.cluster_range = glyph.range
        arcana_text.layout.append_shaped_glyph :: temp, run, (glyph, shaped.style, vertical) :: call
    if not (run.glyphs :: :: is_empty):
        temp.line.runs :: run :: push
    for piece in (arcana_text.layout.collect_line_pieces :: temp.line :: call):
        let mut next = piece
        next.run_id = run_id
        out :: next :: push
    return out

fn middle_piece_bounds(read pieces: List[arcana_text.layout.LineGlyphPiece], available: Int) -> (Int, Int):
    if available <= 0:
        return (0, pieces :: :: len)
    let total = pieces :: :: len
    let mut prefix_count = arcana_text.layout.prefix_piece_count_within_width :: pieces, available / 2 :: call
    let mut used = arcana_text.layout.line_piece_range_width :: pieces, 0, prefix_count :: call
    let mut suffix_start = arcana_text.layout.suffix_piece_start_within_width :: pieces, available - used, prefix_count :: call
    used = used + (arcana_text.layout.line_piece_range_width :: pieces, suffix_start, total :: call)
    let mut remaining = available - used
    while remaining > 0 and prefix_count < suffix_start:
        let next = arcana_text.layout.line_piece_width :: (pieces)[prefix_count] :: call
        if next > remaining:
            break
        prefix_count += 1
        remaining -= next
    while remaining > 0 and prefix_count < suffix_start:
        let next = arcana_text.layout.line_piece_width :: (pieces)[suffix_start - 1] :: call
        if next > remaining:
            break
        suffix_start -= 1
        remaining -= next
    return (prefix_count, suffix_start)

fn append_ellipsis(edit fonts: arcana_text.fonts.FontSystem, edit state: arcana_text.layout.LayoutStageState):
    if state.config.ellipsis == "":
        state.stopped = true
        return
    let original_start = state.line.start
    let original_end = state.line.end
    let pieces = arcana_text.layout.collect_line_pieces :: state.line :: call
    let direction = arcana_text.layout.ellipsis_direction :: pieces, (pieces :: :: len), 0 :: call
    let level = arcana_text.layout.ellipsis_level :: pieces, (pieces :: :: len), 0 :: call
    let preview = arcana_text.layout.ellipsis_pieces :: fonts, (state, original_end, direction, level, 2147483000) :: call
    let ellipsis_width = arcana_text.layout.line_piece_range_width :: preview, 0, (preview :: :: len) :: call
    let available = match state.config.max_width > 0:
        true => arcana_text.layout.max_int :: (state.config.max_width - ellipsis_width), 0 :: call
        false => state.line.width
    let total = pieces :: :: len
    let mut prefix = arcana_text.layout.empty_line_pieces :: :: call
    let mut suffix = arcana_text.layout.empty_line_pieces :: :: call
    let mut prefix_count = 0
    let mut suffix_start = total
    if state.config.ellipsize_mode == (arcana_text.types.EllipsizeMode.Start :: :: call):
        suffix_start = arcana_text.layout.suffix_piece_start_within_width :: pieces, available, 0 :: call
    else:
        if state.config.ellipsize_mode == (arcana_text.types.EllipsizeMode.Middle :: :: call):
            let middle = arcana_text.layout.middle_piece_bounds :: pieces, available :: call
            prefix_count = middle.0
            suffix_start = middle.1
        else:
            prefix_count = arcana_text.layout.prefix_piece_count_within_width :: pieces, available :: call
    prefix = arcana_text.layout.collect_piece_slice :: pieces, 0, prefix_count :: call
    suffix = arcana_text.layout.collect_piece_slice :: pieces, suffix_start, total :: call
    let seam = match state.config.ellipsize_mode:
        arcana_text.types.EllipsizeMode.Start => arcana_text.layout.suffix_seam :: pieces, suffix_start, original_end :: call
        arcana_text.types.EllipsizeMode.Middle => arcana_text.layout.prefix_seam :: pieces, prefix_count, (arcana_text.layout.suffix_seam :: pieces, suffix_start, original_end :: call) :: call
        _ => arcana_text.layout.prefix_seam :: pieces, prefix_count, original_start :: call
    let ellipsis_direction = arcana_text.layout.ellipsis_direction :: pieces, prefix_count, suffix_start :: call
    let ellipsis_level = arcana_text.layout.ellipsis_level :: pieces, prefix_count, suffix_start :: call
    let ellipsis = arcana_text.layout.ellipsis_pieces :: fonts, (state, seam, ellipsis_direction, ellipsis_level, 2147483000) :: call
    let mut combined = arcana_text.layout.empty_line_pieces :: :: call
    if state.config.ellipsize_mode == (arcana_text.types.EllipsizeMode.Start :: :: call):
        combined :: ellipsis :: extend_list
        combined :: suffix :: extend_list
    else:
        if state.config.ellipsize_mode == (arcana_text.types.EllipsizeMode.Middle :: :: call):
            combined :: prefix :: extend_list
            combined :: ellipsis :: extend_list
            combined :: suffix :: extend_list
        else:
            combined :: prefix :: extend_list
            combined :: ellipsis :: extend_list
    arcana_text.layout.rebuild_line_from_pieces :: state, (combined, original_start, original_end) :: call
    state.stopped = true

fn process_run(edit fonts: arcana_text.fonts.FontSystem, edit state: arcana_text.layout.LayoutStageState, read run: arcana_text.types.ShapedRun):
    layout_probe_append :: ("process_run:start start=" + (std.text.from_int :: run.range.start :: call) + " end=" + (std.text.from_int :: run.range.end :: call) + " width=" + (std.text.from_int :: run.width :: call)) :: call
    if state.stopped:
        return
    if run.hard_break:
        arcana_text.layout.append_layout_run :: fonts, state, run :: call
        return
    let line_cap_reached = state.config.max_lines > 0 and (state.lines :: :: len) >= state.config.max_lines - 1
    let display_width = arcana_text.layout.run_display_width :: state, run :: call
    let run_wraps = arcana_text.layout.layout_needs_wrap :: state, display_width :: call
    if line_cap_reached and run_wraps:
        arcana_text.layout.append_ellipsis :: fonts, state :: call
        return
    arcana_text.layout.append_layout_run :: fonts, state, run :: call
    layout_probe_append :: ("process_run:done line_width=" + (std.text.from_int :: state.line.width :: call)) :: call

fn finalize_line(edit state: arcana_text.layout.LayoutStageState):
    layout_probe_append :: ("finalize_line:start runs=" + (std.text.from_int :: (state.line.runs :: :: len) :: call) + " glyphs=" + (std.text.from_int :: (state.line.glyphs :: :: len) :: call)) :: call
    let line_index = state.lines :: :: len
    let line_height = arcana_text.layout.max_int :: state.line.height, state.default_line_height :: call
    let baseline = arcana_text.layout.max_int :: state.line.baseline, state.default_baseline :: call
    let offset = arcana_text.layout.line_start_x :: state, state.line.width :: call
    let ordered_runs = arcana_text.layout.visual_runs :: state.line.runs :: call
    let justify = state.line.justify and state.config.max_width > state.line.width
    let justify_extra = match justify:
        true => state.config.max_width - state.line.width
        false => 0
    let justify_slots = match justify:
        true => arcana_text.layout.justifyable_glyph_count :: ordered_runs :: call
        false => 0
    let final_line_width = match justify and justify_slots > 0:
        true => state.config.max_width
        false => state.line.width
    let mut next_x = offset
    let mut slot_index = 0
    for item in ordered_runs:
        let mut run = item
        let mut finalized_glyphs = arcana_text.layout.empty_glyphs :: :: call
        let run_x = next_x
        let vertical_run = arcana_text.layout.layout_run_is_vertical :: item :: call
        let mut run_extra = 0
        for glyph_item in item.glyphs:
            let mut glyph = glyph_item
            glyph.line_index = line_index
            if vertical_run:
                let local_x = glyph_item.position.0 - item.position.0
                let local_y = glyph_item.position.1 - item.position.1
                glyph.baseline = state.next_top + glyph_item.baseline
                glyph.position = (run_x + local_x, state.next_top + local_y)
                glyph.size = glyph_item.size
            else:
                glyph.baseline = state.next_top + baseline
                let local_x = glyph_item.position.0 - item.position.0
                let hinted_advance = arcana_text.layout.layout_primary_advance :: glyph_item, state.config :: call
                let slot_extra = match justify_slots > 0 and (glyph_item.glyph == " " or glyph_item.glyph == "\t"):
                    true => arcana_text.layout.justify_extra_for_slot :: justify_extra, justify_slots, slot_index :: call
                    false => 0
                let shift_before = run_extra
                if slot_extra > 0:
                    slot_index += 1
                    run_extra += slot_extra
                let glyph_width = hinted_advance + slot_extra
                let glyph_x = match item.direction:
                    arcana_text.types.TextDirection.RightToLeft => run_x + item.size.0 + run_extra - local_x - glyph_width
                    _ => run_x + local_x + shift_before
                glyph.position = (glyph_x, state.next_top + baseline - glyph_item.baseline)
                glyph.size = (glyph_width, line_height)
                glyph.advance = glyph_width
                glyph.x_advance = hinted_advance + slot_extra
            state.signature = arcana_text.shape.types.mix_signature :: state.signature, glyph.position.0 :: call
            state.signature = arcana_text.shape.types.mix_signature :: state.signature, glyph.position.1 :: call
            state.signature = arcana_text.shape.types.mix_signature :: state.signature, glyph.glyph_index + 97 :: call
            let mut glyph_for_snapshot = record yield arcana_text.types.LayoutGlyph from glyph -return 0
                glyph = glyph.glyph
            finalized_glyphs :: glyph :: push
            state.glyphs :: glyph_for_snapshot :: push
        run.position = (run_x, state.next_top)
        run.size = match vertical_run:
            true => item.size
            false => (item.size.0 + run_extra, line_height)
        run.baseline = state.next_top + baseline
        run.glyphs = finalized_glyphs
        state.runs :: run :: push
        next_x += item.size.0 + run_extra
    let mut metrics = arcana_text.types.LineMetrics :: index = line_index, range = (arcana_text.types.TextRange :: start = state.line.start, end = state.line.end :: call), position = (offset, state.next_top) :: call
    metrics.size = (final_line_width, line_height)
    metrics.baseline = state.next_top + baseline
    state.lines :: (arcana_text.types.SnapshotLine :: metrics = metrics, text = state.line.text :: call) :: push
    if final_line_width > state.width:
        state.width = final_line_width
    state.next_top += line_height
    state.height = state.next_top
    state.line = arcana_text.layout.working_line :: state.line.end :: call
    layout_probe_append :: ("finalize_line:done lines=" + (std.text.from_int :: (state.lines :: :: len) :: call) + " height=" + (std.text.from_int :: state.height :: call)) :: call

fn shift_line_metrics(read metrics: arcana_text.types.LineMetrics, line_index_delta: Int, y_delta: Int) -> arcana_text.types.LineMetrics:
    let mut next = metrics
    next.index = metrics.index + line_index_delta
    next.position = (metrics.position.0, metrics.position.1 + y_delta)
    next.baseline = metrics.baseline + y_delta
    return next

fn shift_snapshot_line(read value: arcana_text.types.SnapshotLine, line_index_delta: Int, y_delta: Int) -> arcana_text.types.SnapshotLine:
    let mut next = value
    next.metrics = arcana_text.layout.shift_line_metrics :: value.metrics, line_index_delta, y_delta :: call
    return next

fn shift_layout_glyph(read glyph: arcana_text.types.LayoutGlyph, line_index_delta: Int, y_delta: Int) -> arcana_text.types.LayoutGlyph:
    let mut next = glyph
    next.position = (glyph.position.0, glyph.position.1 + y_delta)
    next.line_index = glyph.line_index + line_index_delta
    next.baseline = glyph.baseline + y_delta
    return next

fn shift_layout_run(read run: arcana_text.types.LayoutRun, line_index_delta: Int, y_delta: Int) -> arcana_text.types.LayoutRun:
    let mut next = run
    next.position = (run.position.0, run.position.1 + y_delta)
    next.baseline = run.baseline + y_delta
    next.glyphs = arcana_text.layout.empty_glyphs :: :: call
    for glyph in run.glyphs:
        next.glyphs :: (arcana_text.layout.shift_layout_glyph :: glyph, line_index_delta, y_delta :: call) :: push
    return next

fn append_prepared_layout_line(edit state: arcana_text.layout.LayoutStageState, read prepared: arcana_text.types.PreparedLayoutLine):
    let y_delta = state.next_top
    let line_index_delta = state.lines :: :: len
    state.signature = arcana_text.shape.types.mix_signature :: state.signature, prepared.start :: call
    state.signature = arcana_text.shape.types.mix_signature :: state.signature, prepared.end :: call
    state.signature = arcana_text.shape.types.mix_signature :: state.signature, y_delta :: call
    state.signature = arcana_text.shape.types.mix_signature :: state.signature, line_index_delta :: call
    state.signature = arcana_text.shape.types.mix_signature :: state.signature, prepared.signature :: call
    for value in prepared.lines:
        state.lines :: (arcana_text.layout.shift_snapshot_line :: value, line_index_delta, y_delta :: call) :: push
    for value in prepared.runs:
        state.runs :: (arcana_text.layout.shift_layout_run :: value, line_index_delta, y_delta :: call) :: push
    for value in prepared.glyphs:
        state.glyphs :: (arcana_text.layout.shift_layout_glyph :: value, line_index_delta, y_delta :: call) :: push
    state.unresolved :: prepared.unresolved :: extend_list
    for value in prepared.fonts_used:
        arcana_text.shape.types.push_unique_match :: state.fonts_used, value :: call
    if prepared.size.0 > state.width:
        state.width = prepared.size.0
    state.next_top += prepared.size.1
    state.height = state.next_top
    state.line = arcana_text.layout.working_line :: prepared.end :: call
    if prepared.stopped:
        state.stopped = true

LayoutPass
LayoutPassState
fn snapshot_active(edit fonts: arcana_text.fonts.FontSystem, read buffer: arcana_text.buffer.TextBuffer, read config: arcana_text.types.LayoutConfig) -> arcana_text.layout.LayoutSnapshot:
    layout_probe_append :: "snapshot_active:start" :: call
    let _scratch = temp: arcana_text.layout.layout_scratch :> value = LayoutPassState.buffer_version <: arcana_text.layout.LayoutScratch
    let mut state = arcana_text.layout.seed_state_from_buffer :: fonts, buffer, config :: call
    let lines = arcana_text.shape.tokens.collect_lines :: buffer :: call
    layout_probe_append :: ("snapshot_active:lines=" + (std.text.from_int :: (lines :: :: len) :: call)) :: call
    for line in lines:
        let prepared_runs = arcana_text.shape.pipeline.prepared_runs_for_line :: fonts, buffer, line :: call
        layout_probe_append :: ("snapshot_active:line prepared_runs=" + (std.text.from_int :: (prepared_runs :: :: len) :: call)) :: call
        for prepared in prepared_runs:
            fonts.shape_cache :: prepared.run :: remember_shaped_run
        let remaining_lines = match state.config.max_lines > 0:
            true => state.config.max_lines - (state.lines :: :: len)
            false => 0
        if state.config.max_lines > 0 and remaining_lines <= 0:
            state.stopped = true
            break
        let cache_key = arcana_text.layout.prepared_layout_line_key :: state, prepared_runs, remaining_lines :: call
        let cached = fonts.shape_cache :: cache_key :: cached_prepared_layout_line
        let prepared_layout = match cached:
            Option.Some(value) => value
            Option.None => arcana_text.layout.prepare_layout_line :: fonts, (state, prepared_runs, remaining_lines) :: call
        if cached :: :: is_none:
            fonts.shape_cache :: cache_key, prepared_layout :: remember_prepared_layout_line
        arcana_text.layout.append_prepared_layout_line :: state, prepared_layout :: call
        if state.stopped:
            break
    let total = std.text.len_bytes :: buffer.text :: call
    if not state.stopped and ((state.lines :: :: is_empty) or not (state.line.glyphs :: :: is_empty) or state.line.start < total):
        arcana_text.layout.finalize_line :: state :: call
    LayoutPassState.done = true
    let mut snapshot = arcana_text.layout.LayoutSnapshot :: source_version = buffer.version, size = ((arcana_text.layout.max_int :: state.width, 1 :: call), (arcana_text.layout.max_int :: state.height, 1 :: call)), signature = state.signature :: call
    snapshot.lines = state.lines
    snapshot.runs = state.runs
    snapshot.glyphs = state.glyphs
    snapshot.unresolved = state.unresolved
    snapshot.fonts_used = state.fonts_used
    layout_probe_append :: ("snapshot_active:done lines=" + (std.text.from_int :: (snapshot.lines :: :: len) :: call) + " glyphs=" + (std.text.from_int :: (snapshot.glyphs :: :: len) :: call)) :: call
    return snapshot

export fn snapshot(edit fonts: arcana_text.fonts.FontSystem, read buffer: arcana_text.buffer.TextBuffer, read config: arcana_text.types.LayoutConfig) -> arcana_text.layout.LayoutSnapshot:
    let ctx = arcana_text.layout.LayoutPassContext :: buffer_version = buffer.version, config = config, font_count = (fonts :: :: count) :: call
    let active = LayoutPass :: ctx :: call
    let _ = active
    return arcana_text.layout.snapshot_active :: fonts, buffer, config :: call

impl LayoutSnapshot:
    fn longest_line(read self: arcana_text.layout.LayoutSnapshot) -> Int:
        return self.size.0

    fn height(read self: arcana_text.layout.LayoutSnapshot) -> Int:
        return self.size.1

