# tpt-io-gcode

A reader, writer and motion summary for G-code programs used by CNC mills, lathes and
3D printers.

## Spec access and licence

G-code is the RS-274 word language, extended differently by each controller vendor.
The LinuxCNC and RepRap documentation is open source. No vendor manual text is copied
into this crate; the word syntax is implemented from the common conventions.

## What it reads

- Blocks, one per line. Blank lines and `%` program delimiters are skipped.
- Line numbers (`N10`).
- Words: an address letter and a number, with the source text kept for round trips.
- Comments in both styles: `; …` (RepRap) and `( … )` (LinuxCNC and Fanuc-style).
- A dialect guess: `LinuxCnc` for parenthesised comments or `O` program numbers,
  `Reprap` for semicolon comments only, otherwise `Generic`.

## Motion summary

`Program::motion_summary` tracks the modal words:

| Word | Effect |
| --- | --- |
| `G90` / `G91` | absolute or incremental positioning |
| `G20` / `G21` | inches or millimetres (lengths converted to mm) |
| `G0`, `G1`, `G2`, `G3` | rapid, linear or arc motion mode |

It reports the counts of rapid, linear and arc moves, the path length of rapid and linear
moves (XYZ, millimetres), and the X/Y bounds reached. Arcs are counted but their length is
not computed.

## What it does not do

- It does not evaluate expressions, subroutines (`M98`), macros, or tool offsets.
- Arc centre and radius words (`I`, `J`, `K`, `R`) are kept as words but not used.
- Only the words listed above affect the summary; cutter compensation (`G41`/`G42`) is
  ignored.
