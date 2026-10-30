# Changelog

## Unreleased

- Initial crate, extracted from the NetCDF-4 reader in `tpt-io-netcdf`.
- Correct decoding of version-2 object headers, version-1 link messages,
  dataspace, version-3 layout messages, filter pipelines, datatype messages
  and attribute messages (versions 1–3).
- Chunked storage assembled for any rank, with partial edge chunks.
- Deflate decoding behind the `std` feature.
