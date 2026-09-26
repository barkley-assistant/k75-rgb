//! `k75d` — tiny JSON-lines daemon exposing the verified K75 operations
//! over a unix socket. This is the API surface the GUI and scripts talk to;
//! it wraps `hidra_probe` (the library), not raw USB.
//!
//! # Safety model
//! - Read-only by default. Writes (`"send": true`) are rejected unless the
//!   daemon was started with `--allow-writes`.
//! - Effect selection is refused unconditionally: the host-side carrier
//!   frame is not yet pinned to a capture (see
//!   `docs/protocol/effect-selection.md`). Fn+Tab does effects natively.
//! - The forbidden report-`0x09` command `0x04` (lights-off hazard) does
//!   not exist in this codebase. Report-`0x05` ISP command `0x45` (flash
//!   erase) is likewise absent.
//!
//! # Protocol (newline-delimited JSON)
//!
//! ```json
//! {"op":"ping"}
//! {"op":"device"}
//! {"op":"map"}
//! {"op":"matrix_baseline","send":false,"repeat":1,"interval_ms":500}
//! {"op":"matrix_slot","slot":30,"send":false}
//! {"op":"save","send":false,"color":"ff0000"}
//! {"op":"save","send":false,"pattern":["ff0000", "00ff00", "…126…"]}
//! ```
//!
//! Every request gets exactly one response line: `{"ok":true,…}` or
//! `{"ok":false,"error":"…"}`.
//!
//! # Usage
//!
//! ```text
//! k75d [--socket /run/k75d.sock] [--allow-writes]
//! ```

use hidra::{HidDevice, Hidra, MaybeFuture, Nusb, NusbDevice};
use hidra_probe::{
    predict_key, ApplyFrame, MatrixFrame, PerKeyColorFrame, Rgb, SaveFlashFrame, INTERFACE,
    KEY_LED_MAP, MATRIX_SLOTS, PID, VID,
};
use serde_json::{json, Value};
use std::error::Error;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

const DEFAULT_SOCKET: &str = "/tmp/k75d.sock";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut socket = DEFAULT_SOCKET.to_string();
    let mut allow_writes = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--socket" => socket = args.next().ok_or("--socket needs a path")?,
            "--allow-writes" => allow_writes = true,
            other => return Err(format!("unknown arg: {other}").into()),
        }
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(serve(&socket, allow_writes))
}

async fn serve(socket: &str, allow_writes: bool) -> Result<(), Box<dyn Error>> {
    let _ = std::fs::remove_file(socket); // clear a stale socket from a dead run
    let listener = UnixListener::bind(socket)?;
    // The daemon typically runs as root (hidraw access) while clients (GUI,
    // scripts) run as the user — let any local user connect.
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o666))?;
    eprintln!("k75d listening on {socket} (writes: {allow_writes})");

    loop {
        let (stream, _) = listener.accept().await?;
        eprintln!("client connected");
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, allow_writes).await {
                eprintln!("client error: {e}");
            }
        });
    }
}

async fn handle_client(stream: UnixStream, allow_writes: bool) -> Result<(), Box<dyn Error>> {
    let (r, mut w) = stream.into_split();
    let mut lines = BufReader::new(r).lines();
    while let Some(line) = lines.next_line().await? {
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(req) => match dispatch(&req, allow_writes).await {
                Ok(v) => v,
                Err(e) => json!({"ok": false, "error": e}),
            },
            Err(e) => json!({"ok": false, "error": format!("bad json: {e}")}),
        };
        w.write_all(serde_json::to_string(&response)?.as_bytes())
            .await?;
        w.write_all(b"\n").await?;
    }
    Ok(())
}

