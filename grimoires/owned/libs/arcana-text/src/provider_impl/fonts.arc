import arcana_text.monaspace
import arcana_text.provider_impl.assets
import arcana_text.provider_impl.engine
import arcana_text.provider_impl.font
import std.bytes
import std.collections.array
import std.collections.list
import std.fs
import std.option
import std.path
import std.result
import std.text
use std.option.Option
use std.result.Result

record FileLoadSpec:
    family_alias: Str
    path: Str
    traits: arcana_text.provider_impl.font.FaceTraits

fn new_collection() -> arcana_text.provider_impl.engine.FontCollectionState:
    return arcana_text.provider_impl.engine.new_collection_state :: :: call

fn empty_strings() -> List[Str]:
    return std.collections.list.new[Str] :: :: call

fn empty_bytes() -> Array[Int]:
    return std.collections.array.from_list[Int] :: (std.collections.list.new[Int] :: :: call) :: call

fn result_err_or[T](read value: Result[T, Str], read fallback: Str) -> Str:
    return match value:
        Result.Ok(_) => fallback
        Result.Err(err) => err

fn push_unique_string(edit values: List[Str], read value: Str):
    for existing in values:
        if existing == value:
            return
    values :: value :: push

fn add_family_name(edit collection: arcana_text.provider_impl.engine.FontCollectionState, family: Str):
    push_unique_string :: collection.registered_families, family :: call

fn family_alias_or_path(read collection: arcana_text.provider_impl.engine.FontCollectionState, path: Str) -> Str:
    for family in collection.registered_families:
        return family
    return match (std.path.stem :: path :: call):
        Result.Ok(value) => value
        Result.Err(_) => (std.path.file_name :: path :: call)

fn monaspace_render_form(read form: arcana_text.monaspace.MonaspaceForm) -> arcana_text.monaspace.MonaspaceForm:
    return match form:
        arcana_text.monaspace.MonaspaceForm.Static => arcana_text.monaspace.MonaspaceForm.Frozen :: :: call
        arcana_text.monaspace.MonaspaceForm.Nerd => arcana_text.monaspace.MonaspaceForm.Frozen :: :: call
        _ => form

fn monaspace_traits_from_path(path: Str) -> arcana_text.provider_impl.font.FaceTraits:
    let mut traits = arcana_text.provider_impl.font.default_traits :: :: call
    let name = std.path.file_name :: path :: call
    if std.text.contains :: name, "ExtraLight" :: call:
        traits.weight = 200
    else:
        if std.text.contains :: name, "Light" :: call:
            traits.weight = 300
    if std.text.contains :: name, "ExtraBold" :: call:
        traits.weight = 800
    else:
        if std.text.contains :: name, "SemiBold" :: call:
            traits.weight = 600
        else:
            if std.text.contains :: name, "Bold" :: call:
                traits.weight = 700
    if std.text.contains :: name, "Medium" :: call:
        traits.weight = 500
    if std.text.contains :: name, "Regular" :: call:
        traits.weight = 400
    if std.text.contains :: name, "SemiWide" :: call:
        traits.width_milli = 115000
    else:
        if std.text.contains :: name, "Wide" :: call:
            traits.width_milli = 130000
    if std.text.contains :: name, "Italic" :: call:
        traits.slant_milli = -12000
    return traits

fn file_load_spec(family_alias: Str, path: Str, read traits: arcana_text.provider_impl.font.FaceTraits) -> arcana_text.provider_impl.fonts.FileLoadSpec:
    return arcana_text.provider_impl.fonts.FileLoadSpec :: family_alias = family_alias, path = path, traits = traits :: call

fn registered_face_path(family_alias: Str, source_path: Str, read traits: arcana_text.provider_impl.font.FaceTraits) -> arcana_text.provider_impl.engine.RegisteredFace:
    let source_label = match (std.path.stem :: source_path :: call):
        Result.Ok(value) => value
        Result.Err(_) => (std.path.file_name :: source_path :: call)
    let mut entry = arcana_text.provider_impl.engine.RegisteredFace :: family_alias = family_alias, source_label = source_label, source_path = source_path :: call
    entry.source_bytes = empty_bytes :: :: call
    entry.traits = traits
    entry.face = Option.None[arcana_text.provider_impl.font.FontFaceState] :: :: call
    entry.load_error = ""
    return entry

fn registered_face_bytes(family_alias: Str, label: Str, read payload: (Array[Int], arcana_text.provider_impl.font.FaceTraits)) -> arcana_text.provider_impl.engine.RegisteredFace:
    let bytes = payload.0
    let traits = payload.1
    let mut entry = arcana_text.provider_impl.engine.RegisteredFace :: family_alias = family_alias, source_label = label, source_path = label :: call
    entry.source_bytes = bytes
    entry.traits = traits
    entry.face = Option.None[arcana_text.provider_impl.font.FontFaceState] :: :: call
    entry.load_error = ""
    return entry

fn push_registered_face(edit collection: arcana_text.provider_impl.engine.FontCollectionState, read entry: arcana_text.provider_impl.engine.RegisteredFace):
    collection.faces :: entry :: push

fn add_source(edit collection: arcana_text.provider_impl.engine.FontCollectionState, source: Str):
    push_unique_string :: collection.registered_sources, source :: call

