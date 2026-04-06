import arcana_text.font_leaf
import std.memory
import std.result
use std.result.Result

export fn load_face_from_view(read request: arcana_text.font_leaf.FaceLoadRequest, read bytes_view: std.memory.ByteView) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    return arcana_text.font_leaf.load_face_from_view :: request, bytes_view :: call

export fn load_face_from_bytes(read request: arcana_text.font_leaf.FaceLoadRequest) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    return arcana_text.font_leaf.load_face_from_bytes :: request :: call

export fn load_face_from_path(family_name: Str, path: Str, read traits: arcana_text.font_leaf.FaceTraits) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    return arcana_text.font_leaf.load_face_from_path :: family_name, path, traits :: call
