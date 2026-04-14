shackle flags WinapiProcessInternals:
    pub(crate) struct WinapiFileStreamState {
        pub(crate) path: String,
        pub(crate) file: std::fs::File,
        pub(crate) readable: bool,
        pub(crate) writable: bool,
    }

    pub(crate) fn runtime_path_string(path: &std::path::Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    pub(crate) fn normalize_lexical_path(path: &std::path::Path) -> std::path::PathBuf {
        let mut normalized = std::path::PathBuf::new();
        let mut saw_root = false;
        for component in path.components() {
            match component {
                std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
                std::path::Component::RootDir => {
                    normalized.push(component.as_os_str());
                    saw_root = true;
                }
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    if !normalized.pop() && !saw_root {
                        normalized.push("..");
                    }
                }
                std::path::Component::Normal(part) => normalized.push(part),
            }
        }
        if normalized.as_os_str().is_empty() && saw_root {
            normalized.push(std::path::Path::new(std::path::MAIN_SEPARATOR_STR));
        }
        normalized
    }

    pub(crate) fn current_working_dir() -> Result<std::path::PathBuf, String> {
        std::env::current_dir()
            .map(|path| normalize_lexical_path(&path))
            .map_err(|err| format!("failed to resolve current directory: {err}"))
    }

    pub(crate) fn resolve_fs_path(path: &str) -> Result<std::path::PathBuf, String> {
        let requested = std::path::PathBuf::from(path);
        Ok(if requested.is_absolute() {
            normalize_lexical_path(&requested)
        } else {
            normalize_lexical_path(&current_working_dir()?.join(requested))
        })
    }

    pub(crate) fn relative_path(
        path: &std::path::Path,
        base: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        let path = normalize_lexical_path(path);
        let base = normalize_lexical_path(base);
        let path_parts = path.components().collect::<Vec<_>>();
        let base_parts = base.components().collect::<Vec<_>>();
        let mut shared = 0usize;
        while shared < path_parts.len()
            && shared < base_parts.len()
            && path_parts[shared] == base_parts[shared]
        {
            shared += 1;
        }
        if shared == 0
            && path_parts.first().is_some_and(|part| matches!(part, std::path::Component::Prefix(_)))
            && base_parts.first().is_some_and(|part| matches!(part, std::path::Component::Prefix(_)))
        {
            return Err(format!(
                "failed to make `{}` relative to `{}`",
                runtime_path_string(&path),
                runtime_path_string(&base)
            ));
        }
        let mut relative = std::path::PathBuf::new();
        for _ in shared..base_parts.len() {
            relative.push("..");
        }
        for component in path_parts.iter().skip(shared) {
            relative.push(component.as_os_str());
        }
        Ok(relative)
    }

    pub(crate) fn file_stream_ref(
        instance: &crate::BindingInstance,
        handle: u64,
    ) -> Result<&WinapiFileStreamState, String> {
        if handle == 0 {
            return Err("FileStream handle must not be 0".to_string());
        }
        crate::shackle::package_state_data_ref(instance)?
            .file_streams
            .get(&handle)
            .ok_or_else(|| format!("invalid FileStream handle `{handle}`"))
    }

    pub(crate) fn file_stream_mut(
        instance: &mut crate::BindingInstance,
        handle: u64,
    ) -> Result<&mut WinapiFileStreamState, String> {
        if handle == 0 {
            return Err("FileStream handle must not be 0".to_string());
        }
        crate::shackle::package_state_data_mut(instance)?
            .file_streams
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid FileStream handle `{handle}`"))
    }

    pub(crate) fn insert_file_stream(
        instance: &mut crate::BindingInstance,
        path: &std::path::Path,
        file: std::fs::File,
        readable: bool,
        writable: bool,
    ) -> Result<u64, String> {
        let state = crate::shackle::package_state_data_mut(instance)?;
        let handle = state.next_file_stream_handle;
        state.next_file_stream_handle += 1;
        state.file_streams.insert(
            handle,
            WinapiFileStreamState {
                path: runtime_path_string(path),
                file,
                readable,
                writable,
            },
        );
        Ok(handle)
    }

    pub(crate) fn encode_u32_le(value: u32, out: &mut Vec<u8>) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn encode_i32_le(value: i32, out: &mut Vec<u8>) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn decode_u32_le(bytes: &[u8], offset: &mut usize) -> Result<u32, String> {
        let start = *offset;
        let slice = bytes
            .get(start..start + 4)
            .ok_or_else(|| "payload truncated".to_string())?;
        *offset += 4;
        Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    pub(crate) fn encode_string_list_payload(values: &[String]) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        encode_u32_le(
            u32::try_from(values.len())
                .map_err(|_| "string list length does not fit in u32".to_string())?,
            &mut out,
        );
        for value in values {
            let bytes = value.as_bytes();
            encode_u32_le(
                u32::try_from(bytes.len())
                    .map_err(|_| "string byte length does not fit in u32".to_string())?,
                &mut out,
            );
            out.extend_from_slice(bytes);
        }
        Ok(out)
    }

    pub(crate) fn decode_string_list_payload(bytes: &[u8]) -> Result<Vec<String>, String> {
        let mut cursor = 0usize;
        let count = decode_u32_le(bytes, &mut cursor)? as usize;
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            let len = decode_u32_le(bytes, &mut cursor)? as usize;
            let slice = bytes
                .get(cursor..cursor + len)
                .ok_or_else(|| "string list payload truncated".to_string())?;
            let value = String::from_utf8(slice.to_vec())
                .map_err(|err| format!("string list payload was not valid UTF-8: {err}"))?;
            cursor += len;
            out.push(value);
        }
        if cursor != bytes.len() {
            return Err("string list payload had trailing bytes".to_string());
        }
        Ok(out)
    }

    pub(crate) fn encode_exec_capture_payload(
        status: i32,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        stdout_utf8: bool,
        stderr_utf8: bool,
    ) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        encode_i32_le(status, &mut out);
        out.push(u8::from(stdout_utf8));
        out.push(u8::from(stderr_utf8));
        encode_u32_le(
            u32::try_from(stdout.len())
                .map_err(|_| "stdout payload length does not fit in u32".to_string())?,
            &mut out,
        );
        out.extend_from_slice(&stdout);
        encode_u32_le(
            u32::try_from(stderr.len())
                .map_err(|_| "stderr payload length does not fit in u32".to_string())?,
            &mut out,
        );
        out.extend_from_slice(&stderr);
        Ok(out)
    }

