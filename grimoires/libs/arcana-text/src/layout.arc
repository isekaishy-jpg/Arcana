import arcana_text.buffer
import arcana_text.fonts
import arcana_text.shape.glyphs
import arcana_text.shape.pipeline
import arcana_text.types
import std.collections.list
import std.option
import std.text
use std.option.Option

record LayoutScratch:
    value: Int

record LayoutPassContext:
    buffer_version: Int
    config: arcana_text.types.LayoutConfig
    font_count: Int

record WorkingLine:
    start: Int
    end: Int
    width: Int
    height: Int
    baseline: Int
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

fn working_line(start: Int) -> arcana_text.layout.WorkingLine:
    let mut line = arcana_text.layout.WorkingLine :: start = start, end = start, width = 0 :: call
    line.height = 0
    line.baseline = 0
    line.text = ""
    line.glyphs = arcana_text.layout.empty_glyphs :: :: call
    line.runs = arcana_text.layout.empty_runs :: :: call
    return line

fn push_unique_match(edit out: List[arcana_text.types.FontMatch], read value: arcana_text.types.FontMatch):
    if value.id.source_index < 0:
        return
    for existing in out:
        if existing.id.source_index == value.id.source_index and existing.id.face_index == value.id.face_index:
            return
    out :: value :: push

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

fn line_start_x(read state: arcana_text.layout.LayoutStageState, line_width: Int) -> Int:
    if state.config.max_width <= 0:
        return 0
    return match state.config.align:
        arcana_text.types.TextAlign.Center => (state.config.max_width - line_width) / 2
        arcana_text.types.TextAlign.Right => state.config.max_width - line_width
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

fn layout_needs_wrap(read state: arcana_text.layout.LayoutStageState, advance: Int) -> Bool:
    return state.config.max_width > 0 and not (state.line.glyphs :: :: is_empty) and state.line.width + advance > state.config.max_width

fn append_shaped_glyph(edit state: arcana_text.layout.LayoutStageState, edit run: arcana_text.types.LayoutRun, read payload: (arcana_text.types.ShapedGlyph, arcana_text.types.SpanStyle)):
    let shaped = payload.0
    let style = payload.1
    if shaped.line_height > state.line.height:
        state.line.height = shaped.line_height
    if shaped.baseline > state.line.baseline:
        state.line.baseline = shaped.baseline
    let mut glyph = arcana_text.types.LayoutGlyph :: glyph = shaped.glyph, range = shaped.range, position = (state.line.width, 0) :: call
    glyph.cluster_range = shaped.cluster_range
    glyph.size = (shaped.advance, shaped.line_height)
    glyph.advance = shaped.advance
    glyph.x_advance = shaped.x_advance
    glyph.y_advance = shaped.y_advance
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
    state.line.width += shaped.advance
    state.line.end = shaped.range.end
    let run_height = arcana_text.layout.max_int :: run.size.1, shaped.line_height :: call
    run.size = (run.size.0 + shaped.advance, run_height)

fn append_layout_run(edit state: arcana_text.layout.LayoutStageState, read shaped: arcana_text.types.ShapedRun):
    if shaped.hard_break:
        if state.config.max_lines > 0 and (state.lines :: :: len) >= state.config.max_lines - 1:
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
    if state.config.max_lines > 0 and (state.lines :: :: len) >= state.config.max_lines - 1 and arcana_text.layout.layout_needs_wrap :: state, shaped.width :: call:
        state.stopped = true
        return
    if arcana_text.layout.layout_needs_wrap :: state, shaped.width :: call:
        arcana_text.layout.finalize_line :: state :: call
    let mut run = arcana_text.types.LayoutRun :: range = shaped.range, position = (state.line.width, 0), size = (0, 0) :: call
    run.kind = shaped.kind
    run.direction = shaped.direction
    run.script = shaped.script
    run.bidi_level = shaped.bidi_level
    run.language_tag = shaped.language_tag
    run.family = arcana_text.fonts.family_or_label :: shaped.match.source :: call
    run.face_id = shaped.match.id
    run.glyphs = arcana_text.layout.empty_glyphs :: :: call
    run.placeholder = shaped.placeholder
    for glyph in shaped.glyphs:
        arcana_text.layout.append_shaped_glyph :: state, run, (glyph, shaped.style) :: call
    if run.glyphs :: :: is_empty:
        return
    state.line.runs :: run :: push

