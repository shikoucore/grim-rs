use crate::buffer::checked_buffer_size;
use crate::{CaptureResult, Error, Region, Result};

/// Blit a capture result into a larger destination buffer at the given offset.
///
/// Handles bounds checking and partial copies when the source extends beyond
/// the destination edges.
pub(crate) fn blit_capture(
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

/// Crop a region from a full RGBA buffer.
pub(crate) fn crop_rgba(
    data: &[u8],
    full_width: u32,
    full_height: u32,
    x: usize,
    y: usize,
    width: u32,
    height: u32,
) -> (Vec<u8>, u32, u32) {
    let w = (width as usize).min(full_width as usize - x.min(full_width as usize));
    let h = (height as usize).min(full_height as usize - y.min(full_height as usize));
    if w == 0 || h == 0 {
        return (Vec::new(), 0, 0);
    }
    let mut out = vec![0u8; w * h * 4];
    for row in 0..h {
        let src_start = ((y + row) * full_width as usize + x) * 4;
        let dst_start = row * w * 4;
        out[dst_start..dst_start + w * 4].copy_from_slice(&data[src_start..src_start + w * 4]);
    }
    (out, w as u32, h as u32)
}

/// Composite a region from multiple outputs into a single capture buffer.
///
/// Calls `capture_fn` for each intersecting output and blits the results
/// into a single RGBA buffer. The closure receives (output_name, local_region,
/// overlay_cursor) and must return a `CaptureResult` with pixels already in
/// the correct orientation and scale for compositing.
pub(crate) fn composite_region<F>(
    region: Region,
    outputs: &[(String, Region)],
    overlay_cursor: bool,
    mut capture_fn: F,
) -> Result<CaptureResult>
where
    F: FnMut(&str, Region, bool) -> Result<CaptureResult>,
{
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
    for (output_name, output_geom) in outputs {
        if let Some(intersection) = output_geom.intersection(&region) {
            if intersection.width() <= 0 || intersection.height() <= 0 {
                continue;
            }
            let local_region = Region::new(
                intersection.x() - output_geom.x(),
                intersection.y() - output_geom.y(),
                intersection.width(),
                intersection.height(),
            );
            let capture = capture_fn(output_name, local_region, overlay_cursor)?;
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
