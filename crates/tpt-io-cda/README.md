# tpt-io-cda

A read-only reader for HL7 Clinical Document Architecture documents (CDA R2 and C-CDA), the XML
format used for clinical documents such as discharge summaries and continuity-of-care records.

## Spec access and licence

CDA is an HL7 standard, and C-CDA implementation guides are published by HL7 under their licence
terms. No standard text, template or sample document is copied into this crate; the test document is
written for these tests.

## What it reads

- The document identity: `id/@root` and `id/@extension`, joined as `root:extension`.
- The `title` and the `effectiveTime/@value`, as written.
- The patient's name: `recordTarget/patientRole/patient/name` given and family parts.
- Every `section` in the body, with its `title` and `code/@code`.

Missing parts are reported as `None`, not as errors.

## What it does not do

- Read only.
- Section narrative (`text`), coded entries, observations, and medication details are not decoded.
- Template ids, author, custodian, and encounter information are not read.
- Document signatures and XML digital signatures are not verified.
