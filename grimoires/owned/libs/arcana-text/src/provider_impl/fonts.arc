import arcana_text.monaspace
import arcana_text.provider_impl.assets
import arcana_text.provider_impl.engine
import std.result
use std.result.Result

fn new_collection() -> arcana_text.provider_impl.engine.FontCollectionState:
    return arcana_text.provider_impl.engine.new_collection_state :: :: call

fn add_family_name(edit collection: arcana_text.provider_impl.engine.FontCollectionState, family: Str):
    collection.registered_families :: family :: push

fn add_source(edit collection: arcana_text.provider_impl.engine.FontCollectionState, source: Str):
    collection.registered_sources :: source :: push

fn set_host_fallback(edit collection: arcana_text.provider_impl.engine.FontCollectionState, enabled: Bool):
    collection.host_fallback_enabled = enabled

fn add_source_ready(edit collection: arcana_text.provider_impl.engine.FontCollectionState, source: Str) -> Result[Unit, Str]:
    add_source :: collection, source :: call
    return Result.Ok[Unit, Str] :: :: call

fn add_monaspace(edit collection: arcana_text.provider_impl.engine.FontCollectionState, read family: arcana_text.monaspace.MonaspaceFamily, read form: arcana_text.monaspace.MonaspaceForm) -> Result[Unit, Str]:
    add_family_name :: collection, (arcana_text.monaspace.family_name :: family, form :: call) :: call
    return match (arcana_text.provider_impl.assets.monaspace_source_path :: family, form :: call):
        Result.Ok(value) => add_source_ready :: collection, value :: call
        Result.Err(err) => Result.Err[Unit, Str] :: err :: call

fn default_collection() -> arcana_text.provider_impl.engine.FontCollectionState:
    let mut collection = new_collection :: :: call
    add_monaspace :: collection, (arcana_text.monaspace.MonaspaceFamily.Neon :: :: call), (arcana_text.monaspace.MonaspaceForm.Variable :: :: call) :: call
    return collection
