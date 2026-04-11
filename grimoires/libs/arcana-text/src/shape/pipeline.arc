import arcana_text.buffer
import arcana_text.shape.cache
import arcana_text.fonts
import arcana_text.shape.glyphs
import arcana_text.shape.styles
import arcana_text.shape.tokens
import arcana_text.shape.types
import std.bytes
import std.fs
import std.option
import std.path
use arcana_text.types as text_types
use std.option.Option

fn shape_probe_flag_path() -> Str:
    return std.path.join :: (std.path.join :: (std.path.cwd :: :: call), "scratch" :: call), "enable_text_fonts_probe" :: call

fn shape_probe_log_path() -> Str:
    return std.path.join :: (std.path.join :: (std.path.cwd :: :: call), "scratch" :: call), "text_shape_probe.log" :: call

fn shape_probe_enabled() -> Bool:
    return std.fs.is_file :: (shape_probe_flag_path :: :: call) :: call

fn shape_probe_append(line: Str):
    if not (shape_probe_enabled :: :: call):
        return
    let _ = std.fs.mkdir_all :: (std.path.parent :: (shape_probe_log_path :: :: call) :: call) :: call
    let opened = std.fs.stream_open_write :: (shape_probe_log_path :: :: call), true :: call
    return match opened:
        std.result.Result.Ok(value) => shape_probe_append_ready :: value, line :: call
        std.result.Result.Err(_) => 0

fn shape_probe_append_ready(take value: std.fs.FileStream, line: Str):
    let mut stream = value
    let bytes = std.bytes.from_str_utf8 :: (line + "\n") :: call
    let _ = std.fs.stream_write :: stream, bytes :: call
    let _ = std.fs.stream_close :: stream :: call

record ShapePassContext:
    buffer_version: Int
    font_count: Int

obj ShapePassState:
    done: Bool
    buffer_version: Int
    font_count: Int
    fn init(edit self: Self, read ctx: ShapePassContext):
        self.done = false
        self.buffer_version = ctx.buffer_version
        self.font_count = ctx.font_count
    fn resume(edit self: Self, read ctx: ShapePassContext):
        self.done = false
        self.buffer_version = ctx.buffer_version
        self.font_count = ctx.font_count

create ShapePass [ShapePassState] context: ShapePassContext scope-exit:
    done: when ShapePassState.done

record ShapeStageState:
    default_style: arcana_text.types.SpanStyle
    paragraph: arcana_text.types.ParagraphStyle
    runs: List[arcana_text.types.ShapedRun]
    plan_keys: List[arcana_text.types.ShapePlanKey]
    unresolved: List[arcana_text.types.UnresolvedGlyph]
    fonts_used: List[arcana_text.types.FontMatch]
    signature: Int
    default_line_height: Int
    default_baseline: Int

fn shift_range(read range: arcana_text.types.TextRange, delta: Int) -> arcana_text.types.TextRange:
    return arcana_text.types.TextRange :: start = range.start + delta, end = range.end + delta :: call

fn shift_placeholder(read value: Option[arcana_text.types.PlaceholderSpec], delta: Int) -> Option[arcana_text.types.PlaceholderSpec]:
    if value :: :: is_none:
        return Option.None[arcana_text.types.PlaceholderSpec] :: :: call
    let spec = value :: (arcana_text.shape.types.fallback_placeholder :: (arcana_text.types.TextRange :: start = 0, end = 0 :: call) :: call) :: unwrap_or
    let mut next = spec
    next.range = arcana_text.shape.pipeline.shift_range :: spec.range, delta :: call
    return Option.Some[arcana_text.types.PlaceholderSpec] :: next :: call

fn shift_shaped_glyph(read glyph: arcana_text.types.ShapedGlyph, delta: Int) -> arcana_text.types.ShapedGlyph:
    let mut next = glyph
    next.range = arcana_text.shape.pipeline.shift_range :: glyph.range, delta :: call
    next.cluster_range = arcana_text.shape.pipeline.shift_range :: glyph.cluster_range, delta :: call
    return next

fn shift_shaped_glyphs(read glyphs: List[arcana_text.types.ShapedGlyph], delta: Int) -> List[arcana_text.types.ShapedGlyph]:
    let mut out = std.collections.list.empty[arcana_text.types.ShapedGlyph] :: :: call
    for glyph in glyphs:
        out :: (arcana_text.shape.pipeline.shift_shaped_glyph :: glyph, delta :: call) :: push
    return out

fn shift_shaped_run(read run: arcana_text.types.ShapedRun, delta: Int) -> arcana_text.types.ShapedRun:
    let mut next = run
    next.range = arcana_text.shape.pipeline.shift_range :: run.range, delta :: call
    next.glyphs = arcana_text.shape.pipeline.shift_shaped_glyphs :: run.glyphs, delta :: call
    next.placeholder = arcana_text.shape.pipeline.shift_placeholder :: run.placeholder, delta :: call
    return next

