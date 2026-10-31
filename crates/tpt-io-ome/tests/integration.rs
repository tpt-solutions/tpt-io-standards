//! Integration tests for the OME-XML and OME-Zarr readers.

use tpt_io_ome::{parse_xml, parse_zarray, parse_zattrs, OmeError};

const OME: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<OME xmlns="http://www.openmicroscopy.org/Schemas/OME/2016-06">
  <Image ID="Image:0" Name="cells">
    <Pixels ID="Pixels:0" DimensionOrder="XYZCT" Type="uint16"
            SizeX="512" SizeY="256" SizeZ="5" SizeC="2" SizeT="3"/>
  </Image>
</OME>"#;

#[test]
fn reads_pixels_from_ome_xml() {
    let p = parse_xml(OME).unwrap();
    assert_eq!(p.id, "Pixels:0");
    assert_eq!(p.dimension_order, "XYZCT");
    assert_eq!(p.pixel_type, "uint16");
    assert_eq!((p.size_x, p.size_y, p.size_z, p.size_c, p.size_t), (512, 256, 5, 2, 3));
}

#[test]
fn ome_xml_errors_are_reported() {
    assert!(matches!(parse_xml(b"<Other/>"), Err(OmeError::Invalid(_))));
    assert!(matches!(parse_xml(b"<OME/>"), Err(OmeError::Invalid(_))));
    let no_size = br#"<OME><Pixels ID="p" SizeX="1" SizeY="1" SizeZ="1" SizeC="1"/></OME>"#;
    assert!(matches!(parse_xml(no_size), Err(OmeError::Invalid(_))));
    assert!(parse_xml(
        br#"<OME><Pixels SizeX="x" SizeY="1" SizeZ="1" SizeC="1" SizeT="1"/></OME>"#
    )
    .is_err());
}

#[test]
fn ome_tiff_descriptions_are_read_back() {
    let description = String::from_utf8(OME.to_vec()).unwrap();
    let image = tpt_io_tiff::Image {
        width: 2,
        height: 2,
        bits_per_sample: 8,
        samples_per_pixel: 1,
        sample_format: 1,
        description: Some(description),
        pixels: vec![1, 2, 3, 4],
    };
    let bytes = tpt_io_tiff::write(&[image]).unwrap();
    let p = tpt_io_ome::from_tiff(&bytes).unwrap();
    assert_eq!(p.size_x, 512);
    let plain = tpt_io_tiff::write(&[tpt_io_tiff::Image {
        width: 1,
        height: 1,
        bits_per_sample: 8,
        samples_per_pixel: 1,
        sample_format: 1,
        description: None,
        pixels: vec![9],
    }])
    .unwrap();
    assert!(matches!(tpt_io_ome::from_tiff(&plain), Err(OmeError::Invalid(_))));
}

const ZATTRS: &[u8] = br#"{
  "multiscales": [{
    "name": "plate",
    "version": "0.4",
    "axes": [
      {"name": "c", "type": "channel"},
      {"name": "y", "type": "space", "unit": "micrometer"},
      {"name": "x", "type": "space", "unit": "micrometer"}
    ],
    "datasets": [
      {"path": "0", "coordinateTransformations": [{"type": "scale", "scale": [1, 0.5, 0.5]}]},
      {"path": "1"}
    ]
  }]
}"#;

#[test]
fn reads_multiscale_axes_and_datasets() {
    let m = parse_zattrs(ZATTRS).unwrap();
    assert_eq!(m.name.as_deref(), Some("plate"));
    assert_eq!(m.axes[1], ("y".to_string(), "space".to_string()));
    assert_eq!(m.datasets, ["0", "1"]);
    assert!(parse_zattrs(b"{}").is_err());
}

const ZARRAY: &[u8] = br#"{
  "chunks": [1, 256, 256],
  "compressor": null,
  "dtype": "<u2",
  "fill_value": 0,
  "filters": null,
  "order": "C",
  "shape": [2, 512, 512],
  "zarr_format": 2
}"#;

#[test]
fn reads_zarray_shape_chunks_and_dtype() {
    let a = parse_zarray(ZARRAY).unwrap();
    assert_eq!(a.shape, [2, 512, 512]);
    assert_eq!(a.chunks, [1, 256, 256]);
    assert_eq!(a.dtype, "<u2");
    assert!(a.uncompressed);
    assert_eq!(a.order, "C");
}

#[test]
fn zarr_version_three_and_bad_sizes_are_rejected() {
    assert!(parse_zarray(br#"{"zarr_format": 3, "shape": [1], "chunks": [1]}"#).is_err());
    assert!(parse_zarray(br#"{"zarr_format": 2, "shape": [-1], "chunks": [1]}"#).is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 167;
    for _ in 0..200 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"<OME>Pixels {}[]:\",0123456789 ";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse_xml(&bytes);
        let _ = parse_zattrs(&bytes);
        let _ = parse_zarray(&bytes);
    }
}
