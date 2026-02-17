use super::transform::{apply_image_transform, flip_vertical};
use super::*;

impl WaylandCapture {
    fn refresh_outputs(&mut self) -> Result<()> {
        self.globals.outputs.clear();
        self.globals.output_info.clear();
        self.globals.output_xdg_map.clear();

        let mut event_queue = self._connection.new_event_queue();
        let qh = event_queue.handle();

        let _registry = self._connection.display().get_registry(&qh, ());

        event_queue.roundtrip(self).map_err(|e| {
            Error::WaylandConnection(format!("Failed to refresh Wayland globals: {}", e))
        })?;
        if self.globals.output_info.is_empty() {
            return Err(Error::NoOutputs);
        }

        for _ in 0..2 {
            event_queue.roundtrip(self).map_err(|e| {
                Error::WaylandConnection(format!("Failed to process output events: {}", e))
            })?;
        }

        if self.globals.xdg_output_manager.is_none() {
            for info in self.globals.output_info.values_mut() {
                if !info.logical_scale_known {
                    guess_output_logical_geometry(info);
                }
            }
        }

        Ok(())
    }

    fn collect_outputs_snapshot(&self) -> Vec<(WlOutput, OutputInfo)> {
        self.globals
            .outputs
            .iter()
            .filter_map(|output| {
                let id = output.id().protocol_id();
                self.globals
                    .output_info
                    .get(&id)
                    .cloned()
                    .map(|info| (output.clone(), info))
            })
            .collect()
    }

