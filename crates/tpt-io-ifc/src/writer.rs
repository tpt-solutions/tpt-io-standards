//! Serialization of an [`IfcFile`] back to STEP-Physical File (SPF) text.
//!
//! The output is canonical: the HEADER section is emitted in the mandated order
//! (`FILE_DESCRIPTION`, `FILE_NAME`, `FILE_SCHEMA`), followed by the DATA
//! section with every instance in file order. Re-parsing the output yields a
//! structurally identical [`IfcFile`] (see the round-trip tests).

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::error::IfcError;
use crate::model::{EntityInstance, IfcFile, IfcValue};

/// Serialize an [`IfcFile`] into SPF (ISO 10303-21) bytes.
pub fn serialize(file: &IfcFile) -> Result<Vec<u8>, IfcError> {
    let mut out = String::new();

    out.push_str("ISO-10303-21;\n");
    out.push_str("HEADER;\n");

    // FILE_DESCRIPTION
    let descs = &file.header.description.descriptions;
    out.push_str(&format!(
        "FILE_DESCRIPTION(('{}'),'{}');\n",
        descs.join("','"),
        file.header.description.implementation_level
    ));

    // FILE_NAME
    let fnm = &file.header.file_name;
    out.push_str(&format!(
        "FILE_NAME('{}','{}',('{}'),('{}'),'{}','{}','{}');\n",
        fnm.name,
        fnm.time_stamp,
        fnm.authors.join("','"),
        fnm.organizations.join("','"),
        fnm.preprocessor_version,
        fnm.originating_system,
        fnm.authorization
    ));

    // FILE_SCHEMA
    out.push_str(&format!("FILE_SCHEMA(('{}'));\n", file.header.schemas.join("','")));

    out.push_str("ENDSEC;\n");

    // DATA
    out.push_str("DATA;\n");
    for inst in &file.instances {
        serialize_instance(&mut out, inst);
        out.push('\n');
    }
    out.push_str("ENDSEC;\n");

    out.push_str("END-ISO-10303-21;\n");
    Ok(out.into_bytes())
}

fn serialize_instance(out: &mut String, inst: &EntityInstance) {
    out.push_str(&format!("#{} = {}(", inst.id, inst.type_name));
    for (i, arg) in inst.args.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        serialize_value(out, arg);
    }
    out.push_str(");");
}

fn serialize_value(out: &mut String, value: &IfcValue) {
    match value {
        IfcValue::Derived => out.push('$'),
        IfcValue::Ref(id) => out.push_str(&format!("#{id}")),
        IfcValue::Enum(e) => out.push_str(&format!(".{e}.")),
        IfcValue::String(s) => out.push_str(&format!("'{}'", s.replace('\'', "''"))),
        IfcValue::Integer(n) => out.push_str(&format!("{n}")),
        IfcValue::Real(r) => {
            // Ensure a decimal point so the value round-trips as a real.
            let s = format!("{r}");
            if s.contains('.') || s.contains('e') || s.contains('E') {
                out.push_str(&s);
            } else {
                out.push_str(&format!("{s}.0"));
            }
        }
        IfcValue::Bool(b) => out.push_str(if *b { ".T." } else { ".F." }),
        IfcValue::List(items) => {
            out.push('(');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                serialize_value(out, item);
            }
            out.push(')');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_str;

    #[test]
    fn round_trips_minimal() {
        let src = "\
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
        let file = parse_str(src).unwrap();
        let bytes = serialize(&file).unwrap();
        let reparsed = parse_str(core::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(file, reparsed);
    }

    #[test]
    fn serializes_all_value_kinds() {
        let file = IfcFile {
            header: crate::model::IfcHeader {
                description: crate::model::FileDescription {
                    descriptions: vec!["ViewDefinition".to_string()],
                    implementation_level: "2;1".to_string(),
                },
                file_name: crate::model::FileName {
                    name: "t.ifc".to_string(),
                    time_stamp: "2024".to_string(),
                    authors: vec![],
                    organizations: vec![],
                    preprocessor_version: String::new(),
                    originating_system: String::new(),
                    authorization: String::new(),
                },
                schemas: vec!["IFC4".to_string()],
            },
            instances: vec![
                EntityInstance::new(
                    1,
                    "IFCCARTESIANPOINT",
                    vec![IfcValue::List(vec![
                        IfcValue::Real(1.0),
                        IfcValue::Real(2.0),
                        IfcValue::Real(3.0),
                    ])],
                ),
                EntityInstance::new(
                    2,
                    "IFCPROPERTY",
                    vec![
                        IfcValue::String("Name".to_string()),
                        IfcValue::Enum("P_SINGLEVALUE".to_string()),
                        IfcValue::Ref(1),
                        IfcValue::Integer(42),
                        IfcValue::Bool(true),
                        IfcValue::Derived,
                    ],
                ),
            ],
        };
        let bytes = serialize(&file).unwrap();
        let text = core::str::from_utf8(&bytes).unwrap();
        assert!(text.contains("#1 = IFCCARTESIANPOINT((1.0,2.0,3.0));"));
        assert!(text.contains("#2 = IFCPROPERTY('Name',.P_SINGLEVALUE.,#1,42,.T.,$);"));
    }
}
