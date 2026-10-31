# tpt-io-omex

Reader and writer for COMBINE archives (OMEX), the ZIP container that bundles a systems-biology model with its
simulation experiment description, plus a summary reader for SED-ML.

## Spec access and licence

The COMBINE archive and SED-ML specifications are open standards. No specification text is copied into this crate;
test archives are built in code.

## What it reads

- `manifest.xml`: each `content` entry's `location`, `format` URI and `master` flag. Every listed file must be
  present in the archive.
- The content files themselves, as bytes.
- SED-ML documents (`sedml::parse_sedml`): `listOfModels`, `listOfSimulations` (element kind and the algorithm's
  `kisaoID`), `listOfTasks` and `listOfOutputs`.

## What it writes

- A `manifest.xml` listing every content file, and the files, in stored ZIP members.

## What it does not do

- `metadata.rdf` is not interpreted; it is only read as an ordinary content file.
- SED-ML is summarised, not executed, and its numeric settings (time courses, changes, data generators) are not read.
- Archives are written without compression.
