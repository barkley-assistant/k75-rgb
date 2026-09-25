//! RedThunder K75 lighting protocol — library core.
//!
//! This module encodes only what has been **firmware-traced and, where noted,
//! visually verified**. Nothing here invents packet bytes. Every constant is
//! traceable to a disassembly offset (see `docs/protocol-audit.md` and
//! `docs/rendering-architecture.md`); unverified paths are gated behind
//! explicit `Verified` / `Traced` markers and never sent by default.
//!
//! # Verified vs traced
//!
//! * [`MatrixFrame`] (report `0x09`, command `0x08`) — **visually verified**:
//!   slot 0 -> Esc, slot 63 -> UK `;`, both green against an otherwise red
//!   board. Transient (~2 s per frame; bounded repeat keeps keys lit).
//! * [`EffectIndexWrite`] (register block `0x5A 0xAC <idx>`) — **firmware
//!   traced only**, never visually verified. The persistence path (effect code
//!   `0x0F83 == 0x13`) is traced but the `0x0F3F -> 0x0F83` mapping is not
//!   proven. Do not send without an observer.

#![forbid(unsafe_code)]

/// USB vendor/product IDs for the K75, interface 1 (the HID lighting interface).
pub const VID: u16 = 0x258a;
pub const PID: u16 = 0x019d;
pub const INTERFACE: i32 = 1;

/// Length of a complete report-0x09 feature report.
pub const REPORT_LEN: usize = 520;

/// Report ID byte for the lighting feature report.
pub const REPORT_ID: u8 = 0x09;

/// Command byte for the matrix frame (report offset 1).
pub const CMD_MATRIX: u8 = 0x08;

/// Offset of the first RGB triple within the report.
pub const MATRIX_START: usize = 8;

/// Number of RGB slots in the matrix (21 x 6).
pub const MATRIX_SLOTS: usize = 21 * 6;

/// Offsets of the RGB payload (report 8..385, inclusive of start, exclusive of end).
pub const MATRIX_END: usize = MATRIX_START + MATRIX_SLOTS * 3;

/// A single RGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const RED: Rgb = Rgb {
        r: 0xff,
        g: 0x00,
        b: 0x00,
    };
    pub const GREEN: Rgb = Rgb {
        r: 0x00,
        g: 0xff,
        b: 0x00,
    };
    pub const OFF: Rgb = Rgb {
        r: 0x00,
        g: 0x00,
        b: 0x00,
    };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }

    pub const fn as_array(&self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }
}

/// A complete report-0x09 / command-0x08 matrix frame.
///
/// Every slot is set to `baseline`, except `highlight` (if any), which is set
/// to `highlight_colour`. This is the discriminating-observation pattern used
/// for the verified slot mapping: one changed slot against a uniform board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixFrame {
    bytes: [u8; REPORT_LEN],
}

impl MatrixFrame {
    /// Build a frame with a uniform `baseline`, optionally overriding one slot.
    ///
    /// Returns `None` if `highlight` is out of range.
    pub fn new(baseline: Rgb, highlight: Option<(usize, Rgb)>) -> Option<Self> {
        if let Some((slot, _)) = highlight {
            if slot >= MATRIX_SLOTS {
                return None;
            }
        }
        let mut bytes = [0u8; REPORT_LEN];
        bytes[0] = REPORT_ID;
        bytes[1] = CMD_MATRIX;
        for slot in 0..MATRIX_SLOTS {
            let colour = match highlight {
                Some((hs, c)) if hs == slot => c,
                _ => baseline,
            };
            let start = MATRIX_START + slot * 3;
            bytes[start..start + 3].copy_from_slice(&colour.as_array());
        }
        Some(MatrixFrame { bytes })
    }

    /// The full 520-byte report, ready to send as a feature report.
    pub fn as_bytes(&self) -> &[u8; REPORT_LEN] {
        &self.bytes
    }

    /// The colour of a single slot (report-space slot index).
    pub fn slot(&self, slot: usize) -> Option<Rgb> {
        if slot >= MATRIX_SLOTS {
            return None;
        }
        let start = MATRIX_START + slot * 3;
        Some(Rgb::new(
            self.bytes[start],
            self.bytes[start + 1],
            self.bytes[start + 2],
        ))
    }
}

/// A traced-but-unverified register-block write (effect index).
///
/// This is the `0x5A 0xAC <idx>` block documented in
/// `docs/rendering-architecture.md`. It is the candidate persistence lever but
/// has **not** been visually verified; the caller must opt in explicitly and is
/// responsible for observing the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectIndexWrite {
    pub index: u8,
}

impl EffectIndexWrite {
    /// Register-block command byte for the effect index.
    pub const CMD: u8 = 0xAC;
    /// Magic byte that must precede the command.
    pub const MAGIC: u8 = 0x5A;

    /// The 20-byte register block, `0x5A 0xAC <index>` followed by zero fill.
    pub fn to_block(&self) -> [u8; 20] {
        let mut block = [0u8; 20];
        block[0] = Self::MAGIC;
        block[1] = Self::CMD;
        block[2] = self.index;
        block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_uniform_baseline() {
        let frame = MatrixFrame::new(Rgb::RED, None).unwrap();
        let bytes = frame.as_bytes();
        assert_eq!(&bytes[..8], &[0x09, 0x08, 0, 0, 0, 0, 0, 0]);
        assert!(bytes[MATRIX_START..MATRIX_END]
            .chunks_exact(3)
            .all(|rgb| rgb == Rgb::RED.as_array()));
        assert!(bytes[MATRIX_END..].iter().all(|b| *b == 0));
    }

    #[test]
    fn highlight_changes_exactly_one_slot() {
        let baseline = MatrixFrame::new(Rgb::RED, None).unwrap();
        for slot in [0, 5, 6, 63, 125] {
            let candidate = MatrixFrame::new(Rgb::RED, Some((slot, Rgb::GREEN))).unwrap();
            let diffs: Vec<usize> = (0..REPORT_LEN)
                .filter(|&i| candidate.as_bytes()[i] != baseline.as_bytes()[i])
                .collect();
            assert_eq!(
                diffs,
                vec![MATRIX_START + slot * 3, MATRIX_START + slot * 3 + 1]
            );
            assert_eq!(candidate.slot(slot), Some(Rgb::GREEN));
        }
    }

    #[test]
    fn out_of_range_slot_rejected() {
        assert!(MatrixFrame::new(Rgb::RED, Some((MATRIX_SLOTS, Rgb::GREEN))).is_none());
        assert!(MatrixFrame::new(Rgb::RED, Some((MATRIX_SLOTS, Rgb::GREEN))).is_none());
    }

    #[test]
    fn effect_block_shape() {
        let block = EffectIndexWrite { index: 0x13 }.to_block();
        assert_eq!(block[0], 0x5A);
        assert_eq!(block[1], 0xAC);
        assert_eq!(block[2], 0x13);
        assert!(block[3..].iter().all(|b| *b == 0));
    }
}
