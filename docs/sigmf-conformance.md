# SigMF conformance fixtures and discrepancies

The pinned upstream schemas are stored unchanged under
[`crates/sigmf/tests/fixtures/upstream/sigmf-1.2.6`](../crates/sigmf/tests/fixtures/upstream/sigmf-1.2.6/README.md).
They are licensed CC BY-SA 4.0; retain the adjacent attribution and license
when redistributing those artifacts.

The tests compile and run representative documents against those schemas. They
also deliberately capture differences between schema validation and normative
SigMF 1.2.6 semantics. Passing an upstream schema is not a full conformance
claim; the library's semantic validation remains separate.

## Versioned discrepancy register

| ID | Version | Upstream artifact | Discrepancy | Library treatment |
| --- | --- | --- | --- | --- |
| SIGMF-1.2.6-001 | 1.2.6 | `collection-schema.json` | The schema accepts tuple-form `core:streams` but rejects object-form streams, although the specification prose recommends objects and permits both forms. | Parsing preserves tuple and object forms; the conformance test records the schema mismatch. |
| SIGMF-1.2.6-002 | 1.2.6 | `collection-schema.json`, `sigmf-schema.json` | The `core:extensions.optional` description says `true` means required, contrary to the normative table where `false` means required. | The description is retained in the upstream fixture; library extension validation follows the normative meaning. |
| SIGMF-1.2.6-003 | 1.2.6 | `sigmf-schema.json` | The datatype pattern is not end-anchored and can accept a valid datatype followed by extra text. | Library datatype parsing validates the entire spelling; the conformance test captures the schema behavior. |
| SIGMF-1.2.6-004 | 1.2.6 | Specification text and examples | `core:offset` describes absolute indexing while some capture descriptions/examples use dataset-relative starts. | The library uses absolute sample indices and maps to file frames by subtracting `core:offset`; dedicated tests cover this interpretation. |

The upstream schema files and their license are pinned verbatim, not edited to
remove these differences. Changes to this register must identify the
specification version and cite the relevant fixture or semantic test.
