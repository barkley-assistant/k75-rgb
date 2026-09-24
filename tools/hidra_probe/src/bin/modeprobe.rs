//! Mode probe: write the F11-style config template through the K75's
//! verified write path (report 0x09 cmd 0x0a = config write op 0x54)
//! followed by cmd 0x0b (apply). Template mode byte placed at a
//! parameterizable offset to search the K75's config layout.
//!
//! Usage: modeprobe <mode_hex> <speed_hex> <bright_hex> <mode_offset_dec>
//!   mode_hex:     effect byte (F11 enum: 01 static, 03 rainbow, 0b wave ...)
//!   speed_hex:    speed nibble (12 slow .. 42 fastest)
//!   bright_hex:   brightness 0..4 (0 off .. 4 full)
//!   mode_offset:  template offset where the mode byte lands (default 0x15)
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

const TEMPLATE: [u8; 141] = [
    0x06, 0x03, 0xB6, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5A, 0xA5,
    0x03, 0x03, 0x00, 0x00, 0x00, 0x02, 0x20, 0x01, 0x00, 0x00, 0x00, 0x00, 0x55, 0x55, 0x01, 0x00,
    0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x20, 0x00, 0x44, 0x07, 0x30, 0x07, 0x23, 0x00, 0x23,
    0x00, 0x23, 0x07, 0x33, 0x07, 0x23, 0x07, 0x23, 0x07, 0x23, 0x07, 0x23, 0x07, 0x23, 0x07, 0x23,
    0x07, 0x23, 0x07, 0x23, 0x07, 0x23, 0x07, 0x23, 0x07, 0x23, 0x07, 0x23, 0x07, 0x23, 0x00, 0x10,
    0x00, 0x10, 0x07, 0x44, 0x07, 0x44, 0x07, 0x44, 0x07, 0x44, 0x07, 0x44, 0x07, 0x44, 0x07, 0x44,
    0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5A, 0xA5, 0x03, 0x03,
];

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = u8::from_str_radix(args.get(1).map(|s| s.as_str()).unwrap_or("03"), 16).unwrap();
    let speed = u8::from_str_radix(args.get(2).map(|s| s.as_str()).unwrap_or("22"), 16).unwrap();
    let bright = u8::from_str_radix(args.get(3).map(|s| s.as_str()).unwrap_or("04"), 16).unwrap();
    let mode_off: usize = args.get(4).map(|s| s.parse().unwrap()).unwrap_or(0x15);

    let mut api = Hidra::<Nusb>::builder().build().expect("hidra");
    api.refresh_devices().expect("refresh");
    let path = api
        .device_list()
        .find(|i| i.vendor_id() == 0x258a && i.product_id() == 0x019d && i.interface_number() == 1)
        .map(|i| i.path().to_string())
        .expect("device");
    let dev = api.open_path(&path).wait().expect("open");
    eprintln!("opened: {}", path);

    // Build the config frame: [09, 0a, template-with-mode, pad] (519 data bytes)
    let mut frame = vec![0u8; 520];
    frame[0] = 0x09;
    frame[1] = 0x0a; // config write (op 0x54)
    for (i, b) in TEMPLATE.iter().enumerate() {
        frame[1 + i] = *b;
    }
    frame[1 + mode_off] = mode;
    frame[1 + 0x29 + (mode as usize - 2) * 2] = speed + bright;
    eprintln!(
        "modeprobe: mode={:02x} speed={:02x} bright={:02x} mode_off={:#x}",
        mode, speed, bright, mode_off
    );

    let send = tokio::time::timeout(Duration::from_millis(1500), dev.send_feature_report(&frame)).await;
    match send {
        Ok(Ok(())) => {}
        other => {
            eprintln!("SET {:02x}: {:?}", frame[1], other);
            return;
        }
    }

    std::thread::sleep(Duration::from_millis(120));
    let mut apply = vec![0u8; 520];
    apply[0] = 0x09;
    apply[1] = 0x0b;
    let _ = tokio::time::timeout(Duration::from_millis(1500), dev.send_feature_report(&apply)).await;
    eprintln!("DONE — check keyboard");
}