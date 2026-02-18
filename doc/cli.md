# CLI Guide (`grim-rs`)

The `grim-rs` binary provides screenshot capture from Wayland compositors.

By default, output is saved to:

- `GRIM_DEFAULT_DIR` (if set), otherwise
- `XDG_PICTURES_DIR` (if it exists), otherwise
- current directory.

## Options

```bash
-h              Show help message and quit
-s <factor>     Set the output image scale factor (e.g. 0.5 for 50%)
-g <geometry>   Set region to capture (format: "x,y widthxheight")
-t png|ppm|jpeg Set output filetype (default: png)
-q <quality>    JPEG quality (0-100, default: 80)
-l <level>      PNG compression level (parser rejects values > 9; default: 6)
-o <output>     Output name to capture (e.g. "DP-1", "HDMI-A-1")
-c              Include cursor in screenshot
```

Note: `-l` currently validates only the upper bound (`> 9` is rejected).

## Examples

```bash
# Build first
cargo build --release

# Capture full screen
cargo run --bin grim-rs

# Capture to a specific filename
cargo run --bin grim-rs -- screenshot.png

# Capture region
cargo run --bin grim-rs -- -g "100,100 800x600" region.png

# Capture with scaling
cargo run --bin grim-rs -- -s 0.5 thumbnail.png

# Capture specific output
cargo run --bin grim-rs -- -o DP-1 monitor.png

# Include cursor
cargo run --bin grim-rs -- -c -o DP-1 with_cursor.png

# JPEG with custom quality
cargo run --bin grim-rs -- -t jpeg -q 90 screenshot.jpg

# PNG with max compression
cargo run --bin grim-rs -- -l 9 compressed.png

# PPM output
cargo run --bin grim-rs -- -t ppm screenshot.ppm

# Combined options
cargo run --bin grim-rs -- -g "0,0 1920x1080" -s 0.8 -c scaled_region.png

# Write to stdout and pipe
cargo run --bin grim-rs -- - > screenshot.png

# Override output directory
GRIM_DEFAULT_DIR=/tmp cargo run --bin grim-rs

# Read region from stdin
echo "100,100 800x600" | cargo run --bin grim-rs -- -g -
```

## Installed binary

After `cargo install grim-rs`:

```bash
grim-rs
grim-rs -g "100,100 800x600" -s 0.5 thumbnail.png
grim-rs -o DP-1 -c monitor.png
grim-rs - | wl-copy
```

Note: the binary is named `grim-rs` to avoid conflict with the original `grim`.
