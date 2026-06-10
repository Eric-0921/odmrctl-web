# OE1022D Ambiguities And Validation

## Locked first-version decisions

- original PDF outranks cleaned markdown
- `RALL?` is the canonical acquisition command
- the first implementation must preserve explicit offset-based parsing

## Known ambiguity buckets

### `oe1022d_clean_operation.md` is not pristine source

The cleaned markdown is useful for grep and long-form reading, but it already mixes in secondary interpretation and revision drift.

Decision:

- do not treat it as the top truth source

### `RALL` frame interpretation

The manual documents:

- frame length
- chunk order
- byte ranges
- update cadence

But it does not fully settle every semantic detail needed for a production parser.

Decision:

- split the future parser model into:
  - confirmed fields
  - inferred fields
  - reserved / trailing holes

### `RSLPD` and external-reference trigger semantics

Supplemental markdown and later observations disagree about available trigger options.

Decision:

- `RSLPD` stays out of first-version command truth because the minimal rebuild can avoid this unresolved branch

### Unit and enum drift in secondary docs

Secondary markdown contains places where units or value interpretations drift.

Decision:

- trust original PDF chapter 4 and chapter 5 first

## Validation still worth doing later

- one clean parser-offset audit against real `RALL` frames
- one command transcript proving chosen fixed config values
- one explicit record of which sensitivity and time-constant indices are used in the first runtime profile