shackle fn process_arg_count_impl() -> Int = helpers.process.arg_count:
    Ok(binding_int(std::env::args().skip(1).count() as i64))

shackle fn process_arg_get_impl(index: Int) -> Str = helpers.process.arg_get:
    if index < 0 {
        return Err("arg_get index must be non-negative".to_string());
    }
    Ok(binding_owned_str(std::env::args().skip(1).nth(index as usize).unwrap_or_default()))

shackle fn process_env_has_impl(read name: Str) -> Bool = helpers.process.env_has:
    Ok(binding_bool(std::env::var_os(&name).is_some()))

shackle fn process_env_get_impl(read name: Str) -> Str = helpers.process.env_get:
    Ok(binding_owned_str(std::env::var(&name).unwrap_or_default()))

shackle fn process_take_last_error_impl() -> Str = helpers.process.take_last_error:
    Ok(binding_owned_str(crate::shackle::take_helper_error(instance)))

shackle fn process_path_cwd_impl() -> Str = helpers.process.path_cwd:
    Ok(binding_owned_str(runtime_path_string(&current_working_dir()?)))

shackle fn process_path_join_impl(read a: Str, read b: Str) -> Str = helpers.process.path_join:
    Ok(binding_owned_str(runtime_path_string(&normalize_lexical_path(
        &std::path::Path::new(&a).join(b),
    ))))

shackle fn process_path_normalize_impl(read path: Str) -> Str = helpers.process.path_normalize:
    Ok(binding_owned_str(runtime_path_string(&normalize_lexical_path(std::path::Path::new(&path)))))

shackle fn process_path_parent_impl(read path: Str) -> Str = helpers.process.path_parent:
    Ok(binding_owned_str(std::path::Path::new(&path)
        .parent()
        .map(runtime_path_string)
        .unwrap_or_default()))

shackle fn process_path_file_name_impl(read path: Str) -> Str = helpers.process.path_file_name:
    Ok(binding_owned_str(std::path::Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()))

shackle fn process_path_ext_impl(read path: Str) -> Str = helpers.process.path_ext:
    Ok(binding_owned_str(std::path::Path::new(&path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_string())
        .unwrap_or_default()))

shackle fn process_path_is_absolute_impl(read path: Str) -> Bool = helpers.process.path_is_absolute:
    Ok(binding_bool(std::path::Path::new(&path).is_absolute()))

