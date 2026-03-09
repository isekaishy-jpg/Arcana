import std.collections.list
import std.fs
import std.io
import std.path
import std.text
import arcana_compiler_core.core
import arcana_compiler_core.fingerprint
import arcana_compiler_core.sources
import arcana_compiler_core.workspace
import bytecode_emit
import ir_lower
import parse
import protocol
import tokenize
import types
import typecheck

fn ensure_clean_workspace_cache(workspace_dir: Str):
    let arcana_dir = std.path.join :: workspace_dir, ".arcana" :: call
    if std.fs.is_dir :: arcana_dir :: call:
        let _ = std.fs.remove_dir_all :: arcana_dir :: call

fn ensure_artifact_dir(workspace_dir: Str, member_name: Str):
    let artifact_dir = std.path.join :: (std.path.join :: (std.path.join :: workspace_dir, ".arcana" :: call), "artifacts" :: call), member_name :: call
    let _ = std.fs.mkdir_all :: artifact_dir :: call

fn run_build_selfhost_full(workspace_dir: Str, options_tag: Str) -> Int:
    let n_tag = std.text.len_bytes :: options_tag :: call
    let sep = std.text.find_byte :: options_tag, 0, 124 :: call
    let mut target_member = options_tag
    let mut emit_ir_dump = false
    if sep >= 0:
        target_member = std.text.slice_bytes :: options_tag, 0, sep :: call
        let tail = std.text.slice_bytes :: options_tag, sep + 1, n_tag :: call
        emit_ir_dump = (tail == "1")
    let mut workspace_name = arcana_compiler_core.workspace.workspace_name_or_default :: workspace_dir :: call
    let deps_rows = arcana_compiler_core.workspace.workspace_dep_rows :: workspace_dir :: call
    let meta_rows = arcana_compiler_core.workspace.workspace_meta_rows :: workspace_dir :: call
    let mut order = arcana_compiler_core.workspace.resolve_workspace_plan_names :: workspace_dir :: call
    let build_set = arcana_compiler_core.core.build_set_for_target :: deps_rows, target_member :: call
    if (std.text.len_bytes :: target_member :: call) > 0 and (not (arcana_compiler_core.core.list_has :: order, target_member :: call)):
        let message = "unknown workspace member '" + target_member + "'"
        let diag = workspace.diag_at :: workspace_dir, "ARC-SHCOMP-BUILD-FAILED", message :: call
        protocol.emit_diag :: diag :: call
        protocol.emit_final :: 1, 0, 0 :: call
        return 1

    let mut order_rev = std.collections.list.new[Str] :: :: call
    let mut order_scan = order
    while (order_scan :: :: len) > 0:
        order_rev :: (order_scan :: :: pop) :: push

    let mut path_rows = std.collections.list.new[Str] :: :: call
    let mut dep_rows = std.collections.list.new[Str] :: :: call
    let mut fingerprint_rows = std.collections.list.new[Str] :: :: call
    let mut artifact_rows = std.collections.list.new[Str] :: :: call
    let mut built_names_members = std.collections.list.new[Str] :: :: call
    let mut built_names_order = std.collections.list.new[Str] :: :: call
    let mut checksum = 0
    let mut errors = 0
    let _ir_dump_dir = std.path.join :: (std.path.join :: workspace_dir, ".arcana" :: call), "logs" :: call

    while (order_rev :: :: len) > 0:
        let name = order_rev :: :: pop
        if not (arcana_compiler_core.core.list_has :: build_set, name :: call):
            continue
        let meta_row = arcana_compiler_core.core.find_member_meta_row :: meta_rows, name :: call
        if (std.text.len_bytes :: meta_row :: call) <= 0:
            continue
        let rel = arcana_compiler_core.core.member_meta_rel :: meta_row :: call
        let kind = arcana_compiler_core.core.member_meta_kind :: meta_row :: call
        let member_dir = std.path.normalize :: (std.path.join :: workspace_dir, rel :: call) :: call
        let member_book = std.path.join :: member_dir, "book.toml" :: call
        let member_text = std.fs.read_text_or :: member_book, "" :: call
        let deps = arcana_compiler_core.core.parse_deps_value :: member_text :: call

        let fingerprint_value = arcana_compiler_core.fingerprint.member_source_fingerprint :: member_dir :: call
        let ext = arcana_compiler_core.core.artifact_ext_for_kind :: kind :: call
        let artifact_rel = arcana_compiler_core.core.artifact_rel_path :: name, fingerprint_value, ext :: call
        let artifact_abs = std.path.join :: workspace_dir, artifact_rel :: call

        workspace.ensure_artifact_dir :: workspace_dir, name :: call
        let mut event_status = "cache_hit"
        if not (std.fs.is_file :: artifact_abs :: call):
            let validate = workspace.validate_compile_sources :: member_dir :: call
            if validate.0 > 0:
                protocol.emit_build_event :: name, "failed", artifact_abs :: call
                let message = "selfhost build failed while validating sources for '" + name + "'"
                let diag = workspace.diag_at :: member_dir, "ARC-SHCOMP-BUILD-FAILED", message :: call
                protocol.emit_diag :: diag :: call
                checksum = protocol.fold_checksum :: checksum, validate.1 :: call
                errors = 1
                break
            let emit = workspace.emit_artifact_from_source :: member_dir, artifact_abs :: call
            if (std.text.len_bytes :: emit :: call) > 0:
                protocol.emit_build_event :: name, "failed", artifact_abs :: call
                let message = "selfhost emit failed while compiling '" + name + "': " + emit
                let diag = workspace.diag_at :: member_dir, "ARC-SHCOMP-UNSUPPORTED-CONSTRUCT", message :: call
                protocol.emit_diag :: diag :: call
                errors = 1
                break
            event_status = "compiled"
        protocol.emit_build_event :: name, event_status, artifact_abs :: call
        checksum = protocol.fold_checksum :: checksum, (std.text.len_bytes :: name :: call) :: call
        checksum = protocol.fold_checksum :: checksum, (std.text.len_bytes :: event_status :: call) :: call
        checksum = protocol.fold_checksum :: checksum, (std.text.len_bytes :: artifact_rel :: call) :: call

        let path_row = arcana_compiler_core.core.lock_path_row :: name, rel :: call
        path_rows :: path_row :: push
        let deps_row = arcana_compiler_core.core.format_dep_row :: name, deps :: call
        dep_rows :: deps_row :: push
        let fp_row = arcana_compiler_core.core.lock_fingerprint_row :: name, fingerprint_value :: call
        fingerprint_rows :: fp_row :: push
        let artifact_row = arcana_compiler_core.core.lock_artifact_row :: name, artifact_rel :: call
        artifact_rows :: artifact_row :: push
        let built_name_members = arcana_compiler_core.core.member_meta_name :: meta_row :: call
        let built_name_order = arcana_compiler_core.core.member_meta_name :: meta_row :: call
        built_names_members :: built_name_members :: push
        built_names_order :: built_name_order :: push
    if errors > 0:
        protocol.emit_final :: 1, 0, checksum :: call
        return 1

    let mut lock_text = ""
    lock_text = lock_text + "version = 3\n"
    lock_text = lock_text + "workspace = \"" + workspace_name + "\"\n\n"
    lock_text = lock_text + "members = " + (arcana_compiler_core.workspace.render_name_list :: built_names_members :: call) + "\n\n"
    lock_text = lock_text + "order = " + (arcana_compiler_core.workspace.render_name_list :: built_names_order :: call) + "\n\n"
    lock_text = lock_text + "[paths]\n"
    lock_text = lock_text + (arcana_compiler_core.workspace.render_row_lines :: path_rows :: call)
    lock_text = lock_text + "\n[deps]\n"
    lock_text = lock_text + (arcana_compiler_core.workspace.render_row_lines :: dep_rows :: call)
    lock_text = lock_text + "\n[fingerprints]\n"
    lock_text = lock_text + (arcana_compiler_core.workspace.render_row_lines :: fingerprint_rows :: call)
    lock_text = lock_text + "\n[artifacts]\n"
    lock_text = lock_text + (arcana_compiler_core.workspace.render_row_lines :: artifact_rows :: call)
    if not (arcana_compiler_core.workspace.write_lock_text :: workspace_dir, lock_text :: call):
        let diag = workspace.diag_at :: workspace_dir, "ARC-SELFHOST-LOCK-WRITE", "failed to write Arcana.lock" :: call
        protocol.emit_diag :: diag :: call
        protocol.emit_final :: 1, 0, checksum :: call
        return 1

    let mut final_checksum = checksum
    final_checksum = protocol.fold_checksum :: final_checksum, (std.text.len_bytes :: workspace_name :: call) :: call
    protocol.emit_final :: 0, 0, final_checksum :: call
    return 0

