import arcana_text.provider_impl.engine
import arcana_text.types
import std.collections.list

fn open(read collection: arcana_text.provider_impl.engine.FontCollectionState, read style: arcana_text.types.ParagraphStyle) -> arcana_text.provider_impl.engine.ParagraphBuilderState:
    return arcana_text.provider_impl.engine.new_builder_state :: collection, style :: call

fn current_style(read builder: arcana_text.provider_impl.engine.ParagraphBuilderState) -> arcana_text.types.TextStyle:
    if builder.style_stack :: :: is_empty:
        return arcana_text.types.default_text_style :: 16777215 :: call
    return builder.style_stack[(builder.style_stack :: :: len) - 1]

fn push_style(edit builder: arcana_text.provider_impl.engine.ParagraphBuilderState, read style: arcana_text.types.TextStyle):
    builder.style_stack :: style :: push

fn pop_style(edit builder: arcana_text.provider_impl.engine.ParagraphBuilderState):
    if not (builder.style_stack :: :: is_empty):
        builder.style_stack :: :: pop
    if builder.style_stack :: :: is_empty:
        builder.style_stack :: (arcana_text.types.default_text_style :: 16777215 :: call) :: push

fn add_text(edit builder: arcana_text.provider_impl.engine.ParagraphBuilderState, text: Str):
    let style = current_style :: builder :: call
    builder.items :: (arcana_text.provider_impl.engine.BuilderItemState.Text :: (arcana_text.provider_impl.engine.TextRunState :: style = style, text = text :: call) :: call) :: push

fn add_placeholder(edit builder: arcana_text.provider_impl.engine.ParagraphBuilderState, read placeholder: arcana_text.types.PlaceholderStyle):
    builder.items :: (arcana_text.provider_impl.engine.BuilderItemState.Placeholder :: placeholder :: call) :: push

fn build(read builder: arcana_text.provider_impl.engine.ParagraphBuilderState) -> arcana_text.provider_impl.engine.ParagraphState:
    return arcana_text.provider_impl.engine.build_paragraph_state :: builder :: call

fn reset(edit builder: arcana_text.provider_impl.engine.ParagraphBuilderState):
    builder.items = std.collections.list.new[arcana_text.provider_impl.engine.BuilderItemState] :: :: call
    builder.style_stack = std.collections.list.new[arcana_text.types.TextStyle] :: :: call
    builder.style_stack :: (arcana_text.types.default_text_style :: 16777215 :: call) :: push
