#!/usr/bin/env python3
"""Generate a small GeoPackage with Python's sqlite3 module, for the GeoPackage reader tests.

Writes tests/samples/cities.gpkg: a `cities` feature table with a point geometry, a text note long enough to
spill onto overflow pages, and enough rows to need interior B-tree pages. Prints the values the tests expect.
"""

import os
import sqlite3
import struct


def gp_point(lon, lat):
    """A GeoPackage geometry blob: the GP header (little-endian, no envelope) followed by little-endian WKB."""
    header = b"GP" + bytes([0, 1]) + struct.pack("<i", 4326)
    wkb = b"\x01" + struct.pack("<I", 1) + struct.pack("<dd", lon, lat)
    return header + wkb


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    path = os.path.join(here, "samples", "cities.gpkg")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if os.path.exists(path):
        os.remove(path)
    con = sqlite3.connect(path)
    cur = con.cursor()
    cur.executescript(
        """
        CREATE TABLE gpkg_spatial_ref_sys (
            srs_name TEXT NOT NULL, srs_id INTEGER PRIMARY KEY, organization TEXT NOT NULL,
            organization_coordsys_id INTEGER NOT NULL, definition TEXT NOT NULL, description TEXT);
        CREATE TABLE gpkg_contents (
            table_name TEXT NOT NULL PRIMARY KEY, data_type TEXT NOT NULL, identifier TEXT UNIQUE,
            description TEXT DEFAULT '', last_change DATETIME NOT NULL, min_x DOUBLE, min_y DOUBLE,
            max_x DOUBLE, max_y DOUBLE, srs_id INTEGER);
        CREATE TABLE gpkg_geometry_columns (
            table_name TEXT NOT NULL, column_name TEXT NOT NULL, geometry_type_name TEXT NOT NULL,
            srs_id INTEGER NOT NULL, z TINYINT NOT NULL, m TINYINT NOT NULL,
            PRIMARY KEY (table_name, column_name));
        CREATE TABLE cities (
            fid INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, geom POINT, name TEXT, note TEXT,
            population INTEGER);
        """
    )
    cur.execute("INSERT INTO gpkg_spatial_ref_sys VALUES ('WGS 84', 4326, 'EPSG', 4326, 'GEOGCS[]', '')")
    cur.execute(
        "INSERT INTO gpkg_contents VALUES ('cities', 'features', 'cities', '', '2026-01-01T00:00:00Z', 0, 0, 0, 0, 4326)"
    )
    cur.execute("INSERT INTO gpkg_geometry_columns VALUES ('cities', 'geom', 'POINT', 4326, 0, 0)")

    cur.execute(
        "INSERT INTO cities (geom, name, note, population) VALUES (?, ?, ?, ?)",
        (gp_point(174.78, -41.29), "Wellington", "capital", 215000),
    )
    long_note = "x" * 6000
    cur.execute(
        "INSERT INTO cities (geom, name, note, population) VALUES (?, ?, ?, ?)",
        (gp_point(174.0, -41.0), "Long", long_note, 1),
    )
    for i in range(3, 2003):
        cur.execute(
            "INSERT INTO cities (geom, name, note, population) VALUES (?, ?, ?, ?)",
            (gp_point(i / 100.0, -i / 100.0), f"town {i}", "n" * (i % 9), i * 3),
        )
    con.commit()
    count = cur.execute("SELECT COUNT(*) FROM cities").fetchone()[0]
    con.close()
    print("wrote", path)
    print("features:", count)
    print("long note length:", len(long_note))


if __name__ == "__main__":
    main()
