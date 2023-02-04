use std::fmt;

use pulldown_cmark::{CowStr, Event, Tag};

use crate::markdown::extension::{Extension, Output};

pub struct Callout;

#[derive(Debug, PartialEq, Clone)]
pub enum CalloutKind {
    Info,
    Success,
    Warning,
    Error,
}

impl Extension for Callout {
    fn process_event<'a>(
        &mut self,
        events: &mut Vec<Event<'a>>,
        event: &Event<'a>,
    ) -> (Option<Vec<Output<'a>>>, bool) {
        match event {
            Event::End(Tag::BlockQuote) => {
                let start_index = events.len()
                    - 1
                    - events
                        .iter_mut()
                        .rev()
                        .position(|tag| match tag {
                            Event::Start(Tag::BlockQuote) => true,
                            _ => false,
                        })
                        .unwrap();

                let mut callout_title = String::new();
                for event in &mut events[start_index + 1..] {
                    match event {
                        Event::Text(text) | Event::Code(text) => {
                            callout_title.push_str(&text.to_string());
                        }
                        Event::End(Tag::Paragraph) => {
                            break;
                        }
                        _ => {}
                    }
                }

                let callout = parse_callout(&callout_title);
                if let Some((callout_type, title)) = callout {
                    for event in &mut events[start_index + 1..] {
                        match event {
                            Event::End(Tag::Paragraph) => {
                                *event = html!("");
                                break;
                            }
                            _ => {
                                *event = html!("");
                            }
                        }
                    }

                    events[start_index] = if title.is_empty() {
                        html!(
                            "<div class=\"callout {}\"><div class=\"callout-content\">",
                            callout_type
                        )
                    } else {
                        html!(
                            "<div class=\"callout {}\"><p class=\"callout-title\">{}</p><div class=\"callout-content\">",
                            callout_type,
                            title
                        )
                    };
                    return (Some(vec![Output::Event(html!("</div></div>"))]), true);
                }
            }
            _ => {}
        }
        (None, false)
    }
}

impl TryFrom<&str> for CalloutKind {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, &'static str> {
        match value {
            "info" => Ok(CalloutKind::Info),
            "notice" => Ok(CalloutKind::Info),
            "success" => Ok(CalloutKind::Success),
            "warning" => Ok(CalloutKind::Warning),
            "warn" => Ok(CalloutKind::Warning),
            "error" => Ok(CalloutKind::Error),
            _ => Err("Unknown callout kind"),
        }
    }
}

impl fmt::Display for CalloutKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CalloutKind::Info => write!(f, "info"),
            CalloutKind::Success => write!(f, "success"),
            CalloutKind::Warning => write!(f, "warning"),
            CalloutKind::Error => write!(f, "error"),
        }
    }
}

fn parse_callout(text: &str) -> Option<(CalloutKind, String)> {
    let callout_types = ["info", "notice", "success", "warn", "warning", "error"];
    let mut words = text.split_whitespace();
    let first_word = words.next().unwrap();
    let title = words
        .map(|s| s.to_string())
        .reduce(|all, words| all + " " + &words)
        .unwrap_or(String::new());

    for callout_type in callout_types {
        if first_word == callout_type {
            if let Ok(kind) = CalloutKind::try_from(callout_type) {
                return Some((kind, title));
            }
        }
    }
    return None;
}
