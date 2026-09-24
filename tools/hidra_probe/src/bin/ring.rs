//! Ring-channel probe (layout traced to disasm):
//! SET report 0x09 data lands at XDATA 0x07FA+n (fcn.0000b0a8), GET 0x09 returns 0x08FA-0x0901 = stream offsets 256..263.
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

async fn get9(dev: &hidra::HidDevice<hidra::NusbDevice>) -> Vec<u8> {
    let mut b = vec![0x09u8; 16];
    match tokio::time::timeout(Duration::from_millis(1200), dev.get_feature_report(&mut b)).await {
        Ok(Ok(n)) => b[..n].to_vec(),
        _ => vec![0xEE],
    }
}

async fn set9(dev: &hidra::HidDevice<hidra::NusbDevice>, data: Vec<u8>) -> String {
    let mut p = vec![0x09u8];
    p.extend(data);
    match tokio::time::timeout(Duration::from_millis(1500), dev.send_feature_report(&p)).await {
        Ok(Ok(())) => "ACK".to_string(),
        Ok(Err(e)) => format!("ERR {:?}", e),
        Err(_) => "TIMEOUT".to_string(),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut api = match Hidra::<Nusb>::builder().build() {
        Ok(a) => a, Err(e) => { eprintln!("init ERR: {:?}", e); return; }
    };
    if let Err(e) = api.refresh_devices() { eprintln!("refresh ERR: {:?}", e); return; }
    let mut path = String::new();
    for info in api.device_list() {
        if info.vendor_id() == 0x258a && info.product_id() == 0x019d && info.interface_number() == 1 {
            path = info.path().to_string(); break;
        }
    }
    if path.is_empty() { eprintln!("device not found"); return; }
    let dev = match api.open_path(&path).wait() { Ok(d) => d, Err(e) => { eprintln!("open ERR: {:?}", e); return; } };

    println!("BEFORE GET 0x09: {:02x?}", get9(&dev).await);

    // TEST A: zeros + command byte 0x25 at [0] + marker at stream offsets 256..263 (wLength=264)
    let mut a = vec![0u8; 264];
    a[0] = 0x25;
    a[256..264].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE]);
    println!("SET A (264B, cmd=25, marker@256): {}", set9(&dev, a).await);
    println!("AFTER  GET 0x09: {:02x?}", get9(&dev).await);

    // TEST B: full 520B of 0xFF -> LED blast candidate
    println!("SET B (520B x FF): {}", set9(&dev, vec![0xFFu8; 520]).await);
    println!("AFTER  GET 0x09: {:02x?}", get9(&dev).await);
}
