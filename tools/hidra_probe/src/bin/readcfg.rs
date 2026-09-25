//! Disabled: this probe sends report-0x09 command 0x04.
//!
//! Live testing showed that command to reload stale flash state and leave the
//! lighting off. It is not a safe read operation. See docs/recovery.md.
fn main() {
    eprintln!("readcfg is disabled: report-0x09 command 0x04 is forbidden. See docs/recovery.md");
    std::process::exit(2);
}
