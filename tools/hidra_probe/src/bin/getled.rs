//! getled — READ the live LED state through the official software protocol.
//!
//! Usage: getled [out.bin]
//!
//! Sends the vendor tool's GetLED request (cmd 0x44, report 0x09,
//! checksummed frame, decoded from the official RedThunder K75 software)
//! and captures the device's response. READ-ONLY toward the device.
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |acc, b| acc.wrapping_add(*b))
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_path = args.get(1).cloned().unwrap_or_else(|| {
        "/home/agent/projects/barkley-assistant/k75-rgb/analysis/led-state.bin".to_string()
    });

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
    eprintln!("opened: {}", path);

    // init handshake from the official tool: [0x81, 0x05] on report 0x06
    // (SendCommand path) returns a 3-byte device token
    let mut pw = vec![0u8; 520];
    pw[0] = 0x06;
    pw[1] = 0x81;
    pw[2] = 0x05;
    match tokio::time::timeout(Duration::from_millis(2500), dev.send_feature_report(&pw)).await {
        Ok(Ok(())) => println!("password request -> OK"),
        Ok(Err(e)) => println!("password send ERR: {:?}", e),
        Err(_) => println!("password send TIMEOUT"),
    }
    let mut pwresp = vec![0u8; 8];
    match tokio::time::timeout(
        Duration::from_millis(2500),
        dev.get_feature_report(&mut pwresp),
    )
    .await
    {
        Ok(Ok(n)) => println!("password response: {:02x?}", &pwresp[..n]),
        Ok(Err(e)) => println!("password get ERR: {:?}", e),
        Err(_) => println!("password get TIMEOUT"),
    }

    // GetLED request frame: [09][chk][44][00][pages=1][idx=0][len lo][len hi][512B zeros]
    let mut req = vec![0u8; 520];
    req[0] = 0x09;
    req[2] = 0x44; // LED cmd
    req[3] = 0x00;
    req[4] = 0x01; // pages
    req[5] = 0x00; // page idx
    req[6] = 0x00; // len lo (0 = read request per CDevG5MS::AccessData GET branch)
    req[7] = 0x00; // len hi
    req[1] = checksum(&req[2..]);

    match tokio::time::timeout(Duration::from_millis(2500), dev.send_feature_report(&req)).await {
        Ok(Ok(())) => println!("GetLED request -> OK"),
        Ok(Err(e)) => {
            eprintln!("send ERR: {:?}", e);
            return;
        }
        Err(_) => {
            eprintln!("send TIMEOUT");
            return;
        }
    }

    tokio::time::sleep(Duration::from_millis(120)).await;

    // the official tool waits for the device's input report (completion
    // signal) before issuing GetFeature
    let mut irq = [0u8; 32];
    match tokio::time::timeout(Duration::from_millis(1500), dev.read(&mut irq)).await {
        Ok(Ok(n)) => println!("input report: {} bytes: {:02x?}", n, &irq[..n]),
        Ok(Err(e)) => println!("input read ERR: {:?}", e),
        Err(_) => println!("input read TIMEOUT (no completion signal)"),
    }

    let mut resp = vec![0u8; 520];
    match tokio::time::timeout(
        Duration::from_millis(2500),
        dev.get_feature_report(&mut resp),
    )
    .await
    {
        Ok(Ok(_n)) => {
            println!("response: {} bytes", resp.len());
            println!("rid={:02x} chk={:02x} cmd={:02x} sub={:02x} pages={:02x} idx={:02x} len={:02x}{:02x}",
                resp[0], resp[1], resp[2], resp[3], resp[4], resp[5], resp[6], resp[7]);
            std::fs::write(&out_path, &resp).expect("write output");
            println!("saved to {}", out_path);
        }
        Ok(Err(e)) => eprintln!("get ERR: {:?}", e),
        Err(_) => eprintln!("get TIMEOUT"),
    }
}
