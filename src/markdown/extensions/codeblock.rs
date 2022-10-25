use once_cell::sync::OnceCell;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag};
use syntect::util::LinesWithEndings;

use crate::markdown::extension::{Extension, Output};
use syntect::html::line_tokens_to_classed_spans;
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};
use syntect::Error;

pub struct CodeBlock;

static SYNTAX_SET: OnceCell<SyntaxSet> = OnceCell::new();

impl Extension for CodeBlock {
    fn process_event<'a>(
        &mut self,
        events: &mut Vec<Event<'a>>,
        event: &Event<'a>,
    ) -> (Option<Vec<Output<'a>>>, bool) {
        match event {
            Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(inner))) => {
                let syntax_set = SYNTAX_SET.get_or_init(|| SyntaxSet::load_defaults_newlines());

                if let Some(syntax) = syntax_set.find_syntax_by_token(inner.to_string().as_str()) {
                    let code_event = events.last_mut().unwrap();
                    if let Some(code) = match code_event {
                        Event::Text(text) => Some(text.to_string()),
                        _ => None,
                    } {
                        let highlighted_code =
                            highlighted_html_for_string(&code, syntax_set, syntax);

                        if let Ok(highlighted_code) = highlighted_code {
                            *code_event = Event::Html(CowStr::from(highlighted_code));

                            return (
                                Some(vec![Output::Event(event.to_owned()), Output::Block("code")]),
                                true,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
        (None, false)
    }
}

fn highlighted_html_for_string(
    s: &str,
    ss: &SyntaxSet,
    syntax: &SyntaxReference,
) -> Result<String, Error> {
    let mut parse_state = ParseState::new(syntax);
    let mut html = String::new();
    let mut scope_stack = ScopeStack::new();
    let mut open_spans = 0;
    let mut first_line = true;

    for line in LinesWithEndings::from(s) {
        let mut parsed_line = parse_state.parse_line(line, ss)?;

        // remove the wrapping <span>
        if first_line {
            parsed_line.remove(0);
        }

        let (formatted_line, delta) = line_tokens_to_classed_spans(
            line,
            parsed_line.as_slice(),
            syntect::html::ClassStyle::Spaced,
            &mut scope_stack,
        )?;

        // since we removed the wrapping span we don't want to close a
        // non-existent span
        if first_line {
            // delta -= 1;
            first_line = false;
        }

        open_spans += delta;
        html.push_str(formatted_line.as_str());
    }

    for _ in 0..open_spans {
        html.push_str("</span>");
    }

    Ok(html)
}
