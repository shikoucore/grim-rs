pub(super) use crate::{
    Box, CaptureParameters, CaptureResult, Error, MultiOutputCaptureResult, Output, Result,
};
pub(super) use std::collections::HashMap;
pub(super) use std::os::fd::{AsRawFd, BorrowedFd};
pub(super) use std::sync::{Arc, Mutex};
pub(super) use wayland_client::{
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_output::WlOutput,
        wl_registry::WlRegistry,
        wl_shm::{Format as ShmFormat, WlShm},
        wl_shm_pool::WlShmPool,
    },
    Connection, Dispatch, Proxy, QueueHandle,
};
pub(super) use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1::ZxdgOutputV1,
};
pub(super) use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

mod capture;
mod scaling;
mod transform;
mod wayland_events;

pub(super) const ZWLR_SCREENCOPY_FRAME_V1_FLAGS_Y_INVERT: u32 = 1;
pub(super) const MAX_ATTEMPTS: usize = 100;

/// Global upper bound for pixel count used by `checked_buffer_size()`.
///
/// This limit is enforced for all image/buffer allocations that are derived
/// from width/height in this module, including:
/// - `capture_region_for_output()` (Wayland buffer allocation via stride×height)
/// - `capture_outputs()` (per-output buffer allocation)
/// - `composite_region()` (final composited image buffer)
/// - `scale_image_data()` and `scale_image_integer_fast()` (scaled image buffers)
/// - `ZwlrScreencopyFrameV1::Buffer` event handling (frame buffer placeholder)
///
/// The goal is to prevent integer overflows and avoid OOM from extreme sizes.
pub(super) const MAX_PIXELS: u64 = 134_217_728;

#[derive(Debug, Clone)]
pub(super) struct FrameState {
    buffer: Option<Vec<u8>>,
    width: u32,
    height: u32,
    format: Option<ShmFormat>,
    ready: bool,
    flags: u32,
}

/// Compute a safe buffer size in bytes for image-like data.
///
/// What it does:
/// - Validates `width × height` without overflow.
/// - Enforces the global `MAX_PIXELS` limit.
/// - Computes the byte size either as `width × height × bytes_per_pixel`
///   or as `row_stride_bytes × height` when a stride is provided.
///
/// Where it is used:
/// - All large allocations in this module (capture, composite, scaling, and
///   Wayland buffer handling) call this helper before allocating memory.
///
/// When to use it:
/// - Call this helper whenever you are about to allocate a buffer whose size
///   depends on `width × height` (and optionally row stride).
/// - Use it for any new code path that creates `Vec<u8>` for image data or
///   sizes a file/mmap based on image dimensions.
///
/// What it gives:
/// - A checked `usize` byte size that is safe to pass to `vec![0u8; size]`
///   or file/mmap sizing, avoiding OOM due to extreme dimensions.
pub(super) fn checked_buffer_size(
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
    row_stride_bytes: Option<u32>,
) -> Result<usize> {
    let pixels = (width as u64)
        .checked_mul(height as u64)
        .ok_or_else(|| Error::InvalidRegion("Image dimensions overflow".to_string()))?;

    if pixels > MAX_PIXELS {
        return Err(Error::InvalidRegion(format!(
            "Image exceeds maximum pixel limit ({})",
            MAX_PIXELS
        )));
    }

    let bytes = match row_stride_bytes {
        Some(stride) => (stride as u64)
            .checked_mul(height as u64)
            .ok_or_else(|| Error::BufferCreation("Buffer size overflow".to_string()))?,
        None => pixels
            .checked_mul(bytes_per_pixel as u64)
            .ok_or_else(|| Error::BufferCreation("Buffer size overflow".to_string()))?,
    };

    usize::try_from(bytes).map_err(|_| Error::BufferCreation("Buffer size overflow".to_string()))
}

/// Safely lock a FrameState mutex, converting poisoned mutex errors to Result.
///
/// This helper function provides proper error handling for mutex locks instead of panicking.
pub(super) fn lock_frame_state(
    frame_state: &Arc<Mutex<FrameState>>,
) -> Result<std::sync::MutexGuard<'_, FrameState>> {
    frame_state
        .lock()
        .map_err(|e| Error::FrameCapture(format!("Frame state mutex poisoned: {}", e)))
}

