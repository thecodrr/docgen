use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag};

use crate::markdown::extension::{Extension, Output};

pub struct MathBlock;

impl Extension for MathBlock {
    fn process_event<'a>(
        &mut self,
        events: &mut Vec<Event<'a>>,
        event: &Event<'a>,
    ) -> (Option<Vec<Output<'a>>>, bool) {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(inner))) => {
                let lang = inner.split(' ').next().unwrap();
                if lang == "math" {
                    return (
                        Some(vec![
                            Output::Event(html!("<div class=\"math\">\n")),
                            Output::Block("math"),
                        ]),
                        true,
                    );
                }
            }
            Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(inner))) => {
                let lang = inner.split(' ').next().unwrap();
                if lang == "math" {
                    let code_event = events.last_mut().unwrap();
                    if let Some(code) = match code_event {
                        Event::Text(text) => Some(text.to_string()),
                        _ => None,
                    } {
                        let opts = katex::Opts::builder()
                            .display_mode(true)
                            .output_type(katex::OutputType::HtmlAndMathml)
                            .build()
                            .unwrap();
                        katex::render_with_opts(&code, &opts).unwrap();
                        if let Ok(html) = katex::render_with_opts(&code, &opts) {
                            println!("katex {:?}", html);
                            *code_event = Event::Html(CowStr::from(html));
                        }
                    }

                    return (Some(vec![Output::Event(html!("</div>"))]), true);
                }
            }
            _ => {}
        }
        (None, false)
    }
}
