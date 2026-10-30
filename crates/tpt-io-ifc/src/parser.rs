//! STEP-Physical File (SPF) parser and [`tpt_io_core::Reader`] implementation.
//!
//! The parser consumes the token stream produced by [`crate::token`] and
//! validates the document structure mandated by ISO 10303-21:
//!
//! ```text
//! ISO-10303-21; HEADER; …; ENDSEC; DATA; …; ENDSEC; END-ISO-10303-21;
//! ```
//!
//! The HEADER section is parsed sequentially into an [`IfcHeader`]. Entity
//! instances in the DATA section(s) are split at `#id` boundaries and parsed
//! in parallel (when the `parallel` feature is enabled) via [`rayon`], since
//! each instance is syntactically independent. A final sequential pass resolves
//! the instance id map and reports dangling references.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::{Encoding, Reader, ValidationSettings, Validator};

use crate::error::IfcError;
use crate::model::{EntityInstance, FileDescription, FileName, IfcFile, IfcHeader, IfcValue};
use crate::token::{tokenize, Token, TokenKind};

/// Parse an IFC document from a string slice.
pub fn parse_str(input: &str) -> Result<IfcFile, IfcError> {
    let tokens = tokenize(input)?;
    parse_tokens(&tokens, input)
}

/// A parsed argument value alongside its source offset (for error reporting).
#[derive(Debug, Clone)]
struct Arg {
    value: IfcValue,
    offset: usize,
}

/// Parse a token stream into an [`IfcFile`].
fn parse_tokens(tokens: &[Token], input: &str) -> Result<IfcFile, IfcError> {
    let mut i = 0usize;

    // 1. File header keyword.
    expect_token(tokens, &mut i, TokenKind::IsoHeader, "ISO-10303-21")?;
    expect_semi(tokens, &mut i)?;

    // 2. HEADER section.
    expect_token_kind(tokens, &mut i, &TokenKind::Header, "HEADER")?;
    expect_semi(tokens, &mut i)?;
    let header = parse_header(tokens, &mut i)?;
    expect_token_kind(tokens, &mut i, &TokenKind::EndSec, "ENDSEC")?;
    expect_semi(tokens, &mut i)?;

    let mut file = IfcFile::new(header);

    // 3. DATA section(s).
    while i < tokens.len() {
        match &tokens[i].kind {
            TokenKind::Data => {
                i += 1;
                expect_semi(tokens, &mut i)?;
                let data_instances = parse_data_section(tokens, &mut i, input)?;
                file.instances.extend(data_instances);
                // Consume the DATA section's trailing ENDSEC;
                expect_token_kind(tokens, &mut i, &TokenKind::EndSec, "ENDSEC")?;
                expect_semi(tokens, &mut i)?;
            }
            TokenKind::IsoTrailer => break,
            other => {
                return Err(IfcError::custom(format!(
                    "expected DATA or END-ISO-10303-21, found {other:?} at offset {}",
                    tokens[i].offset
                )));
            }
        }
    }

    // 4. File trailer keyword.
    if i >= tokens.len() {
        return Err(IfcError::MissingSection("END-ISO-10303-21"));
    }
    expect_token_kind(tokens, &mut i, &TokenKind::IsoTrailer, "END-ISO-10303-21")?;
    expect_semi(tokens, &mut i)?;

    resolve_references(&file)?;
    Ok(file)
}

/// Expect the next token to be `Semi` (`;`).
fn expect_semi(tokens: &[Token], i: &mut usize) -> Result<(), IfcError> {
    expect_token_kind(tokens, i, &TokenKind::Semi, ";")
}

/// Consume the next token, expecting it to equal `expected`.
fn expect_token(
    tokens: &[Token],
    i: &mut usize,
    expected: TokenKind,
    name: &'static str,
) -> Result<(), IfcError> {
    if *i >= tokens.len() {
        return Err(IfcError::MissingSection(name));
    }
    if core::mem::discriminant(&tokens[*i].kind) == core::mem::discriminant(&expected) {
        *i += 1;
        Ok(())
    } else {
        Err(IfcError::custom(format!(
            "expected {name}, found {:?} at offset {}",
            tokens[*i].kind, tokens[*i].offset
        )))
    }
}

/// Consume the next token, asserting its kind matches `expected_kind`.
fn expect_token_kind(
    tokens: &[Token],
    i: &mut usize,
    expected_kind: &TokenKind,
    name: &'static str,
) -> Result<(), IfcError> {
    if *i >= tokens.len() {
        return Err(IfcError::MissingSection(name));
    }
    if core::mem::discriminant(&tokens[*i].kind) == core::mem::discriminant(expected_kind) {
        *i += 1;
        Ok(())
    } else {
        Err(IfcError::custom(format!(
            "expected {name}, found {:?} at offset {}",
            tokens[*i].kind, tokens[*i].offset
        )))
    }
}

