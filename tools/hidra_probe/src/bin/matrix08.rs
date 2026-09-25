//! Volatile, complete report-0x09 / command-0x08 matrix experiment.
//!
//! Firmware path: 0x8906 -> 0x8765 -> 0x7253 -> 0xB0A8 -> 0x7A72 ->
//! 0x7B35 -> 0x7108. This is not a physical key map. No save command is sent.
use hidra::{Hidra, MaybeFuture, Nusb};
use std::error::Error;
use std::time::Duration;

const REPORT_LEN: usize = 520;
const MATRIX_START: usize = 8;
const MATRIX_SLOTS: usize = 21 * 6;
const RED: [u8; 3] = [0xff, 0, 0];
const GREEN: [u8; 3] = [0, 0xff, 0];

fn frame(green_slot: Option<usize>) -> Result<[u8; REPORT_LEN], String> {
    if let Some(slot) = green_slot {
        if slot >= MATRIX_SLOTS {
            return Err(format!("slot must be in 0..{}", MATRIX_SLOTS - 1));
        }
    }
    let mut report = [0u8; REPORT_LEN];
    report[0] = 0x09;
    report[1] = 0x08;
    for slot in 0..MATRIX_SLOTS {
        let start = MATRIX_START + slot * 3;
        report[start..start + 3].copy_from_slice(if Some(slot) == green_slot {
            &GREEN
        } else {
            &RED
        });
    }
    Ok(report)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (green_slot, repeat) = match args.as_slice() {
        [mode] if mode == "baseline" => (None, 1),
        [mode, index] if mode == "slot" => (Some(index.parse::<usize>()?), 1),
        [mode, flag] if mode == "baseline" && flag == "--send" => (None, 1),
        [mode, index, flag] if mode == "slot" && flag == "--send" => {
            (Some(index.parse::<usize>()?), 1)
        }
        [mode, flag, repeat_flag, count]
            if mode == "baseline" && flag == "--send" && repeat_flag == "--repeat" =>
        {
            (None, count.parse::<usize>()?)
        }
        [mode, index, flag, repeat_flag, count]
            if mode == "slot" && flag == "--send" && repeat_flag == "--repeat" =>
        {
            (Some(index.parse::<usize>()?), count.parse::<usize>()?)
        }
        _ => {
            return Err("usage: matrix08 baseline|slot <0..125> [--send [--repeat 1..20]]".into());
        }
    };
    if !(1..=20).contains(&repeat) {
        return Err("repeat count must be in 1..=20".into());
    }
    let send = args.iter().any(|arg| arg == "--send");
    let report = frame(green_slot)?;
    println!(
        "report 0x09 / RAM matrix command 0x08; {} RGB slots, {} bytes; {}",
        MATRIX_SLOTS,
        report.len(),
        if send {
            "sending"
        } else {
            "dry run (no USB access)"
        }
    );
    if let Some(slot) = green_slot {
        println!(
            "green slot {slot} at report offsets {}..{}",
            MATRIX_START + slot * 3,
            MATRIX_START + slot * 3 + 2
        );
    }
    println!("first 32 bytes: {:02x?}", &report[..32]);
    if !send {
        return Ok(());
    }

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
    for index in 0..repeat {
        tokio::time::timeout(Duration::from_secs(3), dev.send_feature_report(&report)).await??;
        println!(
            "transport accepted frame {}/{}; visual effect still requires observation",
            index + 1,
            repeat
        );
        if index + 1 < repeat {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    let mut response = [0u8; REPORT_LEN];
    response[0] = 0x09;
    match tokio::time::timeout(
        Duration::from_secs(3),
        dev.get_feature_report(&mut response),
    )
    .await
    {
        Ok(Ok(n)) => println!("readback ({n} bytes): {:02x?}", &response[..n.min(16)]),
        Ok(Err(err)) => eprintln!("readback failed: {err}"),
        Err(_) => eprintln!("readback timed out"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_matrix_and_header() {
        let report = frame(None).unwrap();
        assert_eq!(&report[..8], &[0x09, 0x08, 0, 0, 0, 0, 0, 0]);
        assert!(report[8..386].chunks_exact(3).all(|rgb| rgb == RED));
        assert!(report[386..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn a_slot_changes_only_its_triple() {
        let baseline = frame(None).unwrap();
        for slot in [0, 5, 6, 63, 125] {
            let candidate = frame(Some(slot)).unwrap();
            let differences: Vec<usize> = (0..REPORT_LEN)
                .filter(|&i| candidate[i] != baseline[i])
                .collect();
            assert_eq!(differences, vec![8 + slot * 3, 9 + slot * 3]);
            assert_eq!(&candidate[8 + slot * 3..11 + slot * 3], &GREEN);
        }
        assert!(frame(Some(MATRIX_SLOTS)).is_err());
    }
}
