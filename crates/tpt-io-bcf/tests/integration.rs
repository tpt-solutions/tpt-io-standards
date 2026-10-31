//! Integration tests for the BCF 2.1 project reader and writer.

use tpt_io_bcf::{parse, write, BcfError, Comment, Project, Topic, ViewpointRef};
use tpt_io_core::zip;

fn project() -> Project {
    Project {
        version: "2.1".to_string(),
        topics: vec![
            Topic {
                guid: "0a1b-issue".to_string(),
                title: "Door & frame clash".to_string(),
                topic_type: Some("Clash".to_string()),
                status: Some("Open".to_string()),
                priority: Some("High".to_string()),
                creation_author: Some("a@example.test".to_string()),
                creation_date: Some("2026-01-02T09:00:00Z".to_string()),
                comments: vec![Comment {
                    guid: "c1".to_string(),
                    author: Some("b@example.test".to_string()),
                    date: Some("2026-01-03T10:00:00Z".to_string()),
                    text: "Moved <by> 50 mm".to_string(),
                }],
                viewpoints: vec![ViewpointRef {
                    guid: "v1".to_string(),
                    file: Some("viewpoint.bcfv".to_string()),
                }],
            },
            Topic {
                guid: "0a1b-note".to_string(),
                title: "Check fire rating".to_string(),
                ..Topic::default()
            },
        ],
    }
}

#[test]
fn round_trip_keeps_topics_comments_and_viewpoints() {
    let p = project();
    assert_eq!(parse(&write(&p)).unwrap(), p);
}

#[test]
fn written_archive_has_version_part_and_one_markup_per_topic() {
    let bytes = write(&project());
    assert!(zip::entry(&bytes, "bcf.version").unwrap().is_some());
    assert!(zip::entry(&bytes, "0a1b-issue/markup.bcfxml").unwrap().is_some());
    assert!(zip::entry(&bytes, "0a1b-note/markup.bcfxml").unwrap().is_some());
}

#[test]
fn reads_version_and_ignores_other_parts() {
    let markup = br#"<Markup><Topic Guid="t1" TopicStatus="Closed"><Title>Hello</Title></Topic></Markup>"#;
    let package = zip::write_stored(&[
        ("bcf.version", br#"<Version VersionId="2.1"/>"#.as_slice()),
        ("t1/markup.bcfxml", markup.as_slice()),
        ("t1/viewpoint.bcfv", b"<VisualizationInfo/>".as_slice()),
    ]);
    let p = parse(&package).unwrap();
    assert_eq!(p.version, "2.1");
    assert_eq!(p.topics.len(), 1);
    assert_eq!(p.topics[0].status.as_deref(), Some("Closed"));
    assert_eq!(p.topics[0].title, "Hello");
    assert!(p.topics[0].comments.is_empty());
}

#[test]
fn missing_version_part_defaults_to_2_1() {
    let package = zip::write_stored(&[("t/markup.bcfxml", b"<Markup><Topic Guid=\"t\"/></Markup>".as_slice())]);
    assert_eq!(parse(&package).unwrap().version, "2.1");
}

#[test]
fn topic_without_guid_is_invalid() {
    let package = zip::write_stored(&[(
        "x/markup.bcfxml",
        b"<Markup><Topic><Title>no guid</Title></Topic></Markup>".as_slice(),
    )]);
    assert!(matches!(parse(&package), Err(BcfError::Invalid(_))));
}

#[test]
fn markup_without_topic_is_invalid() {
    let package = zip::write_stored(&[("x/markup.bcfxml", b"<Markup/>".as_slice())]);
    assert!(matches!(parse(&package), Err(BcfError::Invalid(_))));
}

#[test]
fn non_zip_input_is_a_core_error() {
    assert!(matches!(parse(b"not a BCF project"), Err(BcfError::Core(_))));
}
