# tpt-io-pointcloud

A reader and writer for LAS point clouds, the ASPRS format for LiDAR and photogrammetric surveys.

## Spec access and licence

LAS is published by ASPRS. No specification text or survey data is copied into this crate; test
clouds are built in code.

## What it reads

- The public header: signature, version, point data offset and format, record length, point count
  (`legacy` count for LAS 1.0 to 1.3, the 64-bit count for 1.4), scale and offset.
- Point records in formats 0 to 3:
  - position as `raw * scale + offset`;
  - intensity, return number and number of returns (from the flag byte), classification (bits 0 to 4),
    scan angle, user data and point source id;
  - GPS time for formats 1 and 3, and colour (`red, green, blue`) for formats 2 and 3.

## What it writes

LAS 1.2 with point format 0 or 1. Each coordinate is stored as `round((value − offset) / scale)` and
values that do not fit 32 bits after scaling are rejected. Parsing the output gives back the same
positions and fields.

## What it does not do

- LAZ (compressed) files, point formats 4 to 10, waveform packets, and extra bytes are not read.
- Variable-length and extended-variable-length records are skipped, not decoded.
- The writer does not produce formats 2 or 3 (colour), so colour data does not round-trip through it.
- Classification bits above the low five (the LAS 1.4 flag layout) are not interpreted.
