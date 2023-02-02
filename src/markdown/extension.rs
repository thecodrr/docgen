use pulldown_cmark::{CowStr, Event};

use super::extensions::{link_rewriter::Link, toc::Heading};

pub enum Output<'a> {
    None,
    Event(Event<'a>),
    Link(Link),
    Heading(Heading),

    Block(&'a str),
}

pub trait Extension {
    fn process_event<'a>(
        &mut self,
        events: &mut Vec<Event<'a>>,
        event: &Event<'a>,
    ) -> (Option<Vec<Output<'a>>>, bool);

    fn end_of_doc<'a>(&mut self, _events: &mut Vec<Event<'a>>) -> Option<Vec<Output<'a>>> {
        None
    }
}

pub trait TextExtension {
    fn process_text(&self, text: &CowStr) -> CowStr;
}
