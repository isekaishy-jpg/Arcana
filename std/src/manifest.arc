import std.collections.map
import std.result
import std.text
use std.result.Result

record BookManifest:
    name: Str
    kind: Str
    data: Map[Str, Str]

record LockManifestV3:
    version: Int
    workspace: Str
    data: Map[Str, Str]

fn trim_ws(text: Str) -> Str:
    let n = std.text.len_bytes :: text :: call
    let mut start = 0
    while start < n and (std.text.is_space_byte :: (std.text.byte_at :: text, start :: call) :: call):
        start += 1
    let mut end = n
    while end > start and (std.text.is_space_byte :: (std.text.byte_at :: text, end - 1 :: call) :: call):
        end -= 1
    return std.text.slice_bytes :: text, start, end :: call

fn strip_quotes(text: Str) -> Str:
    let t = std.manifest.trim_ws :: text :: call
    let n = std.text.len_bytes :: t :: call
    if n >= 2 and (std.text.byte_at :: t, 0 :: call) == 34 and (std.text.byte_at :: t, n - 1 :: call) == 34:
        return std.text.slice_bytes :: t, 1, n - 1 :: call
    return t

fn parse_kv(line: Str) -> (Bool, (Str, Str)):
    let eq = std.text.find_byte :: line, 0, 61 :: call
    if eq < 0:
        return (false, ("", ""))
    let key = std.manifest.trim_ws :: (std.text.slice_bytes :: line, 0, eq :: call) :: call
    let value = std.manifest.trim_ws :: (std.text.slice_bytes :: line, eq + 1, (std.text.len_bytes :: line :: call) :: call) :: call
    return (true, (key, value))

fn parse_int_or(text: Str, fallback: Int) -> Int:
    let s = std.manifest.trim_ws :: text :: call
    let n = std.text.len_bytes :: s :: call
    if n == 0:
        return fallback
    let mut sign = 1
    let mut i = 0
    if (std.text.byte_at :: s, 0 :: call) == 45:
        sign = -1
        i = 1
    let mut value = 0
    while i < n:
        let b = std.text.byte_at :: s, i :: call
        if not (std.text.is_digit_byte :: b :: call):
            return fallback
        value = value * 10 + (b - 48)
        i += 1
    return value * sign

export fn parse_book(text: Str) -> Result[BookManifest, Str]:
    let mut name = ""
    let mut kind = ""
    let mut section = ""
    let mut data = std.collections.map.new[Str, Str] :: :: call
    let lines = std.text.split_lines :: text :: call
    for raw in lines:
        let line = std.manifest.trim_ws :: raw :: call
        if (std.text.len_bytes :: line :: call) == 0:
            continue
        if std.text.starts_with :: line, "[" :: call:
            section = line
            continue
        let kv = std.manifest.parse_kv :: line :: call
        if not kv.0:
            continue
        let key = std.manifest.strip_quotes :: kv.1.0 :: call
        let value = std.manifest.strip_quotes :: kv.1.1 :: call
        if section == "":
            if key == "name":
                name = value
            else:
                if key == "kind":
                    kind = value
        let mut full_key = key
        if section != "":
            full_key = section + "." + key
        data :: full_key, value :: set
    let manifest = std.manifest.BookManifest :: name = name, kind = kind, data = data :: call
    return Result.Ok[BookManifest, Str] :: manifest :: call

export fn parse_lock_v3(text: Str) -> Result[LockManifestV3, Str]:
    let mut version = 0
    let mut workspace = ""
    let mut section = ""
    let mut data = std.collections.map.new[Str, Str] :: :: call
    let lines = std.text.split_lines :: text :: call
    for raw in lines:
        let line = std.manifest.trim_ws :: raw :: call
        if (std.text.len_bytes :: line :: call) == 0:
            continue
        if std.text.starts_with :: line, "[" :: call:
            section = line
            continue
        let kv = std.manifest.parse_kv :: line :: call
        if not kv.0:
            continue
        let key = std.manifest.strip_quotes :: kv.1.0 :: call
        let value = std.manifest.strip_quotes :: kv.1.1 :: call
        if section == "":
            if key == "version":
                version = std.manifest.parse_int_or :: value, 0 :: call
            else:
                if key == "workspace":
                    workspace = value
        let mut full_key = key
        if section != "":
            full_key = section + "." + key
        data :: full_key, value :: set
    if version != 3:
        return Result.Err[LockManifestV3, Str] :: "Arcana.lock version must be 3" :: call
    let manifest = std.manifest.LockManifestV3 :: version = version, workspace = workspace, data = data :: call
    return Result.Ok[LockManifestV3, Str] :: manifest :: call
