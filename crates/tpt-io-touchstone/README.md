# tpt-io-touchstone

A reader and writer for Touchstone network-parameter files (`.s1p`, `.s2p`, `.sNp`), the RF
and microwave format for S-, Y-, Z-, H- and G-parameters.

## Spec access and licence

The Touchstone format is maintained by the IBIS Open Forum and is published openly. No text is
copied into this crate; test files are written for these tests.

## What it reads

- The option line: frequency unit (`Hz`, `kHz`, `MHz`, `GHz`), parameter type (`S`, `Y`, `Z`,
  `H`, `G`), format (`MA`, `DB`, `RI`) and reference impedance (`R`). Defaults are `GHz S MA R 50`.
- Comments after `!`, and blank lines.
- Frequency points where each point is a frequency followed by N² complex values. Values may
  wrap across lines.
- Every format is converted to real and imaginary parts. `DB` uses `10^(dB/20)` for magnitude.

The port count is not in the file, so `parse(bytes, ports)` takes it (from the `.sNp`
extension). Matrices are indexed `matrix[row][col]`, so for two-port files `matrix[1][0]` is S21.

## What it writes

An RI option line with `Hz` frequencies, one line per point. Frequencies and values are written
with Rust's shortest round-trip formatting, so parsing the output gives back the same file.

## What it does not do

- Two-port noise data blocks (the `N` section of two-port files) are not read.
- Mixed-mode and Touchstone 2.0 keywords are not read.
- The reference impedance is kept but not applied; values are as written.
- Without `std`, `sin` and `cos` come from a Taylor series on a reduced argument; the crate
  uses the standard library functions when `std` is enabled.