fn fold_len(checksum: Int, text: Str) -> Int:
    return protocol.fold_checksum :: checksum, (std.text.len_bytes :: text :: call) :: call

fn diag_at(path: Str, code: Str, message: Str) -> types.Diag:
    let meta = (code, "error")
    let start = (1, 1)
    let loc = (path, start)
    let tail = (start, message)
    return types.Diag :: meta = meta, loc = loc, tail = tail :: call

fn validate_source_text(path: Str, text: Str) -> (Int, Int):
    let mut errors = 0
    let mut checksum = 0

    let token_stage = tokenize.validate_token_stream :: path, text :: call
    errors += token_stage.0
    checksum = protocol.fold_checksum :: checksum, token_stage.1 :: call

    let parse_stage = parse.validate_parse_structure :: path, text :: call
    errors += parse_stage.0
    checksum = protocol.fold_checksum :: checksum, parse_stage.1 :: call

    return (errors, checksum)

fn validate_compile_sources(read source_path: Str) -> (Int, Int):
    let mut errors = 0
    let mut checksum = 0
    let mut files = arcana_compiler_core.sources.collect_compile_sources :: source_path :: call
    if (files :: :: len) <= 0:
        let message = "selfhost compile target has no .arc sources"
        let diag = workspace.diag_at :: (arcana_compiler_core.core.copy_text :: source_path :: call), "ARC-SHCOMP-LOWER-FAILED", message :: call
        protocol.emit_diag :: diag :: call
        errors += 1
        checksum = protocol.fold_checksum :: checksum, (std.text.len_bytes :: source_path :: call) :: call
        return (errors, checksum)

    let mut rev = std.collections.list.new[Str] :: :: call
    while (files :: :: len) > 0:
        rev :: (files :: :: pop) :: push
    while (rev :: :: len) > 0:
        let path = rev :: :: pop
        let text = std.fs.read_text_or :: path, "" :: call
        checksum = protocol.fold_checksum :: checksum, (std.text.len_bytes :: path :: call) :: call
        checksum = protocol.fold_checksum :: checksum, (std.text.len_bytes :: text :: call) :: call
        let result = workspace.validate_source_text :: path, text :: call
        errors += result.0
        checksum = protocol.fold_checksum :: checksum, result.1 :: call

    let sem = typecheck.validate_semantics_target :: source_path :: call
    errors += sem.0
    checksum = protocol.fold_checksum :: checksum, sem.1 :: call

    let lower = ir_lower.validate_lowering_target :: source_path :: call
    errors += lower.0
    checksum = protocol.fold_checksum :: checksum, lower.1 :: call

    return (errors, checksum)

