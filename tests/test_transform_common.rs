use grim_rs::{apply_image_transform, rotate_180, rotate_270, rotate_90, OutputTransform};

fn make_rgba_2x2() -> Vec<u8> {
    vec![0, 1, 2, 255, 3, 4, 5, 255, 6, 7, 8, 255, 9, 10, 11, 255]
}

#[test]
fn rotate_90_swaps_dimensions() {
    let src = make_rgba_2x2();
    let (result, w, h) = rotate_90(&src, 2, 2);
    assert_eq!(w, 2);
    assert_eq!(h, 2);
    // new(0,0) = old(1,0)
    assert_eq!(&result[0..4], &[6, 7, 8, 255]);
    // new(1,0) = old(0,0)
    assert_eq!(&result[4..8], &[0, 1, 2, 255]);
}

#[test]
fn rotate_180_keeps_dimensions() {
    let src = make_rgba_2x2();
    let (result, w, h) = rotate_180(&src, 2, 2);
    assert_eq!(w, 2);
    assert_eq!(h, 2);
    assert_eq!(&result[0..4], &[9, 10, 11, 255]);
    assert_eq!(&result[12..16], &[0, 1, 2, 255]);
}

#[test]
fn rotate_270_swaps_dimensions() {
    let src = make_rgba_2x2();
    let (result, w, h) = rotate_270(&src, 2, 2);
    assert_eq!(w, 2);
    assert_eq!(h, 2);
    assert_eq!(&result[0..4], &[3, 4, 5, 255]);
    assert_eq!(&result[4..8], &[9, 10, 11, 255]);
}

#[test]
fn rotate_identity_roundtrip() {
    let src = make_rgba_2x2();
    let (r90, w, h) = rotate_90(&src, 2, 2);
    let (r180, w2, h2) = rotate_90(&r90, w, h);
    let (r270, w3, h3) = rotate_90(&r180, w2, h2);
    let (r360, _, _) = rotate_90(&r270, w3, h3);
    assert_eq!(r360, src);
}

#[test]
fn rotate_non_square_image() {
    let src = vec![
        0, 1, 2, 255, 3, 4, 5, 255, 6, 7, 8, 255, 9, 10, 11, 255, 12, 13, 14, 255, 15, 16, 17, 255,
    ];
    let (result, w, h) = rotate_90(&src, 3, 2);
    assert_eq!(w, 2);
    assert_eq!(h, 3);
    let (restored, rw, rh) = rotate_270(&result, w, h);
    assert_eq!(rw, 3);
    assert_eq!(rh, 2);
    assert_eq!(restored, src);
}

#[test]
fn apply_transform_normal_is_identity() {
    let src = vec![10u8, 20, 30, 255, 40, 50, 60, 255];
    let (result, w, h) = apply_image_transform(&src, 2, 1, OutputTransform::Normal);
    assert_eq!(result, src);
    assert_eq!(w, 2);
    assert_eq!(h, 1);
}

#[test]
fn apply_transform_rotate_90_matches_direct() {
    let src = make_rgba_2x2();
    let (result, w, h) = apply_image_transform(&src, 2, 2, OutputTransform::Rotate90);
    let (expected, ew, eh) = rotate_90(&src, 2, 2);
    assert_eq!(w, ew);
    assert_eq!(h, eh);
    assert_eq!(result, expected);
}

#[test]
fn apply_transform_rotate_180_matches_direct() {
    let src = make_rgba_2x2();
    let (result, _w, _h) = apply_image_transform(&src, 2, 2, OutputTransform::Rotate180);
    let (expected, _, _) = rotate_180(&src, 2, 2);
    assert_eq!(result, expected);
}

#[test]
fn apply_transform_rotate_270_matches_direct() {
    let src = make_rgba_2x2();
    let (result, _w, _h) = apply_image_transform(&src, 2, 2, OutputTransform::Rotate270);
    let (expected, _, _) = rotate_270(&src, 2, 2);
    assert_eq!(result, expected);
}
