import arcana_text.buffer
import arcana_text.shape.cache
import arcana_text.fonts
import arcana_text.shape.glyphs
import arcana_text.shape.styles
import arcana_text.shape.tokens
import arcana_text.shape.types

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

fn append_run(edit state: arcana_text.shape.pipeline.ShapeStageState, read prepared: arcana_text.shape.glyphs.PreparedRun):
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
    let matched = fonts :: buffer.style, (arcana_text.shape.tokens.first_visible_char :: buffer.text :: call) :: resolve_style_char
    let line_height = fonts :: matched, buffer.style :: line_height
    let baseline = fonts :: matched, buffer.style :: baseline
    let mut state = arcana_text.shape.pipeline.ShapeStageState :: default_style = default_style, paragraph = buffer.paragraph :: call
    state.runs = arcana_text.shape.types.empty_runs :: :: call
    state.plan_keys = arcana_text.shape.types.empty_plan_keys :: :: call
    state.unresolved = arcana_text.shape.types.empty_unresolved :: :: call
    state.fonts_used = arcana_text.shape.types.empty_matches :: :: call
    state.signature = 23
    state.default_line_height = arcana_text.shape.types.max_int :: line_height, (buffer.style.size + 6) :: call
    state.default_baseline = arcana_text.shape.types.max_int :: baseline, buffer.style.size :: call
    return state

fn fallback_placeholder(read range: arcana_text.types.TextRange) -> arcana_text.types.PlaceholderSpec:
    let mut spec = arcana_text.types.PlaceholderSpec :: range = range, size = (0, 0), alignment = (arcana_text.types.PlaceholderAlignment.Baseline :: :: call) :: call
    spec.baseline = arcana_text.types.TextBaseline.Alphabetic :: :: call
    spec.baseline_offset = 0
    return spec

fn snapshot_active(edit fonts: arcana_text.fonts.FontSystem, read buffer: arcana_text.buffer.TextBuffer) -> arcana_text.shape.types.ShapeSnapshot:
    let mut state = seed_state :: fonts, buffer :: call
    let items = arcana_text.shape.tokens.collect_items :: buffer :: call
    for item in items:
        let span_style = arcana_text.shape.styles.style_for_range :: buffer, item.range :: call
        let mut prepared = arcana_text.shape.glyphs.shape_placeholder :: span_style, (arcana_text.shape.pipeline.fallback_placeholder :: item.range :: call) :: call
        if item.kind == (arcana_text.shape.tokens.ShapeItemKind.Placeholder :: :: call):
            let spec = item.placeholder :: (arcana_text.shape.pipeline.fallback_placeholder :: item.range :: call) :: unwrap_or
            prepared = arcana_text.shape.glyphs.shape_placeholder :: span_style, spec :: call
        else:
            let mut token = arcana_text.shape.tokens.text_token :: item.text, item.range.start, item.range.end :: call
            token.whitespace = item.whitespace
            token.newline = item.newline
            prepared = arcana_text.shape.glyphs.shape_token :: fonts, token, span_style :: call
        fonts.shape_cache :: prepared.run.plan_key :: remember_plan
        fonts.shape_cache :: prepared.run.range, prepared.run.plan_key.font_size + prepared.run.width :: remember_run_signature
        append_run :: state, prepared :: call
    ShapePassState.done = true
    let mut out = arcana_text.shape.types.ShapeSnapshot :: source_version = buffer.version, default_style = state.default_style, paragraph = state.paragraph :: call
    out.signature = state.signature
    out.default_line_height = state.default_line_height
    out.default_baseline = state.default_baseline
    out.runs = state.runs
    out.plan_keys = state.plan_keys
    out.unresolved = state.unresolved
    out.fonts_used = state.fonts_used
    return out

export fn snapshot(edit fonts: arcana_text.fonts.FontSystem, read buffer: arcana_text.buffer.TextBuffer) -> arcana_text.shape.types.ShapeSnapshot:
    let ctx = ShapePassContext :: buffer_version = buffer.version, font_count = (fonts :: :: count) :: call
    let active = ShapePass :: ctx :: call
    let _ = active
    return snapshot_active :: fonts, buffer :: call
