//! Disabled: this probe treated a config/flash-buffer write as a positional
//! per-key color stream. Its zero-filled payload blanked the key lighting.
//! See docs/protocol-audit.md before building a replacement.

fn main() {
    eprintln!("setkey is disabled: positional RGB mapping is unverified and this probe blanked the keys. See docs/protocol-audit.md");
    std::process::exit(2);
}
