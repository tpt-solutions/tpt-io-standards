#!/usr/bin/env python3
"""Generate real HDF5 (NetCDF-4) sample files with h5py for golden testing.

Files written to tests/samples/:
  * example4.h5  - root attrs + 4 contiguous float datasets (time/lat/lon/temp)
  * chunked4.h5  - temp stored chunked, with shuffle + gzip (deflate) filters

The Rust reader should be able to (a) enumerate the datasets, their dimensions
and types, (b) capture chunking + filter metadata, and (c) read contiguous data.
"""

import os
import h5py
import numpy as np


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    samples = os.path.join(here, "samples")
    os.makedirs(samples, exist_ok=True)

    # ---- example4.h5: contiguous datasets, no filters ----
    with h5py.File(os.path.join(samples, "example4.h5"), "w", libver="v108") as f:
        f.attrs["title"] = "synthetic netcdf4"
        f.attrs["version"] = np.int32(1)
        time = f.create_dataset("time", data=np.array([1.0, 2.0], dtype="f4"))
        time.attrs["units"] = "days"
        lat = f.create_dataset("lat", data=np.array([10.0, 20.0, 30.0], dtype="f4"))
        lat.attrs["units"] = "deg_north"
        lon = f.create_dataset("lon", data=np.array([100.0, 110.0, 120.0, 130.0], dtype="f4"))
        lon.attrs["units"] = "deg_east"
        temp = np.zeros((2, 3, 4), dtype="f4")
        for t in range(2):
            for la in range(3):
                for lo in range(4):
                    temp[t, la, lo] = float(t * 100 + la * 10 + lo)
        tds = f.create_dataset("temp", data=temp)
        tds.attrs["units"] = "K"

    # ---- chunked4.h5: chunked + shuffle + gzip ----
    with h5py.File(os.path.join(samples, "chunked4.h5"), "w", libver="v108") as f:
        f.attrs["title"] = "chunked netcdf4"
        temp = np.zeros((4, 6), dtype="f4")
        for i in range(4):
            for j in range(6):
                temp[i, j] = float(i * 10 + j)
        # chunks + shuffle + gzip(deflate) exercises the filter pipeline + layout.
        tds = f.create_dataset(
            "temp",
            data=temp,
            chunks=(2, 3),
            shuffle=True,
            compression="gzip",
            compression_opts=4,
        )
        tds.attrs["units"] = "K"

    print("wrote example4.h5 and chunked4.h5")


if __name__ == "__main__":
    main()