fn append_ellipsis(edit fonts: arcana_text.fonts.FontSystem, edit state: arcana_text.layout.LayoutStageState):
    if state.config.ellipsis == "":
        state.stopped = true
        return
    let run = arcana_text.shape.glyphs.shape_inline :: fonts, state.default_style, (state.config.ellipsis, state.line.end) :: call
    while state.config.max_width > 0 and state.line.width + run.width > state.config.max_width and not (state.line.glyphs :: :: is_empty):
        arcana_text.layout.pop_glyph :: state :: call
    arcana_text.layout.push_unique_match :: state.fonts_used, run.match :: call
    arcana_text.layout.append_layout_run :: state, run :: call
    state.stopped = true

fn process_run(edit fonts: arcana_text.fonts.FontSystem, edit state: arcana_text.layout.LayoutStageState, read run: arcana_text.types.ShapedRun):
    if state.stopped:
        return
    if run.hard_break:
        arcana_text.layout.append_layout_run :: state, run :: call
        return
    if state.config.max_lines > 0 and (state.lines :: :: len) >= state.config.max_lines - 1 and arcana_text.layout.layout_needs_wrap :: state, run.width :: call:
        arcana_text.layout.append_ellipsis :: fonts, state :: call
        return
    arcana_text.layout.append_layout_run :: state, run :: call

fn finalize_line(edit state: arcana_text.layout.LayoutStageState):
    let line_index = state.lines :: :: len
    let line_height = arcana_text.layout.max_int :: state.line.height, state.default_line_height :: call
    let baseline = arcana_text.layout.max_int :: state.line.baseline, state.default_baseline :: call
    let offset = arcana_text.layout.line_start_x :: state, state.line.width :: call
    for item in state.line.runs:
        let mut run = item
        let mut finalized_glyphs = arcana_text.layout.empty_glyphs :: :: call
        for glyph_item in item.glyphs:
            let mut glyph = glyph_item
            glyph.line_index = line_index
            glyph.baseline = state.next_top + baseline
            glyph.position = (glyph_item.position.0 + offset, state.next_top + baseline - glyph_item.baseline)
            glyph.size = (glyph_item.advance, line_height)
            state.signature = arcana_text.layout.max_int :: state.signature, state.signature :: call
            state.signature = (state.signature * 131 + glyph.position.0 + glyph.position.1 + glyph.glyph_index + 97) % 2147483629
            let mut glyph_for_snapshot = record yield arcana_text.types.LayoutGlyph from glyph -return 0
                glyph = glyph.glyph
            finalized_glyphs :: glyph :: push
            state.glyphs :: glyph_for_snapshot :: push
        run.position = (item.position.0 + offset, state.next_top)
        run.size = (item.size.0, line_height)
        run.glyphs = finalized_glyphs
        state.runs :: run :: push
    let mut metrics = arcana_text.types.LineMetrics :: index = line_index, range = (arcana_text.types.TextRange :: start = state.line.start, end = state.line.end :: call), position = (offset, state.next_top) :: call
    metrics.size = (state.line.width, line_height)
    metrics.baseline = state.next_top + baseline
    state.lines :: (arcana_text.types.SnapshotLine :: metrics = metrics, text = state.line.text :: call) :: push
    if state.line.width > state.width:
        state.width = state.line.width
    state.next_top += line_height
    state.height = state.next_top
    state.line = arcana_text.layout.working_line :: state.line.end :: call

LayoutPass
LayoutPassState
fn snapshot_active(edit fonts: arcana_text.fonts.FontSystem, read buffer: arcana_text.buffer.TextBuffer, read config: arcana_text.types.LayoutConfig) -> arcana_text.layout.LayoutSnapshot:
    let shaped = arcana_text.shape.pipeline.snapshot :: fonts, buffer :: call
    let _scratch = temp: arcana_text.layout.layout_scratch :> value = LayoutPassState.buffer_version <: arcana_text.layout.LayoutScratch
    let mut state = arcana_text.layout.seed_state :: shaped, config :: call
    for run in shaped.runs:
        arcana_text.layout.process_run :: fonts, state, run :: call
        if state.stopped:
            break
    let total = std.text.len_bytes :: buffer.text :: call
    if (state.lines :: :: is_empty) or not (state.line.glyphs :: :: is_empty) or state.line.start < total:
        arcana_text.layout.finalize_line :: state :: call
    LayoutPassState.done = true
    let mut snapshot = arcana_text.layout.LayoutSnapshot :: source_version = buffer.version, size = ((arcana_text.layout.max_int :: state.width, 1 :: call), (arcana_text.layout.max_int :: state.height, 1 :: call)), signature = state.signature :: call
    snapshot.lines = state.lines
    snapshot.runs = state.runs
    snapshot.glyphs = state.glyphs
    snapshot.unresolved = state.unresolved
    snapshot.fonts_used = state.fonts_used
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
