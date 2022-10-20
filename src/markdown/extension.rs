use pulldown_cmark::Event;

use super::extensions::{link_rewriter::Link, toc::Heading};

pub enum Output<'a> {
    None,
    Event(Event<'a>),
    Link(Link),
    Heading(Heading),
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
    fn process_text<'a>(&self, text: &'a str) -> String;
}
