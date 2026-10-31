//! The `markup.bcfxml` part of a topic: reading it into a [`Topic`] and writing it back.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::xml::{self, Element};

use crate::{BcfError, Comment, Topic, ViewpointRef};

const MARKUP_NS: &str = "http://www.buildingsmart-tech.org/bcf/markup/2.1";

/// The trimmed text of the first child called `name`, or `None` when it is absent or empty.
fn text(el: &Element, name: &str) -> Option<String> {
    el.child(name).map(|c| c.text_trimmed().to_string()).filter(|s| !s.is_empty())
}

pub(crate) fn topic(root: &Element) -> Result<Topic, BcfError> {
    if root.local_name() != "Markup" {
        return Err(BcfError::Invalid("markup part root is not <Markup>".to_string()));
    }
    let t = root
        .child("Topic")
        .ok_or_else(|| BcfError::Invalid("markup part has no <Topic>".to_string()))?;
    let guid = t
        .attr("Guid")
        .ok_or_else(|| BcfError::Invalid("<Topic> has no Guid".to_string()))?;
    let mut topic = Topic {
        guid: guid.to_string(),
        title: text(t, "Title").unwrap_or_default(),
        topic_type: t.attr("TopicType").map(|s| s.to_string()),
        status: t.attr("TopicStatus").map(|s| s.to_string()),
        priority: text(t, "Priority"),
        creation_author: text(t, "CreationAuthor"),
        creation_date: text(t, "CreationDate"),
        comments: Vec::new(),
        viewpoints: Vec::new(),
    };
    for c in t.children_named("Comment") {
        topic.comments.push(Comment {
            guid: c.attr("Guid").unwrap_or_default().to_string(),
            author: text(c, "Author"),
            date: text(c, "Date"),
            text: text(c, "Comment").unwrap_or_default(),
        });
    }
    for v in t.children_named("Viewpoints") {
        topic.viewpoints.push(ViewpointRef {
            guid: v.attr("Guid").unwrap_or_default().to_string(),
            file: text(v, "Viewpoint"),
        });
    }
    Ok(topic)
}

pub(crate) fn markup(topic: &Topic) -> String {
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(s, "<Markup xmlns=\"{MARKUP_NS}\">");
    let _ = write!(s, "<Topic Guid=\"{}\"", xml::escape_attr(&topic.guid));
    if let Some(t) = &topic.topic_type {
        let _ = write!(s, " TopicType=\"{}\"", xml::escape_attr(t));
    }
    if let Some(st) = &topic.status {
        let _ = write!(s, " TopicStatus=\"{}\"", xml::escape_attr(st));
    }
    s.push_str(">\n");
    element(&mut s, "Title", Some(&topic.title));
    element(&mut s, "Priority", topic.priority.as_deref());
    element(&mut s, "CreationDate", topic.creation_date.as_deref());
    element(&mut s, "CreationAuthor", topic.creation_author.as_deref());
    for c in &topic.comments {
        let _ = writeln!(s, "<Comment Guid=\"{}\">", xml::escape_attr(&c.guid));
        element(&mut s, "Date", c.date.as_deref());
        element(&mut s, "Author", c.author.as_deref());
        element(&mut s, "Comment", Some(&c.text));
        s.push_str("</Comment>\n");
    }
    for v in &topic.viewpoints {
        match &v.file {
            Some(file) => {
                let _ = writeln!(
                    s,
                    "<Viewpoints Guid=\"{}\"><Viewpoint>{}</Viewpoint></Viewpoints>",
                    xml::escape_attr(&v.guid),
                    xml::escape_text(file)
                );
            }
            None => {
                let _ = writeln!(s, "<Viewpoints Guid=\"{}\"/>", xml::escape_attr(&v.guid));
            }
        }
    }
    s.push_str("</Topic>\n</Markup>\n");
    s
}

fn element(s: &mut String, name: &str, value: Option<&str>) {
    if let Some(v) = value {
        let _ = writeln!(s, "<{name}>{}</{name}>", xml::escape_text(v));
    }
}
