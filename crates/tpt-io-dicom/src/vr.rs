//! Value Representation (VR) handling (PS3.5 §6.2).

use core::fmt;

/// A DICOM Value Representation — the data type of an element's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vr {
    /// Application Entity.
    Ae,
    /// Age String.
    As,
    /// Attribute Tag.
    At,
    /// Code String.
    Cs,
    /// Date.
    Da,
    /// Decimal String.
    Ds,
    /// Date Time.
    Dt,
    /// Floating Point Single.
    Fl,
    /// Floating Point Double.
    Fd,
    /// Integer String.
    Is,
    /// Long String.
    Lo,
    /// Long Text.
    Lt,
    /// Other Byte.
    Ob,
    /// Other Double.
    Od,
    /// Other Float.
    Of,
    /// Other Very Long.
    Ov,
    /// Other Word.
    Ow,
    /// Personal Name.
    Pn,
    /// Short String.
    Sh,
    /// Signed Long.
    Sl,
    /// Sequence of Items.
    Sq,
    /// Short Text.
    St,
    /// Signed Very Long.
    Sv,
    /// Short Signed? — no: Time.
    Tm,
    /// Unlimited Characters.
    Uc,
    /// Unique Identifier (UID).
    Ui,
    /// Unsigned Long.
    Ul,
    /// Unknown.
    Un,
    /// Universal Resource Identifier or Universal Resource Locator (URI/URL).
    Ur,
    /// Unsigned Short.
    Us,
    /// Unlimited Text.
    Ut,
    /// Unsigned Very Long.
    Uv,
}

impl Vr {
    /// Parse a two-letter VR code as found in Explicit VR encoding.
    pub fn from_bytes(b: &[u8]) -> Option<Vr> {
        if b.len() < 2 {
            return None;
        }
        let s: [u8; 2] = [b[0].to_ascii_uppercase(), b[1].to_ascii_uppercase()];
        Some(match &s {
            b"AE" => Vr::Ae,
            b"AS" => Vr::As,
            b"AT" => Vr::At,
            b"CS" => Vr::Cs,
            b"DA" => Vr::Da,
            b"DS" => Vr::Ds,
            b"DT" => Vr::Dt,
            b"FL" => Vr::Fl,
            b"FD" => Vr::Fd,
            b"IS" => Vr::Is,
            b"LO" => Vr::Lo,
            b"LT" => Vr::Lt,
            b"OB" => Vr::Ob,
            b"OD" => Vr::Od,
            b"OF" => Vr::Of,
            b"OV" => Vr::Ov,
            b"OW" => Vr::Ow,
            b"PN" => Vr::Pn,
            b"SH" => Vr::Sh,
            b"SL" => Vr::Sl,
            b"SQ" => Vr::Sq,
            b"ST" => Vr::St,
            b"SV" => Vr::Sv,
            b"TM" => Vr::Tm,
            b"UC" => Vr::Uc,
            b"UI" => Vr::Ui,
            b"UL" => Vr::Ul,
            b"UN" => Vr::Un,
            b"UR" => Vr::Ur,
            b"US" => Vr::Us,
            b"UT" => Vr::Ut,
            b"UV" => Vr::Uv,
            _ => return None,
        })
    }

    /// The two-letter ASCII code for this VR.
    pub const fn as_bytes(&self) -> &'static [u8; 2] {
        match self {
            Vr::Ae => b"AE",
            Vr::As => b"AS",
            Vr::At => b"AT",
            Vr::Cs => b"CS",
            Vr::Da => b"DA",
            Vr::Ds => b"DS",
            Vr::Dt => b"DT",
            Vr::Fl => b"FL",
            Vr::Fd => b"FD",
            Vr::Is => b"IS",
            Vr::Lo => b"LO",
            Vr::Lt => b"LT",
            Vr::Ob => b"OB",
            Vr::Od => b"OD",
            Vr::Of => b"OF",
            Vr::Ov => b"OV",
            Vr::Ow => b"OW",
            Vr::Pn => b"PN",
            Vr::Sh => b"SH",
            Vr::Sl => b"SL",
            Vr::Sq => b"SQ",
            Vr::St => b"ST",
            Vr::Sv => b"SV",
            Vr::Tm => b"TM",
            Vr::Uc => b"UC",
            Vr::Ui => b"UI",
            Vr::Ul => b"UL",
            Vr::Un => b"UN",
            Vr::Ur => b"UR",
            Vr::Us => b"US",
            Vr::Ut => b"UT",
            Vr::Uv => b"UV",
        }
    }

    /// Whether values of this VR use the 32-bit (extended) length field in
    /// Explicit VR transfer syntaxes (PS3.5 §7.1.2): `OB, OD, OF, OV, OW, SQ,
    /// UC, UR, UT, UN` plus `SV`/`UV` per newer editions.
    pub const fn uses_extended_length(&self) -> bool {
        matches!(
            self,
            Vr::Ob
                | Vr::Od
                | Vr::Of
                | Vr::Ov
                | Vr::Ow
                | Vr::Sq
                | Vr::Uc
                | Vr::Ur
                | Vr::Ut
                | Vr::Un
                | Vr::Sv
                | Vr::Uv
        )
    }

    /// Whether this is a fixed-width numeric (binary) VR and its width in
    /// bytes per value (PS3.5 §6.2). Returns [`None`] for text and blob VRs.
    pub const fn binary_width(&self) -> Option<usize> {
        match self {
            Vr::At | Vr::Fl => Some(4),
            Vr::Fd => Some(8),
            Vr::Sl | Vr::Ul => Some(4),
            Vr::Sv | Vr::Uv => Some(8),
            Vr::Us => Some(2),
            _ => None,
        }
    }
}

impl fmt::Display for Vr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(core::str::from_utf8(self.as_bytes()).unwrap_or("??"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_back() {
        for vr in [
            Vr::Ae,
            Vr::As,
            Vr::At,
            Vr::Cs,
            Vr::Da,
            Vr::Ds,
            Vr::Dt,
            Vr::Fl,
            Vr::Fd,
            Vr::Is,
            Vr::Lo,
            Vr::Lt,
            Vr::Ob,
            Vr::Od,
            Vr::Of,
            Vr::Ov,
            Vr::Ow,
            Vr::Pn,
            Vr::Sh,
            Vr::Sl,
            Vr::Sq,
            Vr::St,
            Vr::Sv,
            Vr::Tm,
            Vr::Uc,
            Vr::Ui,
            Vr::Ul,
            Vr::Un,
            Vr::Ur,
            Vr::Us,
            Vr::Ut,
            Vr::Uv,
        ] {
            assert_eq!(Vr::from_bytes(vr.as_bytes()), Some(vr));
            assert_eq!(vr.to_string().len(), 2);
        }
        assert_eq!(Vr::from_bytes(b"XX"), None);
        assert_eq!(Vr::from_bytes(b"O"), None);
    }

    #[test]
    fn extended_length_vrs() {
        assert!(Vr::Sq.uses_extended_length());
        assert!(Vr::Ob.uses_extended_length());
        assert!(!Vr::Ui.uses_extended_length());
        assert!(!Vr::Us.uses_extended_length());
    }
}
