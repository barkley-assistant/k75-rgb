//! Register write via report-0x06 'S' 0x01 protocol — CORRECT layout.
//!
//! Decoded fcn.00005001 (verified against working 'R' 'V' read):
//!   buf[0] = 0x06 (report ID, present in buffer)
//!   buf[1] = 'S' (0x53)
//!   buf[2] = 0x01 (first-packet marker)
//!   buf[3] = REG  -> 0x0EFF (register selector)
//!   buf[4..7] = 4 data bytes -> 0x0F07 config buffer
//!
//! Usage: setreg2 <reg> <b0> <b1> <b2> <b3>
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

async fn get6(dev: &hidra::HidDevice<hidra::NusbDevice>) -> Vec<u8> {
    let mut b = vec![0x06u8; 32];
    match tokio::time::timeout(Duration::from_millis(1500), dev.get_feature_report(&mut b)).await {
        Ok(Ok(n)) => b[..n.min(32)].to_vec(),
        _ => vec![0xEE],
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    if args.len() < 5 {
        eprintln!("usage: setreg2 <reg> <b0> <b1> <b2> <b3>");
        return;
    }
    let reg = hex(&args[0]);
    let data: Vec<u8> = args[1..5].iter().map(|s| hex(s)).collect();

    let mut api = match Hidra::<Nusb>::builder().build() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("init ERR: {:?}", e);
            return;
        }
    };
    if let Err(e) = api.refresh_devices() {
        eprintln!("refresh ERR: {:?}", e);
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
            eprintln!("open ERR: {:?}", e);
            return;
        }
    };

    // correct layout: [06, 53, 01, REG, b0, b1, b2, b3]
    let mut p = vec![0x06u8, 0x53, 0x01, reg];
    p.extend_from_slice(&data);

    let before = get6(&dev).await;
    match tokio::time::timeout(Duration::from_millis(2000), dev.send_feature_report(&p)).await {
        Ok(Ok(())) => println!("SET reg={:02x} data={:02x?} ACK", reg, data),
        Ok(Err(e)) => println!("SET ERR {:?}", e),
        Err(_) => println!("SET TIMEOUT"),
    }
    // read back register via 'R' 'V' <reg>
    let rp = vec![0x06u8, 0x52, 0x56, reg];
    let _ = tokio::time::timeout(Duration::from_millis(1500), dev.send_feature_report(&rp)).await;
    let after = get6(&dev).await;
    println!("  before: {:02x?}", before);
    println!("  after : {:02x?}", after);
}