    fn capture_region_for_output(
        &mut self,
        output: &WlOutput,
        region: Box,
        overlay_cursor: bool,
    ) -> Result<CaptureResult> {
        if region.width() <= 0 || region.height() <= 0 {
            return Err(Error::InvalidRegion(
                "Capture region must have positive width and height".to_string(),
            ));
        }
        if region.x() < 0 || region.y() < 0 {
            return Err(Error::InvalidRegion(
                "Capture region origin must be non-negative".to_string(),
            ));
        }

        let screencopy_manager =
            self.globals
                .screencopy_manager
                .as_ref()
                .ok_or(Error::UnsupportedProtocol(
                    "zwlr_screencopy_manager_v1 not available".to_string(),
                ))?;
        let mut event_queue = self._connection.new_event_queue();
        let qh = event_queue.handle();
        let frame_state = Arc::new(Mutex::new(FrameState {
            buffer: None,
            width: 0,
            height: 0,
            format: None,
            ready: false,
            flags: 0,
        }));
        let frame = screencopy_manager.capture_output_region(
            if overlay_cursor { 1 } else { 0 },
            output,
            region.x(),
            region.y(),
            region.width(),
            region.height(),
            &qh,
            frame_state.clone(),
        );

        let mut attempts = 0;
        loop {
            {
                let state = lock_frame_state(&frame_state)?;
                if state.buffer.is_some() || state.ready {
                    if state.ready && state.buffer.is_none() {
                        return Err(Error::FrameCapture(
                            "Frame is ready but buffer was not received".to_string(),
                        ));
                    }
                    break;
                }
            }
            if attempts >= MAX_ATTEMPTS {
                return Err(Error::FrameCapture(
                    "Timeout waiting for frame buffer".to_string(),
                ));
            }
            event_queue.blocking_dispatch(self).map_err(|e| {
                Error::FrameCapture(format!("Failed to dispatch frame events: {}", e))
            })?;
            attempts += 1;
        }

        let shm = self
            .globals
            .shm
            .as_ref()
            .ok_or_else(|| Error::UnsupportedProtocol("wl_shm not available".to_string()))?;

        let (width, height, stride, size, format) = {
            let state = lock_frame_state(&frame_state)?;
            if state.width == 0 || state.height == 0 {
                return Err(Error::CaptureFailed);
            }
            let width = state.width;
            let height = state.height;
            let stride = width * 4;
            let size = checked_buffer_size(width, height, 4, Some(stride))?;
            let format = state.format.unwrap_or(ShmFormat::Xrgb8888);
            (width, height, stride, size, format)
        };

        let mut tmp_file = tempfile::NamedTempFile::new().map_err(|e| {
            Error::BufferCreation(format!("failed to create temporary file: {}", e))
        })?;
        tmp_file.as_file_mut().set_len(size as u64).map_err(|e| {
            Error::BufferCreation(format!("failed to resize buffer to {} bytes: {}", size, e))
        })?;
        let mmap = unsafe {
            memmap2::MmapMut::map_mut(&tmp_file)
                .map_err(|e| Error::BufferCreation(format!("failed to memory-map buffer: {}", e)))?
        };

        let pool = shm.create_pool(
            unsafe { BorrowedFd::borrow_raw(tmp_file.as_file().as_raw_fd()) },
            size as i32,
            &qh,
            (),
        );
        let buffer = pool.create_buffer(
            0,
            width as i32,
            height as i32,
            stride as i32,
            format,
            &qh,
            (),
        );
        frame.copy(&buffer);

        let mut attempts = 0;
        loop {
            {
                let state = lock_frame_state(&frame_state)?;
                if state.ready {
                    if state.buffer.is_none() {
                        return Err(Error::FrameCapture(
                            "Frame is ready but buffer was not received".to_string(),
                        ));
                    }
                    break;
                }
            }
            if attempts >= MAX_ATTEMPTS {
                return Err(Error::FrameCapture(
                    "Timeout waiting for frame capture completion".to_string(),
                ));
            }
            event_queue.blocking_dispatch(self).map_err(|e| {
                Error::FrameCapture(format!("Failed to dispatch frame events: {}", e))
            })?;
            attempts += 1;
        }

        let mut buffer_data = mmap.to_vec();
        match format {
            ShmFormat::Xrgb8888 => {
                for chunk in buffer_data.chunks_exact_mut(4) {
                    let b = chunk[0];
                    let g = chunk[1];
                    let r = chunk[2];
                    chunk[0] = r;
                    chunk[1] = g;
                    chunk[2] = b;
                    chunk[3] = 255;
                }
            }
            ShmFormat::Argb8888 => {}
            _ => {}
        }

        let output_id = output.id().protocol_id();
        let mut final_data = buffer_data;
        let mut final_width = width;
        let mut final_height = height;

        if let Some(info) = self.globals.output_info.get(&output_id) {
            if !matches!(
                info.transform,
                wayland_client::protocol::wl_output::Transform::Normal
            ) {
                let (transformed_data, new_width, new_height) =
                    apply_image_transform(&final_data, final_width, final_height, info.transform);
                final_data = transformed_data;
                final_width = new_width;
                final_height = new_height;
            }
        }

        let flags = {
            let state = lock_frame_state(&frame_state)?;
            state.flags
        };

        if (flags & ZWLR_SCREENCOPY_FRAME_V1_FLAGS_Y_INVERT) != 0 {
            let (inverted_data, inv_width, inv_height) =
                flip_vertical(&final_data, final_width, final_height);
            final_data = inverted_data;
            final_width = inv_width;
            final_height = inv_height;
        }

        Ok(CaptureResult {
            data: final_data,
            width: final_width,
            height: final_height,
        })
    }

