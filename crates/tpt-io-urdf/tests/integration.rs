//! Integration tests for the URDF and SDFormat readers and the URDF writer.

use tpt_io_urdf::{parse, write, Dialect, JointKind, UrdfError};

const ARM: &[u8] = br#"<?xml version="1.0"?>
<robot name="two_link_arm">
  <link name="base"/>
  <link name="upper">
    <inertial><mass value="2.5"/></inertial>
    <visual/>
    <collision/>
  </link>
  <link name="fore">
    <inertial><mass value="1.0"/></inertial>
  </link>
  <joint name="shoulder" type="revolute">
    <parent link="base"/>
    <child link="upper"/>
    <origin xyz="0 0 0.1" rpy="0 0 1.57"/>
    <axis xyz="0 1 0"/>
    <limit lower="-1.5" upper="1.5" effort="10" velocity="2"/>
  </joint>
  <joint name="elbow" type="continuous">
    <parent link="upper"/>
    <child link="fore"/>
    <origin xyz="0.3 0 0"/>
  </joint>
  <link name="sensor"/>
  <joint name="mount" type="fixed">
    <parent link="base"/>
    <child link="sensor"/>
  </joint>
</robot>"#;

const SDF: &[u8] = br#"<sdf version="1.9">
  <model name="cart">
    <link name="chassis">
      <inertial><mass>12.0</mass></inertial>
      <visual name="body"/>
      <collision name="body"/>
    </link>
    <link name="wheel">
      <pose>0 0.2 0 1.5708 0 0</pose>
    </link>
    <joint name="wheel_joint" type="revolute">
      <parent>chassis</parent>
      <child>wheel</child>
      <pose>0.1 0.2 0 0 0 0</pose>
      <axis>
        <xyz>0 0 1</xyz>
        <limit><lower>-1e16</lower><upper>1e16</upper><effort>5</effort><velocity>3</velocity></limit>
      </axis>
    </joint>
  </model>
</sdf>"#;

#[test]
fn reads_a_urdf_arm() {
    let r = parse(ARM).unwrap();
    assert_eq!(r.name, "two_link_arm");
    assert_eq!(r.dialect, Dialect::Urdf);
    assert_eq!(r.links.len(), 4);
    assert_eq!(r.links[1].mass, Some(2.5));
    assert_eq!((r.links[1].visuals, r.links[1].collisions), (1, 1));

    let shoulder = &r.joints[0];
    assert_eq!(shoulder.kind, JointKind::Revolute);
    assert_eq!(shoulder.origin.xyz, [0.0, 0.0, 0.1]);
    assert_eq!(shoulder.axis, [0.0, 1.0, 0.0]);
    let limit = shoulder.limit.unwrap();
    assert_eq!((limit.lower, limit.upper), (-1.5, 1.5));

    let elbow = &r.joints[1];
    assert_eq!(elbow.kind, JointKind::Continuous);
    assert_eq!(elbow.axis, [1.0, 0.0, 0.0]);
    assert!(elbow.limit.is_none());
    assert_eq!(r.joints[2].kind, JointKind::Fixed);
    r.validate().unwrap();
}

#[test]
fn reads_an_sdf_model() {
    let r = parse(SDF).unwrap();
    assert_eq!(r.dialect, Dialect::Sdf);
    assert_eq!(r.name, "cart");
    assert_eq!(r.links[0].mass, Some(12.0));
    let j = &r.joints[0];
    assert_eq!(j.parent, "chassis");
    assert_eq!(j.child, "wheel");
    assert_eq!(j.origin.xyz, [0.1, 0.2, 0.0]);
    assert_eq!(j.axis, [0.0, 0.0, 1.0]);
    assert_eq!(j.limit.unwrap().effort, 5.0);
    assert_eq!(r.links[1].mass, None);
    r.validate().unwrap();
}

#[test]
fn urdf_write_then_parse_is_identity() {
    let r = parse(ARM).unwrap();
    let text = write(&r);
    assert_eq!(parse(&text).unwrap(), r);
}

#[test]
fn validation_rejects_bad_trees() {
    let mut r = parse(ARM).unwrap();
    r.joints[0].parent = "nowhere".into();
    assert!(matches!(r.validate(), Err(UrdfError::Invalid(_))));

    let mut r = parse(ARM).unwrap();
    r.joints[1].child = "upper".into(); // upper now has two parents
    assert!(matches!(r.validate(), Err(UrdfError::Invalid(_))));

    let cyc = parse(
        br#"<robot name="c"><link name="a"/><link name="b"/>
        <joint name="j1" type="fixed"><parent link="a"/><child link="b"/></joint>
        <joint name="j2" type="fixed"><parent link="b"/><child link="a"/></joint></robot>"#,
    )
    .unwrap();
    assert!(matches!(cyc.validate(), Err(UrdfError::Invalid(_))));
}

#[test]
fn rejects_other_roots_and_malformed_numbers() {
    assert!(parse(b"<cube name=\"x\"/>").is_err());
    assert!(parse(b"<robot><link name=\"a\"/></robot>").is_err());
    assert!(parse(br#"<robot name="r"><link name="a"><inertial><mass value="heavy"/></inertial></link></robot>"#).is_err());
    assert!(parse(br#"<robot name="r"><link name="a"><visual></link></robot>"#).is_err());
}

#[test]
fn unknown_joint_types_are_kept() {
    let r = parse(br#"<robot name="r"><link name="a"/><link name="b"/>
        <joint name="ball" type="ball"><parent link="a"/><child link="b"/></joint></robot>"#)
    .unwrap();
    assert_eq!(r.joints[0].kind, JointKind::Other("ball".into()));
    assert_eq!(r.joints[0].kind.as_str(), "ball");
}
