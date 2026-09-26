//! Repeat-and-hold lighting tests for the RedThunder K75.
//!
//! Every packet here traces to a decoded instruction in analysis/disasm_v2.txt.
//! See docs/architecture.md for the derivation of each.
//!
//! Each test loops indefinitely so the keyboard can be watched while it runs,
//! and prints a prompt line between iterations.
//!
//! Tests:
//!   effect <op>      repeat one effect op from the 0x5A/0xF0/0x07/op block
//!   stageapply       the stage-then-apply pair on report 0x09
//!   mode <n>         cycle the 0x0F54 -> 0x0F22 mode chain
//!   colour <R> <G> <B>  the known-good M1 colour path (control)

use hidra::{Hidra, MaybeFuture, Nusb};
use std::io::{Read, Write};
use std::time::Duration;

const HOLD_MS: u64 = 1200;

fn checksum(block: &[u8]) -> u8 {
    // fcn.0000d210: r5 walks 1..r7 summing [0x11C1 + r5], i.e. 0x11C2..0x11C6.
    0xFFu8.wrapping_sub(block.iter().fold(0u8, |acc, b| acc.wrapping_add(*b)))
}

fn open_dev(api: &Hidra<Nusb>) -> hidra::HidDevice<hidra::NusbDevice> {
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
        std::process::exit(1);
    }
    api.open_path(&path).wait().expect("open")
}

/// Print a live count and wait, so the user can watch the keyboard meanwhile.
async fn hold(tick: usize) {
    print!("\r    [{}] watching... (Ctrl-C to stop) ", tick);
    std::io::stdout().flush().ok();
    tokio::time::sleep(Duration::from_millis(HOLD_MS)).await;
}

async fn send9(dev: &hidra::HidDevice<hidra::NusbDevice>, data: &[u8]) -> String {
    let mut p = vec![0x09u8];
    p.extend_from_slice(data);
    match tokio::time::timeout(Duration::from_millis(2000), dev.send_feature_report(&p)).await {
        Ok(Ok(())) => "ACK".to_string(),
        Ok(Err(e)) => format!("ERR {:?}", e),
        Err(_) => "TIMEOUT".to_string(),
    }
}

