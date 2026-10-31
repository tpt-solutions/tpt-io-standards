//! Writing the 3MF package and its XML model part.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::{xml, zip};

use crate::{Model, MODEL_PART};

const NAMESPACE: &str = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02";

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>
"#;

const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>
"#;

pub(crate) fn package(model: &Model) -> Vec<u8> {
    let xml = model_xml(model);
    zip::write_stored(&[
        ("[Content_Types].xml", CONTENT_TYPES.as_bytes()),
        ("_rels/.rels", RELS.as_bytes()),
        (MODEL_PART, xml.as_bytes()),
    ])
}

pub(crate) fn model_xml(model: &Model) -> String {
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        s,
        "<model unit=\"{}\" xmlns=\"{NAMESPACE}\">",
        xml::escape_attr(&model.unit)
    );
    s.push_str("<resources>\n");
    for o in &model.objects {
        let _ = write!(s, "<object id=\"{}\" type=\"model\"", o.id);
        if let Some(name) = &o.name {
            let _ = write!(s, " name=\"{}\"", xml::escape_attr(name));
        }
        s.push_str(">\n<mesh>\n<vertices>\n");
        for v in &o.vertices {
            let _ = writeln!(s, "<vertex x=\"{}\" y=\"{}\" z=\"{}\"/>", v[0], v[1], v[2]);
        }
        s.push_str("</vertices>\n<triangles>\n");
        for t in &o.triangles {
            let _ = writeln!(s, "<triangle v1=\"{}\" v2=\"{}\" v3=\"{}\"/>", t[0], t[1], t[2]);
        }
        s.push_str("</triangles>\n</mesh>\n</object>\n");
    }
    s.push_str("</resources>\n<build>\n");
    for item in &model.build {
        let _ = write!(s, "<item objectid=\"{}\"", item.object_id);
        if let Some(t) = item.transform {
            let values: Vec<String> = t.iter().map(|c| alloc::format!("{c}")).collect();
            let _ = write!(s, " transform=\"{}\"", values.join(" "));
        }
        s.push_str("/>\n");
    }
    s.push_str("</build>\n</model>\n");
    s
}