    fn composite_region(
        &mut self,
        region: Box,
        outputs: &[(WlOutput, OutputInfo)],
        overlay_cursor: bool,
    ) -> Result<CaptureResult> {
        if region.width() <= 0 || region.height() <= 0 {
            return Err(Error::InvalidRegion(
                "Capture region must have positive width and height".to_string(),
            ));
        }

        let dest_width_u32 = u32::try_from(region.width()).map_err(|_| {
            Error::InvalidRegion("Capture region width exceeds supported range".to_string())
        })?;
        let dest_height_u32 = u32::try_from(region.height()).map_err(|_| {
            Error::InvalidRegion("Capture region height exceeds supported range".to_string())
        })?;
        let dest_bytes = checked_buffer_size(dest_width_u32, dest_height_u32, 4, None)?;
        let dest_width = dest_width_u32 as usize;
        let dest_height = dest_height_u32 as usize;
        let mut dest = vec![0u8; dest_bytes];
        let mut any_capture = false;

        for (output, info) in outputs {
            let output_box = Box::new(
                info.logical_x,
                info.logical_y,
                info.logical_width,
                info.logical_height,
            );
            if let Some(intersection) = output_box.intersection(&region) {
                if intersection.width() <= 0 || intersection.height() <= 0 {
                    continue;
                }

                let scale = if info.logical_scale_known && info.logical_scale.is_finite() {
                    info.logical_scale
                } else {
                    info.scale as f64
                };

                // Convert logical coords to physical pixels. For fractional scale, we need to
                // be careful with rounding so we don't miss edge pixels.
                let local_x = (intersection.x() - info.logical_x) as f64;
                let local_y = (intersection.y() - info.logical_y) as f64;
                let local_w = intersection.width() as f64;
                let local_h = intersection.height() as f64;

                let x0 = (local_x * scale).floor() as i32;
                let y0 = (local_y * scale).floor() as i32;
                let x1 = ((local_x + local_w) * scale).ceil() as i32;
                let y1 = ((local_y + local_h) * scale).ceil() as i32;

                // Clamp to output boundaries in physical pixels.
                let x0 = x0.clamp(0, info.width);
                let y0 = y0.clamp(0, info.height);
                let x1 = x1.clamp(0, info.width);
                let y1 = y1.clamp(0, info.height);

                if x1 <= x0 || y1 <= y0 {
                    continue;
                }

                let physical_local_region = Box::new(x0, y0, x1 - x0, y1 - y0);
                let mut capture =
                    self.capture_region_for_output(output, physical_local_region, overlay_cursor)?;

                if scale != 1.0 {
                    capture = self.scale_image_data(capture, 1.0 / scale)?;
                }

                let offset_x = (intersection.x() - region.x()) as usize;
                let offset_y = (intersection.y() - region.y()) as usize;

                blit_capture(
                    &mut dest,
                    dest_width,
                    dest_height,
                    &capture,
                    offset_x,
                    offset_y,
                );
                any_capture = true;
            }
        }

        if !any_capture {
            return Err(Error::InvalidRegion(
                "Capture region does not intersect with any output".to_string(),
            ));
        }

        Ok(CaptureResult {
            data: dest,
            width: region.width() as u32,
            height: region.height() as u32,
        })
    }

    pub fn get_outputs(&mut self) -> Result<Vec<Output>> {
        self.refresh_outputs()?;
        let snapshot = self.collect_outputs_snapshot();
        let outputs = snapshot
            .into_iter()
            .map(|(_, info)| {
                let (x, y, width, height) = if info.logical_scale_known {
                    (
                        info.logical_x,
                        info.logical_y,
                        info.logical_width,
                        info.logical_height,
                    )
                } else {
                    (info.x, info.y, info.width, info.height)
                };

                Output {
                    name: info.name.clone(),
                    geometry: Box::new(x, y, width, height),
                    scale: info.scale,
                    description: info.description.clone(),
                }
            })
            .collect::<Vec<_>>();
        if outputs.is_empty() {
            return Err(Error::NoOutputs);
        }
        Ok(outputs)
    }

    pub fn capture_all(&mut self) -> Result<CaptureResult> {
        self.refresh_outputs()?;
        let snapshot = self.collect_outputs_snapshot();
        if snapshot.is_empty() {
            return Err(Error::NoOutputs);
        }

        let (_, first_info) = &snapshot[0];
        let mut min_x = first_info.logical_x;
        let mut min_y = first_info.logical_y;
        let mut max_x = first_info.logical_x + first_info.logical_width;
        let mut max_y = first_info.logical_y + first_info.logical_height;

        for (_, info) in &snapshot {
            min_x = min_x.min(info.logical_x);
            min_y = min_y.min(info.logical_y);
            max_x = max_x.max(info.logical_x + info.logical_width);
            max_y = max_y.max(info.logical_y + info.logical_height);
        }

        let region = Box::new(min_x, min_y, max_x - min_x, max_y - min_y);
        self.composite_region(region, &snapshot, false)
    }

    pub fn capture_all_with_scale(&mut self, scale: f64) -> Result<CaptureResult> {
        let result = self.capture_all()?;
        self.scale_image_data(result, scale)
    }

    pub fn capture_output(&mut self, output_name: &str) -> Result<CaptureResult> {
        self.refresh_outputs()?;
        let snapshot = self.collect_outputs_snapshot();
        let (output_handle, info) = snapshot
            .into_iter()
            .find(|(_, info)| info.name == output_name)
            .ok_or_else(|| Error::OutputNotFound(output_name.to_string()))?;

        let local_region = Box::new(0, 0, info.width, info.height);
        self.capture_region_for_output(&output_handle, local_region, false)
    }

