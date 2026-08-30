//! CityGML 2.0 / 3.0 XML parser for urban and regional 3D models.
//!
//! The crate parses CityGML documents into a structured [`CityModel`] that
//! exposes buildings, vegetation, water bodies, transportation networks, and
//! other city objects with their LOD 0–4 geometry, semantic surface
//! classifications, and thematic attributes. CityJSON 1.1 serialization is
//! available behind the [`cityjson`] feature.
//!
//! ```
//! use tpt_io_core::Reader;
//! use tpt_io_citygml::parser::CityGmlReader;
//! use tpt_io_citygml::model::CityModel;
//!
//! let src = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
//! <core:CityModel xmlns:core=\"http://www.opengis.net/citygml/2.0\"
//!                 xmlns:gml=\"http://www.opengis.net/gml\">
//!   <core:cityObjectMember>
//!     <bldg:Building xmlns:bldg=\"http://www.opengis.net/citygml/building/2.0\"
//!                    gml:id=\"b-1\">
//!       <bldg:lod1Solid>
//!         <gml:Solid>
//!           <gml:exterior>
//!             <gml:CompositeSurface>
//!               <gml:surfaceMember>
//!                 <gml:Polygon>
//!                   <gml:exterior>
//!                     <gml:LinearRing>
//!                       <gml:posList>0 0 0 1 0 0 1 1 0 0 1 0 0 0 0</gml:posList>
//!                     </gml:LinearRing>
//!                   </gml:exterior>
//!                 </gml:Polygon>
//!               </gml:surfaceMember>
//!             </gml:CompositeSurface>
//!           </gml:exterior>
//!         </gml:Solid>
//!       </bldg:lod1Solid>
//!     </bldg:Building>
//!   </core:cityObjectMember>
//! </core:CityModel>";
//!
//! let reader = CityGmlReader::new();
//! let model: CityModel = reader.parse(src).unwrap();
//! assert_eq!(model.objects.len(), 1);
//! ```
//!
//! CRS handling and transformation utilities are available via the
//! [`geo`] module (and the `geo-integration` feature for `tpt-io-geo`
//! interop). Downstream `eng-*` integrations are behind feature flags.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod error;
pub mod geo;
pub mod model;
pub mod parser;

#[cfg(feature = "cityjson")]
pub mod cityjson;

#[cfg(feature = "eng-geometry-integration")]
pub mod integration;

pub use error::CityGmlError;

#[cfg(feature = "cityjson")]
pub use cityjson::to_cityjson;

pub use model::{
    CityGmlVersion, CityModel, CityObject, CityObjectKind, Crs, Geometry, LinearRing, Lod, Point3,
    Solid, Surface, SurfaceClass, ThematicAttribute, ThematicKey,
};
pub use parser::{parse_str, CityGmlReader, CityGmlValidator};

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
