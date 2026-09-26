# Keyboard case-strip video analysis

Source: `1000085264.mp4` (15.689 seconds; 921 decoded frames). Only the lower/front case strip is analysed, not key backlighting or the separate keyboard at the bottom of the video.

## Reading the log

Left and right are **as seen in the video**. The strip displays a continuous gradient; left/middle/right are descriptive sample locations, not evidence of three independent zones. Colours are the recorded visual appearance, not calibrated hardware RGB values. Timestamps are original video presentation timestamps. Frame numbers are zero-based. The recording has some longer frame intervals, so do not calculate timestamps as frame / 60.

There are 19 held appearances (including the one already active when recording starts), followed by continuous colour movement. The first 18 changes occur roughly 0.52-0.53 seconds apart after the initial partial hold. Pattern 19 holds for about 5.0 seconds. The final change begins with a mixed frame at 14.425 seconds; the warm gradient is clearly visible at 14.441 seconds and then keeps moving.

## Held patterns

The colours describe the settled appearance. “First change” includes a pale/mixed boundary frame where visible; “clear by” is the first clear new-pattern frame. A pattern remains until the next transition.

| Stage | First change (s) | Clear by (s) | Left | Middle | Right | Representative frame |
|---|---:|---:|---|---|---|---:|
| 1 | 0.004 | 0.004 | Turquoise / cyan | Cyan-blue | Violet / purple | 13 |
| 2 | 0.437 | 0.453 | Magenta / pink | Red-orange | Yellow, slightly lime at the tip | 41 |
| 3 | 0.969 | 0.969 | Green / mint | Cyan | Blue | 71 |
| 4 | 1.485 | 1.501 | Violet / purple | Magenta / hot pink | Orange | 101 |
| 5 | 2.017 | 2.033 | Lime / yellow-green | Green-turquoise | Cyan | 133 |
| 6 | 2.549 | 2.549 | Blue | Purple-magenta | Pink-red | 165 |
| 7 | 3.081 | 3.081 | Yellow | Green | Turquoise / teal | 197 |
| 8 | 3.597 | 3.614 | Cyan-blue | Blue-violet | Magenta | 227 |
| 9 | 4.129 | 4.146 | Red-orange / orange | Yellow-lime | Green | 258 |
| 10 | 4.661 | 4.678 | Cyan | Blue / cyan-blue | Purple | 290 |
| 11 | 5.194 | 5.194 | Pink-magenta | Orange / amber | Yellow-lime | 322 |
| 12 | 5.726 | 5.726 | Green-turquoise | Cyan | Blue / blue-violet | 352 |
| 13 | 6.241 | 6.258 | Purple-magenta | Pink-red | Yellow-orange | 382 |
| 14 | 6.774 | 6.774 | Green | Turquoise-cyan | Cyan-blue | 414 |
| 15 | 7.306 | 7.306 | Blue-violet | Magenta | Red / pink-red | 446 |
| 16 | 7.822 | 7.838 | Yellow-green | Green | Cyan / turquoise | 476 |
| 17 | 8.354 | 8.370 | Blue | Violet / purple | Magenta / hot pink | 505 |
| 18 | 8.886 | 8.903 | Amber / orange-yellow | Yellow-green | Green | 535 |
| 19 | 9.418 | 9.418 | Cyan | Blue | Purple-magenta | 698 |

## Continuous movement at the end

These are observation samples, not separate command states. Between them the gradient continues changing. The visible colour bands move towards camera-right: the left progresses from pink through purple and blue to cyan/green; the middle progresses from orange through pink/purple and blue to cyan.

