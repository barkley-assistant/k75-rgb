# Case-strip video evidence — 2026-09-26

Video analysis of the case side-light strip during a 20-frame `matrix
baseline --repeat 20` stream (~10 s at 500 ms/frame) and its aftermath.

Source: `1000085264.mp4` (15.689 s, 921 decoded frames, 60 fps nominal).
All measurements are camera-derived, not calibrated LED values.

## Headline findings

1. **The case strip is a continuous multi-colour gradient**, not a single
   solid-colour zone. Left/middle/right camera samples differ within every
   held pattern (e.g. stage 1: turquoise/cyan left, cyan-blue middle,
   violet/purple right). The video does not prove independently addressable
   sections, but it rules out "one LED, one colour".

2. **19 held patterns at ~0.52 s cadence** during the stream, then pattern
   19 holds ~5.0 s, then continuous colour movement resumes (the normal
   rainbow wave after the stream ends). The ~0.52 s cadence is suspiciously
   close to our 500 ms frame interval — **open question: does each `0x08`
   matrix frame step the case animation, or is the case free-running at
   ~2 steps/s?** Testable by streaming at a different cadence (250 ms or
   1 s) and re-recording.

3. All 19 stages are distinct gradients; stages 1/10/19 are all in the
   cyan-blue-purple family but not exact duplicates (the right end becomes
   more magenta over time). The case cycle does not repeat cleanly over the
   19 stages observed.

4. No sustained off or white state in the video. The pale single-frame
   boundary samples (~17 ms each) are listed but unexplained — exposure
   mixing vs transient hardware output vs commanded colours.

5. Colours move camera-right in the post-stream wave (left progresses
   pink→purple→blue→cyan; middle orange→pink→blue→cyan).

## Held pattern table (camera front view)

| Stage | Clear by (s) | Left | Middle | Right |
|---|---:|---|---|---|
| 1 | 0.004 | Turquoise/cyan | Cyan-blue | Violet/purple |
| 2 | 0.453 | Magenta/pink | Red-orange | Yellow-lime |
| 3 | 0.969 | Green/mint | Cyan | Blue |
| 4 | 1.501 | Violet/purple | Magenta/hot pink | Orange |
| 5 | 2.033 | Lime/yellow-green | Green-turquoise | Cyan |
| 6 | 2.549 | Blue | Purple-magenta | Pink-red |
| 7 | 3.081 | Yellow | Green | Turquoise/teal |
| 8 | 3.614 | Cyan-blue | Blue-violet | Magenta |
| 9 | 4.146 | Red-orange/orange | Yellow-lime | Green |
| 10 | 4.678 | Cyan | Blue/cyan-blue | Purple |
| 11 | 5.194 | Pink-magenta | Orange/amber | Yellow-lime |
| 12 | 5.726 | Green-turquoise | Cyan | Blue/blue-violet |
| 13 | 6.258 | Purple-magenta | Pink-red | Yellow-orange |
| 14 | 6.774 | Green | Turquoise-cyan | Cyan-blue |
| 15 | 7.306 | Blue-violet | Magenta | Red/pink-red |
| 16 | 7.838 | Yellow-green | Green | Cyan/turquoise |
| 17 | 8.370 | Blue | Violet/purple | Magenta/hot pink |
| 18 | 8.903 | Amber/orange-yellow | Yellow-green | Green |
| 19 | 9.418 | Cyan | Blue | Purple-magenta |

(First change precedes "clear by" by ~1 frame per stage; full detail in
`timeline.json`.)

## Reconciliation with live Fn+Tab observations

The same session's live Fn+Tab walk (user, viewed from above) produced an
11-stop case cycle: cyan, white, pulse rotating colours, off, rainbow wave,
rainbow strobe, red, green, blue, yellow, purple.

These are two **different sequences**:
- The video's 19 stages are gradients observed during a matrix-render
  stream (no Fn+Tab involved) at ~0.5 s cadence.
- The Fn+Tab walk is the effect cycle (`0x0F83`) as seen by a human from
  above, where gradients may read as a dominant colour.

The 11-stop walk and the 19-stage gradient sequence do not map 1:1. Both
are observational; neither is yet tied to register writes. The firmware
side (case engine on `0x0F99..0x0F9F`, sub-state `0x0F9B` wrapping at
0x0A, palette `0x0F9F`) predicts small cycle lengths (10-11), closer to the
live walk than to 19.

## Files

- `README.md` — cross-reference and reconciliation (this file)
- `report.md` — full original vision-agent analysis (held patterns,
  boundary frames, methodology, caveats)
- `timeline.json` — machine-readable held patterns + sampled observations
- `all_frames_camera_samples.csv` — per-frame camera RGB/hue at 128 sample
  positions (left/middle/right medians)
- `frames/` — all 921 decoded crops (JPEG, ~58 MB) so `index.html` works
  from a fresh clone
- `index.html` — offline frame viewer with slider + arrow keys; phase
  labels describe the nearby held appearance
- `keyboard_led_held_patterns.jpg`, `keyboard_led_transition_frames.jpg`,
  `keyboard_led_final_animation.jpg` — composite palette sheets
- `manifest.json`, `timeline.csv` — full machine-readable frame manifest
  and timeline
- The source MP4 is intentionally NOT in the repo (see
  `report.md` for its SHA-256: `ecb99b618…c86518`).