# Library Usage Examples (`grim-rs`)

These examples show public API usage for `grim-rs`.

Prerequisites:

- Run inside a Wayland session.
- Use output names returned by `grim.get_outputs()` (examples use placeholders like `DP-1`).

## Basic capture operations

```rust,no_run
use grim_rs::{Box, Grim};

fn main() -> grim_rs::Result<()> {
    let mut grim = Grim::new()?;

    let result = grim.capture_all()?;
    grim.save_png(result.data(), result.width(), result.height(), "screenshot.png")?;

    let region = Box::new(100, 100, 800, 600);
    let result = grim.capture_region(region)?;
    grim.save_png(result.data(), result.width(), result.height(), "region.png")?;

    let result = grim.capture_output("DP-1")?;
    grim.save_png(result.data(), result.width(), result.height(), "output.png")?;

    Ok(())
}
```

## Getting outputs information

```rust,no_run
use grim_rs::Grim;

fn main() -> grim_rs::Result<()> {
    let mut grim = Grim::new()?;
    let outputs = grim.get_outputs()?;

    for output in outputs {
        println!("Output: {}", output.name());
        println!(
            "  Position: ({}, {})",
            output.geometry().x(),
            output.geometry().y()
        );
        println!(
            "  Size: {}x{}",
            output.geometry().width(),
            output.geometry().height()
        );
        println!("  Scale: {}", output.scale());
        if let Some(desc) = output.description() {
            println!("  Description: {}", desc);
        }
    }

    Ok(())
}
```

## Capture with scaling

```rust,no_run
use grim_rs::{Box, Grim};

fn main() -> grim_rs::Result<()> {
    let mut grim = Grim::new()?;

    let result = grim.capture_all_with_scale(0.5)?;
    grim.save_png(result.data(), result.width(), result.height(), "thumbnail.png")?;

    let region = Box::new(0, 0, 1920, 1080);
    let result = grim.capture_region_with_scale(region, 0.8)?;
    grim.save_png(result.data(), result.width(), result.height(), "scaled.png")?;

    let result = grim.capture_output_with_scale("DP-1", 0.5)?;
    grim.save_png(result.data(), result.width(), result.height(), "output_scaled.png")?;

    Ok(())
}
```

## Multiple outputs

```rust,no_run
use grim_rs::{Box, CaptureParameters, Grim};

fn main() -> grim_rs::Result<()> {
    let mut grim = Grim::new()?;

    let parameters = vec![
        CaptureParameters::new("DP-1").overlay_cursor(true),
        CaptureParameters::new("HDMI-A-1").region(Box::new(0, 0, 1920, 1080)),
    ];

    let results = grim.capture_outputs_with_scale(parameters, 0.5)?;
    for (output_name, result) in results.into_outputs() {
        let filename = format!("{}.png", output_name);
        grim.save_png(result.data(), result.width(), result.height(), &filename)?;
    }

    Ok(())
}
```

## Save to different formats

```rust,no_run
use grim_rs::Grim;

fn main() -> grim_rs::Result<()> {
    let mut grim = Grim::new()?;
    let result = grim.capture_all()?;

    grim.save_png(result.data(), result.width(), result.height(), "screenshot.png")?;
    grim.save_png_with_compression(result.data(), result.width(), result.height(), "compressed.png", 9)?;
    grim.save_jpeg(result.data(), result.width(), result.height(), "screenshot.jpg")?;
    grim.save_jpeg_with_quality(result.data(), result.width(), result.height(), "quality.jpg", 95)?;
    grim.save_ppm(result.data(), result.width(), result.height(), "screenshot.ppm")?;

    Ok(())
}
```

## Convert to bytes

```rust,no_run
use grim_rs::Grim;

fn main() -> grim_rs::Result<()> {
    let mut grim = Grim::new()?;
    let result = grim.capture_all()?;

    let png_bytes = grim.to_png(result.data(), result.width(), result.height())?;
    println!("PNG size: {} bytes", png_bytes.len());

    let _png_bytes = grim.to_png_with_compression(result.data(), result.width(), result.height(), 9)?;

    let jpeg_bytes = grim.to_jpeg(result.data(), result.width(), result.height())?;
    println!("JPEG size: {} bytes", jpeg_bytes.len());

    let _jpeg_bytes = grim.to_jpeg_with_quality(result.data(), result.width(), result.height(), 85)?;

    let ppm_bytes = grim.to_ppm(result.data(), result.width(), result.height())?;
    println!("PPM size: {} bytes", ppm_bytes.len());

    Ok(())
}
```

## Write encoded data to stdout

```rust,no_run
use grim_rs::Grim;

fn main() -> grim_rs::Result<()> {
    let mut grim = Grim::new()?;
    let result = grim.capture_all()?;

    grim.write_png_to_stdout(result.data(), result.width(), result.height())?;
    grim.write_png_to_stdout_with_compression(result.data(), result.width(), result.height(), 6)?;
    grim.write_jpeg_to_stdout(result.data(), result.width(), result.height())?;
    grim.write_jpeg_to_stdout_with_quality(result.data(), result.width(), result.height(), 90)?;
    grim.write_ppm_to_stdout(result.data(), result.width(), result.height())?;

    Ok(())
}
```

## Read region from stdin

```rust,no_run
use grim_rs::Grim;

fn main() -> grim_rs::Result<()> {
    let mut grim = Grim::new()?;
    let region = Grim::read_region_from_stdin()?;
    let result = grim.capture_region(region)?;
    grim.save_png(result.data(), result.width(), result.height(), "region.png")?;
    Ok(())
}
```
