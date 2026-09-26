//! Set whole keyboard to one solid color via report 0x09 bulk channel.
//! Decoded frame layout (0x7253 + 0x08FA header):
//!   payload[0] = command opcode (0x0b/0x0c/0x0d = set config family)
//!   payload[1..] = config data
//! Total 519 bytes. Fires once, holds, echoes via GET 0x09 for confirmation.
//! Usage: setcolor <cmd> <R> <G> <B>
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

async fn get9(dev: &hidra::HidDevice<hidra::NusbDevice>) -> Vec<u8> {
    let mut b = vec![0x09u8; 16];
    match tokio::time::timeout(Duration::from_millis(1500), dev.get_feature_report(&mut b)).await {
        Ok(Ok(n)) => b[..n.min(16)].to_vec(),
        _ => vec![0xEE],
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    if args.len() < 4 {
        eprintln!("usage: setcolor <cmd> <R> <G> <B>");
        return;
    }
    let cmd = hex(&args[0]);
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
            eprintln!("open ERR {:?}", e);
            return;
        }
    };

    // Build 519-byte frame
    let mut data = vec![0u8; 519];
    data[0] = cmd;
    data[1] = r;
    data[2] = g;
    data[3] = b;
    // fill remaining with color too, in case the frame is per-key RGB triplets
    let mut i = 4;
    while i + 2 < 519 {
        data[i] = r;
        data[i + 1] = g;
        data[i + 2] = b;
        i += 3;
    }
    let mut p = vec![0x09u8];
    p.extend_from_slice(&data);

    println!("BEFORE GET 0x09: {:02x?}", get9(&dev).await);
    println!(
        "SET 0x09 cmd={:02x} RGB={:02x}{:02x}{:02x} (519B)",
        cmd, r, g, b
    );
    match tokio::time::timeout(Duration::from_millis(2500), dev.send_feature_report(&p)).await {
        Ok(Ok(())) => println!("SET ACCEPTED"),
        Ok(Err(e)) => println!("SET ERR {:?}", e),
        Err(_) => println!("SET TIMEOUT"),
    }
    println!("AFTER  GET 0x09: {:02x?}", get9(&dev).await);
}
