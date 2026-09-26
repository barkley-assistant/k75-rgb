//! K75 probe: send SET_REPORT (feature or output) and GET_REPORT.
//! Usage: hidra_probe [--output] <hex bytes...>
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let use_output = args.first().map(|s| s == "--output").unwrap_or(false);
    let hex_args: Vec<&str> = if use_output {
        args[1..].iter().map(|s| s.as_str()).collect()
    } else {
        args.iter().map(|s| s.as_str()).collect()
    };

    if hex_args.is_empty() {
        eprintln!("usage: hidra_probe [--output] <hex bytes...>");
        std::process::exit(1);
    }

    let payload: Vec<u8> = hex_args
        .iter()
        .map(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0))
        .collect();
    eprintln!(
        "payload: {:02x?} ({} bytes) output={}",
        payload,
        payload.len(),
        use_output
    );

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
        for info in api.device_list() {
            if info.vendor_id() == 0x0603 && info.product_id() == 0x1020 {
                path = info.path().to_string();
                eprintln!("using ISP device: {}", path);
                break;
            }
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

    if use_output {
        eprintln!("sending SET_OUTPUT...");
        match tokio::time::timeout(Duration::from_secs(5), dev.write(&payload)).await {
            Ok(Ok(n)) => eprintln!("ACCEPTED ({} bytes)", n),
            Ok(Err(e)) => eprintln!("ERR: {:?}", e),
            Err(_) => eprintln!("TIMEOUT"),
        }
    } else {
        eprintln!("sending SET_FEATURE...");
        match tokio::time::timeout(Duration::from_secs(5), dev.send_feature_report(&payload)).await
        {
            Ok(Ok(())) => eprintln!("ACCEPTED"),
            Ok(Err(e)) => eprintln!("ERR: {:?}", e),
            Err(_) => eprintln!("TIMEOUT"),
        }
    }

    let mut buf = vec![0x06u8; 1032];
    match tokio::time::timeout(Duration::from_secs(2), dev.get_feature_report(&mut buf)).await {
        Ok(Ok(n)) => eprintln!("GET 0x06: {:02x?} ({} bytes)", &buf[..n.min(32)], n),
        Ok(Err(e)) => eprintln!("GET 0x06 ERR: {:?}", e),
        Err(_) => eprintln!("GET 0x06 TIMEOUT"),
    }
}
