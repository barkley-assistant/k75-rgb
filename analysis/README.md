# analysis/ — raw RE artifacts

Working material from the reverse-engineering process. Not canonical
documentation — for that see `../docs/`. Some files encode hypotheses that
were later disproven; check `../docs/protocol/protocol-audit.md` for what is
currently accepted.

## Contents

| Path | What it is |
|---|---|
| `disasm_v2.txt` | Full 8051 disassembly of the stock firmware (3.1 MB, primary RE source) |
| `key-led-map.txt` | Vendor LED ID table (`idx = col×6 + row`) from the official tool |
| `fn1b46-decode.md` | Subagent decode of the per-key matrix walker (historical, has caveats) |
| `config-dumps/*.bin` | 72-byte config-profile test images (see `build_tests.py`) |
| `test-session-1.md`, `test-session-2.md` | Logs of early live test sessions |
| `verify_matrix_path.py` | Offline firmware-signature guard (run before trusting RE claims) |
| `build_tests.py` | Regenerates the `config-dumps/*.bin` test images |
| `video-evidence-2026-09-26/` | Case-strip video analysis: frames, viewer, per-frame samples |