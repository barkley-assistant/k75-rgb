//! Traceable 'R' 'V' probe: SET [06 52 56 <reg>] then GET 0x06 at several lengths.
//! 'R'=0x52 'V'=0x56 decoded from fcn.00005001 @ 0x50b8/0x50c3; reg=1 → fcn.00008a93 (identity).
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

async fn tget(dev: &hidra::HidDevice<hidra::NusbDevice>, len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    buf[0] = 0x06;
    match tokio::time::timeout(
        Duration::from_millis(1200),
        dev.get_feature_report(&mut buf),
    )
    .await
    {
        Ok(Ok(n)) => buf[..n].to_vec(),
        _ => vec![0xEE],
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let reg: u8 = args
        .first()
        .map(|s| u8::from_str_radix(s, 16).unwrap_or(1))
        .unwrap_or(1);

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

    // BEFORE: GET 0x06 at each length
    for len in [4usize, 8, 16, 24, 32] {
        let r = tget(&dev, len).await;
        println!("BEFORE GET len={:2} -> {:02x?}", len, r);
    }

    // SET 'R' 'V' <reg>  (payload [52 56 reg] — buffer gets [06, 52, 56, reg])
    let cmd = vec![0x06u8, 0x52, 0x56, reg];
    match tokio::time::timeout(Duration::from_millis(1500), dev.send_feature_report(&cmd)).await {
        Ok(Ok(())) => println!("SET [06 52 56 {:02x}] ACCEPTED", reg),
        Ok(Err(e)) => println!("SET ERR {:?}", e),
        Err(_) => println!("SET TIMEOUT"),
    }

    // AFTER: GET 0x06 at each length
    for len in [4usize, 8, 16, 24, 32] {
        let r = tget(&dev, len).await;
        let ascii: String = r
            .iter()
            .map(|&b| {
                if (0x20..0x7f).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        println!("AFTER  GET len={:2} -> {:02x?}  '{}'", len, r, ascii);
    }
}
