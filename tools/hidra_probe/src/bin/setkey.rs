//! setkey — light ONE key (positional LED index) via cmd 0x0a + 0x0b.
//!
//! Usage: setkey <LED_IDX> <R> <G> <B>   (all hex)
//!
//! The cmd 0x0a payload is a positional RGB-triple stream: triple i = LED i
//! in the vendor matrix order (see analysis/key-led-map.txt). All other keys
//! are set to black so the single lit key is unambiguous.
//! Verified base: setcolor2 (M1) fills the same stream.
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        eprintln!("usage: setkey <LED_IDX> <R> <G> <B>  (all hex)");
        return;
    }
    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    let idx = hex(&args[0]) as usize;
    let r = hex(&args[1]);
    let g = hex(&args[2]);
    let b = hex(&args[3]);

    let mut api = match Hidra::<Nusb>::builder().build() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("init ERR {:?}", e);
            return;
        }
    };
    if let Err(e) = api.refresh_devices() {
        eprintln!("refresh ERR {:?}", e);
        return;
    }
    let mut path = String::new();
    for info in api.device_list() {
        if info.vendor_id() == 0x258a && info.product_id() == 0x019d && info.interface_number() == 1 {
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
            eprintln!("open ERR {:?}", e);
            return;
        }
    };
    async fn send9(dev: &hidra::HidDevice<hidra::NusbDevice>, data: &[u8]) -> String {
        let mut p = vec![0x09u8];
        p.extend_from_slice(data);
        match tokio::time::timeout(Duration::from_millis(2500), dev.send_feature_report(&p)).await {
            Ok(Ok(())) => "ACK".to_string(),
            Ok(Err(e)) => format!("ERR {:?}", e),
            Err(_) => "TIMEOUT".to_string(),
        }
    }

    // Step 1: cmd 0x0a — every triple black, triple at LED_IDX = RGB
    let mut frame = vec![0u8; 519];
    frame[0] = 0x0a;
    let pos = 1 + idx * 3;
    if pos + 2 >= 519 {
        eprintln!("LED index out of range (max 172)");
        return;
    }
    frame[pos] = r;
    frame[pos + 1] = g;
    frame[pos + 2] = b;
    println!("cmd 0x0a LED {} = {:02x}{:02x}{:02x} -> {}", idx, r, g, b, send9(&dev, &frame).await);
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Step 2: cmd 0x0b — apply
    let mut apply = vec![0u8; 519];
    apply[0] = 0x0b;
    println!("cmd 0x0b (apply) -> {}", send9(&dev, &apply).await);
    println!("DONE — one key should be lit (LED idx {}), all others off", idx);
}