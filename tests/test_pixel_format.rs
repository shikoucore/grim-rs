use grim_rs::pixel_format::{self, PixelFormat};

#[test]
fn fourcc_known_mappings() {
    assert_eq!(
        pixel_format::fourcc_to_format(0x34325241),
        Some(PixelFormat::Argb8888)
    );
    assert_eq!(
        pixel_format::fourcc_to_format(0x34325258),
        Some(PixelFormat::Xrgb8888)
    );
    assert_eq!(
        pixel_format::fourcc_to_format(0x34324241),
        Some(PixelFormat::Abgr8888)
    );
    assert_eq!(
        pixel_format::fourcc_to_format(0x34324258),
        Some(PixelFormat::Xbgr8888)
    );
}

#[test]
fn fourcc_unknown_returns_none() {
    assert_eq!(pixel_format::fourcc_to_format(0xDEADBEEF), None);
}

#[test]
fn convert_xrgb8888_swaps_and_fills_alpha() {
    let mut data = vec![0x0A, 0x14, 0x1E, 0x63]; // B, G, R, X
    pixel_format::convert_to_rgba(&mut data, PixelFormat::Xrgb8888);
    assert_eq!(data, vec![0x1E, 0x14, 0x0A, 0xFF]); // R, G, B, 255
}

#[test]
fn convert_argb8888_swaps_preserving_alpha() {
    let mut data = vec![0x0A, 0x14, 0x1E, 0x63]; // B, G, R, A
    pixel_format::convert_to_rgba(&mut data, PixelFormat::Argb8888);
    assert_eq!(data, vec![0x1E, 0x14, 0x0A, 0x63]); // R, G, B, A
}

#[test]
fn convert_xbgr8888_fills_alpha_no_swap() {
    let mut data = vec![0x1E, 0x14, 0x0A, 0x00]; // R, G, B, X
    pixel_format::convert_to_rgba(&mut data, PixelFormat::Xbgr8888);
    assert_eq!(data, vec![0x1E, 0x14, 0x0A, 0xFF]); // R, G, B, 255
}

#[test]
fn convert_abgr8888_is_noop() {
    let original = vec![0x1E, 0x14, 0x0A, 0x63]; // R, G, B, A
    let mut data = original.clone();
    pixel_format::convert_to_rgba(&mut data, PixelFormat::Abgr8888);
    assert_eq!(data, original);
}

#[test]
fn convert_empty_buffer_is_noop() {
    let mut data: Vec<u8> = vec![];
    pixel_format::convert_to_rgba(&mut data, PixelFormat::Xrgb8888);
    assert!(data.is_empty());
}

#[test]
fn convert_incomplete_chunk_ignores_trailing_bytes() {
    let mut data = vec![1, 2, 3]; // 3 bytes, not divisible by 4
    pixel_format::convert_to_rgba(&mut data, PixelFormat::Argb8888);
    assert_eq!(data, vec![1, 2, 3]); // unchanged
}

#[test]
fn convert_multiple_pixels() {
    // Two pixels: BGRA(10,20,30,40) + BGRA(50,60,70,80)
    let mut data = vec![0x0A, 0x14, 0x1E, 0x28, 0x32, 0x3C, 0x46, 0x50];
    pixel_format::convert_to_rgba(&mut data, PixelFormat::Argb8888);
    assert_eq!(
        data,
        vec![0x1E, 0x14, 0x0A, 0x28, 0x46, 0x3C, 0x32, 0x50] // RGBA x2
    );
}
