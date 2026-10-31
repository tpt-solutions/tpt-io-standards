# tpt-io-asterix

A reader for ASTERIX (All-purpose STructured EUROCONTROL SURveillance Information EXchange)
binary surveillance data, with a decoder for the CAT048 monoradar target report subset.

## Spec access and licence

ASTERIX categories and their item descriptions are published by EUROCONTROL. No specification
text is copied into this crate. The CAT048 fields listed below are implemented from the item
layouts; test blocks are built in code.

## What it reads

- **Data blocks**: category octet, two-octet big-endian length, and the records inside.
- **FSPEC**: any number of octets, each flagging seven items; the low bit (FX) chains a
  further octet. Item order is the category's UAP, so each flagged bit names an item (FRN).
- **CAT048 items** with a fixed length, decoded:

| FRN | Item | Bytes | Decoded as |
| --- | --- | --- | --- |
| 1 | I048/010 Data Source | 2 | system area and identification codes |
| 2 | I048/140 Time of Day | 3 | 1/128 s units (`time_of_day_s`) |
| 4 | I048/040 Measured Position | 4 | range (NM) and azimuth (degrees) |
| 5 | I048/070 Mode-3/A | 2 | 12-bit code |
| 6 | I048/090 Flight Level | 2 | 1/4 FL units |
| 8 | I048/220 Aircraft Address | 3 | 24-bit address |
| 11 | I048/161 Track Number | 2 | 12-bit number |

## What it does not do

- Items outside the table above (including variable-length items such as the target
  report descriptor) are reported as `AsterixError::UnsupportedItem`. Their lengths are not
  guessed, because a wrong length would corrupt every following field.
- Other categories are framed but their records are not decoded, so they report
  `UnsupportedItem` for their first item.
- Track and quality fields beyond those listed are not decoded.
