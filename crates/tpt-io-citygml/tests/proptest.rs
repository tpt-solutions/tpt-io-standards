//! Property-based tests: generated CityGML documents keep their structure
//! through parsing, and arbitrary input never panics.

use proptest::prelude::*;
use tpt_io_citygml::model::{CityObjectKind, Lod, ThematicKey};
use tpt_io_citygml::parser::parse_str;

/// Build a CityGML 2.0 document with one building per `(lod, year)` pair.
fn building_doc(buildings: &[(u32, u16)]) -> String {
    let mut out = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"
                xmlns:gml="http://www.opengis.net/gml"
                xmlns:bldg="http://www.opengis.net/citygml/building/2.0">"#,
    );
    for (i, (lod, year)) in buildings.iter().enumerate() {
        out.push_str(&format!(
            r#"
  <core:cityObjectMember>
    <bldg:Building gml:id="b-{i}">
      <bldg:yearOfConstruction>{year}</bldg:yearOfConstruction>
      <bldg:lod{lod}MultiSurface>
        <gml:MultiSurface>
          <gml:surfaceMember>
            <gml:Polygon>
              <gml:exterior>
                <gml:LinearRing>
                  <gml:posList>0 0 0 1 0 0 1 1 0 0 0 0</gml:posList>
                </gml:LinearRing>
              </gml:exterior>
            </gml:Polygon>
          </gml:surfaceMember>
        </gml:MultiSurface>
      </bldg:lod{lod}MultiSurface>
    </bldg:Building>
  </core:cityObjectMember>"#
        ));
    }
    out.push_str("\n</core:CityModel>\n");
    out
}

proptest! {
    #[test]
    fn building_structure_survives_parsing(
        buildings in prop::collection::vec((0u32..=4, 1800u16..2100), 1..6),
    ) {
        let doc = building_doc(&buildings);
        let model = parse_str(&doc).unwrap();

        prop_assert_eq!(model.objects.len(), buildings.len());
        for (obj, (lod, year)) in model.objects.iter().zip(&buildings) {
            prop_assert_eq!(obj.kind, CityObjectKind::Building);
            prop_assert_eq!(obj.geometries.len(), 1);
            prop_assert_eq!(obj.geometries[0].lod, Lod::from_u32(*lod).unwrap());
            let year_attr = obj.attributes.iter().find(|a| a.key == ThematicKey::YearBuilt);
            prop_assert_eq!(year_attr.map(|a| a.value.clone()), Some(year.to_string()));
        }
    }

    #[test]
    fn no_panic_on_arbitrary_ascii(input in "[ -~\\n\\t]{0,400}") {
        let _ = parse_str(&input);
    }

    #[test]
    fn no_panic_on_mutated_document(
        buildings in prop::collection::vec((0u32..=4, 1800u16..2100), 1..3),
        cut in 0usize..2000,
    ) {
        // Truncating a valid document at any byte must yield Ok or Err, never a panic.
        let doc = building_doc(&buildings);
        let end = cut.min(doc.len());
        if doc.is_char_boundary(end) {
            let _ = parse_str(&doc[..end]);
        }
    }
}
