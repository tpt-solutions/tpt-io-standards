//! Integration tests for the PLCopen XML reader.

use tpt_io_plcopen::{parse, Language, PlcOpenError, PouKind};

const PROJECT: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<project xmlns="http://www.plcopen.org/xml/tc6_0201">
  <fileHeader companyName="Test" productName="Demo PLC" productVersion="1.0"
              creationDateTime="2024-01-01T00:00:00"/>
  <contentHeader name="Conveyor"/>
  <types>
    <dataTypes>
      <dataType name="MOTOR_STATE"><enum/></dataType>
    </dataTypes>
    <pous>
      <pou name="Counter" pouType="functionBlock">
        <interface>
          <inputVars>
            <variable name="enable"><type><BOOL/></type></variable>
          </inputVars>
          <localVars>
            <variable name="count"><type><INT/></type>
              <initialValue><simpleValue value="0"/></initialValue>
            </variable>
            <variable name="state"><type><derived name="MOTOR_STATE"/></type></variable>
          </localVars>
        </interface>
        <body>
          <ST><xhtml xmlns="http://www.w3.org/1999/xhtml">IF enable THEN count := count + 1; END_IF;</xhtml></ST>
        </body>
      </pou>
      <pou name="Main" pouType="program">
        <interface><localVars><variable name="x"><type><REAL/></type></variable></localVars></interface>
        <body><FBD><inVariable/></FBD></body>
      </pou>
      <pou name="Scale" pouType="function">
        <body><IL><xhtml>LD 1.5&#10;MUL 2</xhtml></IL></body>
      </pou>
    </pous>
  </types>
</project>"#;

#[test]
fn reads_header_data_types_and_pous() {
    let p = parse(PROJECT).unwrap();
    assert_eq!(p.name.as_deref(), Some("Conveyor"));
    assert_eq!(p.product.as_deref(), Some("Demo PLC"));
    assert_eq!(p.data_types, ["MOTOR_STATE"]);
    let names: Vec<&str> = p.pous.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, ["Counter", "Main", "Scale"]);
    assert_eq!(p.pous[0].kind, PouKind::FunctionBlock);
    assert_eq!(p.pous[1].kind, PouKind::Program);
    assert_eq!(p.pous[2].kind, PouKind::Function);
}

#[test]
fn variables_keep_their_section_type_and_initial_value() {
    let p = parse(PROJECT).unwrap();
    let vars = &p.pous[0].variables;
    assert_eq!(vars.len(), 3);
    assert_eq!((vars[0].class.as_str(), vars[0].type_name.as_str()), ("inputVars", "BOOL"));
    assert_eq!(vars[1].initial.as_deref(), Some("0"));
    assert_eq!(vars[2].type_name, "MOTOR_STATE");
}

#[test]
fn bodies_carry_text_for_textual_languages_only() {
    let p = parse(PROJECT).unwrap();
    let st = p.pous[0].body.as_ref().unwrap();
    assert_eq!(st.language, Language::St);
    assert_eq!(st.text, "IF enable THEN count := count + 1; END_IF;");
    let fbd = p.pous[1].body.as_ref().unwrap();
    assert_eq!(fbd.language, Language::Fbd);
    assert!(fbd.text.is_empty());
    let il = p.pous[2].body.as_ref().unwrap();
    assert_eq!(il.language, Language::Il);
    assert_eq!(il.text, "LD 1.5\nMUL 2");
}

#[test]
fn a_pou_without_a_body_has_none() {
    let p = parse(b"<project><types><pous><pou name=\"Empty\" pouType=\"program\"/></pous></types></project>").unwrap();
    assert!(p.pous[0].body.is_none());
    assert!(p.pous[0].variables.is_empty());
}

#[test]
fn malformed_projects_are_errors() {
    assert!(parse(b"<notaproject/>").is_err());
    assert!(parse(b"<project><types><pous><pou/></pous></types></project>").is_err());
    assert!(matches!(
        parse(b"<project><types><pous><pou name=\"x\"><interface><localVars><variable/></localVars></interface></pou></pous></types></project>"),
        Err(PlcOpenError::Core(_))
    ));
}

#[test]
fn unknown_pou_type_is_kept() {
    let p = parse(b"<project><types><pous><pou name=\"c\" pouType=\"class\"/></pous></types></project>").unwrap();
    assert_eq!(p.pous[0].kind, PouKind::Other("class".into()));
}