/// Parse the HEADER section body (up to `ENDSEC`).
fn parse_header(tokens: &[Token], i: &mut usize) -> Result<IfcHeader, IfcError> {
    let mut header = IfcHeader::default();

    while *i < tokens.len() {
        if matches!(tokens[*i].kind, TokenKind::EndSec) {
            break;
        }
        parse_header_entry(tokens, i, &mut header)?;
    }
    Ok(header)
}

/// Parse a single header entry: `NAME(arg, …);`.
fn parse_header_entry(
    tokens: &[Token],
    i: &mut usize,
    header: &mut IfcHeader,
) -> Result<(), IfcError> {
    let name = match &tokens[*i].kind {
        TokenKind::Ident(n) => n.clone(),
        other => {
            return Err(IfcError::custom(format!(
                "expected header entry name, found {other:?} at offset {}",
                tokens[*i].offset
            )));
        }
    };
    let offset = tokens[*i].offset;
    *i += 1;

    expect_token_kind(tokens, i, &TokenKind::LParen, "(")?;
    let args = parse_arg_list(tokens, i)?;
    expect_token_kind(tokens, i, &TokenKind::RParen, ")")?;
    expect_semi(tokens, i)?;

    match name.as_str() {
        "FILE_DESCRIPTION" => apply_file_description(&args, header, offset)?,
        "FILE_NAME" => apply_file_name(&args, header, offset)?,
        "FILE_SCHEMA" => apply_file_schema(&args, header, offset)?,
        other => {
            // Unknown header entries are tolerated (forward-compat).
            let _ = other;
        }
    }
    Ok(())
}

/// Apply a parsed `FILE_DESCRIPTION((...), '...')` to the header.
fn apply_file_description(
    args: &[Arg],
    header: &mut IfcHeader,
    offset: usize,
) -> Result<(), IfcError> {
    if args.is_empty() {
        return Err(IfcError::missing_argument(offset, "FILE_DESCRIPTION", 0));
    }
    let descs = args[0]
        .value
        .as_list()
        .ok_or_else(|| IfcError::invalid_argument(args[0].offset, "descriptions must be a list"))?;
    let mut descriptions = Vec::with_capacity(descs.len());
    for d in descs {
        descriptions.push(
            d.as_string()
                .ok_or_else(|| {
                    IfcError::invalid_argument(offset, "description item must be a string")
                })?
                .to_string(),
        );
    }
    let implementation_level = if args.len() > 1 {
        args[1]
            .value
            .as_string()
            .ok_or_else(|| {
                IfcError::invalid_argument(args[1].offset, "implementation_level must be a string")
            })?
            .to_string()
    } else {
        String::new()
    };
    header.description = FileDescription { descriptions, implementation_level };
    Ok(())
}

/// Apply a parsed `FILE_NAME('…', '…', (...), (...), '...', '...', '...', '...')`.
fn apply_file_name(args: &[Arg], header: &mut IfcHeader, offset: usize) -> Result<(), IfcError> {
    if args.len() < 7 {
        return Err(IfcError::custom(format!(
            "FILE_NAME requires 7 arguments, found {count} at offset {offset}",
            count = args.len()
        )));
    }
    let name = args[0]
        .value
        .as_string()
        .ok_or_else(|| IfcError::invalid_argument(args[0].offset, "expected string (name)"))?
        .to_string();
    let time_stamp = args[1]
        .value
        .as_string()
        .ok_or_else(|| IfcError::invalid_argument(args[1].offset, "expected string (time_stamp)"))?
        .to_string();
    let authors = parse_string_list(&args[2])?;
    let organizations = parse_string_list(&args[3])?;
    let preprocessor_version = args[4]
        .value
        .as_string()
        .ok_or_else(|| {
            IfcError::invalid_argument(args[4].offset, "expected string (preprocessor)")
        })?
        .to_string();
    let originating_system = args[5]
        .value
        .as_string()
        .ok_or_else(|| {
            IfcError::invalid_argument(args[5].offset, "expected string (originating_system)")
        })?
        .to_string();
    let authorization = args[6]
        .value
        .as_string()
        .ok_or_else(|| {
            IfcError::invalid_argument(args[6].offset, "expected string (authorization)")
        })?
        .to_string();
    header.file_name = FileName {
        name,
        time_stamp,
        authors,
        organizations,
        preprocessor_version,
        originating_system,
        authorization,
    };
    Ok(())
}