fn shift_unresolved(read unresolved: List[arcana_text.types.UnresolvedGlyph], delta: Int) -> List[arcana_text.types.UnresolvedGlyph]:
    let mut out = arcana_text.shape.types.empty_unresolved :: :: call
    for value in unresolved:
        let mut next = value
        next.index = next.index + delta
        out :: next :: push
    return out

fn shift_prepared_runs(read runs: List[arcana_text.types.PreparedRun], delta: Int) -> List[arcana_text.types.PreparedRun]:
    let mut out = std.collections.list.empty[arcana_text.types.PreparedRun] :: :: call
    for prepared in runs:
        out :: (arcana_text.types.PreparedRun :: run = (arcana_text.shape.pipeline.shift_shaped_run :: prepared.run, delta :: call), unresolved = (arcana_text.shape.pipeline.shift_unresolved :: prepared.unresolved, delta :: call) :: call) :: push
    return out

fn bool_code(value: Bool) -> Int:
    return match value:
        true => 1
        false => 0

fn item_kind_code(read kind: arcana_text.shape.tokens.ShapeItemKind) -> Int:
    return match kind:
        arcana_text.shape.tokens.ShapeItemKind.Placeholder => 2
        _ => 1

fn placeholder_alignment_code(read value: arcana_text.types.PlaceholderAlignment) -> Int:
    return match value:
        arcana_text.types.PlaceholderAlignment.Middle => 2
        arcana_text.types.PlaceholderAlignment.Top => 3
        arcana_text.types.PlaceholderAlignment.Bottom => 4
        _ => 1

fn text_baseline_code(read value: arcana_text.types.TextBaseline) -> Int:
    return match value:
        arcana_text.types.TextBaseline.Ideographic => 2
        _ => 1

fn underline_code(read value: arcana_text.types.UnderlineStyle) -> Int:
    return match value:
        arcana_text.types.UnderlineStyle.Single => 2
        arcana_text.types.UnderlineStyle.Double => 3
        _ => 1

fn mix_style_signature(seed: Int, read style: arcana_text.types.SpanStyle) -> Int:
    let mut signature = seed
    signature = arcana_text.shape.types.mix_signature :: signature, style.color :: call
    signature = arcana_text.shape.types.mix_signature :: signature, style.background_color :: call
    signature = arcana_text.shape.types.mix_signature :: signature, arcana_text.shape.pipeline.bool_code :: style.background_enabled :: call
    signature = arcana_text.shape.types.mix_signature :: signature, arcana_text.shape.pipeline.underline_code :: style.underline :: call
    signature = arcana_text.shape.types.mix_signature :: signature, arcana_text.shape.pipeline.bool_code :: style.underline_color_enabled :: call
    signature = arcana_text.shape.types.mix_signature :: signature, style.underline_color :: call
    signature = arcana_text.shape.types.mix_signature :: signature, arcana_text.shape.pipeline.bool_code :: style.strikethrough_enabled :: call
    signature = arcana_text.shape.types.mix_signature :: signature, arcana_text.shape.pipeline.bool_code :: style.strikethrough_color_enabled :: call
    signature = arcana_text.shape.types.mix_signature :: signature, style.strikethrough_color :: call
    signature = arcana_text.shape.types.mix_signature :: signature, arcana_text.shape.pipeline.bool_code :: style.overline_enabled :: call
    signature = arcana_text.shape.types.mix_signature :: signature, arcana_text.shape.pipeline.bool_code :: style.overline_color_enabled :: call
    signature = arcana_text.shape.types.mix_signature :: signature, style.overline_color :: call
    signature = arcana_text.shape.types.mix_signature :: signature, style.size :: call
    signature = arcana_text.shape.types.mix_signature :: signature, style.letter_spacing :: call
    signature = arcana_text.shape.types.mix_signature :: signature, style.line_height :: call
    signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.types.feature_signature :: style.features :: call) :: call
    signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.types.axis_signature :: style.axes :: call) :: call
    for family in style.families:
        signature = arcana_text.shape.types.mix_signature_text :: signature, family :: call
    return signature

fn mix_placeholder_signature(seed: Int, read spec: arcana_text.types.PlaceholderSpec, relative_range: arcana_text.types.TextRange) -> Int:
    let mut signature = seed
    signature = arcana_text.shape.types.mix_signature :: signature, relative_range.start :: call
    signature = arcana_text.shape.types.mix_signature :: signature, relative_range.end :: call
    signature = arcana_text.shape.types.mix_signature :: signature, spec.size.0 :: call
    signature = arcana_text.shape.types.mix_signature :: signature, spec.size.1 :: call
    signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.shape.pipeline.placeholder_alignment_code :: spec.alignment :: call) :: call
    signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.shape.pipeline.text_baseline_code :: spec.baseline :: call) :: call
    signature = arcana_text.shape.types.mix_signature :: signature, spec.baseline_offset :: call
    return signature

