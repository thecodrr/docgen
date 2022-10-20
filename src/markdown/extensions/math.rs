use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag};

use crate::markdown::extension::{Extension, Output};

pub struct MathBlock;

impl Extension for MathBlock {
    fn process_event<'a>(
        &mut self,
        _events: &mut Vec<Event<'a>>,
        event: &Event<'a>,
    ) -> (Option<Vec<Output<'a>>>, bool) {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(inner))) => {
                let lang = inner.split(' ').next().unwrap();
                if lang == "math" {
                    return (
                        Some(vec![Output::Event(html!("<div class=\"math\">\n"))]),
                        true,
                    );
                }
            }
            Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(inner))) => {
                let lang = inner.split(' ').next().unwrap();
                if lang == "math" {
                    return (Some(vec![Output::Event(html!("</div>"))]), true);
                }
            }
            _ => {}
        }
        (None, false)
    }
}
