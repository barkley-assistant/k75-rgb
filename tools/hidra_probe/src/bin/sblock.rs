//! Full 19-byte 'S' config-block write (triggers 0x0F3E |= 0x10 apply).
//!
//! 'S' protocol (fcn.00005001):
//!   first packet  [06, 53, 01, REG, b0,b1,b2,b3]  -> REG->0x0EFF, 4B->buffer
//!   continuation  [06, 53, 01, ...8 bytes...]     -> 8B -> buffer+offset
//!   at 19 bytes -> apply (0x0F3E |= 0x10)
//!
//! Config buffer first byte = command (0x55 or 0x4d per fcn.0000b490).
//! We send a full block and let the apply trigger.
//!
//! Usage: sblock <hex bytes...>  (up to 20 bytes)
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

async fn send(dev: &hidra::HidDevice<hidra::NusbDevice>, p: &[u8]) -> String {
    match tokio::time::timeout(Duration::from_millis(2000), dev.send_feature_report(p)).await {
        Ok(Ok(())) => "ACK".to_string(),
        Ok(Err(e)) => format!("ERR {:?}", e),
        Err(_) => "TIMEOUT".to_string(),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    if args.is_empty() { eprintln!("usage: sblock <hex bytes...>"); return; }
    let bytes: Vec<u8> = args.iter().map(|s| hex(s)).collect();

    let mut api = match Hidra::<Nusb>::builder().build() { Ok(a)=>a, Err(e)=>{eprintln!("init {:?}",e);return;} };
    if let Err(e)=api.refresh_devices() { eprintln!("refresh {:?}",e);return; }
    let mut path=String::new();
    for info in api.device_list() {
        if info.vendor_id()==0x258a && info.product_id()==0x019d && info.interface_number()==1 {
            path=info.path().to_string(); break;
        }
    }
    if path.is_empty() { eprintln!("device not found"); return; }
    let dev = match api.open_path(&path).wait() { Ok(d)=>d, Err(e)=>{eprintln!("open {:?}",e);return;} };

    println!("block ({} bytes): {:02x?}", bytes.len(), bytes);

    // first packet: [06, 53, 01, REG, b0,b1,b2,b3] — REG=0, first 4 bytes
    let mut p = vec![0x06u8, 0x53, 0x01, 0x00];
    let n0 = bytes.len().min(4);
    p.extend_from_slice(&bytes[..n0]);
    println!("  first (4B): {}", send(&dev, &p).await);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // continuation packets of 8 bytes
    let mut off = n0;
    while off < bytes.len() {
        let mut cp = vec![0x06u8, 0x53, 0x01, 0x00];
        let end = (off + 8).min(bytes.len());
        let mut chunk = bytes[off..end].to_vec();
        while chunk.len() < 8 { chunk.push(0x00); }
        cp.extend_from_slice(&chunk);
        println!("  cont ({}+8): {}", off, send(&dev, &cp).await);
        tokio::time::sleep(Duration::from_millis(100)).await;
        off = end;
    }
    println!("DONE — check keyboard");
}