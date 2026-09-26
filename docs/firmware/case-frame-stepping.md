# Case light: frame-synced animation stepping

**Live-verified 2026-09-26 (experiment A).**

## Finding

The case/side light animation is **frame-synced to the `0x08` matrix
command**: each matrix frame advances the case animation by exactly one
step.

Evidence:

- 20-frame stream at **500 ms** interval (video, `analysis/video-evidence-2026-09-26/`):
  19 held gradient patterns at ~0.52 s cadence, then a ~5 s hold, then the
  continuous wave resumes after the stream ends.
- 20-frame stream at **1500 ms** interval (live, user-counted):
  ~20 distinct pattern changes, roughly one per 1.5 s.

The case cycle cadence therefore **follows the host frame rate** — it is
not a free-running internal timer.

## Behavioural notes

- Static case stops (cyan, white, red, …) do **not** animate during
  streams: the case holds its current Fn+Tab state. Stepping is only
  visible while the case is on an animated stop (rainbow wave).
- The case state also **survives** matrix streams — the effect selected
  via Fn+Tab is not reset by `0x08` frames.
- Keys render the matrix red during the stream; the case steps
  independently of the key colours.

## Mechanism hypothesis (firmware)

Each `0x08` frame: `0x7108` writes the matrix and sets marker
`0x0E34=0x5A` → scheduler `0x7781` calls the renderer `0x5AF8` →
`0x5BEC` re-arms countdown `0x0F59=0xFA`. The case animation (`0x0F99..0x0F9F`
engine) advances one step per matrix-render cycle. Exact advance site not
yet pinned (candidates: countdown expiry `0x35C6` / `0x23.6` return path,
or the per-frame render loop).

## What this unlocks

Streaming N matrix frames walks the case animation through its cycle
(~19-20 gradient stages, then hold, then wave). The host can therefore
drive the case light without any dedicated case-light packet — **one step
per frame**.

## Open questions

1. **Cycle length and repeatability** — does a 40-frame stream produce two
   identical 19-stage cycles? (Test: 40 frames at 500 ms, re-record.)
2. **Determinism** — does the case always start at stage 1 after a
   factory reset (Fn+Esc ~3 s)? If yes: reset + N frames = deterministic
   case position control.
3. **Direction** — can the case be stepped backwards (fewer frames won't
   go back; is there a reverse trigger)?
4. **Which stage is "off"?** — is one of the cycle stages the off state,
   giving host-controlled case-off via frame counting?