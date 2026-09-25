//! Direct per-LED frame via cmd 0x08 (family-verified layout from RK M75 /
//! Kreo Hive 65: `09 08 00 00 01 00 <len LE> <RGB triplets>`).
//! Test pattern: left half red, right half blue — verifies the LED mapping.
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut api = Hidra::<Nusb>::builder().build().expect("hidra");
    api.refresh_devices().expect("refresh");
    let path = api
        .device_list()
        .find(|i| i.vendor_id() == 0x258a && i.product_id() == 0x019d && i.interface_number() == 1)
        .map(|i| i.path().to_string())
        .expect("device");
    let dev = api.open_path(&path).wait().expect("open");
    eprintln!("opened: {}", path);

    // 81 keys x 3 bytes
    const KEYS: usize = 81;
    let mut frame = vec![0u8; 520];
    frame[0] = 0x09; // report id
    frame[1] = 0x08; // direct-LED packet type
    frame[2] = 0x00; // offset lo
    frame[3] = 0x00; // offset hi
    frame[4] = 0x01;
    frame[5] = 0x00;
    let len = (KEYS * 3) as u16;
    frame[6] = (len & 0xff) as u8;
    frame[7] = (len >> 8) as u8;
    for k in 0..KEYS {
        let (r, g, b) = if k < KEYS / 2 {
            (0xff, 0x00, 0x00)
        } else {
            (0x00, 0x00, 0xff)
        };
        frame[8 + k * 3] = r;
        frame[8 + k * 3 + 1] = g;
        frame[8 + k * 3 + 2] = b;
    }
    eprintln!(
        "direct-LED frame: cmd 0x08, {} keys, len 0x{:04x}",
        KEYS, len
    );
    match tokio::time::timeout(Duration::from_millis(1500), dev.send_feature_report(&frame)).await {
        Ok(Ok(())) => eprintln!("ACK — check keyboard: left half red / right half blue"),
        other => eprintln!("SET ERR: {:?}", other),
    }
}
