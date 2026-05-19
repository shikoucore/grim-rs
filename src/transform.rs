/// Platform-independent output transform representation.
///
/// DXGI provides 4 rotation modes (Normal, Rotate90, Rotate180, Rotate270).
/// Wayland adds 4 flipped variants on top of those.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputTransform {
    Normal,
    Rotate90,
    Rotate180,
    Rotate270,
    #[cfg(target_os = "linux")]
    Flipped,
    #[cfg(target_os = "linux")]
    Flipped90,
    #[cfg(target_os = "linux")]
    Flipped180,
    #[cfg(target_os = "linux")]
    Flipped270,
}

#[cfg(target_os = "linux")]
impl From<wayland_client::protocol::wl_output::Transform> for OutputTransform {
    fn from(t: wayland_client::protocol::wl_output::Transform) -> Self {
        use wayland_client::protocol::wl_output::Transform;
        match t {
            Transform::Normal => OutputTransform::Normal,
            Transform::_90 => OutputTransform::Rotate90,
            Transform::_180 => OutputTransform::Rotate180,
            Transform::_270 => OutputTransform::Rotate270,
            Transform::Flipped => OutputTransform::Flipped,
            Transform::Flipped90 => OutputTransform::Flipped90,
            Transform::Flipped180 => OutputTransform::Flipped180,
            Transform::Flipped270 => OutputTransform::Flipped270,
            _ => OutputTransform::Normal,
        }
    }
}

/// Swap width/height for 90°/270° rotations (including flipped variants).
#[cfg(target_os = "linux")]
pub(crate) fn apply_output_transform(
    transform: OutputTransform,
    width: &mut i32,
    height: &mut i32,
) {
    match transform {
        OutputTransform::Rotate90
        | OutputTransform::Rotate270
        | OutputTransform::Flipped90
        | OutputTransform::Flipped270 => {
            std::mem::swap(width, height);
        }
        _ => {}
    }
}

/// Apply the given transform to image data, returning new pixel buffer and dimensions.
pub fn apply_image_transform(
    data: &[u8],
    width: u32,
    height: u32,
    transform: OutputTransform,
) -> (Vec<u8>, u32, u32) {
    match transform {
        OutputTransform::Normal => (data.to_vec(), width, height),
        OutputTransform::Rotate90 => rotate_90(data, width, height),
        OutputTransform::Rotate180 => rotate_180(data, width, height),
        OutputTransform::Rotate270 => rotate_270(data, width, height),
        #[cfg(target_os = "linux")]
        OutputTransform::Flipped => flip_horizontal(data, width, height),
        #[cfg(target_os = "linux")]
        OutputTransform::Flipped90 => {
            let (d, w, h) = flip_horizontal(data, width, height);
            rotate_90(&d, w, h)
        }
        #[cfg(target_os = "linux")]
        OutputTransform::Flipped180 => flip_vertical(data, width, height),
        #[cfg(target_os = "linux")]
        OutputTransform::Flipped270 => {
            let (d, w, h) = flip_horizontal(data, width, height);
            rotate_270(&d, w, h)
        }
    }
}

/// Rotate image 90 degrees clockwise.
pub fn rotate_90(data: &[u8], width: u32, height: u32) -> (Vec<u8>, u32, u32) {
    let new_width = height;
    let new_height = width;
    let mut rotated = vec![0u8; (new_width * new_height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let src_idx = ((y * width + x) * 4) as usize;
            let new_x = height - 1 - y;
            let new_y = x;
            let dst_idx = ((new_y * new_width + new_x) * 4) as usize;
            rotated[dst_idx..dst_idx + 4].copy_from_slice(&data[src_idx..src_idx + 4]);
        }
    }
    (rotated, new_width, new_height)
}

/// Rotate image 180 degrees.
pub fn rotate_180(data: &[u8], width: u32, height: u32) -> (Vec<u8>, u32, u32) {
    let mut rotated = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let src_idx = ((y * width + x) * 4) as usize;
            let new_x = width - 1 - x;
            let new_y = height - 1 - y;
            let dst_idx = ((new_y * width + new_x) * 4) as usize;
            rotated[dst_idx..dst_idx + 4].copy_from_slice(&data[src_idx..src_idx + 4]);
        }
    }
    (rotated, width, height)
}

/// Rotate image 270 degrees clockwise.
pub fn rotate_270(data: &[u8], width: u32, height: u32) -> (Vec<u8>, u32, u32) {
    let new_width = height;
    let new_height = width;
    let mut rotated = vec![0u8; (new_width * new_height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let src_idx = ((y * width + x) * 4) as usize;
            let new_x = y;
            let new_y = width - 1 - x;
            let dst_idx = ((new_y * new_width + new_x) * 4) as usize;
            rotated[dst_idx..dst_idx + 4].copy_from_slice(&data[src_idx..src_idx + 4]);
        }
    }
    (rotated, new_width, new_height)
}

/// Flip image horizontally.
#[cfg(target_os = "linux")]
pub(crate) fn flip_horizontal(data: &[u8], width: u32, height: u32) -> (Vec<u8>, u32, u32) {
    let mut flipped = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let src_idx = ((y * width + x) * 4) as usize;
            let new_x = width - 1 - x;
            let dst_idx = ((y * width + new_x) * 4) as usize;
            flipped[dst_idx..dst_idx + 4].copy_from_slice(&data[src_idx..src_idx + 4]);
        }
    }
    (flipped, width, height)
}

/// Flip image vertically.
#[cfg(target_os = "linux")]
pub(crate) fn flip_vertical(data: &[u8], width: u32, height: u32) -> (Vec<u8>, u32, u32) {
    let mut flipped = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let src_idx = ((y * width + x) * 4) as usize;
            let new_y = height - 1 - y;
            let dst_idx = ((new_y * width + x) * 4) as usize;
            flipped[dst_idx..dst_idx + 4].copy_from_slice(&data[src_idx..src_idx + 4]);
        }
    }
    (flipped, width, height)
}