shackle fn process_path_stem_impl(read path: Str) -> Str = helpers.process.path_stem:
    crate::shackle::clear_helper_error(instance);
    match std::path::Path::new(&path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .ok_or_else(|| format!("path `{path}` has no stem"))
    {
        Ok(value) => Ok(binding_owned_str(value)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_owned_str(String::new()))
        }
    }

shackle fn process_path_with_ext_impl(read path: Str, read ext: Str) -> Str = helpers.process.path_with_ext:
    let mut updated = std::path::PathBuf::from(path);
    updated.set_extension(ext);
    Ok(binding_owned_str(runtime_path_string(&updated)))

shackle fn process_path_relative_to_impl(read path: Str, read base: Str) -> Str = helpers.process.path_relative_to:
    crate::shackle::clear_helper_error(instance);
    match relative_path(std::path::Path::new(&path), std::path::Path::new(&base))
        .map(|value| runtime_path_string(&value))
    {
        Ok(value) => Ok(binding_owned_str(value)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_owned_str(String::new()))
        }
    }

shackle fn process_path_canonicalize_impl(read path: Str) -> Str = helpers.process.path_canonicalize:
    crate::shackle::clear_helper_error(instance);
    let value = resolve_fs_path(&path).and_then(|resolved| {
        std::fs::canonicalize(&resolved)
            .map(|value| runtime_path_string(&normalize_lexical_path(&value)))
            .map_err(|err| format!("failed to canonicalize `{path}`: {err}"))
    });
    match value {
        Ok(value) => Ok(binding_owned_str(value)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_owned_str(String::new()))
        }
    }

shackle fn process_path_strip_prefix_impl(read path: Str, read prefix: Str) -> Str = helpers.process.path_strip_prefix:
    crate::shackle::clear_helper_error(instance);
    let path = normalize_lexical_path(std::path::Path::new(&path));
    let prefix = normalize_lexical_path(std::path::Path::new(&prefix));
    let value = path
        .strip_prefix(&prefix)
        .map(runtime_path_string)
        .map_err(|_| {
            format!(
                "path `{}` does not start with `{}`",
                runtime_path_string(&path),
                runtime_path_string(&prefix)
            )
        });
    match value {
        Ok(value) => Ok(binding_owned_str(value)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_owned_str(String::new()))
        }
    }

shackle fn process_fs_exists_impl(read path: Str) -> Bool = helpers.process.fs_exists:
    Ok(binding_bool(resolve_fs_path(&path)?.exists()))

shackle fn process_fs_is_file_impl(read path: Str) -> Bool = helpers.process.fs_is_file:
    Ok(binding_bool(resolve_fs_path(&path)?.is_file()))

shackle fn process_fs_is_dir_impl(read path: Str) -> Bool = helpers.process.fs_is_dir:
    Ok(binding_bool(resolve_fs_path(&path)?.is_dir()))

shackle fn process_fs_read_text_impl(read path: Str) -> Str = helpers.process.fs_read_text:
    crate::shackle::clear_helper_error(instance);
    let value = resolve_fs_path(&path).and_then(|resolved| {
        std::fs::read_to_string(&resolved)
            .map_err(|err| format!("failed to read `{}`: {err}", runtime_path_string(&resolved)))
    });
    match value {
        Ok(value) => Ok(binding_owned_str(value)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_owned_str(String::new()))
        }
    }

shackle fn process_fs_read_bytes_impl(read path: Str) -> Bytes = helpers.process.fs_read_bytes:
    crate::shackle::clear_helper_error(instance);
    let value = resolve_fs_path(&path).and_then(|resolved| {
        std::fs::read(&resolved)
            .map_err(|err| format!("failed to read `{}`: {err}", runtime_path_string(&resolved)))
    });
    match value {
        Ok(value) => Ok(binding_owned_bytes(value)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_owned_bytes(Vec::new()))
        }
    }

shackle fn process_fs_write_text_impl(read path: Str, read text: Str) -> Bool = helpers.process.fs_write_text:
    crate::shackle::clear_helper_error(instance);
    let result = resolve_fs_path(&path).and_then(|resolved| {
        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("failed to prepare `{}`: {err}", runtime_path_string(parent)))?;
        }
        std::fs::write(&resolved, text)
            .map_err(|err| format!("failed to write `{}`: {err}", runtime_path_string(&resolved)))
    });
    match result {
        Ok(()) => Ok(binding_bool(true)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_bool(false))
        }
    }

