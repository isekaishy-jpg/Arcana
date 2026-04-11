import arcana_text.buffer
import arcana_text.layout
import arcana_text.queries
import arcana_text.text_units
import arcana_text.types
import std.collections.list
import std.option
import std.text
use std.option.Option

export record EditorChangeItem:
    start: Int
    end: Int
    text: Str
    insert: Bool

export record EditorChange:
    items: List[arcana_text.editor.EditorChangeItem]

export enum EditAction:
    MoveBufferStart
    MoveBufferEnd
    MoveLeft
    MoveRight
    MoveWordLeft
    MoveWordRight
    MoveLineStart
    MoveLineEnd
    MoveUp
    MoveDown
    ExtendBufferStart
    ExtendBufferEnd
    ExtendLeft
    ExtendRight
    ExtendUp
    ExtendDown
    Escape
    SelectAll
    Insert(Str)
    Enter
    Backspace
    Delete
    DeleteWordLeft
    DeleteWordRight
    Indent
    Unindent
    Undo
    Redo
    Click((Int, Int))
    DoubleClick((Int, Int))
    TripleClick((Int, Int))
    Drag((Int, Int))
    CommitComposition(Str)
    UpdateComposition(Str)
    Scroll(Int)

export enum EditorLayer:
    Editor
    Syntax
    Vi

export enum ViMode:
    Normal
    Insert
    Visual
    VisualLine

export enum ViAction:
    Escape
    InsertMode
    AppendMode
    VisualMode
    VisualLineMode
    MoveLeft
    MoveRight
    MoveUp
    MoveDown
    MoveWordLeft
    MoveWordRight
    MoveLineStart
    MoveLineEnd
    DeleteSelection
    DeleteLine
    DeleteChar
    DeleteWord
    YankSelection
    PasteBefore
    PasteAfter
    ReplaceSelection(Str)
    InsertText(Str)
    Enter
    Undo
    Redo

export record ViRegister:
    name: Str
    text: Str

export record SyntaxTheme:
    name: Str

export obj TextEditor:
    cursor: arcana_text.types.Cursor
    selection: arcana_text.types.Selection
    selection_mode: arcana_text.types.SelectionMode
    auto_indent: Bool
    composition: Option[arcana_text.types.CompositionRange]
    collecting_change: Bool
    active_change: arcana_text.editor.EditorChange
    undo_changes: List[arcana_text.editor.EditorChange]
    redo_changes: List[arcana_text.editor.EditorChange]

export obj SyntaxEditor:
    editor: arcana_text.editor.TextEditor
    language: Str
    theme: arcana_text.editor.SyntaxTheme

export obj ViEditor:
    editor: arcana_text.editor.TextEditor
    mode: arcana_text.editor.ViMode
    passthrough: Bool
    changed: Bool
    registers: List[arcana_text.editor.ViRegister]

fn previous_char_start(read text: Str, offset: Int) -> Int:
    return arcana_text.text_units.previous_cluster_start :: text, offset :: call

fn next_char_end(read text: Str, offset: Int) -> Int:
    return arcana_text.text_units.next_cluster_end :: text, offset :: call

fn indent_string(width: Int) -> Str:
    let mut out = ""
    let mut index = 0
    while index < width:
        out = out + " "
        index += 1
    return out

fn adjust_inserted_offset(offset: Int, insert_at: Int, inserted_len: Int) -> Int:
    if offset < insert_at:
        return offset
    return offset + inserted_len

fn adjust_deleted_offset(offset: Int, start: Int, end: Int) -> Int:
    if offset <= start:
        return offset
    if offset >= end:
        return offset - (end - start)
    return start

fn selected_line_starts(read buffer: arcana_text.buffer.TextBuffer, read range: arcana_text.types.TextRange) -> List[Int]:
    let mut starts = std.collections.list.empty[Int] :: :: call
    let mut final_end = range.end
    if final_end > range.start and final_end == (buffer :: final_end :: line_start):
        final_end -= 1
    let mut line_start = buffer :: range.start :: line_start
    let final_start = buffer :: final_end :: line_start
    while line_start <= final_start:
        starts :: line_start :: push
        if line_start >= final_start:
            break
        let line_end = buffer :: line_start :: line_end
        let total = buffer :: :: len_bytes
        if line_end >= total:
            break
        line_start = line_end + 1
    return starts

fn line_midpoint(read snapshot: arcana_text.layout.LayoutSnapshot, line_index: Int) -> (Int, Int):
    let metrics = arcana_text.queries.line_metrics_at :: snapshot, line_index :: call
    return (metrics.position.0, metrics.position.1 + (metrics.size.1 / 2))

fn word_range(read buffer: arcana_text.buffer.TextBuffer, index: Int) -> arcana_text.types.TextRange:
    let bounds = arcana_text.text_units.word_boundary :: buffer.text, index :: call
    return arcana_text.types.TextRange :: start = bounds.0, end = bounds.1 :: call

fn previous_word_start(read text: Str, offset: Int) -> Int:
    if offset <= 0:
        return 0
    let mut cursor = arcana_text.text_units.clamp_offset :: text, offset :: call
    while cursor > 0:
        let prior = arcana_text.text_units.previous_cluster_start :: text, cursor :: call
        let codepoint = arcana_text.text_units.codepoint_at :: text, prior :: call
        if arcana_text.text_units.is_word_codepoint :: codepoint :: call:
            let bounds = arcana_text.text_units.word_boundary :: text, prior :: call
            return bounds.0
        if prior >= cursor:
            break
        cursor = prior
    return 0

fn next_word_end(read text: Str, offset: Int) -> Int:
    let total = std.text.len_bytes :: text :: call
    if offset >= total:
        return total
    let mut cursor = arcana_text.text_units.clamp_offset :: text, offset :: call
    while cursor < total:
        let codepoint = arcana_text.text_units.codepoint_at :: text, cursor :: call
        if arcana_text.text_units.is_word_codepoint :: codepoint :: call:
            let bounds = arcana_text.text_units.word_boundary :: text, cursor :: call
            return bounds.1
        let next = arcana_text.text_units.next_cluster_end :: text, cursor :: call
        if next <= cursor:
            break
        cursor = next
    return total

fn empty_change_items() -> List[arcana_text.editor.EditorChangeItem]:
    return std.collections.list.empty[arcana_text.editor.EditorChangeItem] :: :: call

fn empty_changes() -> List[arcana_text.editor.EditorChange]:
    return std.collections.list.empty[arcana_text.editor.EditorChange] :: :: call

fn empty_registers() -> List[arcana_text.editor.ViRegister]:
    return std.collections.list.empty[arcana_text.editor.ViRegister] :: :: call

