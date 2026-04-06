import arcana_text.types
import std.collections.map
import std.option
use std.option.Option

export obj TextCache:
    generation: Int
    last_snapshot_version: Int
    glyph_surfaces: Map[Str, arcana_text.types.GlyphSurface]

export fn open() -> arcana_text.cache.TextCache:
    return arcana_text.cache.TextCache :: generation = 0, last_snapshot_version = 0, glyph_surfaces = (std.collections.map.empty[Str, arcana_text.types.GlyphSurface] :: :: call) :: call

fn glyph_limit() -> Int:
    return 512

impl TextCache:
    fn touch_snapshot(edit self: arcana_text.cache.TextCache, version: Int):
        if self.last_snapshot_version == version:
            return
        self.generation += 1
        self.last_snapshot_version = version

    fn cached_glyph_surface(read self: arcana_text.cache.TextCache, key: Str) -> Option[arcana_text.types.GlyphSurface]:
        if not (self.glyph_surfaces :: key :: has):
            return Option.None[arcana_text.types.GlyphSurface] :: :: call
        return Option.Some[arcana_text.types.GlyphSurface] :: (self.glyph_surfaces :: key :: get) :: call

    fn remember_glyph_surface(edit self: arcana_text.cache.TextCache, key: Str, read surface: arcana_text.types.GlyphSurface):
        if not (self.glyph_surfaces :: key :: has) and (self.glyph_surfaces :: :: len) >= (arcana_text.cache.glyph_limit :: :: call):
            self.glyph_surfaces :: :: clear
        self.glyph_surfaces :: key, surface :: set
