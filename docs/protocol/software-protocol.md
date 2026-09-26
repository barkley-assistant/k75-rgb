# Official Software Protocol (decompiled from RedThunder K75 UK Setup v2.0)

**Status:** static-analysis findings from the vendor's Windows control tool.
The checksummed `0x44` application format below has **not** been shown to
select the connected keyboard's firmware matrix command `0x08`. Do not use
this format to infer the offsets or checksums of `k75`; the live K75
firmware/transport trace is in [protocol-audit.md](protocol-audit.md).
**Method:** Inno Setup extraction → Ghidra 12.1.4 headless analysis of `OemDrv.exe`
(MSVC x86, MFC) → decompilation of every function referencing
`HidD_SetFeature` / `HidD_GetFeature` (42 functions, module @
0x495260–0x49f101) and the `SendCommand`/`GetCommand` wrappers (13 callers).

No decompiled source is committed to this repository. This document records
only the protocol facts.

---

## 1. Channel architecture

The tool talks to the keyboard exclusively through **HID feature reports**
(`HidD_SetFeature` / `HidD_GetFeature`, class `CHidDev`), never through
interrupt output reports.

Two report IDs are used, selected by a device-state flag
(`device_obj[0x18] == 0x18` → report 9, else report 6):

| Report ID | Role |
|---|---|
| **0x09** | data operations: LED, macro, onboard, raw data, firmware pages |
| **0x06** | register/query operations: password, 0x40-block reads, music data |

The device ACKs data operations by **echoing the command ID** in a
`GetFeature` response; the tool retries up to 3× with 60 ms sleeps, and
verifies `response[2] == cmd_id` before accepting the transfer.

---

## 2. Frame format (report 0x09, 520 bytes total)

```
offset  0  1         2    3   4       5        6     7    8..519
       [09][checksum][cmd][sub][pages][page_idx][len_lo][len_hi][512B data]
```

* `checksum` = 8-bit sum of **all bytes from offset 2 to 519** (mod 256)
* `cmd` = command ID (see §3)
* `pages` = total number of 512-byte pages (`ceil(len/512)`)
* `page_idx` = this page's index (0-based, increments per page)
* `len_lo/len_hi` = data length in **this** page (little-endian; last page
  may be shorter)
* transfers are paged: up to `pages` sequential SetFeature calls
* after each page the tool issues `GetFeature` and requires
  `response[2] == cmd` before continuing

### Send/Get primitives

```
SendCommand(report_id, payload, handle, len):
    buf[0] = report_id; memcpy(buf+1, payload, len);
    HidD_SetFeature(handle, buf, len+1)   // retry x3, Sleep(200)

GetCommand(report_id, out, handle, len):
    buf[0] = report_id;
    HidD_GetFeature(handle, buf, len+1); memcpy(out, buf+1, len)
```

---

## 3. Command IDs observed

| cmd | name | direction | payload |
|---|---|---|---|
| **0x44** | **LED data** | SET + GET | 512B lighting state (mode/brightness/speed/effects + per-key RGB) |
| 0x43 | Macro data | SET + GET | macro definitions |
| 0x4A | OnBoard data | SET + GET | onboard profiles/key mapping |
| 0x88 | RealData | GET | raw data read-back |
| 0x0C | FW page write | SET | `[cmd,0x00,seq,?,len16,512B page]` via `AccessData_Page` (520B, retry×3) |
| 0x81 0x05 | Password | GET | returns `[0x01, b0, b1, b2]` — 3-byte device token |
| 0x40-block | register queries | SET+GET | `[0x40,?,0xb6]`, `[0x40,?,0x83,0xb6]`, `[0x40,?,0x84,0xd4]`, `[0x40,?,0x88,0xb8]` — query/read ops on report 0x06 |
| 0x7A | Music data | SET | 122-byte music-rhythm payload (report 0x06) |

`CDevG5MS::AccessData` / `CDevG5KB::AccessData` are the generic accessors;
each op is identified by its `nCmdID` (the `cmd` byte) in the error logs.

---

## 4. Effect catalog (UI strings ↔ hardware codes)

### Keyboard-tab effects (KB.ini `LedOpt1..19` = UI index → HW code)

| UI | Name | HW code | UI | Name | HW code |
|---|---|---|---|---|---|
| 1 | Fixed_on | 0x01 | 11 | Neon_stream | 0x07 |
| 2 | Respire | 0x03 | 12 | Reaction | 0x11 |
| 3 | Rainbow | 0x02 | 13 | Sine_wave | 0x0C |
| 4 | Flash_away | 0x13 | 14 | Retinue scanning | 0x08 |
| 5 | Raindrops | 0x0F | 15 | Rotating windmill | 0x1C |
| 6 | Rainbow_wheel | 0x0D | 16 | Colorful waterfall | 0x1E |
| 7 | Ripples_shining | 0x14 | 17 | Blossoming | 0x0E |
| 8 | Stars_twinkle | 0x10 | 18 | Rotating storm | 0x1D |
| 9 | Shadow_disappear | 0x12 | 19 | Self-define (custom) | 0x00 |
| 10 | Retro_snake | 0x05 | 20 | OFF | — |