/// Guess logical geometry from physical geometry when xdg_output is not available.
pub(super) fn guess_output_logical_geometry(info: &mut OutputInfo) {
    info.logical_x = info.x;
    info.logical_y = info.y;
    info.logical_width = info.width / info.scale;
    info.logical_height = info.height / info.scale;

    transform::apply_output_transform(
        info.transform,
        &mut info.logical_width,
        &mut info.logical_height,
    );
    info.logical_scale_known = true;
    update_logical_scale(info);
}

/// Infer a (possibly fractional) logical scale.
///
/// Some compositors report a logical size that does not match the integer `wl_output.scale`.
/// To match grim's behavior, we derive the effective scale from the physical mode size and the
/// logical size (taking output transform into account).
pub(super) fn update_logical_scale(info: &mut OutputInfo) {
    if info.width <= 0 || info.height <= 0 || info.logical_width <= 0 || info.logical_height <= 0 {
        return;
    }

    // Match grim's behavior: infer a (possibly fractional) logical scale from the output's
    // physical mode size and the xdg-output logical size.
    let mut physical_width = info.width;
    let mut physical_height = info.height;
    transform::apply_output_transform(info.transform, &mut physical_width, &mut physical_height);

    info.logical_scale = (physical_width as f64) / (info.logical_width as f64);
}

pub(super) fn blit_capture(
    dest: &mut [u8],
    dest_width: usize,
    dest_height: usize,
    capture: &CaptureResult,
    offset_x: usize,
    offset_y: usize,
) {
    let src_width = capture.width as usize;
    let src_height = capture.height as usize;
    if src_width == 0 || src_height == 0 {
        return;
    }
    if offset_x >= dest_width || offset_y >= dest_height {
        return;
    }

    let copy_width = src_width.min(dest_width.saturating_sub(offset_x));
    let copy_height = src_height.min(dest_height.saturating_sub(offset_y));
    if copy_width == 0 || copy_height == 0 {
        return;
    }

    let dest_stride = dest_width * 4;
    let src_stride = src_width * 4;
    let row_bytes = copy_width * 4;

    for row in 0..copy_height {
        let dest_index = (offset_y + row) * dest_stride + offset_x * 4;
        let src_index = row * src_stride;
        dest[dest_index..dest_index + row_bytes]
            .copy_from_slice(&capture.data[src_index..src_index + row_bytes]);
    }
}

#[derive(Clone)]
pub(super) struct OutputInfo {
    name: String,
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    scale: i32,
    transform: wayland_client::protocol::wl_output::Transform,
    logical_x: i32,
    logical_y: i32,
    logical_width: i32,
    logical_height: i32,
    logical_scale_known: bool,
    logical_scale: f64,
    description: Option<String>,
}

pub(super) struct WaylandGlobals {
    compositor: Option<WlCompositor>,
    shm: Option<WlShm>,
    screencopy_manager: Option<ZwlrScreencopyManagerV1>,
    xdg_output_manager: Option<ZxdgOutputManagerV1>,
    outputs: Vec<WlOutput>,
    output_info: HashMap<u32, OutputInfo>,
    output_xdg_map: HashMap<u32, ZxdgOutputV1>,
}

pub struct WaylandCapture {
    _connection: Connection,
    globals: WaylandGlobals,
}

impl WaylandCapture {
    pub fn new() -> Result<Self> {
        let connection = Connection::connect_to_env().map_err(|e| {
            Error::WaylandConnection(format!("Failed to connect to Wayland: {}", e))
        })?;
        let globals = WaylandGlobals {
            compositor: None,
            shm: None,
            screencopy_manager: None,
            xdg_output_manager: None,
            outputs: Vec::new(),
            output_info: HashMap::new(),
            output_xdg_map: HashMap::new(),
        };
        let mut event_queue = connection.new_event_queue();
        let qh = event_queue.handle();
        let _registry = connection.display().get_registry(&qh, ());
        let mut instance = Self {
            _connection: connection,
            globals,
        };
        event_queue.roundtrip(&mut instance).map_err(|e| {
            Error::WaylandConnection(format!("Failed to initialize Wayland globals: {}", e))
        })?;
        if instance.globals.screencopy_manager.is_none() {
            return Err(Error::UnsupportedProtocol(
                "zwlr_screencopy_manager_v1 not available".to_string(),
            ));
        }
        if instance.globals.shm.is_none() {
            return Err(Error::UnsupportedProtocol(
                "wl_shm not available".to_string(),
            ));
        }
        Ok(instance)
    }
}
