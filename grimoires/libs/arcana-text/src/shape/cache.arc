import arcana_text.types
import arcana_text.shape.tokens
import std.collections.list
import std.collections.map
import std.option
import std.text
use arcana_text.shape.types as shape_types
use std.option.Option

export obj ShapeCache:
    generation: Int
    plans: Map[Str, arcana_text.types.ShapePlanKey]
    run_signatures: Map[Str, Int]
    prepared_runs: Map[Str, List[arcana_text.types.PreparedRun]]
    prepared_lines: Map[Str, List[arcana_text.types.PreparedRun]]
    prepared_layout_lines: Map[Str, arcana_text.types.PreparedLayoutLine]

fn empty_plans() -> Map[Str, arcana_text.types.ShapePlanKey]:
    return std.collections.map.empty[Str, arcana_text.types.ShapePlanKey] :: :: call

fn empty_run_signatures() -> Map[Str, Int]:
    return std.collections.map.empty[Str, Int] :: :: call

fn empty_prepared_runs() -> Map[Str, List[arcana_text.types.PreparedRun]]:
    return std.collections.map.empty[Str, List[arcana_text.types.PreparedRun]] :: :: call

fn empty_prepared_lines() -> Map[Str, List[arcana_text.types.PreparedRun]]:
    return std.collections.map.empty[Str, List[arcana_text.types.PreparedRun]] :: :: call

fn empty_prepared_layout_lines() -> Map[Str, arcana_text.types.PreparedLayoutLine]:
    return std.collections.map.empty[Str, arcana_text.types.PreparedLayoutLine] :: :: call

fn prepared_run_limit() -> Int:
    return 512

fn prepared_line_limit() -> Int:
    return 256

fn prepared_layout_line_limit() -> Int:
    return 128

export fn open() -> arcana_text.shape.cache.ShapeCache:
    let mut cache = arcana_text.shape.cache.ShapeCache :: generation = 0, plans = (arcana_text.shape.cache.empty_plans :: :: call), run_signatures = (arcana_text.shape.cache.empty_run_signatures :: :: call) :: call
    cache.prepared_runs = arcana_text.shape.cache.empty_prepared_runs :: :: call
    cache.prepared_lines = arcana_text.shape.cache.empty_prepared_lines :: :: call
    cache.prepared_layout_lines = arcana_text.shape.cache.empty_prepared_layout_lines :: :: call
    return cache

export fn plan_key_text(read key: arcana_text.types.ShapePlanKey) -> Str:
    return (std.text.from_int :: key.face_id.source_index :: call) + ":" + (std.text.from_int :: key.face_id.face_index :: call) + ":" + (std.text.from_int :: key.font_size :: call) + ":" + (std.text.from_int :: key.weight :: call) + ":" + (std.text.from_int :: key.width_milli :: call) + ":" + (std.text.from_int :: key.slant_milli :: call) + ":" + (std.text.from_int :: key.feature_signature :: call) + ":" + (std.text.from_int :: key.axis_signature :: call) + ":" + key.language_tag

export fn run_key(read range: arcana_text.types.TextRange, signature: Int) -> Str:
    return (std.text.from_int :: range.start :: call) + ":" + (std.text.from_int :: range.end :: call) + ":" + (std.text.from_int :: signature :: call)

export fn run_signature_value(read run: arcana_text.types.ShapedRun) -> Int:
    return run.plan_key.font_size + run.width

fn bool_key(value: Bool) -> Str:
    return match value:
        true => "1"
        false => "0"

fn bool_value(value: Bool) -> Int:
    return match value:
        true => 1
        false => 0

fn underline_code(read value: arcana_text.types.UnderlineStyle) -> Int:
    return match value:
        arcana_text.types.UnderlineStyle.Single => 2
        arcana_text.types.UnderlineStyle.Double => 3
        _ => 1

