//! Read-config probe: SET report 0x09 cmd 0x04 (flash read op 0x52),
//! then drain the interrupt-IN endpoint and GET the feature echo.
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
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

    // SET: report 0x09, cmd 0x04, rest zero (firmware reads the config region)
    let mut frame = vec![0u8; 520];
    frame[0] = 0x09;
    frame[1] = 0x04;
    let set_res = tokio::time::timeout(Duration::from_millis(1500), dev.send_feature_report(&frame)).await;
    eprintln!("SET 0x04: {:?}", set_res);

    // drain interrupt-IN
    let mut got = Vec::new();
    for _ in 0..10 {
        let mut buf = vec![0u8; 64];
        let r = tokio::time::timeout(Duration::from_millis(400), dev.read(&mut buf)).await;
        match r {
            Ok(Ok(n)) if n > 0 => got.extend_from_slice(&buf[..n]),
            _ => break,
        }
    }
    eprintln!("IN chunks: {} bytes", got.len());
    for (i, b) in got.iter().enumerate() {
        print!("{:02x} ", b);
        if (i + 1) % 16 == 0 {
            println!();
        }
    }
    println!();

    // GET feature echo — try a full-size buffer in case the read stages more data
    let mut echo = vec![0u8; 520];
    echo[0] = 0x09;
    let get_res = tokio::time::timeout(Duration::from_millis(1500), dev.get_feature_report(&mut echo)).await;
    match get_res {
        Ok(Ok(n)) => {
            eprintln!("GET echo ({} bytes):", n);
            for (i, b) in echo[..n].iter().enumerate() {
                print!("{:02x} ", b);
                if (i + 1) % 16 == 0 {
                    println!();
                }
            }
            println!();
        }
        other => eprintln!("GET echo: {:?}", other),
    }
}