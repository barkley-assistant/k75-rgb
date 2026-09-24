# K75 RGB — Reverse-Engineering Status (honest, 2026-09-24)

## What is VERIFIED (live-tested, traceable)

1. **Color change + save** via report 0x09: cmd `0x0a` (config write, op 0x54,
   fcn.00009308) + `0x0b` (apply) + `0x06` (save, op 0x56). Persists across
   power-cycle and 2.4G wireless. Tools: `setcolor2`, `save_color`.
2. **Report 0x06 register protocol** — fcn.00005001 (@0x5001), fully decoded:
   - `[06, 'S'(0x53), 0x01, REG, b0..b3]` = SET: REG→0x0EFF, 4 bytes→0x0F07
   - continuation: 8 bytes/chunk, at ≥19B sets 0x0F3E|=0x10 (apply)
   - `[06, 'R'(0x52), 'V'(0x56), reg]` = read register (0x01/0x02 = identity)
3. **Complete report-0x09 command table** (M6): 0x03 flash sub-dispatch, 0x04 flash
   read (0x52), 0x05 flash sub-op, 0x06 save, 0x08 direct-LED (fcn.00007108),
   0x0a config write (fcn.00009308), 0x0b/0x0c/0x0d apply.
4. **Config region is host-writable** — live-verified: cmd 0x0a template writes
   alter the applied config (turned lighting off when malformed).
5. **Effect-command pipeline decoded** (M6): fcn.000096ce copies 20B block to
   0x1130, 0x0F7F=1 gates fcn.0000b684 ([0x1130]==0x5A register, ==0x13 apply).
6. **Recovery paths both proven**: Fn+Esc (hold 3s) factory reset; sinowisp ISP
   full re-flash (backup MD5s verified across 4 dumps).

## M7 — Decisive architecture conclusion (2026-09-24)

- **NO live-frame path to mode/brightness/effect.** 0x0F3F (effect index),
  0x0F54 (mode), 0x0F64 (brightness) are written only by firmware constants
  (Fn keys) or by the config-load from data-flash. Verified by subagent decode
  of fcn.00001b46/fcn.000057db (analysis/fn1b46-decode.md, committed with
  frame-base correction).
- **Host route to effects/brightness = config-region write + reload** via
  cmd 0x0a/0x06. Empirically proven (the modeprobe wedge).
- **No host-visible config read.** cmd 0x04 returns only the 8-byte ACK echo;
  it is an internal reload (data-flash → live per-key table). Live-probed.
- **Config layout hypothesis** documented in
  analysis/config-layout-hypothesis.md: mode at +0x0E (0x35, VERIFIED anchor),
  speed/brightness candidates at +0x16..+0x18, per-key gradient tail at +0x21.

## NOT yet done

- **Verified mode/brightness/speed control** — config channel works but the
  exact byte offsets need live write-tests (one byte changed at a time, with
  Fn+Esc as the safety net). See M2-PLAN.md M7 step 4b.
- **Case/side underglow** — separate LED zone; Fn+Tab toggles manually; USB path
  not yet decoded.

## Lesson learned (2026-09-24)

Writing a malformed config (0x0a) + saving (0x06) **persists garbage** that wedges
the lighting engine — Fn keys stop recovering it. Fn+Esc factory reset is the fix.
Probe config writes only with the recovery route ready and prefer read-side probes.