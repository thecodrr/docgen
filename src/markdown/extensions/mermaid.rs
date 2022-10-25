use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag};

use crate::markdown::extension::{Extension, Output};

pub struct MermaidBlock;

impl Extension for MermaidBlock {
    fn process_event<'a>(
        &mut self,
        _events: &mut Vec<Event<'a>>,
        event: &Event<'a>,
    ) -> (Option<Vec<Output<'a>>>, bool) {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(inner))) => {
                let lang = inner.split(' ').next().unwrap();
                if lang == "mermaid" {
                    return (
                        Some(vec![
                            Output::Event(html!("<div class=\"mermaid\">\n")),
                            Output::Block("diagram"),
                        ]),
                        true,
                    );
                }
            }
            Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(inner))) => {
                let lang = inner.split(' ').next().unwrap();
                if lang == "mermaid" {
                    return (Some(vec![Output::Event(html!("</div>"))]), true);
                }
            }
            _ => {}
        }
        (None, false)
    }
}
