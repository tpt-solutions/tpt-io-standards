#!/usr/bin/env python3
"""Generate GRIB2 reference samples with ecCodes for the reader tests.

Writes into tests/samples/:
  * simple_ll.grib2 - a 4 x 3 latitude/longitude field, simple packing, no bitmap
  * simple_ll.expected.txt - the values ecCodes decodes from it, one per line
  * bitmap_ll.grib2 - the same grid with two missing points (bitmap section present)
  * bitmap_ll.expected.txt - the decoded values, with "nan" for missing points

Requires the `eccodes` Python package.
"""

import os

import eccodes


def grid(gid, ni, nj):
    eccodes.codes_set(gid, "gridType", "regular_ll")
    eccodes.codes_set(gid, "Ni", ni)
    eccodes.codes_set(gid, "Nj", nj)
    eccodes.codes_set(gid, "latitudeOfFirstGridPointInDegrees", 60)
    eccodes.codes_set(gid, "longitudeOfFirstGridPointInDegrees", 0)
    eccodes.codes_set(gid, "latitudeOfLastGridPointInDegrees", 50)
    eccodes.codes_set(gid, "longitudeOfLastGridPointInDegrees", 6)
    eccodes.codes_set(gid, "iDirectionIncrementInDegrees", 2)
    eccodes.codes_set(gid, "jDirectionIncrementInDegrees", 5)
    eccodes.codes_set(gid, "packingType", "grid_simple")
    eccodes.codes_set(gid, "bitsPerValue", 16)


def write(path, gid, expected_path, missing=None):
    values = eccodes.codes_get_double_array(gid, "values")
    with open(path, "wb") as f:
        eccodes.codes_write(gid, f)
    with open(expected_path, "w", encoding="utf-8") as f:
        for v in values:
            is_missing = v != v or (missing is not None and v == missing)
            f.write("nan\n" if is_missing else f"{float(v)!r}\n")
    print("wrote", path, len(values), "values")


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    samples = os.path.join(here, "samples")
    os.makedirs(samples, exist_ok=True)
    values = [280.5 + 0.25 * k for k in range(12)]

    gid = eccodes.codes_grib_new_from_samples("regular_ll_sfc_grib2")
    grid(gid, 4, 3)
    eccodes.codes_set_values(gid, values)
    write(os.path.join(samples, "simple_ll.grib2"), gid, os.path.join(samples, "simple_ll.expected.txt"))
    eccodes.codes_release(gid)

    gid = eccodes.codes_grib_new_from_samples("regular_ll_sfc_grib2")
    grid(gid, 4, 3)
    eccodes.codes_set(gid, "bitmapPresent", 1)
    masked = list(values)
    # ecCodes marks missing points with its missingValue (9999 by default), not with NaN.
    masked[5] = 9999.0
    masked[9] = 9999.0
    eccodes.codes_set_values(gid, masked)
    write(
        os.path.join(samples, "bitmap_ll.grib2"),
        gid,
        os.path.join(samples, "bitmap_ll.expected.txt"),
        missing=9999.0,
    )
    eccodes.codes_release(gid)


if __name__ == "__main__":
    main()
