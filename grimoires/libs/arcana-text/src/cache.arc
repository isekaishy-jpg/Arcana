import arcana_text.types
import std.collections.map
import std.option
use std.option.Option

export record CachedGlyphImage:
    offset: (Int, Int)
    surface: arcana_text.types.GlyphSurface

export obj TextCache:
    generation: Int
    last_snapshot_version: Int
    glyph_images: Map[Str, arcana_text.cache.CachedGlyphImage]

export fn open() -> arcana_text.cache.TextCache:
    return arcana_text.cache.TextCache :: generation = 0, last_snapshot_version = 0, glyph_images = (std.collections.map.empty[Str, arcana_text.cache.CachedGlyphImage] :: :: call) :: call

fn glyph_limit() -> Int:
    return 4096

impl TextCache:
    fn touch_snapshot(edit self: arcana_text.cache.TextCache, version: Int):
        if self.last_snapshot_version == version:
            return
        self.generation += 1
        self.last_snapshot_version = version

    fn cached_glyph_image(read self: arcana_text.cache.TextCache, key: Str) -> Option[arcana_text.cache.CachedGlyphImage]:
        if not (self.glyph_images :: key :: has):
            return Option.None[arcana_text.cache.CachedGlyphImage] :: :: call
        return Option.Some[arcana_text.cache.CachedGlyphImage] :: (self.glyph_images :: key :: get) :: call

    fn remember_glyph_image(edit self: arcana_text.cache.TextCache, key: Str, read image: arcana_text.cache.CachedGlyphImage):
        let had_key = self.glyph_images :: key :: has
        let mut mutated = false
        if not had_key and (self.glyph_images :: :: len) >= (arcana_text.cache.glyph_limit :: :: call):
            self.glyph_images :: :: clear
            mutated = true
        self.glyph_images :: key, image :: set
        if not had_key:
            mutated = true
        if mutated:
            self.generation += 1
