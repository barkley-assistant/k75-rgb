# K75 RGB Reverse-Engineering — Handover

## Mission
Reverse-engineer the RedThunder K75 (Sinowealth SH68F90A) RGB protocol from firmware disassembly + live USB probing, then build: Rust daemon (owns USB, exposes HTTP/JSON API) + Electron app. Milestone M1 = ONE confirmed write packet that turns LEDs solid red. User is at the machine and can watch the keyboard / unplug-replug on request.

## Hard rules (user-mandated, non-negotiable)
1. NO invented packets — every byte must trace to a capture, sinowisp source, or disassembly offset.
2. READ-ONLY toward the device unless a step explicitly says write. NEVER flash. **0x45 = flash erase — never send it.**
3. Stock backup exists: `fw/k75_full.bin` MD5 `be1f5410a2f97d4143b96ed56fbb1c11` (bootloader MD5 `3e0ebd0c440af5236d7ff8872343f85d`). Re-dump with sinowisp to verify identity before anything else.
4. Ignore the desktop agent's 2.4G-mode evidence (board was in 2.4G yesterday, `258a:0150` — untrusted). Only the wired-mode `258a:019d` evidence counts.

## Current state (verified minutes ago)
- Device: `258a:019d` wired mode, both interfaces bound to usbhid, hidraw0 (iface 0, keyboard) + hidraw1 (iface 1, vendor/RGB channel).
- Interface 1 has ONLY EP2 IN (16B interrupt). No OUT endpoint → host→device is exclusively control transfers (SET_REPORT).
- **sinowisp WORKS RIGHT NOW.** `sudo /home/agent/.cargo/bin/sinowisp read --platform sh68f90 --vendor_id 0x258a --product_id 0x019d --firmware_size 61440 --bootloader_size 4096 --page_size 2048 --isp_iface_num 1 --isp_report_id 5 --reboot true --format bin -s full <out.bin>` just dumped 65536 bytes again, MD5 identical. The write path is ALIVE. The device is NOT wedged.
- **IMPORTANT:** sinowisp MUST run with sudo (opens /dev/hidraw*; without sudo it says "Device not found" due to EACCES — earlier "not found" results were my permission bug, not the device).
- sinowisp rotates hidra backends (Native=hidraw, then Nusb=raw USB); it reads report descriptors and picks the node containing feature report IDs [5,6,9].

## The blocking discrepancy (solve this first)
- sinowisp: enable_firmware `[05 55 00 00 00 00]` → ACCEPTED → board re-enumerates as ISP `0603:1020` → dump → reboot `[05 5a 00 00 00 00]` → back to `258a:019d`.
- My `tools/hidra_probe` (both Native and Nusb backends): same 6-byte payload on report 0x05 → **ETIMEDOUT** (device NAKs, never completes). GET 0x06 via hidra returns 0 bytes.
- Raw usbfs SET (correct ioctl `0xC0185500`): also ETIMEDOUT. Earlier "device STALLs" conclusions are TAINTED — my ioctl constants were wrong (see below), those errors were kernel-side, never reached the wire.
- So: sinowisp sends something my probes don't — likely sequencing, timeout/retry, or an extra setup step. Diff `enter_isp_mode` / flasher flow in sinowisp source vs my probe. Capture the wire with usbmon during a sinowisp read (`tools/k75_usbmon4.py`, needs sudo) — that's ground truth.

## Known-good protocol structure (from sinowisp source, verified by working dump)
- Report 0x05 commands are 6 bytes: `[0x05, cmd, a0, a1, 0, 0]` (report ID + 5 data; descriptor says 5 data bytes → total 6).
- Report 0x06 transfers: `[0x06, xfer_cmd, data...]`.
- Commands: 0x52 init_read, 0x55 enable_firmware, 0x5a reboot, 0x77 write_page (bootloader context).
- Report descriptor (actual, from sinowisp list): feature IDs 5, 6, 9. 0x06 declares 1031 data bytes but the kernel parses it as 4 and clamps GETs to wLength=4 → returns `06 00 00 00` (descriptor lie).

## My ioctl bugs (fix in any new tooling)
- USBDEVFS_CONTROL must be `0xC0185500` (24-byte struct on x86-64), not `0xC0105500`.
- HIDIOCSFEATURE(len) = `0xC0000000 | (len<<16) | 0x4806`, HIDIOCGFEATURE(len) = `0xC0000000 | (len<<16) | 0x4807` — both are READ|WRITE (bidirectional), size field encodes the buffer length. (Verified against hidra's own ioctl test.)

## Firmware landmarks (trusted wired-mode dump, `analysis/disasm_clean.txt` — 49k lines)
- ISP dispatcher 0xFA3–0x10DB (in main firmware); report buffer 0x0F54; ISP cmd reg 0x0DAF; reset trigger SFR 0x32.
- Command dispatcher at 0x203D/0x2549: bytes 0x01/0x25/0x35/0x45/0x55 → mode writes to 0x0CC7 = 0/1/2/3/4; mode byte 0x0F3F (0x11/0x12) → 0x0BCB.
- RGB color state 0x0F1D (4 bytes R,G,B,+flag) written via helper fcn.000029cd; 0xDB76 table = 16 effect IDs (matches FN+\| cycle); gamma table 0x06ba–0x0719; effect table 0x0EE7 (21-byte entries); "fill N bytes" helper fcn.0000a274; magic byte 0x11E0 == 0x5A gates dispatcher fcn.0000249b via fcn.0000d15f.
- USB ISR vector 0x0043 → ljmp 0x6280.
- fizz-rgb K617 packets (`83 b6`) are NOT in this firmware — do not send them (would be invented packets).

## Suggested next steps
1. usbmon capture of a sinowisp read (needs sudo) → exact wire bytes of enable_firmware + read pages. Diff against hidra_probe failure.
2. Replicate sinowisp's exact open+send sequence (including backend rotation and any retry/ExpectedError handling) in hidra_probe until `[05 55 00 00 00 00]` is ACCEPTED.
3. Then use firmware dispatchers (0x2549, 0x203D, 0x0F3F mode bytes) to derive candidate RGB packets — test with user watching LEDs. M1 = solid red.
4. Then M2: effects/brightness/speed via pattern-matching.
5. App: `tools/hidra_probe` is the daemon seed (Rust + hidra + tokio). Add axum HTTP API + Electron client.

## Files
- Workspace: `/home/agent/workspaces/k75-rgb/` — `fw/k75_full.bin`, `fw/k75_boot.bin`, `analysis/disasm_clean.txt`, `tools/*.py` (probes), `tools/hidra_probe/` (Rust probe, Native+Nusb variants).
- sinowisp source: `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sinowisp-2.1.0/src/` (flasher.rs, isp_device.rs, device_selector.rs).
- hidra 0.0.4 source: same registry dir.
- Rebind interface 1 if unbound: `sudo sh -c 'echo -n "1-1:1.1" > /sys/bus/usb/drivers/usbhid/bind'` (unbind likewise).