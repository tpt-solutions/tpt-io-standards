# tpt-io-tiff

A reader and writer for TIFF images: the image format used in scanners, microscopy, remote sensing and
document imaging, and the container for OME-TIFF.

## Spec access and licence

TIFF 6.0 is published by Adobe and is openly documented. No specification text or sample image is copied
into this crate; test images are built in code.

## What it reads

- The byte-order mark (`II` little-endian, `MM` big-endian) and the magic number 42.
- Every IFD in the chain, with its tagged entries (SHORT and LONG values, inline or at an offset).
- Image size, bits per sample (8, 16 or 32), samples per pixel, sample format, and the
  `ImageDescription`.
- Strips, uncompressed (`Compression` 1) or LZW (5). Pixels are returned little-endian, with big-endian
  files converted.

## What it writes

Little-endian files, one IFD per image, one uncompressed strip per image, `BlackIsZero` photometric
interpretation. Parsing the output gives back the same images.

## What it does not do

- Tiled images, PackBits, Deflate, JPEG, and other compressions are reported as `TiffError::Unsupported`.
- BigTIFF (magic 43) is reported as unsupported.
- Photometric interpretation, colour maps, resolution tags and other metadata beyond the fields above are not
  decoded.
- Bit depths other than 8, 16 and 32 are not decoded.
