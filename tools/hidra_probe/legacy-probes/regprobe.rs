//! Register probe: write a register via 'S' 0x01, then read the register
//! table back via 'R' 'V' 1. Maps register index -> value for M2.
//!
//! Protocol (decoded fcn.00005001):
//!   WRITE: [06, 'S'(53), 01, REG, b0,b1,b2,b3]  -> REG->0x0EFF, 4B->0x0F07
//!   READ : [06, 'R'(52), 'V'(56), 01]           -> identity string (staged)
//!
//! Usage: regprobe <reg> <b0> <b1> <b2> <b3>
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

async fn get6(dev: &hidra::HidDevice<hidra::NusbDevice>, len: usize) -> Vec<u8> {
    let mut b = vec![0x06u8; len];
    match tokio::time::timeout(Duration::from_millis(1500), dev.get_feature_report(&mut b)).await {
        Ok(Ok(n)) => b[..n.min(len)].to_vec(),
        _ => vec![0xEE],
    }
}

async fn set6(dev: &hidra::HidDevice<hidra::NusbDevice>, p: &[u8]) -> String {
    let mut out = vec![0x06u8];
    out.extend_from_slice(p);
    match tokio::time::timeout(Duration::from_millis(2000), dev.send_feature_report(&out)).await {
        Ok(Ok(())) => "ACK".to_string(),
        Ok(Err(e)) => format!("ERR {:?}", e),
        Err(_) => "TIMEOUT".to_string(),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    if args.len() < 5 {
        eprintln!("usage: regprobe <reg> <b0> <b1> <b2> <b3>");
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

    // Build 'S' 0x01 SET packet: [53, 01, REG, b0,b1,b2,b3]
    let mut sp = vec![0x53u8, 0x01, reg];
    sp.extend_from_slice(&data);

    let before = get6(&dev, 32).await;
    let r = set6(&dev, &sp).await;
    // then read back via 'R' 'V' 1
    let mut rp = vec![0x52u8, 0x56, 0x01];
    let _ = set6(&dev, &rp).await;
    let after = get6(&dev, 32).await;

    println!("reg={:02x} data={:02x?} -> SET {}", reg, data, r);
    println!("  before: {:02x?}", before);
    println!("  after : {:02x?}", after);
}
