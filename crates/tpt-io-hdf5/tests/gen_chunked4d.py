#!/usr/bin/env python3
"""Generate tests/samples/chunked4d.h5 with h5py.

A 4-D float32 variable (time x depth x lat x lon) stored chunked with shuffle +
gzip. The chunk shape does not divide the variable shape evenly, so the grid has
partial edge chunks; the reader must place every chunk by its N-D origin. The
values are 0..N-1 in row-major order, so each element's expected value is its
flat index.
"""

import os

import h5py
import numpy as np


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(here, "samples", "chunked4d.h5")
    data = np.arange(2 * 3 * 4 * 5, dtype="f4").reshape(2, 3, 4, 5)
    with h5py.File(out, "w", libver="v108") as f:
        ds = f.create_dataset(
            "temp4d",
            data=data,
            chunks=(1, 2, 3, 2),
            shuffle=True,
            compression="gzip",
            compression_opts=4,
        )
        ds.attrs["units"] = "K"
    print("wrote", out)


if __name__ == "__main__":
    main()
