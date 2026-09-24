//! cfgwrite — write a config image through the verified config channel.
//!
//! Usage: cfgwrite <config.bin>
//!
//! Writes the file contents into the report-0x09 payload via cmd 0x0a
//! (config write, op 0x54) + cmd 0x0b (apply). The image lands in the
//! config region that drives the lighting state (see
//! analysis/config-layout-hypothesis.md). WARNING: malformed images can
//! wedge the lighting engine; Fn+Esc (hold ~3s) recovers. Do NOT send
//! cmd 0x06 (save) until the image is verified visually.
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

async fn send9(dev: &hidra::HidDevice<hidra::NusbDevice>, data: &[u8]) -> String {
    let mut p = vec![0u8; 520];
    p[0] = 0x09;
    p[1..1 + data.len()].copy_from_slice(data);
    match tokio::time::timeout(Duration::from_millis(2500), dev.send_feature_report(&p)).await {
        Ok(Ok(())) => "OK".to_string(),
        Ok(Err(e)) => format!("ERR {:?}", e),
        Err(_) => "TIMEOUT".to_string(),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: cfgwrite <config.bin>");
        return;
    }
    let image = match std::fs::read(&args[1]) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("read {}: {}", args[1], e);
            return;
        }
    };
    eprintln!("config image: {} bytes", image.len());

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
        if info.vendor_id() == 0x258a && info.product_id() == 0x019d && info.interface_number() == 1 {
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

    // cmd 0x0a: config write — image at payload[1..]
    let mut frame = vec![0u8; 519];
    frame[0] = 0x0a;
    let n = image.len().min(518);
    frame[1..1 + n].copy_from_slice(&image[..n]);
    println!("cmd 0x0a (config write, {} bytes) -> {}", n, send9(&dev, &frame).await);
    tokio::time::sleep(Duration::from_millis(150)).await;

    // cmd 0x0b: apply
    let mut apply = vec![0u8; 519];
    apply[0] = 0x0b;
    println!("cmd 0x0b (apply) -> {}", send9(&dev, &apply).await);

    eprintln!("done. check the keyboard. if wedged: Fn+Esc hold ~3s.");
}