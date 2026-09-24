//! Test: write the command block [0x5A, type, sub, op, f1, f2, cksum] across
//! registers REG=1..7 (mapping to config table 0x11C1..0x11C7).
//! Usage: mode_test <type> <sub> <op> <f1> <f2>
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

async fn setreg(dev: &hidra::HidDevice<hidra::NusbDevice>, reg: u8, d0: u8, d1: u8, d2: u8, d3: u8) -> String {
    let p = vec![0x06u8, 0x53, 0x01, reg, d0, d1, d2, d3];
    match tokio::time::timeout(Duration::from_millis(1500), dev.send_feature_report(&p)).await {
        Ok(Ok(())) => "ACK".to_string(),
        Ok(Err(e)) => format!("ERR {:?}", e),
        Err(_) => "TIMEOUT".to_string(),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    if args.len() < 5 { eprintln!("usage: mode_test <type> <sub> <op> <f1> <f2>"); return; }
    let t = hex(&args[0]); let s = hex(&args[1]); let op = hex(&args[2]);
    let f1 = hex(&args[3]); let f2 = hex(&args[4]);
    let cksum = 0xFFu8.wrapping_sub(t.wrapping_add(s).wrapping_add(op).wrapping_add(f1).wrapping_add(f2));

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

    // block: [magic=0x5A, type, sub, op, f1, f2, cksum]
    let block = [0x5Au8, t, s, op, f1, f2, cksum];
    println!("block={:02x?}", block);
    // write REG=1..7 (maps to 0x11C1..0x11C7)
    for i in 0..7u8 {
        let r = setreg(&dev, i+1, block[i as usize], 0, 0, 0).await;
        println!("  REG={} val={:02x} -> {}", i+1, block[i as usize], r);
        tokio::time::sleep(Duration::from_millis(80)).await;
    }
    println!("DONE — check keyboard");
}
