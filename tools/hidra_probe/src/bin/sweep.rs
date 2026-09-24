//! Post-fix sweep: SET_FEATURE + GET_FEATURE on every report ID 0x00..=0x30.
//! Payload: [rid, 0x00, 0x00] (cmd byte 0x00 = no-op in every known dispatcher).
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
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

    for rid in 0u8..=0x30 {
        if rid == 0x05 { println!("rid {:02x}: SKIP (ISP cmd channel; 0x75 boots to ISP)", rid); continue; }
        // SET_FEATURE
        let set_payload = vec![rid, 0x00, 0x00];
        let set_res = match tokio::time::timeout(Duration::from_millis(1500), dev.send_feature_report(&set_payload)).await {
            Ok(Ok(())) => "ACK".to_string(),
            Ok(Err(e)) => format!("ERR {}", short(&e.to_string())),
            Err(_) => "NAK(timeout)".to_string(),
        };
        // GET_FEATURE
        let mut buf = vec![0u8; 64];
        buf[0] = rid;
        let get_res = match tokio::time::timeout(Duration::from_millis(1500), dev.get_feature_report(&mut buf)).await {
            Ok(Ok(n)) => format!("{}B {:02x?}", n, &buf[..n.min(8)]),
            Ok(Err(e)) => format!("ERR {}", short(&e.to_string())),
            Err(_) => "NAK(timeout)".to_string(),
        };
        println!("rid {:02x}: SET[{}] GET[{}]", rid, set_res, get_res);
    }
}

fn short(s: &str) -> String {
    if s.contains("timed out") { "TIMEOUT".into() }
    else if s.contains("stall") { "STALL".into() }
    else if s.contains("overflow") { "EOVERFLOW".into() }
    else if s.contains("protocol") { "EPROTO".into() }
    else { s.chars().take(40).collect() }
}
