# API Reference (`grim-rs`)

This is a practical API index for the public surface.
For full rustdoc details, see [docs.rs/grim-rs](https://docs.rs/grim-rs).

## Core Methods

### Initialization

- `Grim::new()` - Create new Grim instance and connect to Wayland compositor

### Getting Display Information

- `get_outputs()` - Get list of available outputs with their properties (name, geometry, scale)

### Capture Methods

- `capture_all()` - Capture entire screen (all outputs)
- `capture_all_with_scale(scale: f64)` - Capture entire screen with scaling
- `capture_output(output_name: &str)` - Capture specific output by name
- `capture_output_with_scale(output_name: &str, scale: f64)` - Capture output with scaling
- `capture_region(region: Box)` - Capture specific rectangular region
- `capture_region_with_scale(region: Box, scale: f64)` - Capture region with scaling
- `capture_outputs(parameters: Vec<CaptureParameters>)` - Capture multiple outputs with different parameters
- `capture_outputs_with_scale(parameters: Vec<CaptureParameters>, default_scale: f64)` - Capture multiple outputs with scaling

### Saving to Files

- `save_png(&data, width, height, path)` - Save as PNG with default compression (level 6)
- `save_png_with_compression(&data, width, height, path, compression: u8)` - Save as PNG with custom compression
- `save_jpeg(&data, width, height, path)` - Save as JPEG with default quality (80) [requires `jpeg` feature]
- `save_jpeg_with_quality(&data, width, height, path, quality: u8)` - Save as JPEG with custom quality (0-100) [requires `jpeg` feature]
- `save_ppm(&data, width, height, path)` - Save as PPM (uncompressed)

### Converting to Bytes

- `to_png(&data, width, height)` - Convert to PNG bytes with default compression
- `to_png_with_compression(&data, width, height, compression: u8)` - Convert to PNG bytes with custom compression
- `to_jpeg(&data, width, height)` - Convert to JPEG bytes with default quality [requires `jpeg` feature]
- `to_jpeg_with_quality(&data, width, height, quality: u8)` - Convert to JPEG bytes with custom quality [requires `jpeg` feature]
- `to_ppm(&data, width, height)` - Convert to PPM bytes

### Writing to Stdout

- `write_png_to_stdout(&data, width, height)` - Write PNG to stdout with default compression
- `write_png_to_stdout_with_compression(&data, width, height, compression: u8)` - Write PNG to stdout with custom compression
- `write_jpeg_to_stdout(&data, width, height)` - Write JPEG to stdout with default quality [requires `jpeg` feature]
- `write_jpeg_to_stdout_with_quality(&data, width, height, quality: u8)` - Write JPEG to stdout with custom quality [requires `jpeg` feature]
- `write_ppm_to_stdout(&data, width, height)` - Write PPM to stdout

### Stdin Input

- `Grim::read_region_from_stdin()` - Read region specification from stdin (format: `"x,y widthxheight"`)

## Data Structures

### `CaptureResult`

- Fields are private (encapsulated)
- `data()` - Raw RGBA image data as `&[u8]`
- `width()` - Image width in pixels
- `height()` - Image height in pixels
- `into_data()` - Consume and return owned pixel buffer

### `CaptureParameters`

- Fields are private (builder + getters API)
- `CaptureParameters::new(output_name)` - Create parameters for an output
- Builder methods: `.region(...)`, `.overlay_cursor(...)`, `.scale(...)`
- Accessors: `output_name()`, `region_ref()`, `overlay_cursor_enabled()`, `scale_factor()`
- Note: per-output `scale` is currently stored in params; effective scaling in multi-output capture is applied by `capture_outputs_with_scale(..., default_scale)`

### `MultiOutputCaptureResult`

- Fields are private
- `get(output_name)` - Get one output result by name
- `outputs()` - Borrow all output results
- `into_outputs()` - Consume and return `HashMap<String, CaptureResult>`

### `Output`

- Fields are private
- `name()` - Output name (e.g., `eDP-1`, `HDMI-A-1`)
- `geometry()` - Output position and size (`Box`)
- `scale()` - Output scale factor
- `description()` - Optional monitor description

### `Box`

- Fields are private
- `Box::new(x, y, width, height)` - Create region
- Accessors: `x()`, `y()`, `width()`, `height()`
- Utilities: `is_empty()`, `intersects(...)`, `intersection(...)`
- Parse from string: `"x,y widthxheight"`

## Feature Flags

- **`jpeg`** - Enable JPEG support (enabled by default)
  - Adds `save_jpeg*`, `to_jpeg*`, and `write_jpeg_to_stdout*` methods

To disable default features:

```toml
[dependencies]
grim-rs = { version = "0.1", default-features = false }
```
