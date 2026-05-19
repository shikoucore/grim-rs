use crate::{Error, Result};

/// Global upper bound for pixel count used by `checked_buffer_size()`.
///
/// This limit is enforced for all image/buffer allocations that are derived
/// from width/height, including capture, composite, scaling, and encoding paths.
///
/// The goal is to prevent integer overflows and avoid OOM from extreme sizes.
pub(crate) const MAX_PIXELS: u64 = 134_217_728; // 128 megapixels

/// Compute a safe buffer size in bytes for image-like data.
///
/// What it does:
/// - Validates `width × height` without overflow.
/// - Enforces the global `MAX_PIXELS` limit.
/// - Computes the byte size either as `width × height × bytes_per_pixel`
///   or as `row_stride_bytes × height` when a stride is provided.
///
/// Where it is used:
/// - All large allocations in capture, composite, scaling, and Wayland buffer handling.
///
/// When to use it:
/// - Call this helper whenever you are about to allocate a buffer whose size
///   depends on `width × height` (and optionally row stride).
/// - Use it for any new code path that creates `Vec<u8>` for image data or
///   sizes a file/mmap based on image dimensions.
pub fn checked_buffer_size(
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