    pub fn capture_output_with_scale(
        &mut self,
        output_name: &str,
        scale: f64,
    ) -> Result<CaptureResult> {
        let result = self.capture_output(output_name)?;
        self.scale_image_data(result, scale)
    }

    pub fn capture_region(&mut self, region: Box) -> Result<CaptureResult> {
        self.refresh_outputs()?;
        let snapshot = self.collect_outputs_snapshot();
        self.composite_region(region, &snapshot, false)
    }

    pub fn capture_region_with_scale(&mut self, region: Box, scale: f64) -> Result<CaptureResult> {
        let result = self.capture_region(region)?;
        self.scale_image_data(result, scale)
    }

    pub fn capture_outputs(
        &mut self,
        parameters: Vec<CaptureParameters>,
    ) -> Result<MultiOutputCaptureResult> {
        if self.globals.outputs.is_empty() {
            return Err(Error::NoOutputs);
        }

        let screencopy_manager =
            self.globals
                .screencopy_manager
                .as_ref()
                .ok_or(Error::UnsupportedProtocol(
                    "zwlr_screencopy_manager_v1 not available".to_string(),
                ))?;
        let mut event_queue = self._connection.new_event_queue();
        let qh = event_queue.handle();
        let mut frame_states: HashMap<String, Arc<Mutex<FrameState>>> = HashMap::new();
        let mut frames: HashMap<String, ZwlrScreencopyFrameV1> = HashMap::new();

        for param in &parameters {
            let (output_id, output_info) = self
                .globals
                .output_info
                .iter()
                .find(|(_, info)| info.name == param.output_name())
                .ok_or_else(|| Error::OutputNotFound(param.output_name().to_string()))?;

            let output = self
                .globals
                .outputs
                .iter()
                .find(|o| o.id().protocol_id() == *output_id)
                .ok_or_else(|| Error::OutputNotFound(param.output_name().to_string()))?;
            let region = if let Some(region) = param.region_ref() {
                let output_right = output_info.x + output_info.width;
                let output_bottom = output_info.y + output_info.height;
                if region.x() < output_info.x
                    || region.y() < output_info.y
                    || region.x() + region.width() > output_right
                    || region.y() + region.height() > output_bottom
                {
                    return Err(Error::InvalidRegion(
                        "Capture region extends outside output boundaries".to_string(),
                    ));
                }
                *region
            } else {
                Box::new(
                    output_info.x,
                    output_info.y,
                    output_info.width,
                    output_info.height,
                )
            };
            let frame_state = Arc::new(Mutex::new(FrameState {
                buffer: None,
                width: 0,
                height: 0,
                format: None,
                ready: false,
                flags: 0,
            }));
            let frame = screencopy_manager.capture_output_region(
                if param.overlay_cursor_enabled() { 1 } else { 0 },
                output,
                region.x(),
                region.y(),
                region.width(),
                region.height(),
                &qh,
                frame_state.clone(),
            );
            frame_states.insert(param.output_name().to_string(), frame_state);
            frames.insert(param.output_name().to_string(), frame);
        }
        let mut attempts = 0;
        let mut completed_frames = 0;
        let total_frames = parameters.len();
        while completed_frames < total_frames && attempts < MAX_ATTEMPTS {
            completed_frames = frame_states
                .iter()
                .filter(|(_, state)| {
                    state
                        .lock()
                        .ok()
                        .is_some_and(|s| s.buffer.is_some() || s.ready)
                })
                .count();
            if completed_frames >= total_frames {
                break;
            }
            event_queue.blocking_dispatch(self).map_err(|e| {
                Error::FrameCapture(format!("Failed to dispatch frame events: {}", e))
            })?;
            attempts += 1;
        }
        if attempts >= MAX_ATTEMPTS {
            return Err(Error::FrameCapture(
                "Timeout waiting for frame buffers".to_string(),
            ));
        }
        for frame_state in frame_states.values() {
            let state = lock_frame_state(frame_state)?;
            if state.buffer.is_none() {
                return Err(Error::CaptureFailed);
            }
        }
        let mut buffers: HashMap<String, (tempfile::NamedTempFile, memmap2::MmapMut)> =
            HashMap::new();
        for (output_name, frame_state) in &frame_states {
            let (width, height, stride, size) = {
                let state = lock_frame_state(frame_state)?;
                let width = state.width;
                let height = state.height;
                let stride = width * 4;
                let size = checked_buffer_size(width, height, 4, Some(stride))?;
                (width, height, stride, size)
            };
            let mut tmp_file = tempfile::NamedTempFile::new().map_err(|e| {
                Error::BufferCreation(format!(
                    "failed to create temporary file for output '{}': {}",
                    output_name, e
                ))
            })?;
            tmp_file.as_file_mut().set_len(size as u64).map_err(|e| {
                Error::BufferCreation(format!(
                    "failed to resize buffer for output '{}' to {} bytes: {}",
                    output_name, size, e
                ))
            })?;
            let mmap = unsafe {
                memmap2::MmapMut::map_mut(&tmp_file).map_err(|e| {
                    Error::BufferCreation(format!(
                        "failed to memory-map buffer for output '{}': {}",
                        output_name, e
                    ))
                })?
            };
            let shm = self.globals.shm.as_ref().ok_or(Error::UnsupportedProtocol(
                "wl_shm not available".to_string(),
            ))?;
            {
                let format = {
                    let state = lock_frame_state(frame_state)?;
                    state.format.unwrap_or(ShmFormat::Xrgb8888)
                };
                let pool = shm.create_pool(
                    unsafe { BorrowedFd::borrow_raw(tmp_file.as_file().as_raw_fd()) },
                    size as i32,
                    &qh,
                    (),
                );
                let buffer = pool.create_buffer(
                    0,
                    width as i32,
                    height as i32,
                    stride as i32,
                    format,
                    &qh,
                    (),
                );
                if let Some(frame) = frames.get(output_name) {
                    frame.copy(&buffer);
                }
            }
            buffers.insert(output_name.clone(), (tmp_file, mmap));
        }
        let mut attempts = 0;
        let mut completed_frames = 0;
        while completed_frames < total_frames && attempts < MAX_ATTEMPTS {
            completed_frames = frame_states
                .iter()
                .filter(|(_, state)| state.lock().ok().is_some_and(|s| s.ready))
                .count();
            if completed_frames >= total_frames {
                break;
            }
            event_queue.blocking_dispatch(self).map_err(|e| {
                Error::FrameCapture(format!("Failed to dispatch frame events: {}", e))
            })?;
            attempts += 1;
        }
        if attempts >= MAX_ATTEMPTS {
            return Err(Error::FrameCapture(
                "Timeout waiting for frame capture completion".to_string(),
            ));
        }
        for frame_state in frame_states.values() {
            let state = lock_frame_state(frame_state)?;
            if state.ready && state.buffer.is_none() {
                return Err(Error::FrameCapture(
                    "Frame is ready but buffer was not received".to_string(),
                ));
            }
        }
        let mut results: HashMap<String, CaptureResult> = HashMap::new();
        for (output_name, (_tmp_file, mmap)) in buffers {
            let frame_state = &frame_states[&output_name];
            let (width, height) = {
                let state = lock_frame_state(frame_state)?;
                (state.width, state.height)
            };
            let mut buffer_data = mmap.to_vec();
            if let Some(format) = {
                let state = lock_frame_state(frame_state)?;
                state.format
            } {
                match format {
                    ShmFormat::Xrgb8888 => {
                        for chunk in buffer_data.chunks_exact_mut(4) {
                            let b = chunk[0];
                            let g = chunk[1];
                            let r = chunk[2];
                            chunk[0] = r;
                            chunk[1] = g;
                            chunk[2] = b;
                            chunk[3] = 255;
                        }
                    }
                    ShmFormat::Argb8888 => {}
                    _ => {}
                }
            }
            results.insert(
                output_name,
                CaptureResult {
                    data: buffer_data,
                    width,
                    height,
                },
            );
        }
        Ok(MultiOutputCaptureResult::new(results))
    }

    pub fn capture_outputs_with_scale(
        &mut self,
        parameters: Vec<CaptureParameters>,
        default_scale: f64,
    ) -> Result<MultiOutputCaptureResult> {
        let result = self.capture_outputs(parameters)?;
        let mut scaled_results = std::collections::HashMap::new();

        for (output_name, capture_result) in result.into_outputs() {
            let scale = default_scale;
            let scaled_result = self.scale_image_data(capture_result, scale)?;
            scaled_results.insert(output_name, scaled_result);
        }

        Ok(MultiOutputCaptureResult::new(scaled_results))
    }
}
