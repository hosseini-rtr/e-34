AI-Thinker ESP32-CAM with OV2640 sensor.

The Rust firmware in this directory uses the AI-Thinker pin map from
`rusty_esp_image-esp`, JPEG capture, VGA resolution, JPEG quality 10, two
PSRAM frame buffers, and `CAMERA_GRAB_LATEST` for low capture latency.

For a latency-first stream, change `FRAME_SIZE` in `src/main.rs` from `Vga`
to `Qvga`. The board must have working PSRAM. Connect a 5 V supply capable of
at least 500 mA; use an FTDI adapter at 3.3 V logic, with GND, U0R, U0T, and
GPIO0 connected to GND while flashing. Remove GPIO0 from GND to boot normally.
esp-32 dev kit
