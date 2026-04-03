import arcana_text.assets
import arcana_text.monaspace
import arcana_text.provider_impl.fonts
import arcana_text.types
import std.result
use std.result.Result

fn add_monaspace_source_ready(edit collection: arcana_text.types.FontCollection, source: Str) -> Result[Unit, Str]:
    return arcana_text.provider_impl.fonts.add_source_ready :: collection, source :: call

export fn new_collection() -> arcana_text.types.FontCollection:
    # TODO: This still recurses back through the public wrapper. We paused here
    # during memplan work because the missing memory-type substrate was making
    # the text grimoire hard to finish; resume by wiring this to provider_impl.
    return arcana_text.fonts.new_collection :: :: call

export fn add_family_name(edit collection: arcana_text.types.FontCollection, family: Str):
    arcana_text.provider_impl.fonts.add_family_name :: collection, family :: call

export fn add_source(edit collection: arcana_text.types.FontCollection, source: Str):
    let _ = arcana_text.provider_impl.fonts.add_source_ready :: collection, source :: call

export fn add_file(edit collection: arcana_text.types.FontCollection, path: Str) -> Result[Unit, Str]:
    return arcana_text.provider_impl.fonts.add_file :: collection, path :: call

export fn add_dir(edit collection: arcana_text.types.FontCollection, path: Str) -> Result[Int, Str]:
    return arcana_text.provider_impl.fonts.add_dir :: collection, path :: call

export fn add_bytes(edit collection: arcana_text.types.FontCollection, label: Str, read bytes: Array[Int]) -> Result[Unit, Str]:
    return arcana_text.provider_impl.fonts.add_bytes :: collection, label, bytes :: call

export fn set_host_fallback(edit collection: arcana_text.types.FontCollection, enabled: Bool):
    arcana_text.provider_impl.fonts.set_host_fallback :: collection, enabled :: call

export fn clear_cache(edit collection: arcana_text.types.FontCollection):
    arcana_text.provider_impl.fonts.clear_cache :: collection :: call

export fn add_monaspace(edit collection: arcana_text.types.FontCollection, read family: arcana_text.monaspace.MonaspaceFamily, read form: arcana_text.monaspace.MonaspaceForm) -> Result[Unit, Str]:
    let family_alias = arcana_text.monaspace.family_name :: family, form :: call
    return match (arcana_text.assets.monaspace_source_path :: family, form :: call):
        Result.Ok(value) => arcana_text.provider_impl.fonts.add_monaspace_path :: collection, family_alias, value :: call
        Result.Err(err) => Result.Err[Unit, Str] :: err :: call

export fn default_collection() -> arcana_text.types.FontCollection:
    # TODO: This currently inherits the recursive wrapper path through
    # arcana_text.fonts.new_collection until the provider-backed constructor is
    # wired through after the memory work resumes.
    let mut collection = arcana_text.fonts.new_collection :: :: call
    let _ = arcana_text.fonts.add_monaspace :: collection, (arcana_text.monaspace.MonaspaceFamily.Neon :: :: call), (arcana_text.monaspace.MonaspaceForm.Variable :: :: call) :: call
    return collection
