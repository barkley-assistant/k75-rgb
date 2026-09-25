//! 'S' 0x01 register SET via report 0x06 (fcn.00005001 decoded):
//!   payload = [06, 53, 01, REG, b0, b1, b2, b3]
//!   REG -> 0x0EFF (register selector); b0..b3 -> 0x0F07 (config buffer), via fcn.00002877
//! First packet sets 0x0EB5=4; continuation (payload[1]!=1) appends 8 bytes.
//! At >=19 bytes -> 0x0F3E |= 0x10 (apply trigger).
//! Usage: setreg <reg> <b0> <b1> <b2> <b3> [--more hexbytes...]
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

async fn get6(dev: &hidra::HidDevice<hidra::NusbDevice>) -> Vec<u8> {
    let mut b = vec![0x06u8; 64];
    match tokio::time::timeout(Duration::from_millis(1200), dev.get_feature_report(&mut b)).await {
        Ok(Ok(n)) => b[..n.min(64)].to_vec(),
        _ => vec![0xEE],
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    if args.is_empty() {
        eprintln!("usage: setreg <reg> <b0..b3> [more...]");
        return;
    }
    let vals: Vec<u8> = args.iter().map(|s| hex(s)).collect();

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

    println!("BEFORE GET 0x06: {:02x?}", get6(&dev).await);

    // Build the 'S' 0x01 SET: [06, 53, 01, REG, b0,b1,b2,b3]
    let mut p = vec![0x06u8, 0x53, 0x01];
    p.extend_from_slice(&vals);
    println!("SET 'S' 0x01 reg={:02x} data={:02x?}", vals[0], &vals[1..]);
    match tokio::time::timeout(Duration::from_millis(2000), dev.send_feature_report(&p)).await {
        Ok(Ok(())) => println!("SET ACCEPTED"),
        Ok(Err(e)) => println!("SET ERR {:?}", e),
        Err(_) => println!("SET TIMEOUT"),
    }
    println!("AFTER  GET 0x06: {:02x?}", get6(&dev).await);
}
