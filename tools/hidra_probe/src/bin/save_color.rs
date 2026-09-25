//! Full POC sequence: set color (RAM) + SAVE to flash (persistence).
//!   1. cmd 0x0a (per-key write) with RGB filled  -> fcn.00007108
//!   2. cmd 0x0b (apply)                          -> 0x7b40 -> fcn.000029cd
//!   3. cmd 0x06 (SAVE to flash)                  -> fcn.00007393 (op 0x56, ~380B commit)
//! Usage: save_color <R> <G> <B>
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

async fn get9(dev: &hidra::HidDevice<hidra::NusbDevice>) -> Vec<u8> {
    let mut b = vec![0x09u8; 16];
    match tokio::time::timeout(Duration::from_millis(1500), dev.get_feature_report(&mut b)).await {
        Ok(Ok(n)) => b[..n.min(16)].to_vec(),
        _ => vec![0xEE],
    }
}

async fn send9(dev: &hidra::HidDevice<hidra::NusbDevice>, data: &[u8]) -> String {
    let mut p = vec![0x09u8];
    p.extend_from_slice(data);
    match tokio::time::timeout(Duration::from_millis(3000), dev.send_feature_report(&p)).await {
        Ok(Ok(())) => "ACK".to_string(),
        Ok(Err(e)) => format!("ERR {:?}", e),
        Err(_) => "TIMEOUT".to_string(),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    if args.len() < 3 {
        eprintln!("usage: save_color <R> <G> <B>");
        return;
    }
    let r = hex(&args[0]);
    let g = hex(&args[1]);
    let b = hex(&args[2]);

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

    println!("before: {:02x?}", get9(&dev).await);

    // 1. per-key write
    let mut frame = vec![0u8; 519];
    frame[0] = 0x0a;
    let mut i = 1;
    while i + 2 < 519 {
        frame[i] = r;
        frame[i + 1] = g;
        frame[i + 2] = b;
        i += 3;
    }
    println!("1) cmd 0x0a per-key write -> {}", send9(&dev, &frame).await);
    tokio::time::sleep(Duration::from_millis(150)).await;

    // 2. apply
    let mut apply = vec![0u8; 519];
    apply[0] = 0x0b;
    println!("2) cmd 0x0b apply        -> {}", send9(&dev, &apply).await);
    tokio::time::sleep(Duration::from_millis(150)).await;

    // 3. SAVE to flash
    let mut save = vec![0u8; 519];
    save[0] = 0x06;
    println!("3) cmd 0x06 SAVE flash   -> {}", send9(&dev, &save).await);
    // give the flash write time to commit
    tokio::time::sleep(Duration::from_millis(800)).await;

    println!("after: {:02x?}", get9(&dev).await);
    println!(
        "DONE — set {}{}{} and saved. Unplug/replug to verify persistence.",
        args[0], args[1], args[2]
    );
}
