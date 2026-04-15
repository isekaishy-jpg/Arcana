shackle flags WinapiAudioInternals:
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct WinapiAudioDeviceState {
        pub(crate) sample_rate_hz: i64,
        pub(crate) channels: i64,
        pub(crate) gain_milli: i64,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct WinapiAudioBufferState {
        pub(crate) path: String,
        pub(crate) frames: i64,
        pub(crate) channels: i64,
        pub(crate) sample_rate_hz: i64,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct WinapiAudioPlaybackState {
        pub(crate) device: u64,
        pub(crate) buffer: u64,
        pub(crate) paused: bool,
        pub(crate) finished: bool,
        pub(crate) gain_milli: i64,
        pub(crate) looping: bool,
        pub(crate) position_frames: i64,
    }

    fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, String> {
        let slice = bytes
            .get(offset..offset + 2)
            .ok_or_else(|| format!("WAV field at offset {offset} is truncated"))?;
        Ok(u16::from_le_bytes([slice[0], slice[1]]))
    }

    fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, String> {
        let slice = bytes
            .get(offset..offset + 4)
            .ok_or_else(|| format!("WAV field at offset {offset} is truncated"))?;
        Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    pub(crate) fn parse_wav_info(path: &str) -> Result<WinapiAudioBufferState, String> {
        let bytes = std::fs::read(path)
            .map_err(|err| format!("failed to read `{path}`: {err}"))?;
        if bytes.len() < 12 {
            return Err(format!("`{path}` is too small to be a WAV file"));
        }
        if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(format!("`{path}` is not a RIFF/WAVE file"));
        }
        let mut offset = 12usize;
        let mut channels = None;
        let mut sample_rate_hz = None;
        let mut block_align = None;
        let mut data_bytes = None;
        while offset + 8 <= bytes.len() {
            let chunk_id = &bytes[offset..offset + 4];
            let chunk_size = read_u32_le(&bytes, offset + 4)? as usize;
            let data_start = offset + 8;
            let data_end = data_start
                .checked_add(chunk_size)
                .ok_or_else(|| format!("WAV chunk in `{path}` overflowed"))?;
            if data_end > bytes.len() {
                return Err(format!("WAV chunk in `{path}` extends past end of file"));
            }
            if chunk_id == b"fmt " {
                if chunk_size < 16 {
                    return Err(format!("`{path}` has a truncated fmt chunk"));
                }
                channels = Some(read_u16_le(&bytes, data_start + 2)? as i64);
                sample_rate_hz = Some(read_u32_le(&bytes, data_start + 4)? as i64);
                block_align = Some(read_u16_le(&bytes, data_start + 12)? as i64);
            } else if chunk_id == b"data" {
                data_bytes = Some(chunk_size as i64);
            }
            offset = data_end + (chunk_size & 1);
        }
        let channels = channels.ok_or_else(|| format!("`{path}` is missing a fmt chunk"))?;
        let sample_rate_hz =
            sample_rate_hz.ok_or_else(|| format!("`{path}` is missing a sample rate"))?;
        let block_align =
            block_align.ok_or_else(|| format!("`{path}` is missing block alignment"))?;
        let data_bytes = data_bytes.ok_or_else(|| format!("`{path}` is missing a data chunk"))?;
        if channels <= 0 || sample_rate_hz <= 0 || block_align <= 0 {
            return Err(format!("`{path}` has an invalid WAV format"));
        }
        if data_bytes % block_align != 0 {
            return Err(format!("`{path}` has unaligned audio data"));
        }
        Ok(WinapiAudioBufferState {
            path: path.replace('\\', "/"),
            frames: data_bytes / block_align,
            channels,
            sample_rate_hz,
        })
    }

    pub(crate) fn ensure_audio_buffer_matches_device(
        device_sample_rate_hz: i64,
        device_channels: i64,
        buffer_sample_rate_hz: i64,
        buffer_channels: i64,
    ) -> Result<(), String> {
        if device_sample_rate_hz == buffer_sample_rate_hz && device_channels == buffer_channels {
            return Ok(());
        }
        Err(format!(
            "AudioBuffer format {buffer_sample_rate_hz} Hz / {buffer_channels} channel(s) does not match AudioDevice format {device_sample_rate_hz} Hz / {device_channels} channel(s)"
        ))
    }

    pub(crate) fn audio_device_ref(
        instance: &crate::BindingInstance,
        handle: u64,
    ) -> Result<&WinapiAudioDeviceState, String> {
        if handle == 0 {
            return Err("AudioDevice handle must not be 0".to_string());
        }
        crate::shackle::package_state_data_ref(instance)?
            .audio_devices
            .get(&handle)
            .ok_or_else(|| format!("invalid AudioDevice handle `{handle}`"))
    }

    pub(crate) fn audio_device_mut(
        instance: &mut crate::BindingInstance,
        handle: u64,
    ) -> Result<&mut WinapiAudioDeviceState, String> {
        if handle == 0 {
            return Err("AudioDevice handle must not be 0".to_string());
        }
        crate::shackle::package_state_data_mut(instance)?
            .audio_devices
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid AudioDevice handle `{handle}`"))
    }

    pub(crate) fn audio_buffer_ref(
        instance: &crate::BindingInstance,
        handle: u64,
    ) -> Result<&WinapiAudioBufferState, String> {
        if handle == 0 {
            return Err("AudioBuffer handle must not be 0".to_string());
        }
        crate::shackle::package_state_data_ref(instance)?
            .audio_buffers
            .get(&handle)
            .ok_or_else(|| format!("invalid AudioBuffer handle `{handle}`"))
    }

    pub(crate) fn audio_playback_ref(
        instance: &crate::BindingInstance,
        handle: u64,
    ) -> Result<&WinapiAudioPlaybackState, String> {
        if handle == 0 {
            return Err("AudioPlayback handle must not be 0".to_string());
        }
        crate::shackle::package_state_data_ref(instance)?
            .audio_playbacks
            .get(&handle)
            .ok_or_else(|| format!("invalid AudioPlayback handle `{handle}`"))
    }

    pub(crate) fn audio_playback_mut(
        instance: &mut crate::BindingInstance,
        handle: u64,
    ) -> Result<&mut WinapiAudioPlaybackState, String> {
        if handle == 0 {
            return Err("AudioPlayback handle must not be 0".to_string());
        }
        crate::shackle::package_state_data_mut(instance)?
            .audio_playbacks
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid AudioPlayback handle `{handle}`"))
    }

shackle fn audio_take_last_error_impl() -> Str = helpers.audio.take_last_error:
    Ok(binding_owned_str(crate::shackle::take_helper_error(instance)))

shackle fn audio_default_output_impl() -> arcana_winapi.audio_handles.AudioDevice = helpers.audio.default_output:
    crate::shackle::clear_helper_error(instance);
    let state = crate::shackle::package_state_data_mut(instance)?;
    let handle = state.next_audio_device_handle;
    state.next_audio_device_handle += 1;
    state.audio_devices.insert(
        handle,
        WinapiAudioDeviceState {
            sample_rate_hz: 48_000,
            channels: 2,
            gain_milli: 1000,
        },
    );
    Ok(binding_opaque(handle))

shackle fn audio_output_close_impl(take device: arcana_winapi.audio_handles.AudioDevice) -> Bool = helpers.audio.output_close:
    crate::shackle::clear_helper_error(instance);
    let state = crate::shackle::package_state_data_mut(instance)?;
    if !state.audio_devices.contains_key(&device) {
        crate::shackle::set_helper_error(instance, format!("invalid AudioDevice handle `{device}`"));
        return Ok(binding_bool(false));
    }
    state.audio_playbacks.retain(|_, playback| playback.device != device);
    state.audio_devices.remove(&device);
    Ok(binding_bool(true))

shackle fn audio_output_sample_rate_hz_impl(read device: arcana_winapi.audio_handles.AudioDevice) -> Int = helpers.audio.output_sample_rate_hz:
    Ok(binding_int(audio_device_ref(instance, device)?.sample_rate_hz))

shackle fn audio_output_channels_impl(read device: arcana_winapi.audio_handles.AudioDevice) -> Int = helpers.audio.output_channels:
    Ok(binding_int(audio_device_ref(instance, device)?.channels))

shackle fn audio_buffer_load_wav_impl(read path: Str) -> arcana_winapi.audio_handles.AudioBuffer = helpers.audio.buffer_load_wav:
    crate::shackle::clear_helper_error(instance);
    let buffer = match parse_wav_info(&path) {
        Ok(buffer) => buffer,
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            return Ok(binding_opaque(0));
        }
    };
    let state = crate::shackle::package_state_data_mut(instance)?;
    let handle = state.next_audio_buffer_handle;
    state.next_audio_buffer_handle += 1;
    state.audio_buffers.insert(handle, buffer);
    Ok(binding_opaque(handle))

shackle fn audio_buffer_frames_impl(read buffer: arcana_winapi.audio_handles.AudioBuffer) -> Int = helpers.audio.buffer_frames:
    Ok(binding_int(audio_buffer_ref(instance, buffer)?.frames))

shackle fn audio_buffer_channels_impl(read buffer: arcana_winapi.audio_handles.AudioBuffer) -> Int = helpers.audio.buffer_channels:
    Ok(binding_int(audio_buffer_ref(instance, buffer)?.channels))

shackle fn audio_buffer_sample_rate_hz_impl(read buffer: arcana_winapi.audio_handles.AudioBuffer) -> Int = helpers.audio.buffer_sample_rate_hz:
    Ok(binding_int(audio_buffer_ref(instance, buffer)?.sample_rate_hz))

shackle fn audio_play_buffer_impl(edit device: arcana_winapi.audio_handles.AudioDevice, read buffer: arcana_winapi.audio_handles.AudioBuffer) -> arcana_winapi.audio_handles.AudioPlayback = helpers.audio.play_buffer:
    crate::shackle::clear_helper_error(instance);
    let device_state = match audio_device_ref(instance, device) {
        Ok(value) => value.clone(),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            return Ok(binding_opaque(0));
        }
    };
    let buffer_state = match audio_buffer_ref(instance, buffer) {
        Ok(value) => value.clone(),
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            return Ok(binding_opaque(0));
        }
    };
    if let Err(err) = ensure_audio_buffer_matches_device(
        device_state.sample_rate_hz,
        device_state.channels,
        buffer_state.sample_rate_hz,
        buffer_state.channels,
    ) {
        crate::shackle::set_helper_error(instance, err);
        return Ok(binding_opaque(0));
    }
    let state = crate::shackle::package_state_data_mut(instance)?;
    let handle = state.next_audio_playback_handle;
    state.next_audio_playback_handle += 1;
    state.audio_playbacks.insert(
        handle,
        WinapiAudioPlaybackState {
            device,
            buffer,
            paused: false,
            finished: false,
            gain_milli: device_state.gain_milli,
            looping: false,
            position_frames: 0,
        },
    );
    Ok(binding_opaque(handle))

