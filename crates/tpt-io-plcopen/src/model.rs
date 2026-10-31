//! The PLCopen project model.

use alloc::string::String;
use alloc::vec::Vec;

/// What a POU is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PouKind {
    /// `pouType="program"`.
    Program,
    /// `pouType="functionBlock"`.
    FunctionBlock,
    /// `pouType="function"`.
    Function,
    /// Any other type string, kept verbatim.
    Other(String),
}

/// The IEC 61131-3 language of a body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Language {
    /// Structured Text.
    St,
    /// Instruction List.
    Il,
    /// Function Block Diagram (networks not decoded).
    Fbd,
    /// Ladder Diagram (networks not decoded).
    Ld,
    /// Sequential Function Chart (networks not decoded).
    Sfc,
    /// Any other body element, kept by name.
    Other(String),
}

/// A POU body: its language and, for textual languages, its source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    /// The language element the body uses.
    pub language: Language,
    /// The source text for ST and IL; empty for graphical languages.
    pub text: String,
}

/// One variable declaration in a POU interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    /// The interface section it belongs to, e.g. `localVars`, `inputVars`, `outputVars`.
    pub class: String,
    /// Variable name.
    pub name: String,
    /// Declared type: an elementary name such as `INT`, or a derived type's name.
    pub type_name: String,
    /// The initial value as written in `simpleValue`, if any.
    pub initial: Option<String>,
}

/// A program organisation unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pou {
    /// POU name.
    pub name: String,
    /// Program, function block or function.
    pub kind: PouKind,
    /// Interface variables in declaration order.
    pub variables: Vec<Variable>,
    /// The body, when present.
    pub body: Option<Body>,
}

/// A PLCopen project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Project {
    /// `contentHeader/@name`, if present.
    pub name: Option<String>,
    /// `fileHeader/@productName`, if present.
    pub product: Option<String>,
    /// Names of the data types declared under `types/dataTypes`.
    pub data_types: Vec<String>,
    /// POUs in document order.
    pub pous: Vec<Pou>,
}
