# tpt-io-bcf

Reader and writer for BIM Collaboration Format (BCF 2.1) projects. BCF is the buildingSMART exchange format for
issues, clashes and comments raised against an IFC model.

## Spec access and licence

BCF is an open buildingSMART standard. No specification text is copied into this crate; test projects are built
in code.

## What it reads

- `bcf.version`, giving the version (`2.1` when the part is absent).
- Every `markup.bcfxml` part, one per topic: the topic GUID, `TopicType` and `TopicStatus` attributes, `Title`,
  `Priority`, `CreationDate`, `CreationAuthor`, comments (GUID, `Author`, `Date`, text) and `Viewpoints`
  references (GUID and `Viewpoint` file name).

## What it writes

- A project with `bcf.version` and one `<topic GUID>/markup.bcfxml` part per topic, in stored ZIP members.
- The fields listed above. Empty optional text is omitted and reads back as `None`.

## What it does not do

- Viewpoint files (`*.bcfv`), snapshots (`*.png`) and the `Header` / linked IFC file list are not parsed or written.
- Dates are kept as text; they are not validated.
- Only the markup parts are read; extension parts are ignored.