fn replace_register(edit registers: List[arcana_text.editor.ViRegister], read next: arcana_text.editor.ViRegister):
    let mut updated = false
    let total = registers :: :: len
    let mut index = 0
    while index < total:
        if (registers)[index].name == next.name:
            registers[index] = next
            updated = true
            break
        index += 1
    if not updated:
        registers :: next :: push

fn register_value(read registers: List[arcana_text.editor.ViRegister], read name: Str) -> Str:
    for register in registers:
        if register.name == name:
            return register.text
    return ""

fn empty_change() -> arcana_text.editor.EditorChange:
    return arcana_text.editor.EditorChange :: items = (arcana_text.editor.empty_change_items :: :: call) :: call

fn copy_change_item(read item: arcana_text.editor.EditorChangeItem) -> arcana_text.editor.EditorChangeItem:
    return item

fn copy_change(read change: arcana_text.editor.EditorChange) -> arcana_text.editor.EditorChange:
    let mut out = arcana_text.editor.empty_change :: :: call
    for item in change.items:
        out.items :: (arcana_text.editor.copy_change_item :: item :: call) :: push
    return out

fn reversed_change(read change: arcana_text.editor.EditorChange) -> arcana_text.editor.EditorChange:
    let mut out = arcana_text.editor.empty_change :: :: call
    let mut copy = arcana_text.editor.empty_change_items :: :: call
    copy :: change.items :: extend_list
    while not (copy :: :: is_empty):
        let mut item = copy :: :: pop
        item.insert = not item.insert
        out.items :: item :: push
    return out

fn tracks_action(read action: arcana_text.editor.EditAction) -> Bool:
    return match action:
        arcana_text.editor.EditAction.MoveBufferStart => false
        arcana_text.editor.EditAction.MoveBufferEnd => false
        arcana_text.editor.EditAction.Insert(_) => true
        arcana_text.editor.EditAction.Enter => true
        arcana_text.editor.EditAction.Backspace => true
        arcana_text.editor.EditAction.Delete => true
        arcana_text.editor.EditAction.DeleteWordLeft => true
        arcana_text.editor.EditAction.DeleteWordRight => true
        arcana_text.editor.EditAction.Indent => true
        arcana_text.editor.EditAction.Unindent => true
        arcana_text.editor.EditAction.CommitComposition(_) => true
        arcana_text.editor.EditAction.UpdateComposition(_) => true
        _ => false

export fn open(read buffer: arcana_text.buffer.TextBuffer) -> arcana_text.editor.TextEditor:
    let len = buffer :: :: len_bytes
    let cursor = arcana_text.types.Cursor :: offset = len, preferred_x = 0 :: call
    let selection = arcana_text.types.Selection :: anchor = len, focus = len :: call
    let mut editor = arcana_text.editor.TextEditor :: cursor = cursor, selection = selection :: call
    editor.selection_mode = arcana_text.types.SelectionMode.Normal :: :: call
    editor.auto_indent = false
    editor.composition = Option.None[arcana_text.types.CompositionRange] :: :: call
    editor.collecting_change = false
    editor.active_change = arcana_text.editor.empty_change :: :: call
    editor.undo_changes = arcana_text.editor.empty_changes :: :: call
    editor.redo_changes = arcana_text.editor.empty_changes :: :: call
    return editor

export fn open_syntax(read buffer: arcana_text.buffer.TextBuffer) -> arcana_text.editor.SyntaxEditor:
    return arcana_text.editor.SyntaxEditor :: editor = (arcana_text.editor.open :: buffer :: call), language = "", theme = (arcana_text.editor.SyntaxTheme :: name = "" :: call) :: call

export fn open_vi(read buffer: arcana_text.buffer.TextBuffer) -> arcana_text.editor.ViEditor:
    let mut out = arcana_text.editor.ViEditor :: editor = (arcana_text.editor.open :: buffer :: call), mode = (arcana_text.editor.ViMode.Normal :: :: call) :: call
    out.passthrough = false
    out.changed = false
    out.registers = arcana_text.editor.empty_registers :: :: call
    return out