shackle fn process_fs_write_bytes_impl(read path: Str, read bytes: Bytes) -> Bool = helpers.process.fs_write_bytes:
    crate::shackle::clear_helper_error(instance);
    let result = resolve_fs_path(&path).and_then(|resolved| {
        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("failed to prepare `{}`: {err}", runtime_path_string(parent)))?;
        }
        std::fs::write(&resolved, bytes)
            .map_err(|err| format!("failed to write `{}`: {err}", runtime_path_string(&resolved)))
    });
    match result {
        Ok(()) => Ok(binding_bool(true)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_bool(false))
        }
    }

shackle fn process_fs_stream_open_read_impl(read path: Str) -> arcana_winapi.process_handles.FileStream = helpers.process.fs_stream_open_read:
    crate::shackle::clear_helper_error(instance);
    let result = resolve_fs_path(&path).and_then(|resolved| {
        let file = std::fs::File::open(&resolved)
            .map_err(|err| format!("failed to open `{}` for reading: {err}", runtime_path_string(&resolved)))?;
        insert_file_stream(instance, &resolved, file, true, false)
    });
    match result {
        Ok(value) => Ok(binding_int(value as i64)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_int(0))
        }
    }

shackle fn process_fs_stream_open_write_impl(read path: Str, read append: Bool) -> arcana_winapi.process_handles.FileStream = helpers.process.fs_stream_open_write:
    crate::shackle::clear_helper_error(instance);
    let result = resolve_fs_path(&path).and_then(|resolved| {
        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("failed to prepare `{}`: {err}", runtime_path_string(parent)))?;
        }
        let mut options = std::fs::OpenOptions::new();
        options.create(true).write(true);
        if append {
            options.append(true);
        } else {
            options.truncate(true);
        }
        let file = options.open(&resolved)
            .map_err(|err| format!("failed to open `{}` for writing: {err}", runtime_path_string(&resolved)))?;
        insert_file_stream(instance, &resolved, file, false, true)
    });
    match result {
        Ok(value) => Ok(binding_int(value as i64)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_int(0))
        }
    }

shackle fn process_fs_stream_read_impl(edit stream: arcana_winapi.process_handles.FileStream, read max_bytes: Int) -> Bytes = helpers.process.fs_stream_read:
    crate::shackle::clear_helper_error(instance);
    let result = (|| {
        if max_bytes < 0 {
            return Err("fs_stream_read max_bytes must be non-negative".to_string());
        }
        let stream = file_stream_mut(instance, stream)?;
        if !stream.readable {
            return Err(format!("FileStream `{}` is not opened for reading", stream.path));
        }
        let mut buffer = vec![0u8; max_bytes as usize];
        let read = std::io::Read::read(&mut stream.file, &mut buffer)
            .map_err(|err| format!("failed to read from FileStream `{}`: {err}", stream.path))?;
        buffer.truncate(read);
        Ok(buffer)
    })();
    match result {
        Ok(value) => Ok(binding_owned_bytes(value)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_owned_bytes(Vec::new()))
        }
    }

shackle fn process_fs_stream_write_impl(edit stream: arcana_winapi.process_handles.FileStream, read bytes: Bytes) -> Int = helpers.process.fs_stream_write:
    crate::shackle::clear_helper_error(instance);
    let result = (|| {
        let stream = file_stream_mut(instance, stream)?;
        if !stream.writable {
            return Err(format!("FileStream `{}` is not opened for writing", stream.path));
        }
        std::io::Write::write_all(&mut stream.file, &bytes)
            .map_err(|err| format!("failed to write to FileStream `{}`: {err}", stream.path))?;
        Ok(bytes.len() as i64)
    })();
    match result {
        Ok(value) => Ok(binding_int(value)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_int(0))
        }
    }

shackle fn process_fs_stream_eof_impl(read stream: arcana_winapi.process_handles.FileStream) -> Bool = helpers.process.fs_stream_eof:
    crate::shackle::clear_helper_error(instance);
    let result = (|| {
        let stream = file_stream_mut(instance, stream)?;
        if !stream.readable {
            return Err(format!("FileStream `{}` is not opened for reading", stream.path));
        }
        let position = std::io::Seek::stream_position(&mut stream.file)
            .map_err(|err| format!("failed to inspect FileStream `{}`: {err}", stream.path))?;
        let len = stream.file.metadata()
            .map_err(|err| format!("failed to stat FileStream `{}`: {err}", stream.path))?
            .len();
        Ok(position >= len)
    })();
    match result {
        Ok(value) => Ok(binding_bool(value)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_bool(false))
        }
    }

