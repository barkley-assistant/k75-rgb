//! K75 lighting CLI — a thin, safe front-end over the `hidra_probe` library.
//!
//! Subcommands:
//!
//!   matrix baseline           build a uniform red frame (dry run)
//!   matrix slot <0..125>      highlight one slot green against red (dry run)
//!     --send                  actually send via USB
//!     --repeat 1..20          resend at --interval ms (keeps keys lit)
//!     --interval 100..10000   ms between frames (default 500)
//!   save <RRGGBB|R G B>       VERIFIED colour write + apply + flash save
//!     --pattern <file>         per-key pattern instead of uniform (one
//!                              RRGGBB per line, slot order, traced layout)
//!     --send                   actually send the 3-step sequence via USB
//!   map                       print the slot -> physical-key hypothesis table
//!   effect <index>            register block `0x5A 0xAC <index>` (dry run)
//!     --trace                 send the TRACED-ONLY effect-index write
//!
//! `--send`/`--trace` are the only paths that touch the device. Everything
//! else is offline. The `effect` command is firmware-traced but NOT visually
//! verified: sending it requires an observer and explicit `--trace`.
//! `save` writes flash — verify the staged colour on the keys before
//! letting it save (the sequence is the user-verified 2026-09-24 POC).

use hidra::{Hidra, MaybeFuture, Nusb};
use hidra_probe::{
    predict_key, ApplyFrame, EffectIndexWrite, MatrixFrame, PerKeyColorFrame, Rgb, SaveFlashFrame,
    INTERFACE, MATRIX_SLOTS, PID, VID,
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
    Save {
        color: Rgb,
        pattern: Option<[Rgb; MATRIX_SLOTS]>,
        send: bool,
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
        [sub, rest @ ..] if sub == "save" => parse_save(rest),
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

fn parse_save(rest: &[String]) -> Result<Command, String> {
    let mut send = false;
    let mut color: Option<Rgb> = None;
    let mut pattern_path: Option<String> = None;
    let mut it = rest.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--send" => send = true,
            "--pattern" => {
                pattern_path = Some(it.next().ok_or("--pattern needs a file path")?.to_string());
            }
            other => {
                let byte = |s: &str| u8::from_str_radix(s.trim_start_matches("0x"), 16);
                if color.is_some() {
                    return Err(format!("unexpected extra argument: {other}"));
                }
                if other.len() == 6 && other.chars().all(|c| c.is_ascii_hexdigit()) {
                    // single RRGGBB hex argument
                    color = Some(Rgb::new(
                        byte(&other[0..2]).map_err(|_| "bad red byte")?,
                        byte(&other[2..4]).map_err(|_| "bad green byte")?,
                        byte(&other[4..6]).map_err(|_| "bad blue byte")?,
                    ));
                } else {
                    // three separate hex bytes: R G B
                    let r = byte(other).map_err(|_| format!("bad red byte: {other}"))?;
                    let g = byte(
                        it.next()
                            .ok_or("colour needs G and B bytes (or one RRGGBB)")?,
                    )
                    .map_err(|_| "bad green byte")?;
                    let b = byte(it.next().ok_or("colour needs a B byte")?)
                        .map_err(|_| "bad blue byte")?;
                    color = Some(Rgb::new(r, g, b));
                }
            }
        }
    }

    let pattern = match pattern_path {
        Some(path) => Some(load_pattern(&path)?),
        None => None,
    };

    let color = match (&pattern, color) {
        (Some(_), _) => Rgb::OFF, // pattern carries the colours
        (None, Some(c)) => c,
        (None, None) => {
            return Err("save requires a colour (RRGGBB or R G B hex) or --pattern <file>".into())
        }
    };
    Ok(Command::Save {
        color,
        pattern,
        send,
    })
}

