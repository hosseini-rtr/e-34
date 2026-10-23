# E34 Vision Firmware

Rust firmware for an AI-Thinker ESP32-CAM (OV2640) using ESP-IDF and the
optimized `esp32-camera` driver. It captures JPEG frames directly into PSRAM,
keeps two driver buffers in flight, drops stale frames, and reports capture
rate and driver errors over the serial monitor.

## One-time setup

```sh
cargo install cargo-espflash
cargo install espup
espup install
source "$HOME/export-esp.sh"
rustup target add xtensa-esp32-espidf
```

Close any active Python virtual environment before building. ESP-IDF builds
also require the ESP-IDF tools installed by `espup`.

## Build

From this directory:

```sh
cargo build --release
```

For the first build, Cargo downloads the `espressif/esp32-camera` component.
The release profile enables LTO, one codegen unit, size optimization, and an
abort-on-panic image for lower firmware overhead.

## Flash and monitor

Put GPIO0 on GND, connect the FTDI adapter, then replace `/dev/cu.usbserial-XXXX`
with the serial device shown by macOS:

```sh
cargo espflash flash --release --monitor --port /dev/cu.usbserial-XXXX
```

After flashing, remove GPIO0 from GND and press `RST`. The monitor should show
`CAMERA_READY`, followed by periodic `FRAME` lines.

## Tuning

`src/main.rs` defaults to VGA and JPEG quality 10 for high image quality. For
the lowest latency, change `FRAME_SIZE` to `FrameSize::Qvga`; for more detail,
use `FrameSize::Svga` only when the PSRAM and link can sustain the larger JPEGs.