| Time (s) | Frame | Left | Middle | Right |
|---:|---:|---|---|---|
| 14.441 | 846 | Pink-magenta | Orange / amber | Yellow-lime |
| 14.541 | 852 | Magenta | Red-orange | Yellow |
| 14.641 | 858 | Purple-magenta | Pink-red | Yellow-orange |
| 14.741 | 864 | Violet / purple | Hot pink | Orange |
| 14.840 | 870 | Blue-violet | Magenta | Red-orange |
| 14.940 | 876 | Blue | Purple-magenta | Pink-red |
| 15.040 | 882 | Blue / cyan-blue | Violet-magenta | Magenta / pink |
| 15.140 | 888 | Cyan-blue | Violet | Magenta |
| 15.240 | 894 | Cyan | Blue-violet | Purple-magenta |
| 15.339 | 900 | Cyan | Blue | Violet / purple |
| 15.439 | 906 | Turquoise / cyan | Cyan-blue | Blue-violet |
| 15.539 | 912 | Green-turquoise | Cyan | Blue |
| 15.672 | 920 | Green-turquoise | Cyan | Blue |

## Brief pale/mixed boundary frames

The following unusual frames are retained rather than silently dropped. Each lasts one captured frame, about 17 ms here. The video alone cannot determine whether these are exposure mixing, true transient hardware output, or separately commanded colours. Do not treat them as confirmed additional white-light settings without comparing the write log.

| Time (s) | Frame | Left | Middle | Right |
|---:|---:|---|---|---|
| 0.436667 | 26 | Pale lavender / white | Red-orange | Yellow-lime |
| 1.484511 | 85 | Pale cyan / white | Pale cyan | Blue |
| 2.016778 | 117 | Pale cyan-green | Pale green / mint | Pale whitish / grey-lilac |
| 3.596944 | 212 | White / pale cyan | Cyan-turquoise | Green-turquoise |
| 4.129111 | 242 | Salmon / orange | Whitish pale green | Pale green / mint |
| 4.661333 | 274 | Cyan | Cyan-blue | Whitish blue / lavender |
| 6.241411 | 367 | Icy cyan / white | Pale blue | Pale violet |
| 7.821544 | 461 | Icy blue-white | Pale lavender-white | Peach-white |
| 8.353711 | 490 | Pale blue-white | Cyan | Pale cyan-white |
| 8.885956 | 520 | Orange-yellow | Yellow-green | Pale green / cyan |
| 14.424633 | 845 | Icy cyan / white | Blue | Purple-magenta |

## Implications for verification

The whole visible strip changes its gradient together; the video does not establish independently addressable left/middle/right sections. Similar colour families recur, but they are not exact duplicates: notably, stages 1, 10 and 19 all run cyan to blue to purple, with the right end becoming more magenta in the later appearances. Preserve their separate stage numbers when comparing register writes.

No sustained fully-off or solid-white state is visible. The single pale boundary frames are listed above. This recording documents visible output; it does not by itself establish register addresses, byte order, LED count, or which write produced which state. Associate these stage numbers and timestamps with the agent's write log to make that mapping.

## Files and measurement method

`frames/` contains all 921 sequentially decoded, unmodified-colour crops, JPEG quality 96. Crops cover source x=40..1799, y=515..649 (1760 x 135 pixels). Cropping removes the monitor and most unrelated scene content while retaining the full visible front strip and surrounding case.

`index.html` is an offline frame viewer. Extract the entire archive and open it in a browser. Use its slider or arrow keys to step through individual frames. Its phase labels describe the nearby held appearance; they are not per-frame colour classifications during transitions or animation.

`timeline.json` and `timeline.csv` contain the human-reviewed held appearances, sampled animation observations, and pale/mixed boundary frames. The CSV has a `kind` column to distinguish them.

`all_frames_camera_samples.csv` contains one row per decoded frame. Camera RGB values were sampled from the original decoded video, before JPEG export. In each frame the bright strip ridge was located in source y=530..640 using local vertical contrast. At 128 source x positions between 155 and 1690, RGB was taken from a small patch on that ridge. The left/middle/right measurements are medians of 8 columns each, around source x=155..239, 880..964, and 1605..1690 respectively. These are approximate observation regions because the camera moves slightly. The file also contains camera-derived HSV hue in degrees. These clipped/exposure-dependent camera RGB and hue values are NOT direct firmware input values and must not be used to infer RGB ordering without a known-colour test.

The first frame PTS is 0.004200 s; the last is 15.672133 s. MP4 container duration is 15.688767 s. Frame timestamps were obtained from the stream rather than estimated from nominal frame rate.