fn emit_artifact_from_source(read source_path: Str, out_path: Str) -> Str:
    let emit_target = bytecode_emit.validate_emit_target :: out_path :: call
    if emit_target.0 > 0:
        return "unsupported artifact extension in output path"
    return bytecode_emit.emit_artifact_error :: source_path, out_path :: call

export fn run_compile(read source_path: Str, out_path: Str, read extra: List[Str]) -> Int:
    let mut artifact_kind = "app"
    if (std.path.ext :: out_path :: call) == "arclib":
        artifact_kind = "lib"
    let mut errors = 0
    let warnings = 0
    let mut checksum = 0
    let _emit_ir_dump = arcana_compiler_core.core.has_flag :: extra, "--emit-ir-dump" :: call

    let validate = workspace.validate_compile_sources :: source_path :: call
    checksum = protocol.fold_checksum :: checksum, validate.1 :: call
    if validate.0 > 0:
        errors += validate.0
        protocol.emit_final :: errors, warnings, checksum :: call
        return 1

    let emit = workspace.emit_artifact_from_source :: source_path, out_path :: call
    if (std.text.len_bytes :: emit :: call) > 0:
        let message = "selfhost compile emitter could not materialize artifact"
        let full_message = message + ": " + emit
        let diag = diag_at :: (arcana_compiler_core.core.copy_text :: source_path :: call), "ARC-SHCOMP-UNSUPPORTED-CONSTRUCT", full_message :: call
        protocol.emit_diag :: diag :: call
        errors = 1
        checksum = fold_len :: checksum, (arcana_compiler_core.core.copy_text :: source_path :: call) :: call
        protocol.emit_final :: errors, warnings, checksum :: call
        return 1

    let artifact_checksum = bytecode_emit.file_checksum_or_zero :: out_path :: call
    if artifact_checksum <= 0:
        let message = "selfhost lower/emit produced empty artifact checksum"
        let diag = diag_at :: (arcana_compiler_core.core.copy_text :: source_path :: call), "ARC-SHCOMP-LOWER-FAILED", message :: call
        protocol.emit_diag :: diag :: call
        errors = 1
        checksum = fold_len :: checksum, (arcana_compiler_core.core.copy_text :: source_path :: call) :: call
        protocol.emit_final :: errors, warnings, checksum :: call
        return 1

    let bytecode_version = bytecode_emit.bytecode_version_or_zero :: out_path :: call
    let fingerprint = ir_lower.artifact_fingerprint :: artifact_checksum :: call
    let ir_fingerprint = ir_lower.ir_fingerprint :: artifact_checksum :: call
    let left = (artifact_kind, out_path)
    let right_tail = (ir_fingerprint, bytecode_version)
    let right = (fingerprint, right_tail)
    let artifact = types.Artifact :: left = left, right = right :: call
    protocol.emit_artifact :: artifact :: call
    checksum = protocol.fold_checksum :: checksum, artifact_checksum :: call
    checksum = protocol.fold_checksum :: checksum, bytecode_version :: call

    if errors > 0:
        protocol.emit_final :: errors, warnings, checksum :: call
        return 1
    protocol.emit_final :: errors, warnings, checksum :: call
    return 0