### Mode-tab effects (the Fn+Tab family — `tc_ms_led*` strings)

1. Colorful Streaming, 2. Steady, 3. Breathing, 4. Colorful Tail,
5. Neon, 6. Colorful Steady, 7. Flicker, 8. Stars twinkle, 9. Wave,
10. LED OFF

The K75 firmware implements a **subset** (its own effect-code table
`0xDB76` holds 16 codes: `01 02 03 04 05 07 08 09 0a 0b 0c 0d 0f 10 11 13`
— note it lacks 0x06/0x0E/0x12 and the extended codes 0x14/0x1C/0x1D/0x1E
seen in the generic tool table).

---

## 5. KB.ini configuration constants (Dev/kb/KB.ini)

```
Fw=26                     firmware version
VID=0x258a PID=0x019D     wired identity (matches our device)
VID_Wireless=0x3554       wireless dongle VID
PID_Wireless=0x0150       wireless dongle PID
Psd=6,0,0,0,0,7a          packet def: report 6, 122-byte payload
Light=0,1,2,3,4           software brightness steps
LightHW=0,5,10,15,20      hardware brightness values for those steps
Speed=0,1,2,3,4           software speed steps
SpeedHW=0,1,2,3,4         hardware speed values
LedMask=0x22020           LED zone mask
DefLedIndex=10            default effect index (UI index 10 = Retro_snake, HW 0x05)
CRC=1                     CRC/checksum enabled
```

`[KEY]` section: per-key geometry + usage codes + **LED index** (the order
the firmware's per-key RGB table uses — 81 keys, indices 0..80).
`[FN1]`: Fn-layer table; special function codes observed:
`0x0802xxxx` = light mode, `0x0803xxxx` = brightness, `0x0804xxxx` = speed
(parameters 1/2 = up/down) — matches Fn+Tab / Fn+↑↓ / Fn+←→ behavior.

---

## 6. What this means for the project

1. **The official SET path is cmd 0x44 on report 0x09** with the
   checksummed paged frame. Our working `0x0a/0x0b` path is the firmware's
   config-image channel; both reach the LED state.
2. **A genuine device READ exists** (GET with cmd echo + data return) —
   `GetLED` (0x44) can return the live 512-byte lighting state; `GetRealData`
   (0x88) reads raw data. Neither was reachable via our earlier probes.
3. **Brightness/speed ladders are 5 steps** (0/5/10/15/20 and 0..4).
4. **`Fixed_on` (HW 0x01)** is a name in the generic vendor effect catalog.
   It has not been mapped to a working K75 host packet; the red key state
   observed with our `0x0a`/`0x0b` sequence still had a dim moving wave.
5. The 3-byte **password handshake** (`[0x81,0x05]` → `[0x01,b0,b1,b2]`)
   may gate firmware writes.
---

## 7. Live verification against the K75 firmware (2026-09-25)

1. **cmd 0x44 (LED) is NOT implemented in the K75 firmware.** Sending the
   official GetLED frame produced no device response, and the device's
   input-report echo kept showing the previous valid command (0x0a) —
   i.e. the firmware ignored 0x44 entirely. The generic 0x43/0x44/0x4A/0x88
   data ops target other boards in the G5 family.
2. **The K75's real control channels remain the ones already mapped:**
   report-0x09 `0x0a` (config write) + `0x0b` (apply) + `0x06` (save), and
   the report-0x06 `'S'` register protocol. The official tool reaches the
   same state through its `CDevG5KB` device class.
3. **New: the device's 8-byte input report (report 0x09) is a command-echo
   ACK channel** — observed `[09, 0a, 06, 00, 01, 00, 00, 00]` echoing the
   last effective command (0x0a). Usable as a cheap status/ack read-back.
4. **Mode register 0x0F54 values 0x25/0x35/0x45/0x55 = the four Fn+Tab
   modes.** Factory state 0x01 is a distinct "default wave" value. Config
   byte +0x1F selects them (verified: writes apply; the four modes are
   visually near-identical wave variants, matching user observation).
5. The password handshake `[0x81, 0x05]` on report 0x06 is not served by
   the K75 either (feature GET stalls) — consistent with a generic tool
   probing features the K75 firmware lacks.

**Conclusion:** the decompilation closed the loop. The official software's
generic data commands are not the K75's protocol; our independently
reverse-engineered channels are, and now carry vendor-confirmed semantics
(brightness ladder 0/5/10/15/20, speed ladder 0..4, effect names, mode
count). Remaining unknown: the report-0x06 'S' register op that sets
brightness (0x0F64) live.