shackle fn process_fs_stream_close_impl(take stream: arcana_winapi.process_handles.FileStream) -> Bool = helpers.process.fs_stream_close:
    crate::shackle::clear_helper_error(instance);
    let result = (|| {
        let state = crate::shackle::package_state_data_mut(instance)?;
        let Some(mut stream) = state.file_streams.remove(&stream) else {
            return Err(format!("invalid FileStream handle `{stream}`"));
        };
        if stream.writable {
            std::io::Write::flush(&mut stream.file)
                .map_err(|err| format!("failed to flush FileStream `{}` during close: {err}", stream.path))?;
        }
        Ok(())
    })();
    match result {
        Ok(()) => Ok(binding_bool(true)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_bool(false))
        }
    }

shackle fn process_fs_list_dir_impl(read path: Str) -> Bytes = helpers.process.fs_list_dir:
    crate::shackle::clear_helper_error(instance);
    let result = resolve_fs_path(&path).and_then(|resolved| {
        let mut entries = std::fs::read_dir(&resolved)
            .map_err(|err| format!("failed to list `{}`: {err}", runtime_path_string(&resolved)))?
            .map(|entry| {
                entry
                    .map(|entry| runtime_path_string(&normalize_lexical_path(&entry.path())))
                    .map_err(|err| {
                        format!(
                            "failed to read directory entry in `{}`: {err}",
                            runtime_path_string(&resolved)
                        )
                    })
            })
            .collect::<Result<Vec<_>, String>>()?;
        entries.sort();
        encode_string_list_payload(&entries)
    });
    match result {
        Ok(value) => Ok(binding_owned_bytes(value)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_owned_bytes(Vec::new()))
        }
    }

shackle fn process_fs_mkdir_all_impl(read path: Str) -> Bool = helpers.process.fs_mkdir_all:
    crate::shackle::clear_helper_error(instance);
    let result = resolve_fs_path(&path).and_then(|resolved| {
        std::fs::create_dir_all(&resolved)
            .map_err(|err| format!("failed to create `{}`: {err}", runtime_path_string(&resolved)))
    });
    match result {
        Ok(()) => Ok(binding_bool(true)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_bool(false))
        }
    }

shackle fn process_fs_create_dir_impl(read path: Str) -> Bool = helpers.process.fs_create_dir:
    crate::shackle::clear_helper_error(instance);
    let result = resolve_fs_path(&path).and_then(|resolved| {
        std::fs::create_dir(&resolved)
            .map_err(|err| format!("failed to create directory `{}`: {err}", runtime_path_string(&resolved)))
    });
    match result {
        Ok(()) => Ok(binding_bool(true)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_bool(false))
        }
    }

shackle fn process_fs_remove_file_impl(read path: Str) -> Bool = helpers.process.fs_remove_file:
    crate::shackle::clear_helper_error(instance);
    let result = resolve_fs_path(&path).and_then(|resolved| {
        std::fs::remove_file(&resolved)
            .map_err(|err| format!("failed to remove file `{}`: {err}", runtime_path_string(&resolved)))
    });
    match result {
        Ok(()) => Ok(binding_bool(true)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_bool(false))
        }
    }

shackle fn process_fs_remove_dir_impl(read path: Str) -> Bool = helpers.process.fs_remove_dir:
    crate::shackle::clear_helper_error(instance);
    let result = resolve_fs_path(&path).and_then(|resolved| {
        std::fs::remove_dir(&resolved)
            .map_err(|err| format!("failed to remove directory `{}`: {err}", runtime_path_string(&resolved)))
    });
    match result {
        Ok(()) => Ok(binding_bool(true)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_bool(false))
        }
    }

shackle fn process_fs_remove_dir_all_impl(read path: Str) -> Bool = helpers.process.fs_remove_dir_all:
    crate::shackle::clear_helper_error(instance);
    let result = resolve_fs_path(&path).and_then(|resolved| {
        std::fs::remove_dir_all(&resolved)
            .map_err(|err| format!("failed to remove directory tree `{}`: {err}", runtime_path_string(&resolved)))
    });
    match result {
        Ok(()) => Ok(binding_bool(true)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_bool(false))
        }
    }

