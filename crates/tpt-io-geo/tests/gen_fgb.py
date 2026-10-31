#!/usr/bin/env python3
"""Generate FlatGeobuf reference files with GDAL (through pyogrio) for the FlatGeobuf reader tests.

Writes into tests/samples/:
  * fgb_points.fgb - three points with a string, an integer and a double attribute, no spatial index
  * fgb_mixed.fgb - a line, a polygon with a hole, and a multipolygon, with a spatial index

Requires the `pyogrio` and `numpy` Python packages.
"""

import os
import struct

import numpy as np
import pyogrio.raw


def wkb_point(x, y):
    return b"\x01" + struct.pack("<I", 1) + struct.pack("<dd", x, y)


def wkb_linestring(coords):
    body = b"".join(struct.pack("<dd", x, y) for x, y in coords)
    return b"\x01" + struct.pack("<I", 2) + struct.pack("<I", len(coords)) + body


def wkb_ring(coords):
    return struct.pack("<I", len(coords)) + b"".join(struct.pack("<dd", x, y) for x, y in coords)


def wkb_polygon(rings):
    return b"\x01" + struct.pack("<I", 3) + struct.pack("<I", len(rings)) + b"".join(wkb_ring(r) for r in rings)


def wkb_multipolygon(polygons):
    return b"\x01" + struct.pack("<I", 6) + struct.pack("<I", len(polygons)) + b"".join(polygons)


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    samples = os.path.join(here, "samples")
    os.makedirs(samples, exist_ok=True)

    points = [wkb_point(174.78, -41.29), wkb_point(0.5, 1.5), wkb_point(-3.25, 7.0)]
    pts_path = os.path.join(samples, "fgb_points.fgb")
    pyogrio.raw.write(
        pts_path,
        np.array(points, dtype=object),
        [np.array(["Wellington", "Alpha", "Beta"], dtype=object),
         np.array([215000, 7, -2], dtype=np.int32),
         np.array([1.5, 2.25, -0.125], dtype=np.float64)],
        ["name", "count", "ratio"],
        driver="FlatGeobuf",
        geometry_type="Point",
        crs="EPSG:4326",
        layer_options={"SPATIAL_INDEX": "NO"},
    )

    line = wkb_linestring([(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)])
    hole = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0), (0.0, 0.0)]
    inner = [(2.0, 2.0), (4.0, 2.0), (4.0, 4.0), (2.0, 4.0), (2.0, 2.0)]
    polygon = wkb_polygon([hole, inner])
    multi = wkb_multipolygon([wkb_polygon([[(20.0, 20.0), (21.0, 20.0), (21.0, 21.0), (20.0, 20.0)]]),
                              wkb_polygon([[(30.0, 30.0), (31.0, 30.0), (31.0, 31.0), (30.0, 30.0)]])])
    mixed_path = os.path.join(samples, "fgb_mixed.fgb")
    pyogrio.raw.write(
        mixed_path,
        np.array([line, polygon, multi], dtype=object),
        [np.array(["road", "park", "islands"], dtype=object)],
        ["label"],
        driver="FlatGeobuf",
        geometry_type="Unknown",
        crs="EPSG:4326",
        layer_options={"SPATIAL_INDEX": "YES"},
    )

    print("wrote", pts_path, mixed_path)


if __name__ == "__main__":
    main()