shackle fn audio_output_set_gain_milli_impl(edit device: arcana_winapi.audio_handles.AudioDevice, read milli: Int) = helpers.audio.output_set_gain_milli:
    audio_device_mut(instance, device)?.gain_milli = milli;
    Ok(binding_unit())

shackle fn audio_playback_stop_impl(take playback: arcana_winapi.audio_handles.AudioPlayback) -> Bool = helpers.audio.playback_stop:
    crate::shackle::clear_helper_error(instance);
    let state = crate::shackle::package_state_data_mut(instance)?;
    if state.audio_playbacks.remove(&playback).is_none() {
        crate::shackle::set_helper_error(instance, format!("invalid AudioPlayback handle `{playback}`"));
        return Ok(binding_bool(false));
    }
    Ok(binding_bool(true))

shackle fn audio_playback_pause_impl(edit playback: arcana_winapi.audio_handles.AudioPlayback) = helpers.audio.playback_pause:
    audio_playback_mut(instance, playback)?.paused = true;
    Ok(binding_unit())

shackle fn audio_playback_resume_impl(edit playback: arcana_winapi.audio_handles.AudioPlayback) = helpers.audio.playback_resume:
    audio_playback_mut(instance, playback)?.paused = false;
    Ok(binding_unit())

shackle fn audio_playback_playing_impl(read playback: arcana_winapi.audio_handles.AudioPlayback) -> Bool = helpers.audio.playback_playing:
    let playback = audio_playback_ref(instance, playback)?;
    Ok(binding_bool(!playback.paused && !playback.finished))

