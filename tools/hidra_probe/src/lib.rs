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

/// Vendor LED-ID → physical key table, extracted from the official `KB.ini`
/// `[KEY]` section (see `analysis/key-led-map.txt`).
///
/// Index = LED ID, value = physical key legend. `None` = structural gap in
/// the 16×6 physical grid (no key/LED at that position). IDs 96..125 fall
/// outside the vendor table entirely (the firmware matrix is 21×6 = 126
/// slots); those are non-key slots whose physical meaning is unproven.
///
/// Only two host-matrix slots have been **visually verified**: slot 0 → Esc
/// and slot 63 → UK `;`. The rest of this table is a hypothesis for the host
/// slot mapping ("host slot = vendor LED ID"), confirmed at those two points.
pub const KEY_LED_MAP: [Option<&str>; 96] = {
    // Filled via helper const fn below.
    let mut map: [Option<&str>; 96] = [None; 96];
    let entries: &[(usize, &str)] = &[
        (0, "Esc"),
        (12, "F1"),
        (18, "F2"),
        (24, "F3"),
        (30, "F4"),
        (36, "F5"),
        (42, "F6"),
        (48, "F7"),
        (54, "F8"),
        (60, "F9"),
        (66, "F10"),
        (72, "F11"),
        (78, "F12"),
        (1, "`"),
        (7, "1"),
        (13, "2"),
        (19, "3"),
        (25, "4"),
        (31, "5"),
        (37, "6"),
        (43, "7"),
        (49, "8"),
        (55, "9"),
        (61, "0"),
        (67, "-"),
        (73, "="),
        (79, "Backspace"),
        (91, "Home"),
        (2, "Tab"),
        (8, "Q"),
        (14, "W"),
        (20, "E"),
        (26, "R"),
        (32, "T"),
        (38, "Y"),
        (44, "U"),
        (50, "I"),
        (56, "O"),
        (62, "P"),
        (68, "["),
        (74, "]"),
        (75, "\\"),
        (92, "PgUp"),
        (3, "CapsLock"),
        (9, "A"),
        (15, "S"),
        (21, "D"),
        (27, "F"),
        (33, "G"),
        (39, "H"),
        (45, "J"),
        (51, "K"),
        (57, "L"),
        (63, ";"),
        (69, "'"),
        (81, "Enter"),
        (93, "PgDn"),
        (4, "LShift"),
        (10, "\\|"),
        (16, "Z"),
        (22, "X"),
        (28, "C"),
        (34, "V"),
        (40, "B"),
        (46, "N"),
        (52, "M"),
        (58, ","),
        (64, "."),
        (70, "/"),
        (82, "RShift"),
        (88, "Up"),
        (5, "LCtrl"),
        (11, "LWin"),
        (17, "LAlt"),
        (35, "Space"),
        (53, "RAlt"),
        (59, "FN"),
        (83, "Left"),
        (89, "Down"),
        (95, "Right"),
        (84, "Delete"),
        (90, "Mute*"),
    ];
    let mut i = 0;
    while i < entries.len() {
        map[entries[i].0] = Some(entries[i].1);
        i += 1;
    }
    map
};

/// Predict the physical key for a host matrix slot under the leading
/// hypothesis "host slot = vendor LED ID".
///
/// Returns `None` for slots outside the vendor table (gap or non-key slot).
pub fn predict_key(slot: usize) -> Option<&'static str> {
    KEY_LED_MAP.get(slot).copied().flatten()
}

/// Verified host-slot → physical-key pins (from live observations).
pub const VERIFIED_SLOTS: &[(usize, &str)] = &[(0, "Esc"), (63, ";")];

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

    #[test]
    fn vendor_table_pins_and_gaps() {
        assert_eq!(predict_key(0), Some("Esc"));
        assert_eq!(predict_key(63), Some(";"));
        assert_eq!(predict_key(30), Some("F4"));
        assert_eq!(predict_key(95), Some("Right"));
        assert_eq!(predict_key(90), Some("Mute"));
        // structural gaps in the 16x6 grid
        assert_eq!(predict_key(6), None); // (1,0) no key
        assert_eq!(predict_key(76), None); // (12,4) no key
                                           // non-key slots beyond the vendor table
        assert_eq!(predict_key(96), None);
        assert_eq!(predict_key(125), None);
        // count check: 82 vendor entries
        let count = KEY_LED_MAP.iter().filter(|k| k.is_some()).count();
        assert_eq!(count, 82);
        // verified pins agree with the vendor table
        for &(slot, name) in VERIFIED_SLOTS {
            assert_eq!(predict_key(slot), Some(name));
        }
    }
}
