# Changelog

## Unreleased

- Reader and writer for 3MF packages: objects with triangle meshes, build items, and item transforms.
  Reads the `3D/3dmodel.model` part from a ZIP container (stored and deflate members), and writes packages
  with stored members and CRC-32 checksums.
