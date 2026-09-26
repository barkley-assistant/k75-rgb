//! SetMode via the Sinowealth keyboard protocol (from OpenRGB F11 controller,
//! adapted for K75). Report 0x06, 1032-byte buffer (1031 data + report ID).
//!
//! Packet layout (F11 template, verified against OpenRGB SinowealthKeyboardController):
//!   buf[0]  = 0x06 (report ID)
//!   buf[1]  = 0x03 (SetMode command)
//!   buf[2]  = 0xB6
//!   buf[0x0E..0x0F] = 5A A5 (magic)
//!   buf[0x15] = mode (0x01 static, 0x02 respire, 0x03 rainbow, ... 0x15 per-key)
//!   buf[0x29 + (mode-2)*2] = speed + brightness (speed 0x12/0x22/0x32/0x42, bright 0..4)
//!   buf[0x28 + (mode-2)*2] = color mode (0x07 random, 0x00 fixed)
//!   for mode 0x15 (per-key): buf[0x14] = 0x01, buf[0x27] = 0x24
//!
//! Usage: setmode_f11 <mode> <speed> <brightness> [color_mode]
//!   mode: 0x01-0x15 (K75 valid: 01 02 03 04 05 07 08 09 0a 0b 0c 0d 0f 10 11 13)
//!   speed: 12/22/32/42   brightness: 0-4   color_mode: 00 or 07
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

const TEMPLATE: [u8; 141] = [
    0x06, 0x03, 0xb6, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5a, 0xa5,
    0x03, 0x03, 0x00, 0x00, 0x00, 0x02, 0x20, 0x01, 0x00, 0x00, 0x00, 0x00, 0x55, 0x55, 0x01, 0x00,
    0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x20, 0x00, 0x44, 0x07, 0x30, 0x07, 0x23, 0x00, 0x23,
    0x00, 0x23, 0x07, 0x33, 0x07, 0x23, 0x07, 0x23, 0x07, 0x23, 0x07, 0x23, 0x07, 0x23, 0x07, 0x23,
    0x07, 0x23, 0x07, 0x23, 0x07, 0x23, 0x00, 0x10, 0x00, 0x10, 0x07, 0x44, 0x07, 0x44, 0x07, 0x44,
    0x07, 0x44, 0x07, 0x44, 0x07, 0x44, 0x07, 0x44, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
    0x04, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5a, 0xa5, 0x03, 0x03,
];

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    if args.len() < 3 {
        eprintln!("usage: setmode_f11 <mode> <speed> <brightness> [color_mode]");
        eprintln!("  mode 01-15 (K75: 01 02 03 04 05 07 08 09 0a 0b 0c 0d 0f 10 11 13)");
        eprintln!("  speed 12/22/32/42, brightness 0-4, color_mode 00|07");
        return;
    }
    let mode = hex(&args[0]);
    let speed = hex(&args[1]);
    let brightness = hex(&args[2]);
    let color_mode = if args.len() >= 4 { hex(&args[3]) } else { 0x00 };

    let mut buf = vec![0u8; 1032];
    buf[..141].copy_from_slice(&TEMPLATE);

    let mode_byte_index = 0x15usize;
    buf[mode_byte_index] = mode;

    let speed_bright_index = 0x29usize + ((mode as usize).saturating_sub(2)) * 2;
    buf[speed_bright_index] = speed.wrapping_add(brightness);
    buf[speed_bright_index - 1] = color_mode;

    if mode == 0x15 {
        buf[mode_byte_index - 1] = 0x01;
        buf[0x27] = 0x24;
    }

    let mut api = match Hidra::<Nusb>::builder().build() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("init {:?}", e);
            return;
        }
    };
    if let Err(e) = api.refresh_devices() {
        eprintln!("refresh {:?}", e);
        return;
    }
    let mut path = String::new();
    for info in api.device_list() {
        if info.vendor_id() == 0x258a && info.product_id() == 0x019d && info.interface_number() == 1
        {
            path = info.path().to_string();
            break;
        }
    }
    if path.is_empty() {
        eprintln!("device not found");
        return;
    }
    let dev = match api.open_path(&path).wait() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("open {:?}", e);
            return;
        }
    };

    println!(
        "SetMode: mode={:02x} speed={:02x} bright={:02x} color_mode={:02x} ({} bytes)",
        mode,
        speed,
        brightness,
        color_mode,
        buf.len()
    );
    match tokio::time::timeout(Duration::from_millis(3000), dev.send_feature_report(&buf)).await {
        Ok(Ok(())) => println!("ACK"),
        Ok(Err(e)) => println!("ERR {:?}", e),
        Err(_) => println!("TIMEOUT"),
    }
    println!("DONE — check keyboard");
}
