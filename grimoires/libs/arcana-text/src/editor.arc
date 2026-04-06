import arcana_text.buffer
import arcana_text.types
import std.option
import std.text
use std.option.Option

export obj TextEditor:
    cursor: arcana_text.types.Cursor
    selection: arcana_text.types.Selection
    composition: Option[arcana_text.types.CompositionRange]

fn is_word_byte(b: Int) -> Bool:
    return (std.text.is_alpha_byte :: b :: call) or (std.text.is_digit_byte :: b :: call) or b == 95

export fn open(read buffer: arcana_text.buffer.TextBuffer) -> arcana_text.editor.TextEditor:
    let len = buffer :: :: len_bytes
    let cursor = arcana_text.types.Cursor :: offset = len, preferred_x = 0 :: call
    let selection = arcana_text.types.Selection :: anchor = len, focus = len :: call
    return arcana_text.editor.TextEditor :: cursor = cursor, selection = selection, composition = (Option.None[arcana_text.types.CompositionRange] :: :: call) :: call

impl TextEditor:
    fn caret(read self: arcana_text.editor.TextEditor) -> Int:
        return self.cursor.offset

    fn clear_selection(edit self: arcana_text.editor.TextEditor):
        self.selection.anchor = self.cursor.offset
        self.selection.focus = self.cursor.offset

    fn set_cursor(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer, index: Int):
        let total = buffer :: :: len_bytes
        let mut offset = index
        if offset < 0:
            offset = 0
        if offset > total:
            offset = total
        self.cursor.offset = offset
        self.cursor.preferred_x = 0
        self :: :: clear_selection

    fn select_range(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer, start: Int, end: Int):
        let total = buffer :: :: len_bytes
        let range = arcana_text.buffer.normalize_range :: start, end, total :: call
        self.selection.anchor = range.start
        self.selection.focus = range.end
        self.cursor.offset = range.end

    fn has_selection(read self: arcana_text.editor.TextEditor) -> Bool:
        return self.selection.anchor != self.selection.focus

    fn replace_selection(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer, read chunk: Str):
        if not (self :: :: has_selection):
            buffer :: self.cursor.offset, chunk :: insert
            self.cursor.offset += std.text.len_bytes :: chunk :: call
            self :: :: clear_selection
            return 0
        let range = arcana_text.buffer.normalize_range :: self.selection.anchor, self.selection.focus, (buffer :: :: len_bytes) :: call
        buffer :: range.start, range.end, chunk :: replace_range
        self.cursor.offset = range.start + (std.text.len_bytes :: chunk :: call)
        self :: :: clear_selection

    fn insert_text(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer, read chunk: Str):
        self :: buffer, chunk :: replace_selection

    fn delete_backward(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        if self :: :: has_selection:
            self :: buffer, "" :: replace_selection
            return 0
        if self.cursor.offset <= 0:
            return 0
        buffer :: self.cursor.offset - 1, self.cursor.offset :: delete_range
        self.cursor.offset -= 1
        self :: :: clear_selection

    fn delete_forward(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        if self :: :: has_selection:
            self :: buffer, "" :: replace_selection
            return 0
        if self.cursor.offset >= (buffer :: :: len_bytes):
            return 0
        buffer :: self.cursor.offset, self.cursor.offset + 1 :: delete_range
        self :: :: clear_selection

    fn move_left(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        let _ = buffer
        if self.cursor.offset > 0:
            self.cursor.offset -= 1
        self :: :: clear_selection

    fn move_right(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        if self.cursor.offset < (buffer :: :: len_bytes):
            self.cursor.offset += 1
        self :: :: clear_selection

    fn move_word_left(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        let mut cursor = self.cursor.offset
        while cursor > 0 and not (arcana_text.editor.is_word_byte :: (std.text.byte_at :: buffer.text, cursor - 1 :: call) :: call):
            cursor -= 1
        while cursor > 0 and (arcana_text.editor.is_word_byte :: (std.text.byte_at :: buffer.text, cursor - 1 :: call) :: call):
            cursor -= 1
        self.cursor.offset = cursor
        self :: :: clear_selection

    fn move_word_right(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        let total = buffer :: :: len_bytes
        let mut cursor = self.cursor.offset
        while cursor < total and not (arcana_text.editor.is_word_byte :: (std.text.byte_at :: buffer.text, cursor :: call) :: call):
            cursor += 1
        while cursor < total and (arcana_text.editor.is_word_byte :: (std.text.byte_at :: buffer.text, cursor :: call) :: call):
            cursor += 1
        self.cursor.offset = cursor
        self :: :: clear_selection

    fn apply_committed_text(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer, read text: Str):
        self :: buffer, text :: replace_selection
        self.composition = Option.None[arcana_text.types.CompositionRange] :: :: call

    fn apply_composition_text(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer, read text: Str):
        let start = self.cursor.offset
        self :: buffer, text :: replace_selection
        self.composition = Option.Some[arcana_text.types.CompositionRange] :: (arcana_text.types.CompositionRange :: range = (arcana_text.types.TextRange :: start = start, end = (start + (std.text.len_bytes :: text :: call)) :: call), caret = (start + (std.text.len_bytes :: text :: call)) :: call) :: call
