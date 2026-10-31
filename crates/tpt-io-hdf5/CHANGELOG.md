# Changelog

## Unreleased

- Superblock version 0 and version-1 object headers: old-style (symbol-table) groups, as written by MATLAB v7.3 and
  by HDF5 libraries with the earliest file format.
- MATLAB v7.3 reader (`matlab` module): variables with their `MATLAB_class` and column-major dimensions, and
  `as_f64` for `double` and `single` data.

- Initial crate, extracted from the NetCDF-4 reader in `tpt-io-netcdf`.
- Correct decoding of version-2 object headers, version-1 link messages,
  dataspace, version-3 layout messages, filter pipelines, datatype messages
  and attribute messages (versions 1–3).
- Chunked storage assembled for any rank, with partial edge chunks.
- Deflate decoding behind the `std` feature.