export fn run_build(workspace_dir: Str, read extra: List[Str]) -> Int:
    let extra_values = extra
    let mut target_member = ""
    if arcana_compiler_core.core.has_flag :: extra_values, "--member" :: call:
        target_member = arcana_compiler_core.core.parse_member_flag :: extra_values :: call
    let emit_ir_dump = arcana_compiler_core.core.has_flag :: extra_values, "--emit-ir-dump" :: call
    if arcana_compiler_core.core.has_flag :: extra_values, "--clean" :: call:
        workspace.ensure_clean_workspace_cache :: workspace_dir :: call
    if arcana_compiler_core.core.has_flag :: extra_values, "--plan" :: call:
        let mut names = arcana_compiler_core.workspace.resolve_workspace_plan_names :: workspace_dir :: call
        let mut names_rev = std.collections.list.new[Str] :: :: call
        while (names :: :: len) > 0:
            names_rev :: (names :: :: pop) :: push
        let mut checksum = 0
        while (names_rev :: :: len) > 0:
            let name = names_rev :: :: pop
            name :: :: std.io.print
            checksum = protocol.fold_checksum :: checksum, (std.text.len_bytes :: name :: call) :: call
        protocol.emit_final :: 0, 0, checksum :: call
        return 0
    let mut options_tag = target_member + "|"
    if emit_ir_dump:
        options_tag = options_tag + "1"
    else:
        options_tag = options_tag + "0"
    return workspace.run_build_selfhost_full :: workspace_dir, options_tag :: call