/// Parse an argument that must be a list of strings.
fn parse_string_list(a: &Arg) -> Result<Vec<String>, IfcError> {
    let list = a
        .value
        .as_list()
        .ok_or_else(|| IfcError::invalid_argument(a.offset, "expected list argument"))?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        out.push(
            item.as_string()
                .ok_or_else(|| {
                    IfcError::invalid_argument(a.offset, "expected string item in list")
                })?
                .to_string(),
        );
    }
    Ok(out)
}

/// Apply a parsed `FILE_SCHEMA(('IFC4'))`.
fn apply_file_schema(args: &[Arg], header: &mut IfcHeader, offset: usize) -> Result<(), IfcError> {
    if args.is_empty() {
        return Err(IfcError::missing_argument(offset, "FILE_SCHEMA", 0));
    }
    let list = args[0]
        .value
        .as_list()
        .ok_or_else(|| IfcError::invalid_argument(args[0].offset, "FILE_SCHEMA expects a list"))?;
    let mut schemas = Vec::with_capacity(list.len());
    for s in list {
        schemas.push(
            s.as_string()
                .ok_or_else(|| {
                    IfcError::invalid_argument(args[0].offset, "schema must be a string")
                })?
                .to_string(),
        );
    }
    header.schemas = schemas;
    Ok(())
}

/// Parse the DATA section body, returning the parsed instances.
///
/// Instances are first located by scanning for `#id =` boundaries; the slice
/// of tokens belonging to each instance is then handed to
/// [`parse_instance_tokens`] — in parallel when the `parallel` feature is on.
fn parse_data_section(
    tokens: &[Token],
    i: &mut usize,
    _input: &str,
) -> Result<Vec<EntityInstance>, IfcError> {
    // Collect index ranges [start, end) for each instance.
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut inst_start: Option<usize> = None;

    while *i < tokens.len() {
        match &tokens[*i].kind {
            TokenKind::Ref(_) if inst_start.is_none() => {
                inst_start = Some(*i);
            }
            TokenKind::Semi if inst_start.is_some() => {
                let start = inst_start.take();
                ranges.push((start.unwrap(), *i + 1)); // include the Semi
            }
            TokenKind::EndSec => break,
            _ => {}
        }
        *i += 1;
    }

    // Parse each instance's token range, in parallel when the `parallel`
    // feature is enabled and the workload warrants it.
    let results: Vec<Result<EntityInstance, IfcError>> = parse_ranges(tokens, &ranges);

    let mut instances = Vec::with_capacity(results.len());
    for r in results {
        instances.push(r?);
    }
    Ok(instances)
}

/// Parse a set of token ranges into entity instances.
///
/// When the `parallel` feature is enabled and there are enough instances to
/// amortize the thread-pool overhead, the work is distributed across the Rayon
/// thread pool. Otherwise it falls back to sequential parsing.
fn parse_ranges(
    tokens: &[Token],
    ranges: &[(usize, usize)],
) -> Vec<Result<EntityInstance, IfcError>> {
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        let slices: Vec<&[Token]> = ranges.iter().map(|(s, e)| &tokens[*s..*e]).collect();
        if slices.len() >= 64 {
            slices.into_par_iter().map(parse_instance_tokens).collect()
        } else {
            slices.into_iter().map(parse_instance_tokens).collect()
        }
    }
    #[cfg(not(feature = "parallel"))]
    {
        ranges.iter().map(|(s, e)| parse_instance_tokens(&tokens[*s..*e])).collect()
    }
}

/// Parse a single instance token slice: `#id = TYPE(args…);`.
fn parse_instance_tokens(tok: &[Token]) -> Result<EntityInstance, IfcError> {
    let mut i = 0usize;
    let id = match &tok[i].kind {
        TokenKind::Ref(id) => *id,
        _ => {
            return Err(IfcError::custom(format!("expected instance id, found {:?}", tok[i].kind)));
        }
    };
    i += 1;

    if !matches!(tok[i].kind, TokenKind::Equals) {
        return Err(IfcError::custom(format!(
            "expected '=' after instance id, found {:?} at offset {}",
            tok[i].kind, tok[i].offset
        )));
    }
    i += 1;

    let type_name = match &tok[i].kind {
        TokenKind::Ident(n) => n.clone(),
        other => {
            return Err(IfcError::custom(format!(
                "expected entity type name, found {other:?} at offset {}",
                tok[i].offset
            )));
        }
    };
    i += 1;

    // Optional argument list.
    let args = if i < tok.len() && matches!(tok[i].kind, TokenKind::LParen) {
        i += 1;
        parse_arg_list(tok, &mut i)?
    } else {
        Vec::new()
    };

    Ok(EntityInstance { id, type_name, args: args.into_iter().map(|a| a.value).collect() })
}

