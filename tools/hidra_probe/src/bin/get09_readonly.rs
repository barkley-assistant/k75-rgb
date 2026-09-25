//! Read the report-0x09 feature response without sending a feature report.
use hidra::{Hidra, MaybeFuture, Nusb};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut api = Hidra::<Nusb>::builder().build()?;
    api.refresh_devices()?;
    let path = api
        .device_list()
        .find(|info| {
            info.vendor_id() == 0x258a
                && info.product_id() == 0x019d
                && info.interface_number() == 1
        })
        .ok_or("K75 interface 1 not found")?
        .path()
        .to_string();
    let dev = api.open_path(&path).wait()?;
    let mut response = vec![0u8; 520];
    response[0] = 0x09;
    let received = tokio::time::timeout(
        Duration::from_secs(3),
        dev.get_feature_report(&mut response),
    )
    .await??;
    println!(
        "received {received} bytes: {:02x?}",
        &response[..received.min(32)]
    );
    Ok(())
}
