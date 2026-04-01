import arcana_text.provider_impl.builder
import arcana_text.types

export fn open(read collection: arcana_text.types.FontCollection, read style: arcana_text.types.ParagraphStyle) -> arcana_text.types.ParagraphBuilder:
    return arcana_text.builder.open :: collection, style :: call

export fn push_style(edit builder: arcana_text.types.ParagraphBuilder, read style: arcana_text.types.TextStyle):
    arcana_text.provider_impl.builder.push_style :: builder, style :: call

export fn pop_style(edit builder: arcana_text.types.ParagraphBuilder):
    arcana_text.provider_impl.builder.pop_style :: builder :: call

export fn add_text(edit builder: arcana_text.types.ParagraphBuilder, text: Str):
    arcana_text.provider_impl.builder.add_text :: builder, text :: call

export fn add_placeholder(edit builder: arcana_text.types.ParagraphBuilder, read placeholder: arcana_text.types.PlaceholderStyle):
    arcana_text.provider_impl.builder.add_placeholder :: builder, placeholder :: call

export fn build(read builder: arcana_text.types.ParagraphBuilder) -> arcana_text.types.Paragraph:
    return arcana_text.builder.build :: builder :: call

export fn reset(edit builder: arcana_text.types.ParagraphBuilder):
    arcana_text.provider_impl.builder.reset :: builder :: call
