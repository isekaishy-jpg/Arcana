import arcana_text.assets
import arcana_text.types
import std.collections.list
import std.path

export enum MonaspaceFamily:
    Argon
    Krypton
    Neon
    Radon
    Xenon

export enum MonaspaceForm:
    Variable

export enum MonaspaceFeature:
    TextureHealing
    RepeatSpacing

export enum MonaspaceStylisticSet:
    Ss01
    Ss02
    Ss03
    Ss04
    Ss05
    Ss06
    Ss07
    Ss08
    Ss09
    Ss10

export fn default_family() -> arcana_text.monaspace.MonaspaceFamily:
    return arcana_text.monaspace.MonaspaceFamily.Neon :: :: call

export fn family_name(read family: arcana_text.monaspace.MonaspaceFamily) -> Str:
    return match family:
        arcana_text.monaspace.MonaspaceFamily.Argon => "Monaspace Argon"
        arcana_text.monaspace.MonaspaceFamily.Krypton => "Monaspace Krypton"
        arcana_text.monaspace.MonaspaceFamily.Neon => "Monaspace Neon"
        arcana_text.monaspace.MonaspaceFamily.Radon => "Monaspace Radon"
        arcana_text.monaspace.MonaspaceFamily.Xenon => "Monaspace Xenon"

export fn file_name(read family: arcana_text.monaspace.MonaspaceFamily, read form: arcana_text.monaspace.MonaspaceForm) -> Str:
    let _ = form
    return (arcana_text.monaspace.family_name :: family :: call) + " Var.ttf"

export fn variable_font_path(read family: arcana_text.monaspace.MonaspaceFamily) -> Str:
    let family_dir = std.path.join :: (arcana_text.assets.monaspace_variable_root :: :: call), (arcana_text.monaspace.family_name :: family :: call) :: call
    return std.path.join :: family_dir, (arcana_text.monaspace.file_name :: family, (arcana_text.monaspace.MonaspaceForm.Variable :: :: call) :: call) :: call

export fn feature_tag(read feature: arcana_text.monaspace.MonaspaceFeature) -> Str:
    return match feature:
        arcana_text.monaspace.MonaspaceFeature.TextureHealing => "calt"
        _ => "liga"

export fn stylistic_set_tag(read value: arcana_text.monaspace.MonaspaceStylisticSet) -> Str:
    return match value:
        arcana_text.monaspace.MonaspaceStylisticSet.Ss01 => "ss01"
        arcana_text.monaspace.MonaspaceStylisticSet.Ss02 => "ss02"
        arcana_text.monaspace.MonaspaceStylisticSet.Ss03 => "ss03"
        arcana_text.monaspace.MonaspaceStylisticSet.Ss04 => "ss04"
        arcana_text.monaspace.MonaspaceStylisticSet.Ss05 => "ss05"
        arcana_text.monaspace.MonaspaceStylisticSet.Ss06 => "ss06"
        arcana_text.monaspace.MonaspaceStylisticSet.Ss07 => "ss07"
        arcana_text.monaspace.MonaspaceStylisticSet.Ss08 => "ss08"
        arcana_text.monaspace.MonaspaceStylisticSet.Ss09 => "ss09"
        _ => "ss10"

export fn feature_enabled(read tag: Str) -> arcana_text.types.FontFeature:
    let mut feature = arcana_text.types.FontFeature :: tag = tag, value = 1, enabled = true :: call
    return feature

export fn utility_feature(read feature: arcana_text.monaspace.MonaspaceFeature) -> arcana_text.types.FontFeature:
    return arcana_text.monaspace.feature_enabled :: (arcana_text.monaspace.feature_tag :: feature :: call) :: call

export fn stylistic_set_feature(read value: arcana_text.monaspace.MonaspaceStylisticSet) -> arcana_text.types.FontFeature:
    return arcana_text.monaspace.feature_enabled :: (arcana_text.monaspace.stylistic_set_tag :: value :: call) :: call

export fn character_variant(tag: Str, value: Int) -> arcana_text.types.FontFeature:
    let mut feature = arcana_text.types.FontFeature :: tag = tag, value = value, enabled = true :: call
    return feature

export fn weight_axis(value: Int) -> arcana_text.types.FontAxis:
    return arcana_text.types.FontAxis :: tag = "wght", value = value :: call

export fn width_axis(value: Int) -> arcana_text.types.FontAxis:
    return arcana_text.types.FontAxis :: tag = "wdth", value = value :: call

export fn slant_axis(value: Int) -> arcana_text.types.FontAxis:
    return arcana_text.types.FontAxis :: tag = "slnt", value = value :: call

export fn recommended_code_features() -> List[arcana_text.types.FontFeature]:
    let mut out = std.collections.list.empty[arcana_text.types.FontFeature] :: :: call
    out :: (arcana_text.monaspace.utility_feature :: (arcana_text.monaspace.MonaspaceFeature.TextureHealing :: :: call) :: call) :: push
    out :: (arcana_text.monaspace.stylistic_set_feature :: (arcana_text.monaspace.MonaspaceStylisticSet.Ss01 :: :: call) :: call) :: push
    out :: (arcana_text.monaspace.stylistic_set_feature :: (arcana_text.monaspace.MonaspaceStylisticSet.Ss02 :: :: call) :: call) :: push
    out :: (arcana_text.monaspace.stylistic_set_feature :: (arcana_text.monaspace.MonaspaceStylisticSet.Ss03 :: :: call) :: call) :: push
    out :: (arcana_text.monaspace.stylistic_set_feature :: (arcana_text.monaspace.MonaspaceStylisticSet.Ss04 :: :: call) :: call) :: push
    out :: (arcana_text.monaspace.stylistic_set_feature :: (arcana_text.monaspace.MonaspaceStylisticSet.Ss05 :: :: call) :: call) :: push
    out :: (arcana_text.monaspace.stylistic_set_feature :: (arcana_text.monaspace.MonaspaceStylisticSet.Ss06 :: :: call) :: call) :: push
    out :: (arcana_text.monaspace.stylistic_set_feature :: (arcana_text.monaspace.MonaspaceStylisticSet.Ss07 :: :: call) :: call) :: push
    out :: (arcana_text.monaspace.stylistic_set_feature :: (arcana_text.monaspace.MonaspaceStylisticSet.Ss08 :: :: call) :: call) :: push
    out :: (arcana_text.monaspace.stylistic_set_feature :: (arcana_text.monaspace.MonaspaceStylisticSet.Ss09 :: :: call) :: call) :: push
    out :: (arcana_text.monaspace.stylistic_set_feature :: (arcana_text.monaspace.MonaspaceStylisticSet.Ss10 :: :: call) :: call) :: push
    out :: (arcana_text.monaspace.utility_feature :: (arcana_text.monaspace.MonaspaceFeature.RepeatSpacing :: :: call) :: call) :: push
    return out

export fn apply_recommended_code_features(edit style: arcana_text.types.TextStyle):
    style.features = arcana_text.monaspace.recommended_code_features :: :: call

export fn apply_recommended_code_features_span(edit style: arcana_text.types.SpanStyle):
    style.features = arcana_text.monaspace.recommended_code_features :: :: call