fn prepared_line_key(read buffer: arcana_text.buffer.TextBuffer, read line: arcana_text.shape.tokens.ShapeLine) -> Str:
    let mut signature = 53
    signature = arcana_text.shape.types.mix_signature_text :: signature, line.text :: call
    signature = arcana_text.shape.types.mix_signature :: signature, arcana_text.shape.pipeline.bool_code :: line.hard_break :: call
    signature = arcana_text.shape.types.mix_signature :: signature, (line.break_range.end - line.break_range.start) :: call
    for item in line.items:
        let relative_start = item.range.start - line.range.start
        let relative_end = item.range.end - line.range.start
        let relative_range = arcana_text.types.TextRange :: start = relative_start, end = relative_end :: call
        signature = arcana_text.shape.types.mix_signature :: signature, (arcana_text.shape.pipeline.item_kind_code :: item.kind :: call) :: call
        signature = arcana_text.shape.types.mix_signature :: signature, relative_start :: call
        signature = arcana_text.shape.types.mix_signature :: signature, relative_end :: call
        signature = arcana_text.shape.types.mix_signature :: signature, arcana_text.shape.pipeline.bool_code :: item.whitespace :: call
        signature = arcana_text.shape.types.mix_signature :: signature, arcana_text.shape.pipeline.bool_code :: item.newline :: call
        signature = arcana_text.shape.types.mix_signature_text :: signature, item.text :: call
        let style = arcana_text.shape.styles.style_for_range :: buffer, item.range :: call
        signature = arcana_text.shape.pipeline.mix_style_signature :: signature, style :: call
        if item.kind == (arcana_text.shape.tokens.ShapeItemKind.Placeholder :: :: call):
            let spec = item.placeholder :: (arcana_text.shape.types.fallback_placeholder :: item.range :: call) :: unwrap_or
            signature = arcana_text.shape.pipeline.mix_placeholder_signature :: signature, spec, relative_range :: call
    return (std.text.from_int :: signature :: call) + ":" + (std.text.from_int :: (line.items :: :: len) :: call) + ":" + line.text

fn append_run(edit state: arcana_text.shape.pipeline.ShapeStageState, read prepared: text_types.PreparedRun):
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
    arcana_text.shape.types.push_unique_plan_key :: state.plan_keys, run.plan_key :: call
    state.runs :: run :: push
    state.unresolved :: prepared.unresolved :: extend_list

fn seed_state(edit fonts: arcana_text.fonts.FontSystem, read buffer: arcana_text.buffer.TextBuffer) -> arcana_text.shape.pipeline.ShapeStageState:
    let default_style = arcana_text.shape.styles.span_style_from_text_style :: buffer.style :: call
    shape_probe_append :: "seed_state:style_ready" :: call
    let first_visible = arcana_text.shape.tokens.first_visible_char :: buffer.text :: call
    shape_probe_append :: ("seed_state:first_visible `" + first_visible + "`") :: call
    let matched = fonts :: buffer.style, first_visible :: resolve_style_char
    shape_probe_append :: ("seed_state:matched source=" + (std.text.from_int :: matched.id.source_index :: call) + " face=" + (std.text.from_int :: matched.id.face_index :: call)) :: call
    let line_height = fonts :: matched, buffer.style :: line_height
    shape_probe_append :: ("seed_state:line_height " + (std.text.from_int :: line_height :: call)) :: call
    let baseline = fonts :: matched, buffer.style :: baseline
    shape_probe_append :: ("seed_state:baseline " + (std.text.from_int :: baseline :: call)) :: call
    let mut state = arcana_text.shape.pipeline.ShapeStageState :: default_style = default_style, paragraph = buffer.paragraph :: call
    state.runs = arcana_text.shape.types.empty_runs :: :: call
    state.plan_keys = arcana_text.shape.types.empty_plan_keys :: :: call
    state.unresolved = arcana_text.shape.types.empty_unresolved :: :: call
    state.fonts_used = arcana_text.shape.types.empty_matches :: :: call
    state.signature = 23
    state.default_line_height = arcana_text.shape.types.max_int :: line_height, (buffer.style.size + 6) :: call
    state.default_baseline = arcana_text.shape.types.max_int :: baseline, buffer.style.size :: call
    return state

