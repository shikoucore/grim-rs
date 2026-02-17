pub(super) fn apply_output_transform(
    transform: wayland_client::protocol::wl_output::Transform,
    width: &mut i32,
    height: &mut i32,
) {
    use wayland_client::protocol::wl_output::Transform;

    match transform {
        Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270 => {
            std::mem::swap(width, height);
        }
        _ => {}
    }
}

/// Apply transform to captured image data based on rotation and flip.
///
/// This handles basic 90/180/270 degree rotations and horizontal flips.
pub(super) fn apply_image_transform(
    data: &[u8],
    width: u32,
    height: u32,
    transform: wayland_client::protocol::wl_output::Transform,
) -> (Vec<u8>, u32, u32) {
    use wayland_client::protocol::wl_output::Transform;

    match transform {
        Transform::Normal => {
            // No transformation needed
            (data.to_vec(), width, height)
        }
        Transform::_90 => {
            // Rotate 90 degrees clockwise
            rotate_90(data, width, height)
        }
        Transform::_180 => {
            // Rotate 180 degrees
            rotate_180(data, width, height)
        }
        Transform::_270 => {
            // Rotate 270 degrees clockwise
            rotate_270(data, width, height)
        }
        Transform::Flipped => {
            // Horizontal flip only
            flip_horizontal(data, width, height)
        }
        Transform::Flipped90 => {
            // Flip then rotate 90
            let (flipped_data, w, h) = flip_horizontal(data, width, height);
            rotate_90(&flipped_data, w, h)
        }
        Transform::Flipped180 => {
            // Flip then rotate 180 (equivalent to vertical flip)
            flip_vertical(data, width, height)
        }
        Transform::Flipped270 => {
            // Flip then rotate 270
            let (flipped_data, w, h) = flip_horizontal(data, width, height);
            rotate_270(&flipped_data, w, h)
        }
        _ => {
            // Unknown transform, return as-is
            (data.to_vec(), width, height)
        }
    }
}

/// Rotate image 90 degrees clockwise.
pub(super) fn rotate_90(data: &[u8], width: u32, height: u32) -> (Vec<u8>, u32, u32) {
    let new_width = height;
    let new_height = width;
    let mut rotated = vec![0u8; (new_width * new_height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let src_idx = ((y * width + x) * 4) as usize;
            // For 90° rotation: new_x = height - 1 - y, new_y = x
            let new_x = height - 1 - y;
            let new_y = x;
            let dst_idx = ((new_y * new_width + new_x) * 4) as usize;

            rotated[dst_idx..dst_idx + 4].copy_from_slice(&data[src_idx..src_idx + 4]);
        }
    }

    (rotated, new_width, new_height)
}

/// Rotate image 180 degrees.
pub(super) fn rotate_180(data: &[u8], width: u32, height: u32) -> (Vec<u8>, u32, u32) {
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
pub(super) fn rotate_270(data: &[u8], width: u32, height: u32) -> (Vec<u8>, u32, u32) {
    let new_width = height;
    let new_height = width;
    let mut rotated = vec![0u8; (new_width * new_height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let src_idx = ((y * width + x) * 4) as usize;
            // For 270° rotation: new_x = y, new_y = width - 1 - x
            let new_x = y;
            let new_y = width - 1 - x;
            let dst_idx = ((new_y * new_width + new_x) * 4) as usize;

            rotated[dst_idx..dst_idx + 4].copy_from_slice(&data[src_idx..src_idx + 4]);
        }
    }

    (rotated, new_width, new_height)
}

/// Flip image horizontally.
pub(super) fn flip_horizontal(data: &[u8], width: u32, height: u32) -> (Vec<u8>, u32, u32) {
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
pub(super) fn flip_vertical(data: &[u8], width: u32, height: u32) -> (Vec<u8>, u32, u32) {
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
