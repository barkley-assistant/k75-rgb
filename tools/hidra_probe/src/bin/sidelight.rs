//! sidelight — drive the side/case underglow zone (candidate: cmd 0x08).
//!
//! Usage: sidelight <R> <G> <B>   (all hex, applied to all 6 side LEDs)
//!        sidelight R1G1B1 R2G2B2 R3G3B3 R4G4B4 R5G5B5 R6G6B6  (6 triples)
//!
//! Frame basis: cmd 0x08 handler writes 6 x RGB (18 bytes) into the side
//! LED zone at XDATA 0x0379. Payload layout is best-known: 18 bytes at
//! payload[1..19] after the command byte. LIVE-VERIFY on the device —
//! if no change, the offset/route needs adjustment before use.
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: sidelight <R> <G> <B>   or   6 triples of R G B");
        return;
    }
    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    let mut colors: Vec<u8> = Vec::new();
    if args.len() == 3 {
        for _ in 0..6 {
            colors.push(hex(&args[0]));
            colors.push(hex(&args[1]));
            colors.push(hex(&args[2]));
        }
    } else if args.len() == 18 {
        colors = args.iter().map(|s| hex(s)).collect();
    } else {
        eprintln!("expected 3 args (uniform) or 18 args (per-LED)");
        return;
    }

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
    frame[0] = 0x08;
    frame[1..19].copy_from_slice(&colors);
    let mut p = vec![0x09u8];
    p.extend_from_slice(&frame);
    println!("cmd 0x08 side-LED 6xRGB -> {}", match tokio::time::timeout(Duration::from_millis(2500), dev.send_feature_report(&p)).await {
        Ok(Ok(())) => "ACK".to_string(),
        Ok(Err(e)) => format!("ERR {:?}", e),
        Err(_) => "TIMEOUT".to_string(),
    });
    println!("DONE — check the side light");
}