fn shape_line_uncached(edit fonts: arcana_text.fonts.FontSystem, read buffer: arcana_text.buffer.TextBuffer, read line: arcana_text.shape.tokens.ShapeLine) -> List[arcana_text.types.PreparedRun]:
    let resolved_line = arcana_text.shape.glyphs.resolve_line :: line.text, line.range.start :: call
    let mut runs = std.collections.list.empty[arcana_text.types.PreparedRun] :: :: call
    for item in line.items:
        let span_style = arcana_text.shape.styles.style_for_range :: buffer, item.range :: call
        if item.kind == (arcana_text.shape.tokens.ShapeItemKind.Placeholder :: :: call):
            let spec = item.placeholder :: (arcana_text.shape.types.fallback_placeholder :: item.range :: call) :: unwrap_or
            let context = arcana_text.shape.glyphs.resolved_cluster_for_range :: resolved_line, spec.range :: call
            runs :: (arcana_text.shape.glyphs.shape_placeholder_with_context :: span_style, spec, context :: call) :: push
        else:
            let mut token = arcana_text.shape.tokens.text_token :: item.text, item.range.start, item.range.end :: call
            token.whitespace = item.whitespace
            token.newline = item.newline
            let request = arcana_text.shape.glyphs.ShapeTokenRunsInLineRequest :: token = token, span_style = span_style, line = resolved_line :: call
            runs :: (arcana_text.shape.glyphs.shape_token_runs_in_line :: fonts, request :: call) :: extend_list
    if line.hard_break:
        let newline_text = std.text.slice_bytes :: buffer.text, line.break_range.start, line.break_range.end :: call
        let span_style = arcana_text.shape.styles.style_for_range :: buffer, line.break_range :: call
        let mut token = arcana_text.shape.tokens.text_token :: newline_text, line.break_range.start, line.break_range.end :: call
        token.newline = true
        let request = arcana_text.shape.glyphs.ShapeTokenRunsInLineRequest :: token = token, span_style = span_style, line = resolved_line :: call
        runs :: (arcana_text.shape.glyphs.shape_token_runs_in_line :: fonts, request :: call) :: extend_list
    return runs

export fn prepared_runs_for_line(edit fonts: arcana_text.fonts.FontSystem, read buffer: arcana_text.buffer.TextBuffer, read line: arcana_text.shape.tokens.ShapeLine) -> List[arcana_text.types.PreparedRun]:
    let cache_key = arcana_text.shape.pipeline.prepared_line_key :: buffer, line :: call
    let cached = fonts.shape_cache :: cache_key :: cached_prepared_line
    if cached :: :: is_some:
        return arcana_text.shape.pipeline.shift_prepared_runs :: (cached :: (std.collections.list.empty[arcana_text.types.PreparedRun] :: :: call) :: unwrap_or), line.range.start :: call
    let runs = arcana_text.shape.pipeline.shape_line_uncached :: fonts, buffer, line :: call
    fonts.shape_cache :: cache_key, (arcana_text.shape.pipeline.shift_prepared_runs :: runs, (0 - line.range.start) :: call) :: remember_prepared_line
    return runs

ShapePass
ShapePassState
fn snapshot_active(edit fonts: arcana_text.fonts.FontSystem, read buffer: arcana_text.buffer.TextBuffer) -> arcana_text.shape.types.ShapeSnapshot:
    shape_probe_append :: ("snapshot_active:start bytes=" + (std.text.from_int :: (std.text.len_bytes :: buffer.text :: call) :: call)) :: call
    let mut state = seed_state :: fonts, buffer :: call
    shape_probe_append :: "snapshot_active:seed_ready" :: call
    let lines = arcana_text.shape.tokens.collect_lines :: buffer :: call
    shape_probe_append :: ("snapshot_active:lines=" + (std.text.from_int :: (lines :: :: len) :: call)) :: call
    for line in lines:
        let prepared_runs = arcana_text.shape.pipeline.prepared_runs_for_line :: fonts, buffer, line :: call
        shape_probe_append :: ("snapshot_active:line prepared_runs=" + (std.text.from_int :: (prepared_runs :: :: len) :: call)) :: call
        for prepared in prepared_runs:
            fonts.shape_cache :: prepared.run :: remember_shaped_run
            append_run :: state, prepared :: call
    let mut out = arcana_text.shape.types.ShapeSnapshot :: source_version = buffer.version, default_style = state.default_style, paragraph = state.paragraph :: call
    out.signature = state.signature
    out.default_line_height = state.default_line_height
    out.default_baseline = state.default_baseline
    out.runs = state.runs
    out.plan_keys = state.plan_keys
    out.unresolved = state.unresolved
    out.fonts_used = state.fonts_used
    shape_probe_append :: ("snapshot_active:done runs=" + (std.text.from_int :: (out.runs :: :: len) :: call)) :: call
    return out

export fn snapshot(edit fonts: arcana_text.fonts.FontSystem, read buffer: arcana_text.buffer.TextBuffer) -> arcana_text.shape.types.ShapeSnapshot:
    return snapshot_active :: fonts, buffer :: call
