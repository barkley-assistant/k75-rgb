//! Send a full 1031-byte feature report on 0x06 with a solid-red candidate header.
//! Usage: full_report <hex header bytes...>  (rest zero-padded to 1030 data bytes)
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let header: Vec<u8> = args.iter().map(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0)).collect();

    let mut payload = vec![0u8; 1031];
    payload[0] = 0x06;
    for (i, b) in header.iter().enumerate() {
        if i + 1 < 1031 { payload[i + 1] = *b; }
    }
    eprintln!("sending 1031-byte report 0x06, header: {:02x?}", &payload[..1 + header.len().min(32)]);

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
    eprintln!("opened: {}", path);

    match tokio::time::timeout(Duration::from_secs(5), dev.send_feature_report(&payload)).await {
        Ok(Ok(())) => eprintln!("ACCEPTED"),
        Ok(Err(e)) => eprintln!("ERR: {:?}", e),
        Err(_) => eprintln!("TIMEOUT"),
    }
}
