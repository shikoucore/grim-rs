use super::*;

impl WaylandCapture {
    pub(super) fn scale_image_data(
        &self,
        capture_result: CaptureResult,
        scale: f64,
    ) -> Result<CaptureResult> {
        if scale == 1.0 {
            return Ok(capture_result);
        }

        let scale_int = scale as u32;
        if scale > 1.0 && (scale - (scale_int as f64)).abs() < 0.01 && (2..=4).contains(&scale_int)
        {
            return self.scale_image_integer_fast(capture_result, scale_int);
        }

        let old_width = capture_result.width;
        let old_height = capture_result.height;
        let new_width = ((old_width as f64) * scale) as u32;
        let new_height = ((old_height as f64) * scale) as u32;

        if new_width == 0 || new_height == 0 {
            return Err(Error::InvalidRegion(
                "Scaled dimensions must be positive".to_string(),
            ));
        }
        checked_buffer_size(new_width, new_height, 4, None)?;

        use image::{imageops, ImageBuffer, Rgba};

        let img =
            ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(old_width, old_height, capture_result.data)
                .ok_or_else(|| {
                    Error::ScalingFailed(format!(
                        "failed to create image buffer for scaling {}x{} -> {}x{}",
                        old_width, old_height, new_width, new_height
                    ))
                })?;

        let filter = if scale > 1.0 {
            imageops::FilterType::Nearest
        } else if scale >= 0.75 {
            imageops::FilterType::Triangle
        } else if scale >= 0.5 {
            imageops::FilterType::CatmullRom
        } else {
            imageops::FilterType::Lanczos3
        };

        let scaled_img = imageops::resize(&img, new_width, new_height, filter);

        Ok(CaptureResult {
            data: scaled_img.into_raw(),
            width: new_width,
            height: new_height,
        })
    }

    /// Fast scaling for integer multipliers (2x, 3x, 4x)
    ///
    /// Uses nearest neighbor without floating point operations for maximum performance.
    /// Each pixel from the source image is duplicated into a factor×factor block of pixels.
    ///
    /// # Performance
    ///
    /// This implementation is 20-30x faster than `image::imageops::resize` because it:
    /// - Avoids roundf calls (~258ms for 30M pixels)
    /// - Avoids float→u8 conversion (~241ms)
    /// - Avoids exp calls in interpolation (~223ms)
    /// - Uses simple memory block copying
    pub(super) fn scale_image_integer_fast(
        &self,
        capture: CaptureResult,
        factor: u32,
    ) -> Result<CaptureResult> {
        let old_width = capture.width as usize;
        let old_height = capture.height as usize;
        let new_width = old_width * (factor as usize);
        let new_height = old_height * (factor as usize);
        let new_width_u32 = u32::try_from(new_width).map_err(|_| {
            Error::ScalingFailed("Scaled width exceeds supported range".to_string())
        })?;
        let new_height_u32 = u32::try_from(new_height).map_err(|_| {
            Error::ScalingFailed("Scaled height exceeds supported range".to_string())
        })?;
        let new_bytes = checked_buffer_size(new_width_u32, new_height_u32, 4, None)?;

        let mut new_data = vec![0u8; new_bytes];

        for old_y in 0..old_height {
            for old_x in 0..old_width {
                let old_idx = (old_y * old_width + old_x) * 4;
                let pixel = [
                    capture.data[old_idx],
                    capture.data[old_idx + 1],
                    capture.data[old_idx + 2],
                    capture.data[old_idx + 3],
                ];

                for dy in 0..factor as usize {
                    for dx in 0..factor as usize {
                        let new_x = old_x * (factor as usize) + dx;
                        let new_y = old_y * (factor as usize) + dy;
                        let new_idx = (new_y * new_width + new_x) * 4;

                        new_data[new_idx..new_idx + 4].copy_from_slice(&pixel);
                    }
                }
            }
        }

        Ok(CaptureResult::new(
            new_data,
            new_width as u32,
            new_height as u32,
        ))
    }
}
