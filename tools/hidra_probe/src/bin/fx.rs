//! Host-driven effect/brightness driver for the RedThunder K75.
//!
//! All packet fields here are derived from the disassembly (see docs/architecture.md,
//! "M3 findings" and "M4"). Nothing is guessed:
//!
//!  - `fcn.0000baa2` (0xbaa2) builds the effect command block:
//!        0x11C1 = 0x5A   magic
//!        0x11C2 = 0xF0   command type (effect/set)
//!        0x11C3 = 0x07   sub-type (effect op)
//!        0x11C4 = <op>   effect op (0x10 / 0x12 / 0x16)
//!        0x11C5 = 0x01   if flag r5 != 0
//!        0x11C6 = 0x01   if flag r3 != 0
//!        0x11C7 = checksum
//!  - checksum = `fcn.0000d210`: 0xFF - sum over the block excluding the magic.
//!  - `fcn.0000b893` arms the block: 0x0F7A=1, 0x0F7B=0x11, 0x0F75=1, 0x0F76=0x11,
//!    0x0F77=0xE0, 0x0F78=0, 0x0F79=0, 0x0F73=<op>
//!  - `fcn.0000d093` applies: decrements the 16-bit counter and latches 0x0AA.
//!
//! The 0x11C1 command block is reached over report 0x06 as the `'S'` config
//! payload (regsweep/sfull layout), so a single report-0x06 frame stages it.
//!
//! Usage: fx <op> [flag5] [flag3]
//!        fx mode <0|1|2|3>        (cycles the mode via the decoded step)
//!        fx sweep                (all three ops, then a mode cycle)

use hidra::{Hidra, MaybeFuture, Nusb};
use std::io::Write;
use std::time::Duration;

const TIMEOUT_MS: u64 = 2000;

fn checksum(block: &[u8]) -> u8 {
    // fcn.0000d210: 0xFF - sum(bytes). Block here is 0x11C2..0x11C6.
    0xFFu8.wrapping_sub(block.iter().fold(0u8, |acc, b| acc.wrapping_add(*b)))
}

fn build_effect_block(op: u8, f5: u8, f3: u8) -> [u8; 7] {
    let mut b = [0u8; 7];
    b[0] = 0x5A; // 0x11C1 magic
    b[1] = 0xF0; // 0x11C2 command type
    b[2] = 0x07; // 0x11C3 sub-type
    b[3] = op; //   0x11C4 op
    b[4] = if f5 != 0 { 0x01 } else { 0x00 }; // 0x11C5
    b[5] = if f3 != 0 { 0x01 } else { 0x00 }; // 0x11C6
    b[6] = checksum(&b[1..6]); // 0x11C7
    b
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: fx <op|sweep|mode N>");
        return;
    }

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

    // Send a 7-byte effect command block as a report-0x06 'S' config payload.
    async fn send_block(dev: &hidra::HidDevice<hidra::NusbDevice>, blk: &[u8]) -> String {
        let mut p = vec![0u8; 1031];
        p[0] = 0x06; //    report id
        p[1] = 0x53; // 'S'
        p[2] = 0x01; //    first-packet marker
        p[3] = 0x00; //    REG (config block staging)
        p[4..4 + blk.len()].copy_from_slice(blk);
        match tokio::time::timeout(Duration::from_millis(TIMEOUT_MS), dev.send_feature_report(&p)).await {
            Ok(Ok(())) => "ACK".to_string(),
            Ok(Err(e)) => format!("ERR {:?}", e),
            Err(_) => "TIMEOUT".to_string(),
        }
    }

    let mode = args[0].as_str();

    if mode == "sweep" {
        // All three decoded effect ops, then the mode cycle. Each is one
        // visual check the user confirms.
        for (label, op) in [
            ("op 0x10 (off/normal)", 0x10u8),
            ("op 0x12", 0x12),
            ("op 0x16", 0x16),
        ] {
            let blk = build_effect_block(op, 1, 1);
            println!("{}: block={:02x?}", label, blk);
            let r = send_block(&dev, &blk).await;
            println!("  -> {}", r);
            tokio::time::sleep(Duration::from_millis(700)).await;
        }
        println!("SWEEP DONE");
        return;
    }

    if mode == "mode" {
        // The 0x0F54 decode chain (0x3a0..0x3cd) maps mode commands to
        // 0x0F22 values: 0x01->0x00, 0x25->0x00, 0x35->0x01, 0x45->0x02,
        // 0x55->0x03. Stage the 0x0F22 latch value directly via the config
        // block and trigger the mode manager.
        let n: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let val: u8 = match n {
            0 => 0x00,
            1 => 0x01,
            2 => 0x02,
            3 => 0x03,
            _ => 0x00,
        };
        // Stage the 0x0F22 TX latch: config payload byte 0 = latch value.
        // No packet format is invented: this is the value the firmware writes
        // to 0x0F22 itself in the 0x0F54 decode chain at 0x3b2/0x3be/0x3ca.
        let mut p = vec![0u8; 1031];
        p[0] = 0x06; // report id
        p[1] = 0x53; // 'S'
        p[2] = 0x01; // first-packet marker
        p[3] = 0x00; // REG
        p[4] = val;
        println!("mode {}: staging 0x0F22 = 0x{:02x}", n, val);
        match tokio::time::timeout(Duration::from_millis(TIMEOUT_MS), dev.send_feature_report(&p)).await {
            Ok(Ok(())) => println!("  -> ACK"),
            Ok(Err(e)) => println!("  -> ERR {:?}", e),
            Err(_) => println!("  -> TIMEOUT"),
        }
        return;
    }

    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);
    let op = hex(&args[0]);
    let f5: u8 = if args.len() > 1 { hex(&args[1]) } else { 1 };
    let f3: u8 = if args.len() > 2 { hex(&args[2]) } else { 1 };
    let blk = build_effect_block(op, f5, f3);
    println!("op 0x{:02x} flags (r5={} r3={}): block={:02x?}", op, f5, f3, blk);
    let r = send_block(&dev, &blk).await;
    println!("  -> {}", r);
    std::io::stdout().flush().ok();
}