async fn send6(dev: &hidra::HidDevice<hidra::NusbDevice>, blk: &[u8], reg: u8) -> String {
    let mut p = vec![0u8; 1031];
    p[0] = 0x06;
    p[1] = 0x53;
    p[2] = 0x01;
    p[3] = reg;
    p[4..4 + blk.len()].copy_from_slice(blk);
    match tokio::time::timeout(Duration::from_millis(2000), dev.send_feature_report(&p)).await {
        Ok(Ok(())) => "ACK".to_string(),
        Ok(Err(e)) => format!("ERR {:?}", e),
        Err(_) => "TIMEOUT".to_string(),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: livetest <effect|stageapply|mode|colour> [args...]");
        return;
    }
    let hex = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0);

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
    let dev = open_dev(&api);

    match args[0].as_str() {
        // ---------------------------------------------------------------
        // effect: repeat one decoded effect op, forever.
        //   block from fcn.0000baa2: 5A F0 07 <op> [01] [01] <cksum>
        // ---------------------------------------------------------------
        "effect" => {
            let op = hex(args.get(1).map(|s| s.as_str()).unwrap_or("10"));
            let f5: u8 = if args.len() > 2 { hex(&args[2]) } else { 1 };
            let f3: u8 = if args.len() > 3 { hex(&args[3]) } else { 1 };
            let mut blk = [0x5A, 0xF0, 0x07, op, 0, 0, 0];
            blk[4] = if f5 != 0 { 0x01 } else { 0x00 };
            blk[5] = if f3 != 0 { 0x01 } else { 0x00 };
            blk[6] = checksum(&blk[1..6]);
            println!(
                "REPEATING effect op 0x{:02x} flags r5={} r3={}  block={:02x?}",
                op, f5, f3, blk
            );
            println!("QUESTION: does repeating this change the lighting? (y/n)");
            let mut tick = 0usize;
            loop {
                tick += 1;
                let r = send6(&dev, &blk, 0x00).await;
                if tick == 1 || tick % 10 == 0 {
                    println!("\n    iteration {} -> {}", tick, r);
                }
                hold(tick).await;
            }
        }

        // ---------------------------------------------------------------
        // stageapply: stage then apply, per docs/architecture.md.
        //   stage: report 0x09 cmd 0x13, r5=2 (call site 0x02e2)
        //   then the 19-byte config table with its checksum.
        // ---------------------------------------------------------------
        "stageapply" => {
            println!("REPEATING stage+apply pair (report 0x09)");
            println!("QUESTION: does applying the staged table change the lighting? (y/n)");
            let mut tick = 0usize;
            loop {
                tick += 1;
                // stage: 19-byte table. Only the fields the decoder pins are
                // set; the rest stay zero. Magic + length + arm byte.
                let mut t = [0u8; 19];
                t[0] = 0x5A; // magic (fcn.0000baa2 / b7d2)
                t[1] = 0x01; // value
                t[2] = 0x05; // REG + 5 (fcn.0000b7d2)
                t[4] = 0x01; // the byte split into 0x0EEA/0x0EEB nibbles
                t[19 - 1] = checksum(&t[1..18]); // trailing checksum, xrl'd at 0x23ac
                let r = send9(&dev, &t).await;
                if tick == 1 || tick % 10 == 0 {
                    println!("\n    iteration {} stage -> {}", tick, r);
                }
                hold(tick).await;

                // apply: command byte 0x13 (the value call site 0x02e2 emits)
                let ap = [0x13u8];
                let r2 = send9(&dev, &ap).await;
                if tick == 1 || tick % 10 == 0 {
                    println!("    iteration {} apply -> {}", tick, r2);
                }
                hold(tick).await;
            }
        }

        // ---------------------------------------------------------------
        // mode: cycle through the four decoded mode latch values.
        // ---------------------------------------------------------------
        "mode" => {
            println!("CYCLING mode values 0x0F22 <- 0x00/0x01/0x02/0x03");
            println!("QUESTION: does any value produce a different effect? (y/n + which)");
            let mut tick = 0usize;
            loop {
                for n in 0u8..4 {
                    tick += 1;
                    let r = send6(&dev, &[n], 0x00).await;
                    if tick <= 8 || tick % 8 == 0 {
                        println!("\n    mode {} -> {}", n, r);
                    }
                    hold(tick).await;
                }
            }
        }

        // ---------------------------------------------------------------
        // colour: the known-good M1 path, as a control that the harness works.
        // ---------------------------------------------------------------
        "colour" => {
            let r = hex(args.get(1).map(|s| s.as_str()).unwrap_or("ff"));
            let g = hex(args.get(2).map(|s| s.as_str()).unwrap_or("00"));
            let b = hex(args.get(3).map(|s| s.as_str()).unwrap_or("00"));
            println!(
                "REPEATING colour ff{:02x}{:02x} (control, known-good path)",
                g, b
            );
            println!("QUESTION: does the keyboard turn red? (y/n)");
            let mut tick = 0usize;
            loop {
                tick += 1;
                let mut frame = vec![0u8; 519];
                frame[0] = 0x0a;
                let mut i = 1;
                while i + 2 < 519 {
                    frame[i] = r;
                    frame[i + 1] = g;
                    frame[i + 2] = b;
                    i += 3;
                }
                let s1 = send9(&dev, &frame).await;
                let mut apply = vec![0u8; 519];
                apply[0] = 0x0b;
                let s2 = send9(&dev, &apply).await;
                if tick == 1 || tick % 5 == 0 {
                    println!("\n    iteration {} write={} apply={}", tick, s1, s2);
                }
                hold(tick * 2).await;
            }
        }

        other => eprintln!("unknown test {:?}", other),
    }

    // Unreachable, but keeps the compiler honest about the Read import.
    let _ = std::io::stdin().read(&mut [0u8; 0]);
}