/// Load a pattern file: one `RRGGBB` per line, slot order (0..125), `#` comments,
/// missing slots = OFF.
fn load_pattern(path: &str) -> Result<[Rgb; MATRIX_SLOTS], String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    let mut slots = [Rgb::OFF; MATRIX_SLOTS];
    let mut count = 0usize;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if count >= MATRIX_SLOTS {
            return Err(format!("{path}: more than {MATRIX_SLOTS} colour lines"));
        }
        let hex = line.trim_start_matches("0x");
        if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("{path} line {}: expected RRGGBB", count + 1));
        }
        let byte = |s: &str| u8::from_str_radix(s, 16).map_err(|_| "bad hex");
        slots[count] = Rgb::new(byte(&hex[0..2])?, byte(&hex[2..4])?, byte(&hex[4..6])?);
        count += 1;
    }
    Ok(slots)
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
  k75 save <RRGGBB | R G B> [--send]
  k75 save --pattern <file> [--send]
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
        Command::Save {
            color,
            pattern,
            send,
        } => run_save(color, pattern, send).await?,
        Command::Map => run_map()?,
        Command::Effect { index, trace } => run_effect(index, trace).await?,
    }
    Ok(())
}

fn run_map() -> Result<(), Box<dyn Error>> {
    println!("slot -> physical key (hypothesis: host slot = vendor LED ID)");
    println!(
        "verified live (14 keys): 0 Esc, 7 1, 10 \\|, 14 W, 27 F, 30 F4, 34 V, 35 Space, 51 K, 55 9, 56 O, 63 ;, 84 Delete, 95 Right"
    );
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

async fn run_save(
    color: Rgb,
    pattern: Option<[Rgb; MATRIX_SLOTS]>,
    send: bool,
) -> Result<(), Box<dyn Error>> {
    let write = match pattern {
        Some(slots) => PerKeyColorFrame::from_slots(&slots),
        None => PerKeyColorFrame::new(color),
    };
    let apply = ApplyFrame::new();
    let save = SaveFlashFrame::new();

    println!("verified colour write + apply + flash save (2026-09-24 POC):");
    match pattern {
        Some(slots) => {
            let lit = slots.iter().filter(|c| **c != Rgb::OFF).count();
            println!(
                "  per-key pattern: {lit}/{} slots lit (stride 0x12, slot i @ payload 2+i*3)",
                MATRIX_SLOTS
            );
            println!(
                "  first 6 slots: {}",
                slots[..6]
                    .iter()
                    .map(|c| format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
        None => println!("  colour #{:02x}{:02x}{:02x}", color.r, color.g, color.b),
    }
    println!("  1) cmd 0x0a per-key write  (uniform fill, 172 RGB triples)");
    println!("  2) cmd 0x0b apply           (-> PWM)");
    println!("  3) cmd 0x06 flash save      (op 0x56 commit, ~380 B)");
    println!("  first 16 bytes: {:02x?}", &write.as_bytes()[..16]);

    if !send {
        println!("dry run (no USB access). add --send to write and save.");
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

    println!("1) per-key write -> ",);
    tokio::time::timeout(
        Duration::from_secs(3),
        dev.send_feature_report(write.as_bytes()),
    )
    .await??;
    println!("ACK");
    tokio::time::sleep(Duration::from_millis(150)).await;

    println!("2) apply -> ");
    tokio::time::timeout(
        Duration::from_secs(3),
        dev.send_feature_report(apply.as_bytes()),
    )
    .await??;
    println!("ACK");
    tokio::time::sleep(Duration::from_millis(150)).await;

    println!("3) flash save -> ");
    tokio::time::timeout(
        Duration::from_secs(3),
        dev.send_feature_report(save.as_bytes()),
    )
    .await??;
    println!("ACK");
    tokio::time::sleep(Duration::from_millis(800)).await;
    println!(
        "DONE. {}",
        if pattern.is_some() {
            "keys should now show the pattern."
        } else {
            "keys should now be the staged colour."
        }
    );
    println!("Verify persistence: unplug/replug, and check 2.4 GHz mode too.");
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
