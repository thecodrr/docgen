use std::fmt;

use pulldown_cmark::{CowStr, Event, Tag};

use crate::markdown::extension::{Extension, Output};

pub struct Tasklist;

impl Extension for Tasklist {
    fn process_event<'a>(
        &mut self,
        events: &mut Vec<Event<'a>>,
        event: &Event<'a>,
    ) -> (Option<Vec<Output<'a>>>, bool) {
        match event {
            Event::End(Tag::List(_)) => {
                let is_tasklist = events.iter_mut().rev().any(|tag| match tag {
                    Event::TaskListMarker(_) => true,
                    _ => false,
                });

                if is_tasklist {
                    let start_index = events.len()
                        - 1
                        - events
                            .iter_mut()
                            .rev()
                            .position(|tag| match tag {
                                Event::Start(Tag::List(_)) => true,
                                _ => false,
                            })
                            .unwrap();
                    events[start_index] = html!("<ul class=\"checklist\">");
                }
            }
            _ => {}
        }
        (None, false)
    }
}
