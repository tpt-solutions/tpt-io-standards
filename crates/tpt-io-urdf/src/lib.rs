//! URDF and SDFormat robot descriptions: reader, writer and kinematic-tree validation.
//!
//! URDF (`<robot>`) and SDFormat (`<sdf><model>`) both describe a robot as links connected by
//! joints. This crate reads either dialect into a common [`Robot`] model with each link's mass
//! and visual/collision counts, and each joint's type, parent and child links, origin pose, axis
//! and limits. Writing produces URDF.
//!
//! ```
//! let urdf = br#"<robot name="arm">
//!   <link name="base"/>
//!   <link name="upper"><visual/></link>
//!   <joint name="shoulder" type="revolute">
//!     <parent link="base"/><child link="upper"/>
//!     <origin xyz="0 0 0.1" rpy="0 0 0"/><axis xyz="0 1 0"/>
//!     <limit lower="-1.5" upper="1.5" effort="10" velocity="2"/>
//!   </joint>
//! </robot>"#;
//! let robot = tpt_io_urdf::parse(urdf).unwrap();
//! assert_eq!(robot.joints[0].kind, tpt_io_urdf::JointKind::Revolute);
//! robot.validate().unwrap();
//! ```
//!
//! Mesh references, materials, sensors, plugins and transmissions are not modelled and are
//! dropped. Gazebo and other tool extensions are ignored.
//!
//! Spec access: the URDF XML format is documented on the ROS wiki (BSD licence) and the SDFormat
//! specification is published by Open Robotics under Apache-2.0. No specification text is copied.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod model;
pub mod reader;
pub mod writer;

pub use error::UrdfError;
pub use model::{Dialect, Joint, JointKind, Limit, Link, Pose, Robot};
pub use reader::parse;
pub use writer::write;