fn families_key(read families: List[Str]) -> Str:
    let mut out = ""
    let mut index = 0
    for family in families:
        if index > 0:
            out = out + "|"
        out = out + family
        index += 1
    return out

fn features_key(read features: List[arcana_text.types.FontFeature]) -> Str:
    let mut out = ""
    let mut index = 0
    for feature in features:
        if index > 0:
            out = out + "|"
        out = out + feature.tag + ":" + (std.text.from_int :: feature.value :: call) + ":" + (arcana_text.shape.cache.bool_key :: feature.enabled :: call)
        index += 1
    return out

fn axes_key(read axes: List[arcana_text.types.FontAxis]) -> Str:
    let mut out = ""
    let mut index = 0
    for axis in axes:
        if index > 0:
            out = out + "|"
        out = out + axis.tag + ":" + (std.text.from_int :: axis.value :: call)
        index += 1
    return out

export fn prepared_run_key(read token: arcana_text.shape.tokens.TextToken, read style: arcana_text.types.SpanStyle, bidi_signature: Int) -> Str:
    let mut signature = 41
    signature = shape_types.mix_signature :: signature, style.color :: call
    signature = shape_types.mix_signature :: signature, style.background_color :: call
    signature = shape_types.mix_signature :: signature, style.underline_color :: call
    signature = shape_types.mix_signature :: signature, style.strikethrough_color :: call
    signature = shape_types.mix_signature :: signature, style.overline_color :: call
    signature = shape_types.mix_signature :: signature, underline_code :: style.underline :: call
    signature = shape_types.mix_signature :: signature, bool_value :: style.underline_color_enabled :: call
    signature = shape_types.mix_signature :: signature, bool_value :: style.strikethrough_enabled :: call
    signature = shape_types.mix_signature :: signature, bool_value :: style.strikethrough_color_enabled :: call
    signature = shape_types.mix_signature :: signature, bool_value :: style.overline_enabled :: call
    signature = shape_types.mix_signature :: signature, bool_value :: style.overline_color_enabled :: call
    signature = shape_types.mix_signature :: signature, style.size :: call
    signature = shape_types.mix_signature :: signature, style.letter_spacing :: call
    signature = shape_types.mix_signature :: signature, style.line_height :: call
    signature = shape_types.mix_signature :: signature, (arcana_text.types.feature_signature :: style.features :: call) :: call
    signature = shape_types.mix_signature :: signature, (arcana_text.types.axis_signature :: style.axes :: call) :: call
    signature = shape_types.mix_signature :: signature, bidi_signature :: call
    signature = shape_types.mix_signature_text :: signature, token.text :: call
    return (arcana_text.shape.cache.bool_key :: token.newline :: call) + ":" + (arcana_text.shape.cache.bool_key :: token.whitespace :: call) + ":" + (std.text.from_int :: style.color :: call) + ":" + (arcana_text.shape.cache.bool_key :: style.background_enabled :: call) + ":" + (std.text.from_int :: style.background_color :: call) + ":" + (std.text.from_int :: (arcana_text.shape.cache.underline_code :: style.underline :: call) :: call) + ":" + (arcana_text.shape.cache.bool_key :: style.underline_color_enabled :: call) + ":" + (std.text.from_int :: style.underline_color :: call) + ":" + (arcana_text.shape.cache.bool_key :: style.strikethrough_enabled :: call) + ":" + (arcana_text.shape.cache.bool_key :: style.strikethrough_color_enabled :: call) + ":" + (std.text.from_int :: style.strikethrough_color :: call) + ":" + (arcana_text.shape.cache.bool_key :: style.overline_enabled :: call) + ":" + (arcana_text.shape.cache.bool_key :: style.overline_color_enabled :: call) + ":" + (std.text.from_int :: style.overline_color :: call) + ":" + (std.text.from_int :: style.size :: call) + ":" + (std.text.from_int :: style.letter_spacing :: call) + ":" + (std.text.from_int :: style.line_height :: call) + ":" + (arcana_text.shape.cache.families_key :: style.families :: call) + ":" + (arcana_text.shape.cache.features_key :: style.features :: call) + ":" + (arcana_text.shape.cache.axes_key :: style.axes :: call) + ":" + (std.text.from_int :: bidi_signature :: call) + ":" + (std.text.from_int :: signature :: call) + ":" + token.text

