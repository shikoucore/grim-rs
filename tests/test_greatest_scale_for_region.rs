use grim_rs::{Box as GrimBox, Error, Grim};

fn bounding_box(outputs: &[grim_rs::Output]) -> GrimBox {
    let first = outputs.first().expect("outputs must be non-empty");
    let mut min_x = first.geometry().x();
    let mut min_y = first.geometry().y();
    let mut max_x = first.geometry().x() + first.geometry().width();
    let mut max_y = first.geometry().y() + first.geometry().height();

    for output in outputs.iter().skip(1) {
        let g = output.geometry();
        min_x = min_x.min(g.x());
        min_y = min_y.min(g.y());
        max_x = max_x.max(g.x() + g.width());
        max_y = max_y.max(g.y() + g.height());
    }

    GrimBox::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

#[test]
fn test_greatest_scale_for_region_skips_without_wayland() {
    // This test is intentionally best-effort: CI/headless environments may not have
    // a running Wayland compositor or any outputs.
    let mut grim = match Grim::new() {
        Ok(grim) => grim,
        Err(_) => return,
    };

    let outputs = match grim.get_outputs() {
        Ok(outputs) if !outputs.is_empty() => outputs,
        _ => return,
    };

    // Region that intersects at least one output should succeed and return a finite scale.
    let first = outputs.first().unwrap();
    let intersecting = GrimBox::new(first.geometry().x(), first.geometry().y(), 1, 1);

    let scale = grim
        .greatest_scale_for_region(Some(intersecting))
        .expect("expected intersecting region to yield a scale");

    assert!(scale.is_finite());
    assert!(scale >= 1.0);

    // Region that intersects all outputs should yield a scale >= a region that intersects
    // only one output (since the method returns the maximum scale across intersecting outputs).
    let bb = bounding_box(&outputs);
    let scale_all = grim
        .greatest_scale_for_region(Some(bb))
        .expect("expected bounding-box region to yield a scale");
    assert!(scale_all.is_finite());
    assert!(scale_all >= scale);

    // None means "consider all outputs", so it should match the bounding-box region.
    let scale_none = grim
        .greatest_scale_for_region(None)
        .expect("expected None region to yield a scale");
    assert!(scale_none.is_finite());
    assert!((scale_none - scale_all).abs() < 1e-6);

    // Region far outside the bounding box should fail with InvalidRegion.
    let far_away = GrimBox::new(
        bb.x() + bb.width() + 10_000,
        bb.y() + bb.height() + 10_000,
        10,
        10,
    );

    let err = grim
        .greatest_scale_for_region(Some(far_away))
        .expect_err("expected non-intersecting region to error");

    match err {
        Error::InvalidRegion(msg) => {
            assert!(msg.contains("does not intersect"));
        }
        other => panic!("expected InvalidRegion, got {other:?}"),
    }
}
