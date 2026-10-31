# Changelog

## Unreleased

- Initial reader for URDF and the first `<model>` of SDFormat, into a common
  `Robot` model with links, joints, poses, axes and limits.
- URDF writer that re-reads to an equal `Robot`.
- `Robot::validate` for unique names, known joint endpoints, single parents and
  acyclic joint graphs.
