# tpt-io-urdf

A reader for URDF (`.urdf`) and SDFormat (`.sdf`) robot descriptions, a writer for
URDF, and kinematic-tree validation.

## Spec access and licence

- **URDF** is documented on the ROS wiki, which is BSD-licensed.
- **SDFormat** is published by Open Robotics under the Apache-2.0 licence.

No specification text, robot models or meshes from either source are copied into
this crate. Test documents are written for these tests.

## What it reads

Both dialects give a `Robot` with links and joints.

| Item | URDF | SDFormat |
| --- | --- | --- |
| Robot / model name | `<robot name>` | first `<model name>` in `<sdf>` |
| Link mass | `<inertial><mass value>` | `<inertial><mass>` text |
| Visual / collision | counted | counted |
| Joint parent / child | `<parent link>` / `<child link>` | `<parent>` / `<child>` text |
| Joint origin | `<origin xyz rpy>` | joint `<pose>` (6 numbers, or 3) |
| Axis | `<axis xyz>` | `<axis><xyz>` |
| Limits | `<limit lower upper effort velocity>` | `<axis><limit>` children |

Joint types `fixed`, `revolute`, `continuous`, `prismatic`, `floating` and `planar`
map to `JointKind`. Any other type string is kept as `JointKind::Other`.

## Validation

`Robot::validate` checks that link and joint names are unique, that every joint's
parent and child are links in the robot, that no link has two parents, and that the
joint graph has no cycles.

## What it does not do

- Mesh references, materials, sensors, plugins, transmissions and Gazebo extensions
  are not modelled and are dropped.
- SDFormat frames and `include` elements are not resolved; poses are taken as written.
- The writer produces URDF only. SDFormat output is not written.
