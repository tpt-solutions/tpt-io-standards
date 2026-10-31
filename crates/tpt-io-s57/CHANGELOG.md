# Changelog

## Unreleased

- Initial reader for IHO S-57 exchange files: the ISO/IEC 8211 framing (leaders, directories,
  field areas and the data descriptive record), the dataset parameters (`DSID`, `DSPM`), and
  feature objects with their attributes (`FRID`, `FOID`, `ATTF`) and resolved geometry
  (`FSPT`, `VRID`, `VRPT`, `SG2D`, `SG3D`).
- Read only. No writer, and no reader for update files that depend on a base file.
