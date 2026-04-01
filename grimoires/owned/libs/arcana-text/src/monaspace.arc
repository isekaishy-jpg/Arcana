import arcana_text.types
import std.text

export enum MonaspaceFamily:
    Neon
    Argon
    Xenon
    Radon
    Krypton

export enum MonaspaceForm:
    Variable
    Static
    Frozen
    Nerd

fn family_base(read family: arcana_text.monaspace.MonaspaceFamily) -> Str:
    return match family:
        arcana_text.monaspace.MonaspaceFamily.Argon => "Monaspace Argon"
        arcana_text.monaspace.MonaspaceFamily.Xenon => "Monaspace Xenon"
        arcana_text.monaspace.MonaspaceFamily.Radon => "Monaspace Radon"
        arcana_text.monaspace.MonaspaceFamily.Krypton => "Monaspace Krypton"
        _ => "Monaspace Neon"

export fn family_name(read family: arcana_text.monaspace.MonaspaceFamily, read form: arcana_text.monaspace.MonaspaceForm) -> Str:
    let base = arcana_text.monaspace.family_base :: family :: call
    return match form:
        arcana_text.monaspace.MonaspaceForm.Static => base + " Static"
        arcana_text.monaspace.MonaspaceForm.Frozen => base + " Frozen"
        arcana_text.monaspace.MonaspaceForm.Nerd => base + " Nerd Font"
        _ => base

export fn feature(tag: Str, value: Int) -> arcana_text.types.FontFeature:
    return arcana_text.types.FontFeature :: tag = tag, value = value :: call

export fn axis(tag: Str, value_milli: Int) -> arcana_text.types.FontAxis:
    return arcana_text.types.FontAxis :: tag = tag, value_milli = value_milli :: call

export fn calt(enabled: Bool) -> arcana_text.types.FontFeature:
    if enabled:
        return arcana_text.monaspace.feature :: "calt", 1 :: call
    return arcana_text.monaspace.feature :: "calt", 0 :: call

export fn liga(enabled: Bool) -> arcana_text.types.FontFeature:
    if enabled:
        return arcana_text.monaspace.feature :: "liga", 1 :: call
    return arcana_text.monaspace.feature :: "liga", 0 :: call

export fn ss(index: Int, enabled: Bool) -> arcana_text.types.FontFeature:
    let mut tag = "ss0"
    if index >= 10:
        tag = "ss"
    tag = tag + (std.text.from_int :: index :: call)
    if enabled:
        return arcana_text.monaspace.feature :: tag, 1 :: call
    return arcana_text.monaspace.feature :: tag, 0 :: call

export fn cv(index: Int, enabled: Bool) -> arcana_text.types.FontFeature:
    let mut tag = "cv0"
    if index >= 10:
        tag = "cv"
    tag = tag + (std.text.from_int :: index :: call)
    if enabled:
        return arcana_text.monaspace.feature :: tag, 1 :: call
    return arcana_text.monaspace.feature :: tag, 0 :: call

export fn weight(value_milli: Int) -> arcana_text.types.FontAxis:
    return arcana_text.monaspace.axis :: "wght", value_milli :: call

export fn width(value_milli: Int) -> arcana_text.types.FontAxis:
    return arcana_text.monaspace.axis :: "wdth", value_milli :: call

export fn slant(value_milli: Int) -> arcana_text.types.FontAxis:
    return arcana_text.monaspace.axis :: "slnt", value_milli :: call
