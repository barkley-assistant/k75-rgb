//! K75 lighting CLI — a thin, safe front-end over the `hidra_probe` library.
//!
//! Subcommands:
//!
//!   matrix baseline           build a uniform red frame (dry run)
//!   matrix slot <0..125>      highlight one slot green against red (dry run)
//!     --send                  actually send via USB
//!     --repeat 1..20          resend at --interval ms (keeps keys lit)
//!     --interval 100..10000   ms between frames (default 500)
//!   map                       print the slot -> physical-key hypothesis table
//!   effect <index>            register block `0x5A 0xAC <index>` (dry run)
//!     --trace                 send the TRACED-ONLY effect-index write
//!
//! `--send`/`--trace` are the only paths that touch the device. Everything
//! else is offline. The `effect` command is firmware-traced but NOT visually
//! verified: sending it requires an observer and explicit `--trace`.

use hidra::{Hidra, MaybeFuture, Nusb};
use hidra_probe::{
    predict_key, EffectIndexWrite, MatrixFrame, Rgb, INTERFACE, MATRIX_SLOTS, PID, VID,
};
use std::error::Error;
use std::time::Duration;

#[derive(Debug)]
enum Command {
    Matrix {
        highlight: Option<usize>,
        send: bool,
        repeat: usize,
        interval_ms: u64,
    },
    Map,
    Effect {
        index: u8,
        trace: bool,
    },
}

fn parse() -> Result<Command, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [sub, rest @ ..] if sub == "matrix" => parse_matrix(rest),
        [sub] if sub == "map" => Ok(Command::Map),
        [sub, rest @ ..] if sub == "effect" => parse_effect(rest),
        _ => Err(usage()),
    }
}

fn parse_matrix(rest: &[String]) -> Result<Command, String> {
    let mut highlight = None;
    let mut send = false;
    let mut repeat = 1usize;
    let mut interval_ms = 500u64;

    let mut it = rest.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "baseline" => highlight = None,
            "slot" => {
                let idx = it
                    .next()
                    .ok_or("slot requires an index")?
                    .parse::<usize>()
                    .map_err(|_| "slot index must be a number")?;
                if idx >= MATRIX_SLOTS {
                    return Err(format!("slot must be in 0..{}", MATRIX_SLOTS - 1));
                }
                highlight = Some(idx);
            }
            "--send" => send = true,
            "--repeat" => {
                repeat = it
                    .next()
                    .ok_or("--repeat requires a count")?
                    .parse::<usize>()
                    .map_err(|_| "repeat must be a number")?;
                if !(1..=20).contains(&repeat) {
                    return Err("repeat must be in 1..=20".into());
                }
            }
            "--interval" => {
                interval_ms = it
                    .next()
                    .ok_or("--interval requires milliseconds")?
                    .parse::<u64>()
                    .map_err(|_| "interval must be a number")?;
                if !(100..=10_000).contains(&interval_ms) {
                    return Err("interval must be in 100..=10000 ms".into());
                }
            }
            other => return Err(format!("unknown matrix argument: {other}")),
        }
    }
    Ok(Command::Matrix {
        highlight,
        send,
        repeat,
        interval_ms,
    })
}

fn parse_effect(rest: &[String]) -> Result<Command, String> {
    let mut index = None;
    let mut trace = false;
    let mut it = rest.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--trace" => trace = true,
            other => {
                let parsed = if let Some(hex) = other.strip_prefix("0x") {
                    u8::from_str_radix(hex, 16)
                } else {
                    other.parse::<u8>()
                };
                index = Some(parsed.map_err(|_| format!("invalid effect index: {other}"))?);
            }
        }
    }
    let index = index.ok_or("effect requires an index (0..=0x13)")?;
    Ok(Command::Effect { index, trace })
}

fn usage() -> String {
    "\
usage:
  k75 matrix baseline [--send [--repeat 1..20] [--interval 100..10000]]
  k75 matrix slot <0..125> [--send [--repeat 1..20] [--interval 100..10000]]
  k75 effect <0..0x13> [--trace]
"
    .to_string()
}

