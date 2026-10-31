#!/usr/bin/env python3
"""Generate ASAM MDF 4.10 reference files with asammdf for the reader tests.

Writes into tests/samples/:
  * plain.mf4 - two channels (a float64 `speed` and a uint16 `count`), uncompressed
  * compressed.mf4 - the same data with deflate-compressed data blocks (DZ)
  * expected.txt - the values of each channel as written: one line per channel, `name value value ...`

Requires the `asammdf` and `numpy` Python packages.
"""

import os

import numpy as np
from asammdf import MDF, Signal


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    samples = os.path.join(here, "samples")
    os.makedirs(samples, exist_ok=True)
    t = np.arange(100) * 0.01
    speed = 50.0 + 10.0 * np.sin(t)
    count = (np.arange(100) * 3).astype(np.uint16)
    signals = [
        Signal(samples=speed, timestamps=t, name="speed", unit="km/h"),
        Signal(samples=count, timestamps=t, name="count"),
    ]
    mdf = MDF(version="4.10")
    mdf.append(signals, comment="tpt-io-mdf sample", common_timebase=True)
    mdf.save(os.path.join(samples, "plain.mf4"), overwrite=True, compression=0)
    mdf.save(os.path.join(samples, "compressed.mf4"), overwrite=True, compression=2)

    with open(os.path.join(samples, "expected.txt"), "w", encoding="utf-8") as f:
        f.write("speed " + " ".join(repr(float(v)) for v in speed) + "\n")
        f.write("count " + " ".join(str(int(v)) for v in count) + "\n")
    print("wrote samples to", samples)


if __name__ == "__main__":
    main()
