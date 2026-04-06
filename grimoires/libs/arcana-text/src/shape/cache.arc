import arcana_text.types
import std.collections.map
import std.text

export obj ShapeCache:
    generation: Int
    plans: Map[Str, arcana_text.types.ShapePlanKey]
    run_signatures: Map[Str, Int]

fn empty_plans() -> Map[Str, arcana_text.types.ShapePlanKey]:
    return std.collections.map.empty[Str, arcana_text.types.ShapePlanKey] :: :: call

fn empty_run_signatures() -> Map[Str, Int]:
    return std.collections.map.empty[Str, Int] :: :: call

export fn open() -> arcana_text.shape.cache.ShapeCache:
    return arcana_text.shape.cache.ShapeCache :: generation = 0, plans = (arcana_text.shape.cache.empty_plans :: :: call), run_signatures = (arcana_text.shape.cache.empty_run_signatures :: :: call) :: call

export fn plan_key_text(read key: arcana_text.types.ShapePlanKey) -> Str:
    return (std.text.from_int :: key.face_id.source_index :: call) + ":" + (std.text.from_int :: key.face_id.face_index :: call) + ":" + (std.text.from_int :: key.font_size :: call) + ":" + (std.text.from_int :: key.weight :: call) + ":" + (std.text.from_int :: key.width_milli :: call) + ":" + (std.text.from_int :: key.slant_milli :: call) + ":" + (std.text.from_int :: key.feature_signature :: call) + ":" + (std.text.from_int :: key.axis_signature :: call) + ":" + key.language_tag

export fn run_key(read range: arcana_text.types.TextRange, signature: Int) -> Str:
    return (std.text.from_int :: range.start :: call) + ":" + (std.text.from_int :: range.end :: call) + ":" + (std.text.from_int :: signature :: call)

impl ShapeCache:
    fn remember_plan(edit self: arcana_text.shape.cache.ShapeCache, read key: arcana_text.types.ShapePlanKey):
        let text = arcana_text.shape.cache.plan_key_text :: key :: call
        let stored = key
        self.plans :: text, stored :: set

    fn cached_plan(read self: arcana_text.shape.cache.ShapeCache, read key: arcana_text.types.ShapePlanKey) -> Bool:
        return self.plans :: (arcana_text.shape.cache.plan_key_text :: key :: call) :: has

    fn remember_run_signature(edit self: arcana_text.shape.cache.ShapeCache, read range: arcana_text.types.TextRange, signature: Int):
        self.run_signatures :: (arcana_text.shape.cache.run_key :: range, signature :: call), signature :: set
        self.generation += 1

    fn cached_run_signature(read self: arcana_text.shape.cache.ShapeCache, read range: arcana_text.types.TextRange, signature: Int) -> Bool:
        return self.run_signatures :: (arcana_text.shape.cache.run_key :: range, signature :: call) :: has
