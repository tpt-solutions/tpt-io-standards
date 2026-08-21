//! Data model for parsed WellLog files (LAS 2.0/3.0 ASCII).

use alloc::string::String;
use alloc::vec::Vec;

/// A curve (log channel) declared in the `~C` (Curve Information) section.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Curve {
    /// Curve mnemonic, e.g. `DEPT`, `GR`, `ILD`.
    pub mnemonic: String,
    /// Unit of measure, e.g. `M`, `API`, `OHMM`.
    pub unit: String,
    /// API log type code (the value field of a `~C` line).
    pub api_code: String,
    /// Human-readable description.
    pub description: String,
}

/// A `~W` (Well Information) or `~O` (Parameter) line: mnemonic, unit, value,
/// description.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Parameter {
    /// Field mnemonic.
    pub mnemonic: String,
    /// Unit of measure.
    pub unit: String,
    /// Stored value (kept as text to preserve formatting).
    pub value: String,
    /// Human-readable description.
    pub description: String,
}

/// Well identification and header metadata from the `~W` section. Commonly used
/// mnemonics are surfaced as typed fields; everything else is preserved in
/// [`WellInfo::extra`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WellInfo {
    /// `WELL` — well name.
    pub well_name: Option<String>,
    /// `FLD` — field name.
    pub field: Option<String>,
    /// `LOC` — location.
    pub location: Option<String>,
    /// `CTY` — county.
    pub county: Option<String>,
    /// `STA` — state/province.
    pub state: Option<String>,
    /// `CNT` — country.
    pub country: Option<String>,
    /// `SRVC` — service company.
    pub service_company: Option<String>,
    /// `DATE` — logging date.
    pub date: Option<String>,
    /// `API` — API well number / UWI.
    pub api: Option<String>,
    /// `NULL` — numeric null (empty) value for the dataset.
    pub null_value: f64,
    /// Any unrecognized `~W` mnemonics, preserved verbatim.
    pub extra: Vec<Parameter>,
}

/// A fully parsed LAS well-log.
#[derive(Debug, Clone, PartialEq)]
pub struct WellLog {
    /// `VERS` from the `~V` section (e.g. `2.0`).
    pub version: f32,
    /// `WRAP` from the `~V` section (`YES` ⇒ multi-line rows).
    pub wrap: bool,
    /// Well header metadata.
    pub well: WellInfo,
    /// Parameter lines from the `~O` section.
    pub parameters: Vec<Parameter>,
    /// Curve definitions from the `~C` section (column 0 is the index curve).
    pub curves: Vec<Curve>,
    /// Depth/index + log matrix: `data[row][col]`, one column per curve.
    pub data: Vec<Vec<f64>>,
}

impl WellLog {
    /// Number of samples (data rows).
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the log contains no data rows.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Index (depth) values — column 0, the first curve.
    pub fn index_values(&self) -> impl Iterator<Item = f64> + '_ {
        self.data.iter().map(|row| row[0])
    }

    /// A single channel by column index.
    pub fn channel(&self, col: usize) -> impl Iterator<Item = f64> + '_ {
        self.data.iter().map(move |row| row[col])
    }
}
