import arcana_text.assets
import arcana_text.monaspace
import arcana_text.provider_impl.fonts
import arcana_text.types
import std.result
use std.result.Result

fn add_monaspace_source_ready(edit collection: arcana_text.types.FontCollection, source: Str) -> Result[Unit, Str]:
    arcana_text.provider_impl.fonts.add_source :: collection, source :: call
    return Result.Ok[Unit, Str] :: :: call

export fn new_collection() -> arcana_text.types.FontCollection:
    return arcana_text.fonts.new_collection :: :: call

export fn add_family_name(edit collection: arcana_text.types.FontCollection, family: Str):
    arcana_text.provider_impl.fonts.add_family_name :: collection, family :: call

export fn add_source(edit collection: arcana_text.types.FontCollection, source: Str):
    arcana_text.provider_impl.fonts.add_source :: collection, source :: call

export fn set_host_fallback(edit collection: arcana_text.types.FontCollection, enabled: Bool):
    arcana_text.provider_impl.fonts.set_host_fallback :: collection, enabled :: call

export fn add_monaspace(edit collection: arcana_text.types.FontCollection, read family: arcana_text.monaspace.MonaspaceFamily, read form: arcana_text.monaspace.MonaspaceForm) -> Result[Unit, Str]:
    arcana_text.provider_impl.fonts.add_family_name :: collection, (arcana_text.monaspace.family_name :: family, form :: call) :: call
    return match (arcana_text.assets.monaspace_source_path :: family, form :: call):
        Result.Ok(value) => add_monaspace_source_ready :: collection, value :: call
        Result.Err(err) => Result.Err[Unit, Str] :: err :: call

export fn default_collection() -> arcana_text.types.FontCollection:
    let mut collection = arcana_text.fonts.new_collection :: :: call
    arcana_text.fonts.add_monaspace :: collection, (arcana_text.monaspace.MonaspaceFamily.Neon :: :: call), (arcana_text.monaspace.MonaspaceForm.Variable :: :: call) :: call
    return collection
