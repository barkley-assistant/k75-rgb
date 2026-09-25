//! Disabled: this probe attributed a 21x6 RGB matrix writer to a six-LED
//! side zone without proving the USB staging layout.
//! See docs/protocol-audit.md before building a replacement.

fn main() {
    eprintln!("sidelight is disabled: the six-LED side-zone packet is not supported by firmware evidence. See docs/protocol-audit.md");
    std::process::exit(2);
}
