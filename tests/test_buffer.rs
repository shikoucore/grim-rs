use grim_rs::checked_buffer_size;

#[test]
fn standard_1080p_rgba() {
    let size = checked_buffer_size(1920, 1080, 4, None).unwrap();
    assert_eq!(size, 1920 * 1080 * 4);
}

#[test]
fn one_pixel() {
    let size = checked_buffer_size(1, 1, 4, None).unwrap();
    assert_eq!(size, 4);
}

#[test]
fn zero_width_is_zero_size() {
    let size = checked_buffer_size(0, 1080, 4, None).unwrap();
    assert_eq!(size, 0);
}

#[test]
fn zero_height_is_zero_size() {
    let size = checked_buffer_size(1920, 0, 4, None).unwrap();
    assert_eq!(size, 0);
}

#[test]
fn with_row_stride() {
    let size = checked_buffer_size(1920, 1080, 4, Some(2048 * 4)).unwrap();
    assert_eq!(size, 2048 * 4 * 1080);
}

#[test]
fn row_stride_smaller_than_width_pitch() {
    let size = checked_buffer_size(100, 10, 4, Some(50)).unwrap();
    assert_eq!(size, 500);
}

#[test]
fn exceeds_max_pixels() {
    let result = checked_buffer_size(20000, 20000, 4, None);
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("maximum pixel limit"));
}

#[test]
fn exactly_at_max_pixels() {
    let size = checked_buffer_size(8192, 16384, 4, None).unwrap();
    assert_eq!(size as u64, 134_217_728 * 4);
}

#[test]
fn dimensions_exceed_max_pixels() {
    let result = checked_buffer_size(u32::MAX, 2, 4, None);
    assert!(result.is_err());
}

#[test]
fn byte_size_exceeds_limit() {
    let result = checked_buffer_size(u16::MAX as u32, u16::MAX as u32, u8::MAX as u32, None);
    assert!(result.is_err());
}

#[test]
fn stride_with_large_height_rejected() {
    let result = checked_buffer_size(1, u32::MAX, 4, Some(u32::MAX));
    assert!(result.is_err());
}

#[test]
fn result_fits_in_usize() {
    let size = checked_buffer_size(1920, 1080, 4, None).unwrap();
    let _buf: Vec<u8> = vec![0u8; size];
}
