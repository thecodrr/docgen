use pulldown_cmark::{CowStr, Event, Tag};
use serde::Serialize;
use slug::slugify;

use crate::markdown::extension::{Extension, Output};

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Heading {
    pub title: String,
    pub anchor: String,
    pub level: u32,
}

pub struct TableOfContents {
    pub current_heading: Option<Heading>,
}

impl Extension for TableOfContents {
    fn process_event<'a>(
        &mut self,
        events: &mut Vec<Event<'a>>,
        event: &Event<'a>,
    ) -> (Option<Vec<Output<'a>>>, bool) {
        match event.to_owned() {
            Event::Start(Tag::Heading(level @ 1..=6)) => {
                self.current_heading = Some(Heading {
                    level,
                    anchor: String::new(),
                    title: String::new(),
                });
            }
            Event::End(Tag::Heading(_)) => {
                let mut heading = self.current_heading.take().unwrap();
                heading.anchor = slugify(&heading.title);

                if let Some(header_start) = events.iter_mut().rev().find(|tag| match tag {
                    Event::Start(Tag::Heading(_)) => true,
                    _ => false,
                }) {
                    *header_start = html!("<h{} id=\"{}\">", heading.level, heading.anchor);
                }

                return (Some(vec![Output::Heading(heading)]), false);
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(heading) = &mut self.current_heading {
                    heading.title.push_str(&text);
                }
            }
            _ => {}
        }
        (None, false)
    }
}
