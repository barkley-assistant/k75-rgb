//! State oracle: read GET 0x06 (4B) + GET 0x09 (8B), optionally send a SET first, diff.
//! Usage: statediff                  — just read state
//!        statediff <hex...>         — read, SET_FEATURE [06, hex...], read again, diff
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

async fn read_state(dev: &hidra::HidDevice<hidra::NusbDevice>) -> (Vec<u8>, Vec<u8>) {
    let mut b6 = vec![0x06u8; 8];
    let mut b9 = vec![0x09u8; 16];
    let s6 = match tokio::time::timeout(Duration::from_millis(1500), dev.get_feature_report(&mut b6)).await {
        Ok(Ok(n)) => b6[..n].to_vec(),
        _ => vec![0xEE],
    };
    let s9 = match tokio::time::timeout(Duration::from_millis(1500), dev.get_feature_report(&mut b9)).await {
        Ok(Ok(n)) => b9[..n].to_vec(),
        _ => vec![0xEE],
    };
    (s6, s9)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let payload: Vec<u8> = args.iter().map(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0)).collect();

    let mut api = match Hidra::<Nusb>::builder().build() {
        Ok(a) => a, Err(e) => { eprintln!("init ERR: {:?}", e); return; }
    };
    if let Err(e) = api.refresh_devices() { eprintln!("refresh ERR: {:?}", e); return; }
    let mut path = String::new();
    for info in api.device_list() {
        if info.vendor_id() == 0x258a && info.product_id() == 0x019d && info.interface_number() == 1 {
            path = info.path().to_string(); break;
        }
    }
    if path.is_empty() { eprintln!("device not found"); return; }
    let dev = match api.open_path(&path).wait() { Ok(d) => d, Err(e) => { eprintln!("open ERR: {:?}", e); return; } };

    let (a6, a9) = read_state(&dev).await;
    println!("BEFORE  06:{:02x?}  09:{:02x?}", a6, a9);

    if !payload.is_empty() {
        let mut full = vec![0x06u8];
        full.extend_from_slice(&payload);
        match tokio::time::timeout(Duration::from_millis(1500), dev.send_feature_report(&full)).await {
            Ok(Ok(())) => println!("SET [06 {:02x?}] ACCEPTED", payload),
            Ok(Err(e)) => println!("SET ERR {:?}", e),
            Err(_) => println!("SET TIMEOUT"),
        }
    }

    let (b6, b9) = read_state(&dev).await;
    println!("AFTER   06:{:02x?}  09:{:02x?}", b6, b9);
    if a6 != b6 || a9 != b9 { println!("*** STATE CHANGED ***"); }
}