fn copy_prepared_runs(read runs: List[arcana_text.types.PreparedRun]) -> List[arcana_text.types.PreparedRun]:
    let mut out = std.collections.list.empty[arcana_text.types.PreparedRun] :: :: call
    out :: runs :: extend_list
    return out

fn copy_snapshot_lines(read values: List[arcana_text.types.SnapshotLine]) -> List[arcana_text.types.SnapshotLine]:
    let mut out = std.collections.list.empty[arcana_text.types.SnapshotLine] :: :: call
    out :: values :: extend_list
    return out

fn copy_layout_runs(read values: List[arcana_text.types.LayoutRun]) -> List[arcana_text.types.LayoutRun]:
    let mut out = std.collections.list.empty[arcana_text.types.LayoutRun] :: :: call
    out :: values :: extend_list
    return out

fn copy_layout_glyphs(read values: List[arcana_text.types.LayoutGlyph]) -> List[arcana_text.types.LayoutGlyph]:
    let mut out = std.collections.list.empty[arcana_text.types.LayoutGlyph] :: :: call
    out :: values :: extend_list
    return out

fn copy_unresolved(read values: List[arcana_text.types.UnresolvedGlyph]) -> List[arcana_text.types.UnresolvedGlyph]:
    let mut out = std.collections.list.empty[arcana_text.types.UnresolvedGlyph] :: :: call
    out :: values :: extend_list
    return out

fn copy_matches(read values: List[arcana_text.types.FontMatch]) -> List[arcana_text.types.FontMatch]:
    let mut out = std.collections.list.empty[arcana_text.types.FontMatch] :: :: call
    out :: values :: extend_list
    return out

fn copy_prepared_layout_line(read value: arcana_text.types.PreparedLayoutLine) -> arcana_text.types.PreparedLayoutLine:
    let mut out = value
    out.lines = arcana_text.shape.cache.copy_snapshot_lines :: value.lines :: call
    out.runs = arcana_text.shape.cache.copy_layout_runs :: value.runs :: call
    out.glyphs = arcana_text.shape.cache.copy_layout_glyphs :: value.glyphs :: call
    out.unresolved = arcana_text.shape.cache.copy_unresolved :: value.unresolved :: call
    out.fonts_used = arcana_text.shape.cache.copy_matches :: value.fonts_used :: call
    return out