shackle fn process_fs_copy_file_impl(read from: Str, read to: Str) -> Bool = helpers.process.fs_copy_file:
    crate::shackle::clear_helper_error(instance);
    let result = resolve_fs_path(&from).and_then(|from_resolved| {
        let to_resolved = resolve_fs_path(&to)?;
        if let Some(parent) = to_resolved.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("failed to prepare `{}`: {err}", runtime_path_string(parent)))?;
        }
        std::fs::copy(&from_resolved, &to_resolved).map_err(|err| {
            format!(
                "failed to copy `{}` to `{}`: {err}",
                runtime_path_string(&from_resolved),
                runtime_path_string(&to_resolved)
            )
        })?;
        Ok(())
    });
    match result {
        Ok(()) => Ok(binding_bool(true)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_bool(false))
        }
    }

shackle fn process_fs_rename_impl(read from: Str, read to: Str) -> Bool = helpers.process.fs_rename:
    crate::shackle::clear_helper_error(instance);
    let result = resolve_fs_path(&from).and_then(|from_resolved| {
        let to_resolved = resolve_fs_path(&to)?;
        if let Some(parent) = to_resolved.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("failed to prepare `{}`: {err}", runtime_path_string(parent)))?;
        }
        std::fs::rename(&from_resolved, &to_resolved).map_err(|err| {
            format!(
                "failed to rename `{}` to `{}`: {err}",
                runtime_path_string(&from_resolved),
                runtime_path_string(&to_resolved)
            )
        })?;
        Ok(())
    });
    match result {
        Ok(()) => Ok(binding_bool(true)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_bool(false))
        }
    }

shackle fn process_fs_file_size_impl(read path: Str) -> Int = helpers.process.fs_file_size:
    crate::shackle::clear_helper_error(instance);
    let result = resolve_fs_path(&path).and_then(|resolved| {
        let len = std::fs::metadata(&resolved)
            .map_err(|err| format!("failed to stat `{}`: {err}", runtime_path_string(&resolved)))?
            .len();
        i64::try_from(len).map_err(|_| format!("file size for `{path}` does not fit in i64"))
    });
    match result {
        Ok(value) => Ok(binding_int(value)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_int(0))
        }
    }

shackle fn process_fs_modified_unix_ms_impl(read path: Str) -> Int = helpers.process.fs_modified_unix_ms:
    crate::shackle::clear_helper_error(instance);
    let result = resolve_fs_path(&path).and_then(|resolved| {
        let modified = std::fs::metadata(&resolved)
            .map_err(|err| format!("failed to stat `{}`: {err}", runtime_path_string(&resolved)))?
            .modified()
            .map_err(|err| format!("failed to read modified time for `{}`: {err}", runtime_path_string(&resolved)))?;
        let duration = modified
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|err| format!("modified time for `{}` predates unix epoch: {err}", runtime_path_string(&resolved)))?;
        i64::try_from(duration.as_millis()).map_err(|_| {
            format!(
                "modified time for `{}` does not fit in i64 milliseconds",
                runtime_path_string(&resolved)
            )
        })
    });
    match result {
        Ok(value) => Ok(binding_int(value)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_int(0))
        }
    }

shackle fn process_exec_status_impl(read program: Str, read args: Bytes) -> Int = helpers.process.process_exec_status:
    crate::shackle::clear_helper_error(instance);
    let result = decode_string_list_payload(&args).and_then(|argv| {
        let status = std::process::Command::new(&program)
            .args(&argv)
            .status()
            .map_err(|err| format!("failed to run process `{program}`: {err}"))?;
        Ok(i64::from(status.code().unwrap_or(-1)))
    });
    match result {
        Ok(value) => Ok(binding_int(value)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_int(0))
        }
    }

shackle fn process_exec_capture_impl(read program: Str, read args: Bytes) -> Bytes = helpers.process.process_exec_capture:
    crate::shackle::clear_helper_error(instance);
    let result = decode_string_list_payload(&args).and_then(|argv| {
        let output = std::process::Command::new(&program)
            .args(&argv)
            .output()
            .map_err(|err| format!("failed to run process `{program}`: {err}"))?;
        let status = output.status.code().unwrap_or(-1);
        let stdout_utf8 = std::str::from_utf8(&output.stdout).is_ok();
        let stderr_utf8 = std::str::from_utf8(&output.stderr).is_ok();
        encode_exec_capture_payload(status, output.stdout, output.stderr, stdout_utf8, stderr_utf8)
    });
    match result {
        Ok(value) => Ok(binding_owned_bytes(value)),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            Ok(binding_owned_bytes(Vec::new()))
        }
    }

