#!/usr/bin/env python3
"""Generate golden NetCDF-3 classic sample files (no netCDF4 dependency).

Emits two fixtures under tests/samples:
  * example.nc  - fixed dims (time, lat, lon) + a 3-D data variable
  * record.nc   - an unlimited (record) time dimension + a record variable

The byte layout is built by hand to match the classic NetCDF-3 format that
tpt-io-netcdf parses, so the Rust test acts as an independent validator.
"""

import struct
import os

SIG = b"CDF"
VERSION = 1


def pad4(b: bytes) -> bytes:
    n = len(b)
    p = (n + 3) & ~3
    return b + b"\x00" * (p - n)


def name(s: str) -> bytes:
    bs = s.encode()
    return struct.pack(">I", len(bs)) + pad4(bs)


def attr(nm: str, nc_type: int, value: bytes) -> bytes:
    size = {1: 1, 2: 1, 3: 2, 4: 4, 5: 4, 6: 8}[nc_type]
    nelems = len(value) // size
    return name(nm) + struct.pack(">i", nc_type) + struct.pack(">I", nelems) + pad4(value)


def f32(x: float) -> bytes:
    return struct.pack(">f", x)


def build(dims, global_attrs, vars_spec, numrecs=0, unlimited_idx=None):
    """vars_spec: list of (name, nc_type, dim_ids, attrs, data_bytes)."""
    # Classic NetCDF layout:
    #   * non-record variables are stored contiguously in the data section.
    #   * record (unlimited-dimension) variables are stored in an interleaved
    #     record section: for each record, every record variable's slice.
    # Two-pass: write variable headers with placeholder `begin` fields, then
    # patch them once the true data offsets are known.
    out = bytearray()
    out += SIG + bytes([VERSION]) + struct.pack(">I", numrecs)

    # dimensions
    out += struct.pack(">i", 10) + struct.pack(">I", len(dims))
    for i, (dname, dlen) in enumerate(dims):
        if unlimited_idx is not None and i == unlimited_idx:
            out += name(dname) + struct.pack(">I", 0xFFFFFFFF)
        else:
            out += name(dname) + struct.pack(">I", dlen)

    # global attributes
    out += struct.pack(">i", 12) + struct.pack(">I", len(global_attrs))
    for (an, at, av) in global_attrs:
        out += attr(an, at, av)

    # variables (with placeholder begins). Record membership: first dim is the
    # unlimited one.
    out += struct.pack(">i", 11) + struct.pack(">I", len(vars_spec))
    begin_positions = []
    var_is_record = []
    for (vname, vtype, dim_ids, vattrs, vdata) in vars_spec:
        rec = unlimited_idx is not None and len(dim_ids) > 0 and dim_ids[0] == unlimited_idx
        var_is_record.append(rec)
        out += name(vname) + struct.pack(">i", vtype)
        out += struct.pack(">I", len(dim_ids))
        for d in dim_ids:
            out += struct.pack(">I", d)
        out += struct.pack(">I", len(vattrs))
        for (an, at, av) in vattrs:
            out += attr(an, at, av)
        out += struct.pack(">I", len(vdata))  # vlen
        begin_positions.append(len(out))
        out += struct.pack(">I", 0)  # placeholder begin

    data_start = len(out)
    per = max(numrecs, 1)

    # Non-record variables: contiguous from data_start.
    cursor = data_start
    nonrec_begins = {}
    for idx, (rec, (_, _, _, _, vdata)) in enumerate(zip(var_is_record, vars_spec)):
        if not rec:
            nonrec_begins[idx] = cursor
            cursor += len(vdata)
    rec_section_start = cursor

    # Record variables: within each record block, slices are sequential.
    rc = rec_section_start
    rec_begins = {}
    for idx, (rec, (_, _, _, _, vdata)) in enumerate(zip(var_is_record, vars_spec)):
        if rec:
            rec_begins[idx] = rc
            rc += len(vdata) // per

    for idx in range(len(vars_spec)):
        b = nonrec_begins[idx] if idx in nonrec_begins else rec_begins[idx]
        out[begin_positions[idx]:begin_positions[idx] + 4] = struct.pack(">I", b)

    # Data section: non-record vars, then interleaved record slices.
    for idx, (rec, (_, _, _, _, vdata)) in enumerate(zip(var_is_record, vars_spec)):
        if not rec:
            out += vdata
    for r in range(numrecs):
        for idx, (rec, (_, _, _, _, vdata)) in enumerate(zip(var_is_record, vars_spec)):
            if rec:
                p = len(vdata) // per
                out += vdata[r * p:(r + 1) * p]
    return bytes(out)


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    samples = os.path.join(here, "samples")
    os.makedirs(samples, exist_ok=True)

    # ---- example.nc: time(2) x lat(3) x lon(4) ----
    time_vals = f32(1.0) + f32(2.0)
    lat_vals = f32(10.0) + f32(20.0) + f32(30.0)
    lon_vals = f32(100.0) + f32(110.0) + f32(120.0) + f32(130.0)
    temp_vals = b""
    for t in range(2):
        for la in range(3):
            for lo in range(4):
                temp_vals += f32(float(t * 100 + la * 10 + lo))

    example = build(
        dims=[("time", 2), ("lat", 3), ("lon", 4)],
        global_attrs=[("title", 2, b"test"), ("version", 4, struct.pack(">i", 1))],
        vars_spec=[
            ("time", 5, [0], [("units", 2, b"days")], time_vals),
            ("lat", 5, [1], [("units", 2, b"deg_north")], lat_vals),
            ("lon", 5, [2], [("units", 2, b"deg_east")], lon_vals),
            ("temp", 5, [0, 1, 2], [("units", 2, b"K")], temp_vals),
        ],
    )
    with open(os.path.join(samples, "example.nc"), "wb") as f:
        f.write(example)

    # ---- record.nc: unlimited time(2) x lat(3) ----
    rtime_vals = f32(1.5) + f32(2.5)
    rlat_vals = f32(5.0) + f32(15.0) + f32(25.0)
    # record variable temp[time, lat]: per record 3 floats, two records.
    rtemp = b""
    for rec in range(2):
        for la in range(3):
            rtemp += f32(float(rec * 10 + la))

    record = build(
        dims=[("time", 2), ("lat", 3)],
        global_attrs=[("source", 2, b"synthetic")],
        vars_spec=[
            ("time", 5, [0], [("units", 2, b"days")], rtime_vals),
            ("lat", 5, [1], [("units", 2, b"deg_north")], rlat_vals),
            ("temp", 5, [0, 1], [("units", 2, b"K")], rtemp),
        ],
        numrecs=2,
        unlimited_idx=0,
    )
    with open(os.path.join(samples, "record.nc"), "wb") as f:
        f.write(record)

    print("wrote", os.path.join(samples, "example.nc"), len(example), "bytes")
    print("wrote", os.path.join(samples, "record.nc"), len(record), "bytes")


if __name__ == "__main__":
    main()