impl ShapeCache:
    fn remember_plan(edit self: arcana_text.shape.cache.ShapeCache, read key: arcana_text.types.ShapePlanKey):
        let text = arcana_text.shape.cache.plan_key_text :: key :: call
        if self.plans :: text :: has:
            return
        let stored = key
        self.plans :: text, stored :: set
        self.generation += 1

    fn cached_plan(read self: arcana_text.shape.cache.ShapeCache, read key: arcana_text.types.ShapePlanKey) -> Bool:
        return self.plans :: (arcana_text.shape.cache.plan_key_text :: key :: call) :: has

    fn remember_run_signature(edit self: arcana_text.shape.cache.ShapeCache, read range: arcana_text.types.TextRange, signature: Int):
        let key = arcana_text.shape.cache.run_key :: range, signature :: call
        if self.run_signatures :: key :: has:
            return
        self.run_signatures :: key, signature :: set
        self.generation += 1

    fn cached_run_signature(read self: arcana_text.shape.cache.ShapeCache, read range: arcana_text.types.TextRange, signature: Int) -> Bool:
        return self.run_signatures :: (arcana_text.shape.cache.run_key :: range, signature :: call) :: has

    fn cached_prepared_runs(read self: arcana_text.shape.cache.ShapeCache, key: Str) -> Option[List[arcana_text.types.PreparedRun]]:
        if not (self.prepared_runs :: key :: has):
            return Option.None[List[arcana_text.types.PreparedRun]] :: :: call
        let stored = self.prepared_runs :: key :: get
        return Option.Some[List[arcana_text.types.PreparedRun]] :: (arcana_text.shape.cache.copy_prepared_runs :: stored :: call) :: call

    fn remember_prepared_runs(edit self: arcana_text.shape.cache.ShapeCache, key: Str, read runs: List[arcana_text.types.PreparedRun]):
        let had_key = self.prepared_runs :: key :: has
        let mut mutated = false
        if not had_key and (self.prepared_runs :: :: len) >= (arcana_text.shape.cache.prepared_run_limit :: :: call):
            self.prepared_runs :: :: clear
            mutated = true
        self.prepared_runs :: key, (arcana_text.shape.cache.copy_prepared_runs :: runs :: call) :: set
        if not had_key:
            mutated = true
        if mutated:
            self.generation += 1

    fn cached_prepared_line(read self: arcana_text.shape.cache.ShapeCache, key: Str) -> Option[List[arcana_text.types.PreparedRun]]:
        if not (self.prepared_lines :: key :: has):
            return Option.None[List[arcana_text.types.PreparedRun]] :: :: call
        let stored = self.prepared_lines :: key :: get
        return Option.Some[List[arcana_text.types.PreparedRun]] :: (arcana_text.shape.cache.copy_prepared_runs :: stored :: call) :: call

    fn remember_prepared_line(edit self: arcana_text.shape.cache.ShapeCache, key: Str, read runs: List[arcana_text.types.PreparedRun]):
        let had_key = self.prepared_lines :: key :: has
        let mut mutated = false
        if not had_key and (self.prepared_lines :: :: len) >= (arcana_text.shape.cache.prepared_line_limit :: :: call):
            self.prepared_lines :: :: clear
            mutated = true
        self.prepared_lines :: key, (arcana_text.shape.cache.copy_prepared_runs :: runs :: call) :: set
        if not had_key:
            mutated = true
        if mutated:
            self.generation += 1

    fn cached_prepared_layout_line(read self: arcana_text.shape.cache.ShapeCache, key: Str) -> Option[arcana_text.types.PreparedLayoutLine]:
        if not (self.prepared_layout_lines :: key :: has):
            return Option.None[arcana_text.types.PreparedLayoutLine] :: :: call
        let stored = self.prepared_layout_lines :: key :: get
        return Option.Some[arcana_text.types.PreparedLayoutLine] :: (arcana_text.shape.cache.copy_prepared_layout_line :: stored :: call) :: call

    fn remember_prepared_layout_line(edit self: arcana_text.shape.cache.ShapeCache, key: Str, read value: arcana_text.types.PreparedLayoutLine):
        let had_key = self.prepared_layout_lines :: key :: has
        let mut mutated = false
        if not had_key and (self.prepared_layout_lines :: :: len) >= (arcana_text.shape.cache.prepared_layout_line_limit :: :: call):
            self.prepared_layout_lines :: :: clear
            mutated = true
        self.prepared_layout_lines :: key, (arcana_text.shape.cache.copy_prepared_layout_line :: value :: call) :: set
        if not had_key:
            mutated = true
        if mutated:
            self.generation += 1

    fn remember_shaped_run(edit self: arcana_text.shape.cache.ShapeCache, read run: arcana_text.types.ShapedRun):
        if run.plan_key.face_id.source_index >= 0 and not (self :: run.plan_key :: cached_plan):
            self :: run.plan_key :: remember_plan
        let signature = arcana_text.shape.cache.run_signature_value :: run :: call
        if not (self :: run.range, signature :: cached_run_signature):
            self :: run.range, signature :: remember_run_signature