/// Parse a comma-separated argument list until the matching `)`.
fn parse_arg_list(tokens: &[Token], i: &mut usize) -> Result<Vec<Arg>, IfcError> {
    let mut args: Vec<Arg> = Vec::new();
    // Empty list: immediately followed by RParen.
    if *i < tokens.len() && matches!(tokens[*i].kind, TokenKind::RParen) {
        return Ok(args);
    }
    loop {
        let offset = tokens[*i].offset;
        let value = parse_value(tokens, i)?;
        args.push(Arg { value, offset });
        if *i >= tokens.len() {
            break;
        }
        if matches!(tokens[*i].kind, TokenKind::Comma) {
            *i += 1;
            // Tolerate a trailing comma before the closing ')'.
            if *i < tokens.len() && matches!(tokens[*i].kind, TokenKind::RParen) {
                break;
            }
            continue;
        }
        if matches!(tokens[*i].kind, TokenKind::RParen) {
            break;
        }
        return Err(IfcError::custom(format!(
            "expected ',' or ')' in argument list, found {:?} at offset {}",
            tokens[*i].kind, tokens[*i].offset
        )));
    }
    Ok(args)
}

/// Parse a single value (atom or nested list).
fn parse_value(tokens: &[Token], i: &mut usize) -> Result<IfcValue, IfcError> {
    if *i >= tokens.len() {
        return Err(IfcError::MissingSection("value"));
    }
    let offset = tokens[*i].offset;
    let kind = &tokens[*i].kind;
    let value = match kind {
        TokenKind::Derived | TokenKind::Redefined => IfcValue::Derived,
        TokenKind::Ref(id) => IfcValue::Ref(*id),
        TokenKind::Enum(e) => {
            // Surface booleans: .T. / .F.
            if e == "T" {
                IfcValue::Bool(true)
            } else if e == "F" {
                IfcValue::Bool(false)
            } else {
                IfcValue::Enum(e.clone())
            }
        }
        TokenKind::String(s) => IfcValue::String(s.clone()),
        TokenKind::Integer(s) => {
            let n: i64 = s.parse().map_err(|_| IfcError::invalid_number(offset, s.as_str()))?;
            IfcValue::Integer(n)
        }
        TokenKind::Real(s) => {
            let n: f64 = s.parse().map_err(|_| IfcError::invalid_number(offset, s.as_str()))?;
            IfcValue::Real(n)
        }
        TokenKind::LParen => {
            *i += 1; // consume '('
            let list = parse_arg_list(tokens, i)?;
            if *i < tokens.len() && matches!(tokens[*i].kind, TokenKind::RParen) {
                *i += 1; // consume ')'
            }
            return Ok(IfcValue::List(list.into_iter().map(|a| a.value).collect()));
        }
        other => {
            return Err(IfcError::custom(format!(
                "unexpected token {other:?} where a value was expected at offset {offset}"
            )));
        }
    };
    *i += 1;
    Ok(value)
}

/// Validate that every `#id` reference in the file points to a real instance.
fn resolve_references(file: &IfcFile) -> Result<(), IfcError> {
    let mut ids = BTreeMap::new();
    for inst in &file.instances {
        ids.insert(inst.id, ());
    }
    for inst in &file.instances {
        check_value_refs(&inst.args, &ids, inst.id)?;
    }
    Ok(())
}

