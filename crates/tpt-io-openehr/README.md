# tpt-io-openehr

A read-only reader for openEHR compositions in the Reference Model JSON form, the clinical document
unit of the openEHR electronic health record standard.

## Spec access and licence

openEHR is published by the openEHR Foundation under its open licence. No specification text,
archetype or template is copied into this crate; the test composition is written for these tests.

## What it reads

- The composition `name.value`.
- `archetype_details`: `archetype_id.value`, `template_id.value` and `rm_version`.
- `language.code_string`.
- The top-level `content` entries: `_type`, `name.value` and `archetype_node_id`.

## What it does not do

- Read only.
- ADL archetype and template files are not parsed.
- Entries below the top level (sections, cluster and element trees) are not walked.
- Data values, the RM type constraints, and the openEHR XML form are not read.
