//! Full 1031-byte report-0x06 'S' write (correct report size per descriptor).
//!
//! Report 0x06 descriptor: count 0x0407 = 1031 bytes, feature (b1 02).
//! 'S' layout (fcn.00005001):
//!   buf[0] = 0x06 (report ID)
//!   buf[1] = 'S' (0x53)
//!   buf[2] = 0x01 (first-packet)
//!   buf[3] = REG -> 0x0EFF
//!   buf[4..] = config block data (up to 1031-4 bytes)
//!
//! Usage: sfull <hex bytes...>  (config block; zero-padded to 1031)
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    let data: Vec<u8> = args.iter().map(|s| hex(s)).collect();
    if data.is_empty() {
        eprintln!("usage: sfull <hex bytes...>");
        return;
    }

    // build 1031-byte report: [06, 53, 01, REG, ...data...]
    let mut payload = vec![0u8; 1031];
    payload[0] = 0x06;
    payload[1] = 0x53;
    payload[2] = 0x01;
    payload[3] = 0x00; // REG
    for (i, b) in data.iter().enumerate() {
        if 4 + i < 1031 {
            payload[4 + i] = *b;
        }
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
        "sending 1031-byte 'S' block, {} data bytes: {:02x?}",
        data.len(),
        &data[..data.len().min(40)]
    );
    match tokio::time::timeout(
        Duration::from_millis(2500),
        dev.send_feature_report(&payload),
    )
    .await
    {
        Ok(Ok(())) => println!("ACK"),
        Ok(Err(e)) => println!("ERR {:?}", e),
        Err(_) => println!("TIMEOUT"),
    }
    println!("DONE — check keyboard");
}
