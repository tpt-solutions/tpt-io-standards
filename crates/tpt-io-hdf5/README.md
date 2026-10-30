# tpt-io-hdf5

A pure-Rust reader for HDF5 files, the container format behind NetCDF-4,
MATLAB v7.3 and many scientific datasets. It has no C dependencies.

## What it reads

- Superblock versions 0, 2 and 3.
- Version-2 object headers, including continuation blocks.
- Groups with version-1 link messages (hard links).
- Dataspaces (versions 1 and 2) and datatypes: fixed-point (integer),
  floating-point and fixed-length string.
- Attributes (message versions 1, 2 and 3).
- Dataset storage: compact, contiguous and chunked (version-1 chunk B-tree,
  any rank).
- Filter pipelines: deflate and shuffle, applied in order.

## What it does not read yet

These return `Hdf5Error::Unsupported` rather than wrong data:

- Variable-length data, including `str` attributes written by h5py. The value
  is the raw 16-byte global-heap reference.
- Object header version 1, layout messages versions 1, 2 and 4, dense link
  and attribute storage (fractal heaps), and version-2 chunk indexes.
- Soft and external links. They are skipped.
- Filters other than deflate and shuffle.

Data is returned as raw little-endian bytes. Chunked variables are assembled
in full in memory; there is no streaming API yet.

## Features

- `std` (default): enables zlib (deflate) decoding through `flate2`.
- `alloc`: `no_std` plus `alloc`. Compressed chunks return `Unsupported`.

## Example

```rust
let bytes = std::fs::read("temp.h5")?;
let file = tpt_io_hdf5::parse(&bytes)?;
for ds in &file.datasets {
    println!("{} {:?} {:?}", ds.name, ds.dims, ds.datatype);
    let raw = ds.read_data(&bytes)?.try_raw().unwrap_or_default();
    println!("{} bytes", raw.len());
}
```

## Testing

Golden files are written by `h5py`; `tests/gen_hdf5.py` and
`tests/gen_chunked4d.py` regenerate them. The 4-D fixture uses chunk sizes that
do not divide the shape, so partial edge chunks are covered.