fn add_file_with_traits(edit collection: arcana_text.provider_impl.engine.FontCollectionState, read spec: arcana_text.provider_impl.fonts.FileLoadSpec) -> Result[Unit, Str]:
    if not (std.fs.is_file :: spec.path :: call):
        return Result.Err[Unit, Str] :: ("font file does not exist: " + spec.path) :: call
    add_source :: collection, spec.path :: call
    push_registered_face :: collection, (registered_face_path :: spec.family_alias, spec.path, spec.traits :: call) :: call
    return Result.Ok[Unit, Str] :: :: call

fn add_file(edit collection: arcana_text.provider_impl.engine.FontCollectionState, path: Str) -> Result[Unit, Str]:
    let spec = file_load_spec :: (family_alias_or_path :: collection, path :: call), path, (arcana_text.provider_impl.font.default_traits :: :: call) :: call
    return add_file_with_traits :: collection, spec :: call

fn add_dir(edit collection: arcana_text.provider_impl.engine.FontCollectionState, path: Str) -> Result[Int, Str]:
    let entries_result = std.fs.list_dir :: path :: call
    if entries_result :: :: is_err:
        return Result.Err[Int, Str] :: (result_err_or :: entries_result, "failed to list font directory" :: call) :: call
    let entries = entries_result :: (empty_strings :: :: call) :: unwrap_or
    let mut added = 0
    for entry in entries:
        let ext = std.path.ext :: entry :: call
        if ext != "ttf" and ext != "otf":
            continue
        let add_result = add_file :: collection, entry :: call
        if add_result :: :: is_ok:
            added += 1
    return Result.Ok[Int, Str] :: added :: call

fn add_bytes(edit collection: arcana_text.provider_impl.engine.FontCollectionState, label: Str, read bytes: Array[Int]) -> Result[Unit, Str]:
    if (std.bytes.len :: bytes :: call) <= 0:
        return Result.Err[Unit, Str] :: "font bytes are empty" :: call
    let family_alias = family_alias_or_path :: collection, label :: call
    let traits = arcana_text.provider_impl.font.default_traits :: :: call
    let payload = (bytes, traits)
    add_source :: collection, label :: call
    push_registered_face :: collection, (registered_face_bytes :: family_alias, label, payload :: call) :: call
    return Result.Ok[Unit, Str] :: :: call

fn set_host_fallback(edit collection: arcana_text.provider_impl.engine.FontCollectionState, enabled: Bool):
    collection.host_fallback_enabled = enabled

fn add_source_ready(edit collection: arcana_text.provider_impl.engine.FontCollectionState, source: Str) -> Result[Unit, Str]:
    return add_file :: collection, source :: call

fn add_monaspace_path(edit collection: arcana_text.provider_impl.engine.FontCollectionState, family_alias: Str, path: Str) -> Result[Unit, Str]:
    add_family_name :: collection, family_alias :: call
    return add_file_with_traits :: collection, (file_load_spec :: family_alias, path, (monaspace_traits_from_path :: path :: call) :: call) :: call

fn add_monaspace(edit collection: arcana_text.provider_impl.engine.FontCollectionState, read family: arcana_text.monaspace.MonaspaceFamily, read form: arcana_text.monaspace.MonaspaceForm) -> Result[Unit, Str]:
    let family_alias = arcana_text.monaspace.family_name :: family, form :: call
    add_family_name :: collection, family_alias :: call
    let render_form = monaspace_render_form :: form :: call
    return match (arcana_text.provider_impl.assets.monaspace_source_path :: family, render_form :: call):
        Result.Ok(value) => add_file_with_traits :: collection, (file_load_spec :: family_alias, value, (monaspace_traits_from_path :: value :: call) :: call) :: call
        Result.Err(err) => Result.Err[Unit, Str] :: err :: call

fn cleared_face_value(read face: arcana_text.provider_impl.font.FontFaceState) -> arcana_text.provider_impl.font.FontFaceState:
    let mut cleared = face
    cleared.bitmap_cache = std.collections.map.new[Str, arcana_text.provider_impl.font.GlyphBitmap] :: :: call
    return cleared

fn cleared_face_option(read value: Option[arcana_text.provider_impl.font.FontFaceState]) -> Option[arcana_text.provider_impl.font.FontFaceState]:
    return match value:
        Option.Some(face) => Option.Some[arcana_text.provider_impl.font.FontFaceState] :: (cleared_face_value :: face :: call) :: call
        Option.None => Option.None[arcana_text.provider_impl.font.FontFaceState] :: :: call

fn clear_cache(edit collection: arcana_text.provider_impl.engine.FontCollectionState):
    let mut faces = std.collections.list.new[arcana_text.provider_impl.engine.RegisteredFace] :: :: call
    for entry in collection.faces:
        let mut next = entry
        next.face = cleared_face_option :: entry.face :: call
        faces :: next :: push
    collection.faces = faces

fn default_collection() -> arcana_text.provider_impl.engine.FontCollectionState:
    let mut collection = new_collection :: :: call
    let _ = add_monaspace :: collection, (arcana_text.monaspace.MonaspaceFamily.Neon :: :: call), (arcana_text.monaspace.MonaspaceForm.Variable :: :: call) :: call
    return collection
