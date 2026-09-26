//! Sweep register indices via report-0x06 'S' write, watching for lighting change.
//!
//! For each REG 0..N: send 'S' write [06,53,01,REG,b0,b1,b2,b3] (1031-byte report),
//! with distinctive data. Reports which REGs ACK.
//!
//! Usage: regsweep <data0> <data1> <data2> <data3> [max_reg]
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    if args.len() < 4 {
        eprintln!("usage: regsweep <d0> <d1> <d2> <d3> [max_reg]");
        return;
    }
    let d = [hex(&args[0]), hex(&args[1]), hex(&args[2]), hex(&args[3])];
    let max = if args.len() >= 5 { hex(&args[4]) } else { 32 };

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

    for reg in 0..=max {
        let mut p = vec![0u8; 1031];
        p[0] = 0x06;
        p[1] = 0x53;
        p[2] = 0x01;
        p[3] = reg as u8;
        p[4..8].copy_from_slice(&d);
        let r = match tokio::time::timeout(Duration::from_millis(1500), dev.send_feature_report(&p))
            .await
        {
            Ok(Ok(())) => "ACK".to_string(),
            Ok(Err(e)) => format!("ERR {:?}", e),
            Err(_) => "TIMEOUT".to_string(),
        };
        println!("REG={:02x} data={:02x?} -> {}", reg, d, r);
        tokio::time::sleep(Duration::from_millis(60)).await;
    }
    println!("DONE — check keyboard");
}
