//! raw09 — send a bare report-0x09 frame with only the command byte set.
//!
//! Usage: raw09 <CMD>   (hex)
//!
//! 519-byte frame, payload[0] = CMD, rest zeros. For testing the untried
//! apply variants (0x0b known-good, 0x0c / 0x0d untested) and the reload
//! command (0x04) in isolation.
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: raw09 <CMD> (hex)");
        return;
    }
    let cmd = u8::from_str_radix(args[0].trim_start_matches("0x"), 16).unwrap_or(0);

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

    let mut frame = vec![0u8; 519];
    frame[0] = cmd;
    let mut p = vec![0x09u8];
    p.extend_from_slice(&frame);
    println!("raw09 cmd {:02x} -> {}", cmd, match tokio::time::timeout(Duration::from_millis(2500), dev.send_feature_report(&p)).await {
        Ok(Ok(())) => "ACK".to_string(),
        Ok(Err(e)) => format!("ERR {:?}", e),
        Err(_) => "TIMEOUT".to_string(),
    });
}