shackle fn audio_playback_paused_impl(read playback: arcana_winapi.audio_handles.AudioPlayback) -> Bool = helpers.audio.playback_paused:
    Ok(binding_bool(audio_playback_ref(instance, playback)?.paused))

shackle fn audio_playback_finished_impl(read playback: arcana_winapi.audio_handles.AudioPlayback) -> Bool = helpers.audio.playback_finished:
    Ok(binding_bool(audio_playback_ref(instance, playback)?.finished))

shackle fn audio_playback_set_gain_milli_impl(edit playback: arcana_winapi.audio_handles.AudioPlayback, read milli: Int) = helpers.audio.playback_set_gain_milli:
    audio_playback_mut(instance, playback)?.gain_milli = milli;
    Ok(binding_unit())

shackle fn audio_playback_set_looping_impl(edit playback: arcana_winapi.audio_handles.AudioPlayback, read looping: Bool) = helpers.audio.playback_set_looping:
    audio_playback_mut(instance, playback)?.looping = looping;
    Ok(binding_unit())

shackle fn audio_playback_looping_impl(read playback: arcana_winapi.audio_handles.AudioPlayback) -> Bool = helpers.audio.playback_looping:
    Ok(binding_bool(audio_playback_ref(instance, playback)?.looping))

shackle fn audio_playback_position_frames_impl(read playback: arcana_winapi.audio_handles.AudioPlayback) -> Int = helpers.audio.playback_position_frames:
    Ok(binding_int(audio_playback_ref(instance, playback)?.position_frames))

