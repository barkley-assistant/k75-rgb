//! regset — write one register via the report-0x06 'S' SET protocol.
//!
//! Usage: regset <REG> <v0> <v1> <v2> <v3>
//!
//! Frame (verified against fcn.00005001): [06, 0x53, 01, REG, b0..b3].
//! 'S' packets can stage without applying; this probe sends only a short
//! frame. An ACK or no visible effect does not prove a register value changed.
//! See docs/protocol-audit.md before interpreting results.
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 5 {
        eprintln!("usage: regset <REG> <v0> <v1> <v2> <v3>  (all hex)");
        return;
    }
    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    let reg = hex(&args[0]);
    let vals: Vec<u8> = args[1..5].iter().map(|s| hex(s)).collect();

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

    let mut p = vec![0x06u8, 0x53, 0x01, reg];
    p.extend_from_slice(&vals);
    println!("'S' short stage reg={:02x} val={:02x?}", reg, vals);
    match tokio::time::timeout(Duration::from_millis(2500), dev.send_feature_report(&p)).await {
        Ok(Ok(())) => println!("-> transport OK; apply and live register change NOT verified"),
        Ok(Err(e)) => println!("-> ERR {:?}", e),
        Err(_) => println!("-> TIMEOUT"),
    }
}