fn check_value_refs(
    args: &[IfcValue],
    ids: &BTreeMap<usize, ()>,
    owner: usize,
) -> Result<(), IfcError> {
    for v in args {
        match v {
            IfcValue::Ref(id) => {
                if !ids.contains_key(id) {
                    return Err(IfcError::unknown_instance(0, *id));
                }
                let _ = owner;
            }
            IfcValue::List(inner) => check_value_refs(inner, ids, owner)?,
            _ => {}
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Reader / Validator implementations
// ---------------------------------------------------------------------------

/// Reader for IFC (SPF) documents, implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct IfcReader {
    validation: ValidationSettings,
}

impl IfcReader {
    /// Create a reader with default validation (syntactic + semantic).
    pub fn new() -> Self {
        Self { validation: ValidationSettings::new() }
    }

    /// Set the validation configuration.
    pub fn with_validation(mut self, validation: ValidationSettings) -> Self {
        self.validation = validation;
        self
    }
}

impl Reader<IfcFile> for IfcReader {
    type Error = IfcError;

    fn parse(&self, input: &[u8]) -> Result<IfcFile, IfcError> {
        let text = Encoding::detect(input)
            .as_str(input)
            .ok_or(IfcError::Unsupported("non-UTF-8 IFC input"))?;
        let file = parse_str(text)?;
        self.validation.run(&IfcValidator, &file)?;
        Ok(file)
    }
}

impl tpt_io_core::Writer<IfcFile> for IfcReader {
    type Error = IfcError;

    fn serialize(&self, value: &IfcFile) -> Result<Vec<u8>, IfcError> {
        crate::writer::serialize(value)
    }
}

/// Syntactic / semantic validation for a parsed [`IfcFile`].
#[derive(Debug, Clone, Copy, Default)]
pub struct IfcValidator;

impl Validator<IfcFile> for IfcValidator {
    type Error = IfcError;

    fn validate_syntactic(&self, file: &IfcFile) -> Result<(), IfcError> {
        // Each instance id must be unique.
        let mut seen = BTreeMap::new();
        for inst in &file.instances {
            if seen.contains_key(&inst.id) {
                return Err(IfcError::custom(format!("duplicate instance id #{}", inst.id)));
            }
            seen.insert(inst.id, ());
        }
        // Schema must be declared.
        if file.header.schemas.is_empty() {
            return Err(IfcError::MissingSection("FILE_SCHEMA"));
        }
        Ok(())
    }

    fn validate_semantic(&self, file: &IfcFile) -> Result<(), IfcError> {
        // A well-formed file has at least one instance.
        if file.instances.is_empty() {
            return Err(IfcError::custom("DATA section contains no instances".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_io_core::Reader;

    const MINIMAL: &str = "\
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('test.ifc','2024-01-01',('Author'),('Org'),'Pre','Sys','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1 = IFCWALL('guid',$,'Wall',$,$,$,$,$);
ENDSEC;
END-ISO-10303-21;
";

    #[test]
    fn parses_minimal_file() {
        let file = parse_str(MINIMAL).unwrap();
        assert_eq!(file.header.schemas, vec!["IFC4"]);
        assert_eq!(file.header.file_name.name, "test.ifc");
        assert_eq!(file.instances.len(), 1);
        let inst = &file.instances[0];
        assert_eq!(inst.id, 1);
        assert_eq!(inst.type_name, "IFCWALL");
        assert_eq!(inst.args.len(), 8);
        assert_eq!(inst.args[0], IfcValue::String("guid".to_string()));
        assert_eq!(inst.args[1], IfcValue::Derived);
    }

    #[test]
    fn rejects_invalid_header() {
        let src = "NOT-A-HEADER;\n";
        assert!(parse_str(src).is_err());
    }

    #[test]
    fn rejects_dangling_reference() {
        let src = "\
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition'),'2;1');
FILE_NAME('t','2024',(),(),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1 = IFCWALL('g',$,'w',$,#99,$);
ENDSEC;
END-ISO-10303-21;
";
        let res = parse_str(src);
        assert!(res.is_err(), "expected error for dangling ref, got {res:?}");
    }

    #[test]
    fn reader_runs_validation() {
        let f = IfcReader::new().parse(MINIMAL.as_bytes()).unwrap();
        assert_eq!(f.instances.len(), 1);
    }

    #[test]
    fn parses_multiple_instances() {
        let src = "\
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition'),'2;1');
FILE_NAME('t','2024',(),(),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1 = IFCPERSON($,'Name',$,$,$,$,$);
#2 = IFCORGANIZATION($,'Org',$,$,$);
#3 = IFCPERSONANDORGANIZATION(#1,#2,$);
ENDSEC;
END-ISO-10303-21;
";
        let f = parse_str(src).unwrap();
        assert_eq!(f.instances.len(), 3);
        assert!(f.instance(3).is_some());
        assert_eq!(f.instance(3).unwrap().args[0], IfcValue::Ref(1));
    }

    #[test]
    fn booleans_and_nested_lists() {
        let src = "\
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition'),'2;1');
FILE_NAME('t','2024',(),(),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1 = IFCCARTESIANPOINT((1.0,2.0,3.0));
#2 = IFCBOOLEANCLIPPINGRESULT(.T.,#1,(1,2));
ENDSEC;
END-ISO-10303-21;
";
        let f = parse_str(src).unwrap();
        let pt = f.instance(1).unwrap();
        assert_eq!(
            pt.args[0],
            IfcValue::List(vec![IfcValue::Real(1.0), IfcValue::Real(2.0), IfcValue::Real(3.0)])
        );
        let clip = f.instance(2).unwrap();
        assert_eq!(clip.args[0], IfcValue::Bool(true));
    }
}
