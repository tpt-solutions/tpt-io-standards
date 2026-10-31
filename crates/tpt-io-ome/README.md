# tpt-io-ome

Readers for OME microscopy metadata: the OME-XML stored in OME-TIFF files, and the JSON metadata of
OME-Zarr stores.

## Spec access and licence

OME-XML and OME-Zarr are open specifications from the Open Microscopy Environment. No specification text,
image or dataset is copied into this crate; the tests use small documents written for them.

## What it reads

- **OME-XML** (`parse_xml`): the first `Pixels` element of the document: its ID, `DimensionOrder`,
  `Type`, and `SizeX`, `SizeY`, `SizeZ`, `SizeC`, `SizeT`.
- **OME-TIFF** (`from_tiff`): the `ImageDescription` of the first IFD of a TIFF file, read as OME-XML.
  The file is parsed with `tpt-io-tiff`.
- **OME-Zarr** metadata: `parse_zattrs` returns the first multiscale's name, axes (name and type) and
  dataset paths; `parse_zarray` returns a Zarr format 2 array's shape, chunks, dtype, order, and whether
  it is uncompressed.

## What it does not do

- Only the first image and its first `Pixels` are read.
- Pixel data is not decoded here. For OME-TIFF use `tpt-io-tiff`.
- Zarr chunks are not read, and Zarr format 3 (`zarr.json`) is rejected.
- Channels, planes, and per-plane metadata (`TiffData`, `Plane`) are not read.
