//! Lighting command via report 0x09 (all traced):
//! payload [cmd, a0, a1, 0, 0, ...] -> 0x0F54/0x0F55/0x0F56 command block -> mode dispatcher (0x01/0x25/0x35/0x45/0x55 = modes 0-4).
//! Total SET size 519 = the real feature length (descriptor "519-count"; 520 NAKs).
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

async fn get9(dev: &hidra::HidDevice<hidra::NusbDevice>) -> Vec<u8> {
    let mut b = vec![0x09u8; 16];
    match tokio::time::timeout(Duration::from_millis(1200), dev.get_feature_report(&mut b)).await {
        Ok(Ok(n)) => b[..n].to_vec(),
        _ => vec![0xEE],
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let bytes: Vec<u8> = args
        .iter()
        .map(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0))
        .collect();

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

    // 519-byte payload, [cmd a0 a1 0 0] front, zero pad
    let mut data = vec![0u8; 519];
    for (i, b) in bytes.iter().enumerate() {
        if i < 519 {
            data[i] = *b;
        }
    }
    let mut p = vec![0x09u8];
    p.extend_from_slice(&data);
    println!(
        "sending SET 0x09 (519B): [{:02x?}] ...",
        &data[..5.min(data.len())]
    );
    match tokio::time::timeout(Duration::from_millis(2000), dev.send_feature_report(&p)).await {
        Ok(Ok(())) => println!("SET ACCEPTED"),
        Ok(Err(e)) => println!("SET ERR {:?}", e),
        Err(_) => println!("SET TIMEOUT"),
    }
    println!("readback GET 0x09: {:02x?}", get9(&dev).await);
}