async fn dispatch(req: &Value, allow_writes: bool) -> Result<Value, String> {
    let op = req.get("op").and_then(Value::as_str).unwrap_or("");
    let wants_write = req.get("send").and_then(Value::as_bool).unwrap_or(false);

    match op {
        "ping" => Ok(json!({"ok": true, "device": device_present().await, "writes": allow_writes})),
        "device" => match list_device() {
            Ok(_) => Ok(json!({
                "ok": true,
                "present": true,
                "vid": format!("{VID:04x}"),
                "pid": format!("{PID:04x}"),
                "interface": INTERFACE,
            })),
            Err(e) => Ok(json!({
                "ok": true,
                "present": false,
                "detail": e,
                "hint": "run k75d as root, or install a udev rule granting hidraw access",
            })),
        },
        "map" => Ok(json!({
            "ok": true,
            "slots": MATRIX_SLOTS,
            "keys": (0..KEY_LED_MAP.len())
                .filter_map(|s| KEY_LED_MAP[s].map(|k| json!({"slot": s, "key": k})))
                .collect::<Vec<_>>(),
        })),
        "matrix_baseline" | "matrix_slot" => {
            if wants_write && !allow_writes {
                Err("write refused: daemon started without --allow-writes".into())
            } else {
                let slot = if op == "matrix_slot" {
                    req.get("slot")
                        .and_then(Value::as_u64)
                        .ok_or("matrix_slot needs a slot number")? as usize
                } else {
                    usize::MAX // baseline: light everything
                };
                if slot != usize::MAX && slot >= MATRIX_SLOTS {
                    Err(format!("slot out of range 0..{MATRIX_SLOTS}"))?;
                }
                let repeat = req
                    .get("repeat")
                    .and_then(Value::as_u64)
                    .unwrap_or(1)
                    .min(20);
                let interval = req
                    .get("interval_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(500)
                    .clamp(100, 10_000);
                run_matrix(slot, wants_write, repeat as u32, interval).await
            }
        }
        "save" => {
            if wants_write && !allow_writes {
                Err("write refused: daemon started without --allow-writes".into())
            } else {
                run_save(req, wants_write).await
            }
        }
        "effect" => Err(
            "effect selection is send-gated: the host carrier frame is not yet pinned to a \
             capture (docs/protocol/effect-selection.md). Use Fn+Tab."
                .into(),
        ),
        "" => Err("missing op".into()),
        other => Err(format!("unknown op: {other}")),
    }
}

async fn device_present() -> bool {
    // Presence detection must not require an open handle (needs root for
    // hidraw writes) — the device list is enough to answer "is it there".
    list_device().is_ok()
}

/// Enumerate and find the K75 interface (no open, no permissions needed).
fn list_device() -> Result<String, String> {
    let mut api = Hidra::<Nusb>::builder()
        .build()
        .map_err(|e| e.to_string())?;
    api.refresh_devices().map_err(|e| e.to_string())?;
    let path = api
        .device_list()
        .find(|i| {
            i.vendor_id() == VID && i.product_id() == PID && i.interface_number() == INTERFACE
        })
        .map(|i| i.path().to_string())
        .ok_or_else(|| "K75 interface 1 not found".to_string())?;
    Ok(path)
}

fn open_device() -> Result<(Hidra<Nusb>, HidDevice<NusbDevice>), String> {
    let mut api = Hidra::<Nusb>::builder()
        .build()
        .map_err(|e| e.to_string())?;
    api.refresh_devices().map_err(|e| e.to_string())?;
    let path = api
        .device_list()
        .find(|i| {
            i.vendor_id() == VID && i.product_id() == PID && i.interface_number() == INTERFACE
        })
        .ok_or("K75 interface 1 not found")?
        .path()
        .to_string();
    let dev = api.open_path(&path).wait().map_err(|e| e.to_string())?;
    Ok((api, dev))
}

async fn run_matrix(
    slot: usize,
    send: bool,
    repeat: u32,
    interval_ms: u64,
) -> Result<Value, String> {
    let frame = MatrixFrame::new(Rgb::RED, (slot != usize::MAX).then_some((slot, Rgb::GREEN)))
        .ok_or("invalid highlight slot")?;
    let expected = if slot == usize::MAX {
        "all 14 key LEDs".to_string()
    } else {
        format!("slot {slot} → {}", predict_key(slot).unwrap_or("?"))
    };

    if !send {
        return Ok(json!({
            "ok": true,
            "dry_run": true,
            "expected": expected,
            "first_16": format!("{:02x?}", &frame.as_bytes()[..16]),
        }));
    }

    let (_api, dev) = open_device()?;
    for _ in 0..repeat {
        tokio::time::timeout(
            Duration::from_secs(3),
            dev.send_feature_report(frame.as_bytes().as_slice()),
        )
        .await
        .map_err(|_| "send timed out")?
        .map_err(|e| e.to_string())?;
        // one frame = one case-animation step; never spam
        tokio::time::sleep(Duration::from_millis(interval_ms)).await;
    }
    Ok(json!({"ok": true, "sent": repeat, "expected": expected}))
}

async fn run_save(req: &Value, send: bool) -> Result<Value, String> {
    let (write, label) = match (req.get("color"), req.get("pattern")) {
        (Some(c), _) => {
            let hex = c.as_str().ok_or("color must be an RRGGBB string")?;
            let rgb = parse_hex(hex)?;
            (PerKeyColorFrame::new(rgb), format!("uniform #{hex}"))
        }
        (None, Some(p)) => {
            let list = p
                .as_array()
                .ok_or("pattern must be an array of 126 RRGGBB strings")?;
            if list.len() > MATRIX_SLOTS {
                Err(format!(
                    "pattern has {} entries, max {MATRIX_SLOTS}",
                    list.len()
                ))?;
            }
            let mut slots = [Rgb::OFF; MATRIX_SLOTS];
            for (i, item) in list.iter().enumerate() {
                let hex = item
                    .as_str()
                    .ok_or("pattern entries must be RRGGBB strings")?;
                slots[i] = parse_hex(hex)?;
            }
            let lit = slots.iter().filter(|c| **c != Rgb::OFF).count();
            (
                PerKeyColorFrame::from_slots(&slots),
                format!("pattern: {lit}/{MATRIX_SLOTS} slots lit"),
            )
        }
        (None, None) => Err("save needs color (RRGGBB) or pattern (array of 126 RRGGBB)")?,
    };
    let apply = ApplyFrame::new();
    let save = SaveFlashFrame::new();

    if !send {
        return Ok(json!({
            "ok": true,
            "dry_run": true,
            "staged": label,
            "sequence": ["cmd 0x0a write", "cmd 0x0b apply", "cmd 0x06 flash save"],
        }));
    }

    let (_api, dev) = open_device()?;
    let seq: [(&str, &[u8]); 3] = [
        ("write", write.as_bytes()),
        ("apply", apply.as_bytes()),
        ("save", save.as_bytes()),
    ];
    for (name, frame) in seq {
        tokio::time::timeout(Duration::from_secs(3), dev.send_feature_report(frame))
            .await
            .map_err(|_| format!("{name} send timed out"))?
            .map_err(|e| e.to_string())?;
    }
    Ok(json!({"ok": true, "sent": 3, "staged": label}))
}

fn parse_hex(hex: &str) -> Result<Rgb, String> {
    let h = hex.trim_start_matches("0x");
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("bad RRGGBB: {hex}"));
    }
    let byte = |s: &str| u8::from_str_radix(s, 16).map_err(|_| "bad hex".to_string());
    Ok(Rgb::new(byte(&h[0..2])?, byte(&h[2..4])?, byte(&h[4..6])?))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirror handle_client's error wrapping: dispatch -> JSON response.
    fn call(req: Value, allow_writes: bool) -> Value {
        let rt = tokio::runtime::Runtime::new().unwrap();
        match rt.block_on(dispatch(&req, allow_writes)) {
            Ok(v) => v,
            Err(e) => json!({"ok": false, "error": e}),
        }
    }

    #[test]
    fn parse_hex_ok() {
        let c = parse_hex("ff0080").unwrap();
        assert_eq!((c.r, c.g, c.b), (0xff, 0x00, 0x80));
        let c = parse_hex("0x00ff00").unwrap();
        assert_eq!((c.r, c.g, c.b), (0x00, 0xff, 0x00));
    }

    #[test]
    fn parse_hex_bad() {
        assert!(parse_hex("ff00").is_err());
        assert!(parse_hex("gggggg").is_err());
    }

    #[test]
    fn dispatch_unknown_op() {
        let r = call(json!({"op": "nope"}), false);
        assert_eq!(r["ok"], false);
        assert!(r["error"].as_str().unwrap().contains("unknown op"));
    }

    #[test]
    fn dispatch_effect_always_refused() {
        let r = call(json!({"op": "effect", "index": 5}), true);
        assert_eq!(r["ok"], false);
        assert!(r["error"].as_str().unwrap().contains("send-gated"));
    }

    #[test]
    fn dispatch_write_gating() {
        let r = call(
            json!({"op": "matrix_slot", "slot": 30, "send": true}),
            false,
        );
        assert_eq!(r["ok"], false);
        assert!(r["error"].as_str().unwrap().contains("--allow-writes"));
    }

    #[test]
    fn dispatch_map_shape() {
        let r = call(json!({"op": "map"}), false);
        assert_eq!(r["ok"], true);
        assert_eq!(r["slots"], MATRIX_SLOTS as u64);
        assert!(r["keys"].as_array().unwrap().len() > 10);
    }

    #[test]
    fn dispatch_save_dry_run_pattern_too_long() {
        let long = vec!["000000"; MATRIX_SLOTS + 1];
        let r = call(json!({"op": "save", "pattern": long}), false);
        assert_eq!(r["ok"], false);
        assert!(r["error"].as_str().unwrap().contains("max"));
    }
}