fn open_device() -> Result<Hidra<Nusb>, String> {
    let mut api = Hidra::<Nusb>::builder()
        .build()
        .map_err(|e| e.to_string())?;
    api.refresh_devices().map_err(|e| e.to_string())?;
    Ok(api)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let cmd = parse()?;
    match cmd {
        Command::Matrix {
            highlight,
            send,
            repeat,
            interval_ms,
        } => run_matrix(highlight, send, repeat, interval_ms).await?,
        Command::Map => run_map()?,
        Command::Effect { index, trace } => run_effect(index, trace).await?,
    }
    Ok(())
}

fn run_map() -> Result<(), Box<dyn Error>> {
    println!("slot -> physical key (hypothesis: host slot = vendor LED ID)");
    println!("verified live: 0 -> Esc, 63 -> ';'  |  everything else unverified");
    println!("{}", "-".repeat(46));
    for slot in 0..MATRIX_SLOTS {
        let kind = if slot < 96 {
            match predict_key(slot) {
                Some(key) => key.to_string(),
                None => "GAP".to_string(),
            }
        } else {
            "NON-KEY".to_string()
        };
        let mark = if slot == 0 || slot == 63 {
            "  <verified"
        } else {
            ""
        };
        println!("  {slot:>3}  {kind:<10}{mark}");
    }
    println!("{}", "-".repeat(46));
    println!("GAP = structural hole in the 16x6 key grid (no key there)");
    println!("NON-KEY = slot beyond the vendor table (physical meaning unproven)");
    Ok(())
}

async fn run_matrix(
    highlight: Option<usize>,
    send: bool,
    repeat: usize,
    interval_ms: u64,
) -> Result<(), Box<dyn Error>> {
    let frame = MatrixFrame::new(Rgb::RED, highlight.map(|s| (s, Rgb::GREEN)))
        .ok_or("invalid highlight slot")?;

    println!("report 0x09 / command 0x08 matrix frame:");
    println!(
        "  slots: {MATRIX_SLOTS} RGB (report offsets 8..385), {} bytes",
        frame.as_bytes().len()
    );
    if let Some(slot) = highlight {
        println!("  highlight slot {slot} green against red baseline");
        match predict_key(slot) {
            Some(key) => println!("  predicted key: {key} (unverified unless slot 0 or 63)"),
            None if slot < 96 => println!("  predicted: GAP (no key at this position)"),
            None => println!("  predicted: NON-KEY slot (outside vendor table)"),
        }
    } else {
        println!("  uniform red baseline");
    }
    println!("  first 16 bytes: {:02x?}", &frame.as_bytes()[..16]);

    if !send {
        println!("dry run (no USB access). add --send to write.");
        return Ok(());
    }

    let api = open_device()?;
    let path = api
        .device_list()
        .find(|i| {
            i.vendor_id() == VID && i.product_id() == PID && i.interface_number() == INTERFACE
        })
        .ok_or("K75 interface 1 not found")?
        .path()
        .to_string();
    let dev = api.open_path(&path).wait()?;

    for n in 0..repeat {
        tokio::time::timeout(
            Duration::from_secs(3),
            dev.send_feature_report(frame.as_bytes()),
        )
        .await??;
        println!("frame {}/{} accepted by transport", n + 1, repeat);
        if n + 1 < repeat {
            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
        }
    }
    println!(
        "note: this command is transient (~2 s per frame); repeat keeps keys lit while streaming (interval {interval_ms} ms)."
    );
    Ok(())
}

async fn run_effect(index: u8, trace: bool) -> Result<(), Box<dyn Error>> {
    let block = EffectIndexWrite { index }.to_block();
    println!("effect-index register block (TRACED ONLY, not visually verified):");
    println!("  block: {:02x?}", &block[..3]);
    println!("  remaining 17 bytes zero-filled");
    if !trace {
        println!("dry run. sending this writes 0x0F3F (effect index); pass --trace to send.");
        return Ok(());
    }
    // The register block is streamed via a byte-stream ingress (0x0EF6 ->
    // 0x1130), NOT the 0x08 matrix report. The exact carrier report for this
    // path is not yet pinned to a verified command, so this is intentionally
    // conservative: it refuses to send until that carrier is proven.
    return Err(
        "effect-index register block ingress is traced but its carrier report is not yet pinned; refusing to send an unverified packet"
            .into(),
    );
}
