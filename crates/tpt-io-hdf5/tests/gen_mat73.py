#!/usr/bin/env python3
"""Generate a MATLAB v7.3-shaped sample for the MAT reader tests.

Writes tests/samples/mat73_sample.mat: a 512-byte user block with the MATLAB header text, then an HDF5
container whose datasets carry MATLAB_class attributes, the way MATLAB writes them. MATLAB stores arrays
column-major, so each dataset's HDF5 shape is the reverse of the MATLAB dimensions.

Requires h5py and numpy.
"""

import os

import h5py
import numpy as np


HEADER = b"MATLAB 7.3 MAT-file, Platform: PCWIN64, Created on: Sat Jan  3 12:00:00 2026".ljust(116)
HEADER += b"\x00" * 8 + b"\x00\x02" + b"IM"


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    path = os.path.join(here, "samples", "mat73_sample.mat")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with h5py.File(path, "w", userblock_size=512) as f:
        # MATLAB double 2x3 (column-major), stored as HDF5 shape (3, 2).
        a = f.create_dataset("a", data=np.array([[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]]))
        a.attrs["MATLAB_class"] = np.bytes_(b"double")
        # MATLAB char 'hi', stored as uint16 code units.
        c = f.create_dataset("name", data=np.array([[104, 105]], dtype=np.uint16))
        c.attrs["MATLAB_class"] = np.bytes_(b"char")
        # MATLAB double 4x1, stored as HDF5 shape (1, 4) and deflate-compressed.
        z = f.create_dataset(
            "z", data=np.array([[0.5, 1.5, 2.5, 3.5]]), chunks=(1, 2), compression="gzip"
        )
        z.attrs["MATLAB_class"] = np.bytes_(b"double")
        # A struct group: not a dataset, so the reader ignores it.
        f.create_group("s")
    with open(path, "r+b") as fh:
        fh.write(HEADER.ljust(512, b"\x00"))
    print("wrote", path)


if __name__ == "__main__":
    main()