impl TextEditor:
    fn layer(read self: arcana_text.editor.TextEditor) -> arcana_text.editor.EditorLayer:
        return arcana_text.editor.EditorLayer.Editor :: :: call

    fn caret(read self: arcana_text.editor.TextEditor) -> Int:
        return self.cursor.offset

    fn selection_mode(read self: arcana_text.editor.TextEditor) -> arcana_text.types.SelectionMode:
        return self.selection_mode

    fn set_selection_mode(edit self: arcana_text.editor.TextEditor, mode: arcana_text.types.SelectionMode):
        self.selection_mode = mode

    fn auto_indent(read self: arcana_text.editor.TextEditor) -> Bool:
        return self.auto_indent

    fn set_auto_indent(edit self: arcana_text.editor.TextEditor, enabled: Bool):
        self.auto_indent = enabled

    fn tab_width(read self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer) -> Int:
        return buffer :: :: tab_width

    fn set_tab_width(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer, tab_width: Int):
        buffer :: tab_width :: set_tab_width

    fn can_undo(read self: arcana_text.editor.TextEditor) -> Bool:
        return not (self.undo_changes :: :: is_empty)

    fn can_redo(read self: arcana_text.editor.TextEditor) -> Bool:
        return not (self.redo_changes :: :: is_empty)

    fn start_change(edit self: arcana_text.editor.TextEditor):
        if self.collecting_change:
            return
        self.collecting_change = true
        self.active_change = arcana_text.editor.empty_change :: :: call

    fn finish_change(edit self: arcana_text.editor.TextEditor) -> Option[arcana_text.editor.EditorChange]:
        if not self.collecting_change:
            return Option.None[arcana_text.editor.EditorChange] :: :: call
        self.collecting_change = false
        let change = arcana_text.editor.copy_change :: self.active_change :: call
        self.active_change = arcana_text.editor.empty_change :: :: call
        if change.items :: :: is_empty:
            return Option.None[arcana_text.editor.EditorChange] :: :: call
        return Option.Some[arcana_text.editor.EditorChange] :: change :: call

    fn record_change_item(edit self: arcana_text.editor.TextEditor, read item: arcana_text.editor.EditorChangeItem):
        if not self.collecting_change:
            return
        self.active_change.items :: item :: push

    fn record_insert_change(edit self: arcana_text.editor.TextEditor, read range: arcana_text.types.TextRange, read text: Str):
        if text == "":
            return
        let item = arcana_text.editor.EditorChangeItem :: start = range.start, end = range.end, text = text, insert = true :: call
        self :: item :: record_change_item

    fn record_delete_change(edit self: arcana_text.editor.TextEditor, read range: arcana_text.types.TextRange, read text: Str):
        if range.start >= range.end:
            return
        let item = arcana_text.editor.EditorChangeItem :: start = range.start, end = range.end, text = text, insert = false :: call
        self :: item :: record_change_item

    fn commit_change(edit self: arcana_text.editor.TextEditor):
        let change = self :: :: finish_change
        return match change:
            Option.Some(value) => self :: value :: commit_change_ready
            Option.None => 0

    fn commit_change_ready(edit self: arcana_text.editor.TextEditor, read change: arcana_text.editor.EditorChange):
        self.undo_changes :: (arcana_text.editor.copy_change :: change :: call) :: push
        self.redo_changes = arcana_text.editor.empty_changes :: :: call

    fn apply_change(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer, read change: arcana_text.editor.EditorChange) -> Bool:
        if change.items :: :: is_empty:
            return false
        for item in change.items:
            if item.insert:
                buffer :: item.start, item.text :: insert
                self.cursor.offset = item.end
            else:
                buffer :: item.start, item.end :: delete_range
                self.cursor.offset = item.start
        self.cursor.preferred_x = 0
        self.selection.anchor = self.cursor.offset
        self.selection.focus = self.cursor.offset
        self.selection_mode = arcana_text.types.SelectionMode.Normal :: :: call
        self.composition = Option.None[arcana_text.types.CompositionRange] :: :: call
        return true

    fn undo(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer) -> Bool:
        if self.undo_changes :: :: is_empty:
            return false
        let change = self.undo_changes :: :: pop
        let reversed = arcana_text.editor.reversed_change :: change :: call
        let applied = self :: buffer, reversed :: apply_change
        if applied:
            self.redo_changes :: change :: push
        return applied

    fn redo(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer) -> Bool:
        if self.redo_changes :: :: is_empty:
            return false
        let change = self.redo_changes :: :: pop
        let applied = self :: buffer, change :: apply_change
        if applied:
            self.undo_changes :: change :: push
        return applied

    fn clear_selection(edit self: arcana_text.editor.TextEditor):
        self.selection.anchor = self.cursor.offset
        self.selection.focus = self.cursor.offset
        self.selection_mode = arcana_text.types.SelectionMode.Normal :: :: call

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

    fn select_range(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer, read requested: arcana_text.types.TextRange):
        let total = buffer :: :: len_bytes
        let range = arcana_text.buffer.normalize_range :: requested.start, requested.end, total :: call
        self.selection.anchor = range.start
        self.selection.focus = range.end
        self.cursor.offset = range.end
        self.selection_mode = arcana_text.types.SelectionMode.Normal :: :: call

    fn has_selection(read self: arcana_text.editor.TextEditor) -> Bool:
        return self.selection.anchor != self.selection.focus

    fn begin_extend(edit self: arcana_text.editor.TextEditor):
        if not (self :: :: has_selection):
            self.selection.anchor = self.cursor.offset
            self.selection.focus = self.cursor.offset
            self.selection_mode = arcana_text.types.SelectionMode.Normal :: :: call

    fn set_selection_focus(edit self: arcana_text.editor.TextEditor, index: Int):
        self.selection.focus = index
        self.cursor.offset = index

    fn selection_range(read self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer) -> Option[arcana_text.types.TextRange]:
        if not (self :: :: has_selection):
            return Option.None[arcana_text.types.TextRange] :: :: call
        let range = arcana_text.buffer.normalize_range :: self.selection.anchor, self.selection.focus, (buffer :: :: len_bytes) :: call
        return Option.Some[arcana_text.types.TextRange] :: range :: call

    fn selection_text(read self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer) -> Option[Str]:
        let range_opt = self :: buffer :: selection_range
        return match range_opt:
            Option.Some(range) => Option.Some[Str] :: (std.text.slice_bytes :: buffer.text, range.start, range.end :: call) :: call
            Option.None => Option.None[Str] :: :: call

    fn copy_selection(read self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer) -> Option[Str]:
        return self :: buffer :: selection_text

    fn selection_boxes(read self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer, read snapshot: arcana_text.layout.LayoutSnapshot) -> List[arcana_text.types.RangeBox]:
        let range_opt = self :: buffer :: selection_range
        return match range_opt:
            Option.Some(range) => arcana_text.queries.range_boxes :: snapshot, range :: call
            Option.None => std.collections.list.empty[arcana_text.types.RangeBox] :: :: call

    fn delete_selection(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer) -> Bool:
        if not (self :: :: has_selection):
            return false
        self :: buffer, "" :: replace_selection
        return true

    fn delete_recorded_range(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer, read range: arcana_text.types.TextRange):
        if range.end <= range.start:
            return
        let removed = buffer :: range.start, range.end :: copy_range
        buffer :: range.start, range.end :: delete_range
        self :: range, removed :: record_delete_change

    fn replace_selection(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer, read chunk: Str):
        if not (self :: :: has_selection):
            let start = self.cursor.offset
            buffer :: self.cursor.offset, chunk :: insert
            self.cursor.offset += std.text.len_bytes :: chunk :: call
            let inserted = arcana_text.types.TextRange :: start = start, end = self.cursor.offset :: call
            self :: inserted, chunk :: record_insert_change
            self :: :: clear_selection
            return 0
        let range = arcana_text.buffer.normalize_range :: self.selection.anchor, self.selection.focus, (buffer :: :: len_bytes) :: call
        let removed = buffer :: range.start, range.end :: copy_range
        buffer :: range, chunk :: replace_range
        self :: range, removed :: record_delete_change
        let inserted = arcana_text.types.TextRange :: start = range.start, end = range.start + (std.text.len_bytes :: chunk :: call) :: call
        self :: inserted, chunk :: record_insert_change
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
        let start = arcana_text.editor.previous_char_start :: buffer.text, self.cursor.offset :: call
        let range = arcana_text.types.TextRange :: start = start, end = self.cursor.offset :: call
        self :: buffer, range :: delete_recorded_range
        self.cursor.offset = start
        self :: :: clear_selection

    fn delete_forward(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        if self :: :: has_selection:
            self :: buffer, "" :: replace_selection
            return 0
        if self.cursor.offset >= (buffer :: :: len_bytes):
            return 0
        let next = arcana_text.editor.next_char_end :: buffer.text, self.cursor.offset :: call
        let range = arcana_text.types.TextRange :: start = self.cursor.offset, end = next :: call
        self :: buffer, range :: delete_recorded_range
        self :: :: clear_selection

    fn delete_word_left(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        if self :: :: has_selection:
            self :: buffer, "" :: replace_selection
            return 0
        if self.cursor.offset <= 0:
            return 0
        let start = arcana_text.editor.previous_word_start :: buffer.text, self.cursor.offset :: call
        let range = arcana_text.types.TextRange :: start = start, end = self.cursor.offset :: call
        self :: buffer, range :: delete_recorded_range
        self.cursor.offset = start
        self :: :: clear_selection

    fn delete_word_right(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        if self :: :: has_selection:
            self :: buffer, "" :: replace_selection
            return 0
        let total = buffer :: :: len_bytes
        if self.cursor.offset >= total:
            return 0
        let end = arcana_text.editor.next_word_end :: buffer.text, self.cursor.offset :: call
        let range = arcana_text.types.TextRange :: start = self.cursor.offset, end = end :: call
        self :: buffer, range :: delete_recorded_range
        self :: :: clear_selection

    fn move_buffer_start(edit self: arcana_text.editor.TextEditor):
        self.cursor.offset = 0
        self.cursor.preferred_x = 0
        self :: :: clear_selection

    fn move_buffer_end(edit self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer):
        self.cursor.offset = buffer :: :: len_bytes
        self.cursor.preferred_x = 0
        self :: :: clear_selection

    fn move_left(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        if self.cursor.offset > 0:
            self.cursor.offset = arcana_text.editor.previous_char_start :: buffer.text, self.cursor.offset :: call
            self.cursor.preferred_x = 0
        self :: :: clear_selection

    fn move_right(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        if self.cursor.offset < (buffer :: :: len_bytes):
            self.cursor.offset = arcana_text.editor.next_char_end :: buffer.text, self.cursor.offset :: call
            self.cursor.preferred_x = 0
        self :: :: clear_selection

    fn move_word_left(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        self.cursor.offset = arcana_text.editor.previous_word_start :: buffer.text, self.cursor.offset :: call
        self.cursor.preferred_x = 0
        self :: :: clear_selection

    fn move_word_right(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        self.cursor.offset = arcana_text.editor.next_word_end :: buffer.text, self.cursor.offset :: call
        self.cursor.preferred_x = 0
        self :: :: clear_selection

    fn move_line_start(edit self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer):
        self.cursor.offset = buffer :: self.cursor.offset :: line_start
        self.cursor.preferred_x = 0
        self :: :: clear_selection

    fn move_line_end(edit self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer):
        self.cursor.offset = buffer :: self.cursor.offset :: line_end
        self.cursor.preferred_x = 0
        self :: :: clear_selection

    fn move_up(edit self: arcana_text.editor.TextEditor, read snapshot: arcana_text.layout.LayoutSnapshot):
        let current_line = arcana_text.queries.line_index_for_offset :: snapshot, self.cursor.offset :: call
        if current_line <= 0:
            return
        let caret = arcana_text.queries.caret_box :: snapshot, self.cursor.offset :: call
        let preferred_x = match self.cursor.preferred_x > 0:
            true => self.cursor.preferred_x
            false => caret.position.0
        let target = arcana_text.editor.line_midpoint :: snapshot, current_line - 1 :: call
        let hit = arcana_text.queries.hit_test :: snapshot, (preferred_x, target.1) :: call
        self.cursor.offset = hit.index
        self.cursor.preferred_x = preferred_x
        self :: :: clear_selection

    fn move_down(edit self: arcana_text.editor.TextEditor, read snapshot: arcana_text.layout.LayoutSnapshot):
        let current_line = arcana_text.queries.line_index_for_offset :: snapshot, self.cursor.offset :: call
        let line_count = arcana_text.queries.line_count :: snapshot :: call
        if current_line + 1 >= line_count:
            return
        let caret = arcana_text.queries.caret_box :: snapshot, self.cursor.offset :: call
        let preferred_x = match self.cursor.preferred_x > 0:
            true => self.cursor.preferred_x
            false => caret.position.0
        let target = arcana_text.editor.line_midpoint :: snapshot, current_line + 1 :: call
        let hit = arcana_text.queries.hit_test :: snapshot, (preferred_x, target.1) :: call
        self.cursor.offset = hit.index
        self.cursor.preferred_x = preferred_x
        self :: :: clear_selection

    fn extend_left(edit self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer):
        self :: :: begin_extend
        if self.cursor.offset > 0:
            self.cursor.offset = arcana_text.editor.previous_char_start :: buffer.text, self.cursor.offset :: call
        self.selection.focus = self.cursor.offset

    fn extend_buffer_start(edit self: arcana_text.editor.TextEditor):
        self :: :: begin_extend
        self.cursor.offset = 0
        self.selection.focus = self.cursor.offset

    fn extend_right(edit self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer):
        self :: :: begin_extend
        if self.cursor.offset < (buffer :: :: len_bytes):
            self.cursor.offset = arcana_text.editor.next_char_end :: buffer.text, self.cursor.offset :: call
        self.selection.focus = self.cursor.offset

    fn extend_word_left(edit self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer):
        self :: :: begin_extend
        self.cursor.offset = arcana_text.editor.previous_word_start :: buffer.text, self.cursor.offset :: call
        self.cursor.preferred_x = 0
        self.selection.focus = self.cursor.offset

    fn extend_word_right(edit self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer):
        self :: :: begin_extend
        self.cursor.offset = arcana_text.editor.next_word_end :: buffer.text, self.cursor.offset :: call
        self.cursor.preferred_x = 0
        self.selection.focus = self.cursor.offset

    fn extend_line_start(edit self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer):
        self :: :: begin_extend
        self.cursor.offset = buffer :: self.cursor.offset :: line_start
        self.cursor.preferred_x = 0
        self.selection.focus = self.cursor.offset

    fn extend_line_end(edit self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer):
        self :: :: begin_extend
        self.cursor.offset = buffer :: self.cursor.offset :: line_end
        self.cursor.preferred_x = 0
        self.selection.focus = self.cursor.offset

    fn extend_buffer_end(edit self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer):
        self :: :: begin_extend
        self.cursor.offset = buffer :: :: len_bytes
        self.selection.focus = self.cursor.offset

    fn extend_up(edit self: arcana_text.editor.TextEditor, read snapshot: arcana_text.layout.LayoutSnapshot):
        self :: :: begin_extend
        let current_line = arcana_text.queries.line_index_for_offset :: snapshot, self.cursor.offset :: call
        if current_line <= 0:
            return
        let caret = arcana_text.queries.caret_box :: snapshot, self.cursor.offset :: call
        let preferred_x = match self.cursor.preferred_x > 0:
            true => self.cursor.preferred_x
            false => caret.position.0
        let target = arcana_text.editor.line_midpoint :: snapshot, current_line - 1 :: call
        let hit = arcana_text.queries.hit_test :: snapshot, (preferred_x, target.1) :: call
        self.cursor.offset = hit.index
        self.cursor.preferred_x = preferred_x
        self.selection.focus = self.cursor.offset

    fn extend_down(edit self: arcana_text.editor.TextEditor, read snapshot: arcana_text.layout.LayoutSnapshot):
        self :: :: begin_extend
        let current_line = arcana_text.queries.line_index_for_offset :: snapshot, self.cursor.offset :: call
        let line_count = arcana_text.queries.line_count :: snapshot :: call
        if current_line + 1 >= line_count:
            return
        let caret = arcana_text.queries.caret_box :: snapshot, self.cursor.offset :: call
        let preferred_x = match self.cursor.preferred_x > 0:
            true => self.cursor.preferred_x
            false => caret.position.0
        let target = arcana_text.editor.line_midpoint :: snapshot, current_line + 1 :: call
        let hit = arcana_text.queries.hit_test :: snapshot, (preferred_x, target.1) :: call
        self.cursor.offset = hit.index
        self.cursor.preferred_x = preferred_x
        self.selection.focus = self.cursor.offset

    fn select_all(edit self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer):
        self.selection.anchor = 0
        self.selection.focus = buffer :: :: len_bytes
        self.cursor.offset = self.selection.focus
        self.selection_mode = arcana_text.types.SelectionMode.Normal :: :: call

    fn select_word(edit self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer, index: Int):
        let range = arcana_text.editor.word_range :: buffer, index :: call
        self.selection.anchor = range.start
        self.selection.focus = range.end
        self.cursor.offset = range.end
        self.selection_mode = arcana_text.types.SelectionMode.Word :: :: call

    fn select_line(edit self: arcana_text.editor.TextEditor, read buffer: arcana_text.buffer.TextBuffer, index: Int):
        let range = buffer :: index :: line_range
        self.selection.anchor = range.start
        self.selection.focus = range.end
        self.cursor.offset = range.end
        self.selection_mode = arcana_text.types.SelectionMode.Line :: :: call

    fn click(edit self: arcana_text.editor.TextEditor, read snapshot: arcana_text.layout.LayoutSnapshot, point: (Int, Int)):
        let hit = arcana_text.queries.hit_test :: snapshot, point :: call
        self.cursor.offset = hit.index
        self.cursor.preferred_x = 0
        self :: :: clear_selection

    fn double_click(edit self: arcana_text.editor.TextEditor, read payload: (arcana_text.buffer.TextBuffer, arcana_text.layout.LayoutSnapshot, (Int, Int))):
        let buffer = payload.0
        let snapshot = payload.1
        let point = payload.2
        let hit = arcana_text.queries.hit_test :: snapshot, point :: call
        self.cursor.offset = hit.index
        self.cursor.preferred_x = 0
        self :: buffer, hit.index :: select_word

    fn triple_click(edit self: arcana_text.editor.TextEditor, read payload: (arcana_text.buffer.TextBuffer, arcana_text.layout.LayoutSnapshot, (Int, Int))):
        let buffer = payload.0
        let snapshot = payload.1
        let point = payload.2
        let hit = arcana_text.queries.hit_test :: snapshot, point :: call
        self.cursor.offset = hit.index
        self.cursor.preferred_x = 0
        self :: buffer, hit.index :: select_line

    fn drag(edit self: arcana_text.editor.TextEditor, read payload: (arcana_text.buffer.TextBuffer, arcana_text.layout.LayoutSnapshot, (Int, Int))):
        let buffer = payload.0
        let snapshot = payload.1
        let point = payload.2
        let hit = arcana_text.queries.hit_test :: snapshot, point :: call
        let anchor = self.selection.anchor
        if not (self :: :: has_selection):
            self.selection.anchor = self.cursor.offset
        let mut focus = hit.index
        if self.selection_mode == (arcana_text.types.SelectionMode.Word :: :: call):
            let range = arcana_text.editor.word_range :: buffer, hit.index :: call
            focus = match hit.index < anchor:
                true => range.start
                false => range.end
        if self.selection_mode == (arcana_text.types.SelectionMode.Line :: :: call):
            let range = buffer :: hit.index :: line_range
            focus = match hit.index < anchor:
                true => range.start
                false => range.end
        self.selection.focus = focus
        self.cursor.offset = focus
        self.cursor.preferred_x = 0

    fn insert_newline(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        let indent = match self.auto_indent:
            true => buffer :: self.cursor.offset :: line_indentation
            false => ""
        self :: buffer, ("\n" + indent) :: insert_text

    fn indent(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        let selection = self :: buffer :: selection_range
        let base = match selection:
            Option.Some(range) => range
            Option.None => arcana_text.types.TextRange :: start = self.cursor.offset, end = self.cursor.offset :: call
        let mut starts = arcana_text.editor.selected_line_starts :: buffer, base :: call
        let prefix = arcana_text.editor.indent_string :: (buffer :: :: tab_width) :: call
        let prefix_len = std.text.len_bytes :: prefix :: call
        while not (starts :: :: is_empty):
            let insert_at = starts :: :: pop
            buffer :: insert_at, prefix :: insert
            let inserted = arcana_text.types.TextRange :: start = insert_at, end = insert_at + prefix_len :: call
            self :: inserted, prefix :: record_insert_change
            self.cursor.offset = arcana_text.editor.adjust_inserted_offset :: self.cursor.offset, insert_at, prefix_len :: call
            self.selection.anchor = arcana_text.editor.adjust_inserted_offset :: self.selection.anchor, insert_at, prefix_len :: call
            self.selection.focus = arcana_text.editor.adjust_inserted_offset :: self.selection.focus, insert_at, prefix_len :: call

    fn unindent(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer):
        let selection = self :: buffer :: selection_range
        let base = match selection:
            Option.Some(range) => range
            Option.None => arcana_text.types.TextRange :: start = self.cursor.offset, end = self.cursor.offset :: call
        let mut starts = arcana_text.editor.selected_line_starts :: buffer, base :: call
        let tab_width = buffer :: :: tab_width
        while not (starts :: :: is_empty):
            let line_start = starts :: :: pop
            let line_end = buffer :: line_start :: line_end
            let mut remove_end = line_start
            if remove_end < line_end:
                let first = std.text.byte_at :: buffer.text, remove_end :: call
                if first == 9:
                    remove_end += 1
                else:
                    let mut count = 0
                    while remove_end < line_end and count < tab_width:
                        let value = std.text.byte_at :: buffer.text, remove_end :: call
                        if value != 32:
                            break
                        remove_end += 1
                        count += 1
            if remove_end <= line_start:
                continue
            let range = arcana_text.types.TextRange :: start = line_start, end = remove_end :: call
            self :: buffer, range :: delete_recorded_range
            self.cursor.offset = arcana_text.editor.adjust_deleted_offset :: self.cursor.offset, line_start, remove_end :: call
            self.selection.anchor = arcana_text.editor.adjust_deleted_offset :: self.selection.anchor, line_start, remove_end :: call
            self.selection.focus = arcana_text.editor.adjust_deleted_offset :: self.selection.focus, line_start, remove_end :: call

    fn escape(edit self: arcana_text.editor.TextEditor):
        self.composition = Option.None[arcana_text.types.CompositionRange] :: :: call
        self :: :: clear_selection

    fn apply_action(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer, read payload: (arcana_text.layout.LayoutSnapshot, arcana_text.editor.EditAction)):
        let snapshot = payload.0
        let action = payload.1
        let should_track = arcana_text.editor.tracks_action :: action :: call
        if should_track:
            self :: :: start_change
        match action:
            arcana_text.editor.EditAction.MoveBufferStart => self :: :: move_buffer_start
            arcana_text.editor.EditAction.MoveBufferEnd => self :: buffer :: move_buffer_end
            arcana_text.editor.EditAction.MoveLeft => self :: buffer :: move_left
            arcana_text.editor.EditAction.MoveRight => self :: buffer :: move_right
            arcana_text.editor.EditAction.MoveWordLeft => self :: buffer :: move_word_left
            arcana_text.editor.EditAction.MoveWordRight => self :: buffer :: move_word_right
            arcana_text.editor.EditAction.MoveLineStart => self :: buffer :: move_line_start
            arcana_text.editor.EditAction.MoveLineEnd => self :: buffer :: move_line_end
            arcana_text.editor.EditAction.MoveUp => self :: snapshot :: move_up
            arcana_text.editor.EditAction.MoveDown => self :: snapshot :: move_down
            arcana_text.editor.EditAction.ExtendBufferStart => self :: :: extend_buffer_start
            arcana_text.editor.EditAction.ExtendBufferEnd => self :: buffer :: extend_buffer_end
            arcana_text.editor.EditAction.ExtendLeft => self :: buffer :: extend_left
            arcana_text.editor.EditAction.ExtendRight => self :: buffer :: extend_right
            arcana_text.editor.EditAction.ExtendUp => self :: snapshot :: extend_up
            arcana_text.editor.EditAction.ExtendDown => self :: snapshot :: extend_down
            arcana_text.editor.EditAction.Escape => self :: :: escape
            arcana_text.editor.EditAction.SelectAll => self :: buffer :: select_all
            arcana_text.editor.EditAction.Insert(text) => self :: buffer, text :: insert_text
            arcana_text.editor.EditAction.Enter => self :: buffer :: insert_newline
            arcana_text.editor.EditAction.Backspace => self :: buffer :: delete_backward
            arcana_text.editor.EditAction.Delete => self :: buffer :: delete_forward
            arcana_text.editor.EditAction.DeleteWordLeft => self :: buffer :: delete_word_left
            arcana_text.editor.EditAction.DeleteWordRight => self :: buffer :: delete_word_right
            arcana_text.editor.EditAction.Indent => self :: buffer :: indent
            arcana_text.editor.EditAction.Unindent => self :: buffer :: unindent
            arcana_text.editor.EditAction.Undo => self :: buffer :: undo
            arcana_text.editor.EditAction.Redo => self :: buffer :: redo
            arcana_text.editor.EditAction.Click(point) => self :: snapshot, point :: click
            arcana_text.editor.EditAction.DoubleClick(point) => self :: (buffer, snapshot, point) :: double_click
            arcana_text.editor.EditAction.TripleClick(point) => self :: (buffer, snapshot, point) :: triple_click
            arcana_text.editor.EditAction.Drag(point) => self :: (buffer, snapshot, point) :: drag
            arcana_text.editor.EditAction.CommitComposition(text) => self :: buffer, text :: apply_committed_text
            arcana_text.editor.EditAction.UpdateComposition(text) => self :: buffer, text :: apply_composition_text
            arcana_text.editor.EditAction.Scroll(_) => 0
        if should_track:
            self :: :: commit_change

    fn apply_committed_text(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer, read text: Str):
        self :: buffer, text :: replace_selection
        self.composition = Option.None[arcana_text.types.CompositionRange] :: :: call

    fn apply_composition_text(edit self: arcana_text.editor.TextEditor, edit buffer: arcana_text.buffer.TextBuffer, read text: Str):
        let start = self.cursor.offset
        self :: buffer, text :: replace_selection
        self.composition = Option.Some[arcana_text.types.CompositionRange] :: (arcana_text.types.CompositionRange :: range = (arcana_text.types.TextRange :: start = start, end = (start + (std.text.len_bytes :: text :: call)) :: call), caret = (start + (std.text.len_bytes :: text :: call)) :: call) :: call

    fn cursor_position(read self: arcana_text.editor.TextEditor, read snapshot: arcana_text.layout.LayoutSnapshot) -> (Int, Int):
        let caret = arcana_text.queries.caret_box :: snapshot, self.cursor.offset :: call
        return caret.position

    fn cursor_box(read self: arcana_text.editor.TextEditor, read snapshot: arcana_text.layout.LayoutSnapshot) -> arcana_text.types.CaretBox:
        return arcana_text.queries.caret_box :: snapshot, self.cursor.offset :: call

impl SyntaxEditor:
    fn layer(read self: arcana_text.editor.SyntaxEditor) -> arcana_text.editor.EditorLayer:
        return arcana_text.editor.EditorLayer.Syntax :: :: call

    fn editor(read self: arcana_text.editor.SyntaxEditor) -> arcana_text.editor.TextEditor:
        return self.editor

    fn language(read self: arcana_text.editor.SyntaxEditor) -> Str:
        return self.language

    fn set_language(edit self: arcana_text.editor.SyntaxEditor, read language: Str):
        self.language = language

    fn theme(read self: arcana_text.editor.SyntaxEditor) -> arcana_text.editor.SyntaxTheme:
        return self.theme

    fn theme_name(read self: arcana_text.editor.SyntaxEditor) -> Str:
        return self.theme.name

    fn set_theme(edit self: arcana_text.editor.SyntaxEditor, read theme: arcana_text.editor.SyntaxTheme):
        self.theme = theme

    fn set_theme_name(edit self: arcana_text.editor.SyntaxEditor, read name: Str):
        self.theme = arcana_text.editor.SyntaxTheme :: name = name :: call

    fn clear_highlighting(edit self: arcana_text.editor.SyntaxEditor, edit buffer: arcana_text.buffer.TextBuffer):
        buffer :: :: clear_spans

    fn set_highlighting(edit self: arcana_text.editor.SyntaxEditor, edit buffer: arcana_text.buffer.TextBuffer, read spans: List[arcana_text.types.TextSpan]):
        if spans :: :: is_empty:
            self :: buffer :: clear_highlighting
            return
        buffer :: spans :: set_spans

    fn apply_action(edit self: arcana_text.editor.SyntaxEditor, edit buffer: arcana_text.buffer.TextBuffer, read payload: (arcana_text.layout.LayoutSnapshot, arcana_text.editor.EditAction)):
        self.editor :: buffer, payload :: apply_action

    fn cursor_position(read self: arcana_text.editor.SyntaxEditor, read snapshot: arcana_text.layout.LayoutSnapshot) -> (Int, Int):
        return self.editor :: snapshot :: cursor_position

    fn cursor_box(read self: arcana_text.editor.SyntaxEditor, read snapshot: arcana_text.layout.LayoutSnapshot) -> arcana_text.types.CaretBox:
        return self.editor :: snapshot :: cursor_box

    fn selection_range(read self: arcana_text.editor.SyntaxEditor, read buffer: arcana_text.buffer.TextBuffer) -> Option[arcana_text.types.TextRange]:
        return self.editor :: buffer :: selection_range

    fn selection_text(read self: arcana_text.editor.SyntaxEditor, read buffer: arcana_text.buffer.TextBuffer) -> Option[Str]:
        return self.editor :: buffer :: selection_text

    fn selection_boxes(read self: arcana_text.editor.SyntaxEditor, read buffer: arcana_text.buffer.TextBuffer, read snapshot: arcana_text.layout.LayoutSnapshot) -> List[arcana_text.types.RangeBox]:
        return self.editor :: buffer, snapshot :: selection_boxes

impl ViEditor:
    fn layer(read self: arcana_text.editor.ViEditor) -> arcana_text.editor.EditorLayer:
        return arcana_text.editor.EditorLayer.Vi :: :: call

    fn mode(read self: arcana_text.editor.ViEditor) -> arcana_text.editor.ViMode:
        return self.mode

    fn set_mode(edit self: arcana_text.editor.ViEditor, mode: arcana_text.editor.ViMode):
        self.mode = mode
        if mode == (arcana_text.editor.ViMode.Visual :: :: call):
            self.editor.selection_mode = arcana_text.types.SelectionMode.Normal :: :: call
            self.editor :: begin_extend
        if mode == (arcana_text.editor.ViMode.VisualLine :: :: call):
            self.editor.selection_mode = arcana_text.types.SelectionMode.Line :: :: call
            self.editor :: begin_extend
        if mode == (arcana_text.editor.ViMode.Normal :: :: call):
            self.editor.selection_mode = arcana_text.types.SelectionMode.Normal :: :: call
            self.editor :: clear_selection

    fn passthrough(read self: arcana_text.editor.ViEditor) -> Bool:
        return self.passthrough

    fn set_passthrough(edit self: arcana_text.editor.ViEditor, enabled: Bool):
        self.passthrough = enabled

    fn changed(read self: arcana_text.editor.ViEditor) -> Bool:
        return self.changed

    fn set_changed(edit self: arcana_text.editor.ViEditor, changed: Bool):
        self.changed = changed

    fn save_point(edit self: arcana_text.editor.ViEditor):
        self.changed = false

    fn editor(read self: arcana_text.editor.ViEditor) -> arcana_text.editor.TextEditor:
        return self.editor

    fn store_register(edit self: arcana_text.editor.ViEditor, read name: Str, read text: Str):
        let register = arcana_text.editor.ViRegister :: name = name, text = text :: call
        arcana_text.editor.replace_register :: self.registers, register :: call

    fn yank_selection(edit self: arcana_text.editor.ViEditor, read buffer: arcana_text.buffer.TextBuffer):
        let copied = self.editor :: buffer :: copy_selection
        if copied :: :: is_some:
            self :: "_", (copied :: "" :: unwrap_or) :: store_register

    fn paste_text(edit self: arcana_text.editor.ViEditor, edit buffer: arcana_text.buffer.TextBuffer, read text: Str):
        if text == "":
            return
        self.editor :: buffer, text :: replace_selection
        self.changed = true

    fn enter_insert_after(edit self: arcana_text.editor.ViEditor, edit buffer: arcana_text.buffer.TextBuffer):
        self.editor :: buffer :: move_right
        self.mode = arcana_text.editor.ViMode.Insert :: :: call

    fn delete_line(edit self: arcana_text.editor.ViEditor, edit buffer: arcana_text.buffer.TextBuffer):
        self.editor :: buffer, self.editor.cursor.offset :: select_line
        self :: buffer :: yank_selection
        if self.editor :: buffer :: delete_selection:
            self.changed = true
        self.mode = arcana_text.editor.ViMode.Normal :: :: call

    fn delete_word(edit self: arcana_text.editor.ViEditor, edit buffer: arcana_text.buffer.TextBuffer):
        self.editor :: begin_extend
        self.editor :: buffer :: extend_word_right
        self :: buffer :: yank_selection
        if self.editor :: buffer :: delete_selection:
            self.changed = true
        self.mode = arcana_text.editor.ViMode.Normal :: :: call

    fn delete_char(edit self: arcana_text.editor.ViEditor, edit buffer: arcana_text.buffer.TextBuffer):
        self.editor :: begin_extend
        self.editor :: buffer :: move_right
        self :: buffer :: yank_selection
        if self.editor :: buffer :: delete_selection:
            self.changed = true
        self.mode = arcana_text.editor.ViMode.Normal :: :: call

    fn delete_selected(edit self: arcana_text.editor.ViEditor, edit buffer: arcana_text.buffer.TextBuffer):
        self :: buffer :: yank_selection
        if self.editor :: buffer :: delete_selection:
            self.changed = true
        self.mode = arcana_text.editor.ViMode.Normal :: :: call

    fn paste_before(edit self: arcana_text.editor.ViEditor, edit buffer: arcana_text.buffer.TextBuffer):
        let text = arcana_text.editor.register_value :: self.registers, "_" :: call
        self :: buffer, text :: paste_text
        self.mode = arcana_text.editor.ViMode.Normal :: :: call

    fn paste_after(edit self: arcana_text.editor.ViEditor, edit buffer: arcana_text.buffer.TextBuffer):
        let text = arcana_text.editor.register_value :: self.registers, "_" :: call
        self.editor :: buffer :: move_right
        self :: buffer, text :: paste_text
        self.mode = arcana_text.editor.ViMode.Normal :: :: call

    fn insert_or_replace(edit self: arcana_text.editor.ViEditor, edit buffer: arcana_text.buffer.TextBuffer, read text: Str):
        if self.mode == (arcana_text.editor.ViMode.Insert :: :: call):
            self.editor :: buffer, text :: insert_text
            self.changed = true
            return
        self :: buffer, text :: paste_text

    fn undo_action(edit self: arcana_text.editor.ViEditor, edit buffer: arcana_text.buffer.TextBuffer):
        if self.editor :: buffer :: undo:
            self.changed = true

    fn redo_action(edit self: arcana_text.editor.ViEditor, edit buffer: arcana_text.buffer.TextBuffer):
        if self.editor :: buffer :: redo:
            self.changed = true

    fn apply_motion(edit self: arcana_text.editor.ViEditor, edit buffer: arcana_text.buffer.TextBuffer, read payload: (arcana_text.layout.LayoutSnapshot, arcana_text.editor.ViAction)):
        let snapshot = payload.0
        let action = payload.1
        let normal = self.mode == (arcana_text.editor.ViMode.Normal :: :: call) or self.mode == (arcana_text.editor.ViMode.Insert :: :: call)
        if action == (arcana_text.editor.ViAction.MoveLeft :: :: call):
            if normal:
                self.editor :: buffer :: move_left
            else:
                self.editor :: buffer :: extend_left
            return
        if action == (arcana_text.editor.ViAction.MoveRight :: :: call):
            if normal:
                self.editor :: buffer :: move_right
            else:
                self.editor :: buffer :: extend_right
            return
        if action == (arcana_text.editor.ViAction.MoveUp :: :: call):
            if normal:
                self.editor :: snapshot :: move_up
            else:
                self.editor :: snapshot :: extend_up
            return
        if action == (arcana_text.editor.ViAction.MoveDown :: :: call):
            if normal:
                self.editor :: snapshot :: move_down
            else:
                self.editor :: snapshot :: extend_down
            return
        if action == (arcana_text.editor.ViAction.MoveWordLeft :: :: call):
            if normal:
                self.editor :: buffer :: move_word_left
            else:
                self.editor :: buffer :: extend_word_left
            return
        if action == (arcana_text.editor.ViAction.MoveWordRight :: :: call):
            if normal:
                self.editor :: buffer :: move_word_right
            else:
                self.editor :: buffer :: extend_word_right
            return
        if action == (arcana_text.editor.ViAction.MoveLineStart :: :: call):
            if normal:
                self.editor :: buffer :: move_line_start
            else:
                self.editor :: buffer :: extend_line_start
            return
        if action == (arcana_text.editor.ViAction.MoveLineEnd :: :: call):
            if normal:
                self.editor :: buffer :: move_line_end
            else:
                self.editor :: buffer :: extend_line_end
            return

    fn apply_action(edit self: arcana_text.editor.ViEditor, edit buffer: arcana_text.buffer.TextBuffer, read payload: (arcana_text.layout.LayoutSnapshot, arcana_text.editor.ViAction)):
        let snapshot = payload.0
        let action = payload.1
        if self.passthrough:
            match action:
                arcana_text.editor.ViAction.ReplaceSelection(text) => self.editor :: buffer, text :: insert_text
                arcana_text.editor.ViAction.InsertText(text) => self.editor :: buffer, text :: insert_text
                arcana_text.editor.ViAction.Enter => self.editor :: buffer :: insert_newline
                arcana_text.editor.ViAction.Undo => self.editor :: buffer :: undo
                arcana_text.editor.ViAction.Redo => self.editor :: buffer :: redo
                _ => 0
            return
        if action == (arcana_text.editor.ViAction.MoveLeft :: :: call) or action == (arcana_text.editor.ViAction.MoveRight :: :: call) or action == (arcana_text.editor.ViAction.MoveUp :: :: call) or action == (arcana_text.editor.ViAction.MoveDown :: :: call) or action == (arcana_text.editor.ViAction.MoveWordLeft :: :: call) or action == (arcana_text.editor.ViAction.MoveWordRight :: :: call) or action == (arcana_text.editor.ViAction.MoveLineStart :: :: call) or action == (arcana_text.editor.ViAction.MoveLineEnd :: :: call):
            self :: buffer, (snapshot, action) :: apply_motion
            return
        match action:
            arcana_text.editor.ViAction.Escape => self :: (arcana_text.editor.ViMode.Normal :: :: call) :: set_mode
            arcana_text.editor.ViAction.InsertMode => self.mode = (arcana_text.editor.ViMode.Insert :: :: call)
            arcana_text.editor.ViAction.AppendMode => self :: buffer :: enter_insert_after
            arcana_text.editor.ViAction.VisualMode => self :: (arcana_text.editor.ViMode.Visual :: :: call) :: set_mode
            arcana_text.editor.ViAction.VisualLineMode => self :: buffer :: enter_visual_line_mode
            arcana_text.editor.ViAction.DeleteSelection => self :: buffer :: delete_selected
            arcana_text.editor.ViAction.DeleteLine => self :: buffer :: delete_line
            arcana_text.editor.ViAction.DeleteChar => self :: buffer :: delete_char
            arcana_text.editor.ViAction.DeleteWord => self :: buffer :: delete_word
            arcana_text.editor.ViAction.YankSelection => self :: buffer :: yank_selection
            arcana_text.editor.ViAction.PasteBefore => self :: buffer :: paste_before
            arcana_text.editor.ViAction.PasteAfter => self :: buffer :: paste_after
            arcana_text.editor.ViAction.ReplaceSelection(text) => self :: buffer, text :: paste_text
            arcana_text.editor.ViAction.InsertText(text) => self :: buffer, text :: insert_or_replace
            arcana_text.editor.ViAction.Enter => self.editor :: buffer :: insert_newline
            arcana_text.editor.ViAction.Undo => self :: buffer :: undo_action
            arcana_text.editor.ViAction.Redo => self :: buffer :: redo_action
            _ => 0

    fn enter_visual_line_mode(edit self: arcana_text.editor.ViEditor, read buffer: arcana_text.buffer.TextBuffer):
        self :: (arcana_text.editor.ViMode.VisualLine :: :: call) :: set_mode
        self.editor :: buffer, self.editor.cursor.offset :: select_line
