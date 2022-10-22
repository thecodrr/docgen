use std::collections::HashMap;

use pulldown_cmark::{html, CowStr, Event, Options, Parser};

use super::{
    extension::{Extension, Output, TextExtension},
    extensions::{
        callout::Callout,
        codeblock::CodeBlock,
        emoji::EmojiConverter,
        link_rewriter::{Link, LinkRewriter},
        math::MathBlock,
        mermaid::MermaidBlock,
        tabs::Tabs,
        toc::{Heading, TableOfContents},
    },
};

pub struct MarkdownParser {
    pub extensions: Vec<Box<dyn Extension>>,
    pub text_processors: Vec<Box<dyn TextExtension>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedMarkdown {
    pub html: String,
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
}

pub struct ParseOptions {
    /// Changes the root URL for any links that point to the current domain.
    pub url_root: String,
    pub link_rewrite_rules: HashMap<String, String>,
    pub url_params: Vec<(String, String)>,
    pub root_dir: Option<String>,
    // pub resolve_embeds: Option<Box<dyn Fn(String) -> Option<String>>>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        ParseOptions {
            url_root: String::from("/"),
            link_rewrite_rules: HashMap::new(),
            url_params: vec![],
            root_dir: None,
        }
    }
}

impl MarkdownParser {
    pub fn new(options: Option<ParseOptions>) -> Self {
        let parse_opts = options.unwrap_or(ParseOptions::default());

        let url_root = parse_opts.url_root.to_owned();
        let link_rewrite_rules = parse_opts.link_rewrite_rules.to_owned();
        let url_params = parse_opts.url_params.to_owned();

        let extensions: Vec<Box<dyn Extension>> = vec![
            Box::new(Callout),
            Box::new(MermaidBlock),
            Box::new(MathBlock),
            Box::new(Tabs {
                current_tabgroup: None,
                current_tab: None,
            }),
            Box::new(CodeBlock),
            Box::new(LinkRewriter {
                url_root,
                link_rewrite_rules,
                url_params,
                current_link: None,
            }),
            Box::new(TableOfContents {
                current_heading: None,
            }),
        ];

        let text_processors: Vec<Box<dyn TextExtension>> = vec![Box::new(EmojiConverter)];

        MarkdownParser {
            extensions,
            text_processors,
        }
    }

    pub fn parse(&mut self, input: &str) -> ParsedMarkdown {
        let mut parser = Parser::new_ext(input, Options::all())
            .into_iter()
            .peekable();

        let mut events: Vec<Event> = Vec::new();
        let mut links: Vec<Link> = Vec::new();
        let mut headings: Vec<Heading> = Vec::new();

        while let Some(ev) = parser.next() {
            let event = &mut ev.to_owned();

            match event {
                Event::Text(text) => {
                    let mut copy = text.to_string();
                    for extension in &self.text_processors {
                        copy = extension.process_text(&copy);
                        *event = Event::Text(CowStr::from(copy.to_owned()));
                    }
                }
                Event::Html(_) => {
                    continue;
                }
                _ => {}
            }

            let mut handled = false;
            for extension in &mut self.extensions {
                let (output, is_handled) = extension.process_event(&mut events, &event);

                handle_output(output, &mut events, &mut links, &mut headings);

                if is_handled {
                    handled = true;
                    break;
                }
            }

            if !handled {
                events.push(event.to_owned());
            }
        }

        for extension in &mut self.extensions {
            let output = extension.end_of_doc(&mut events);
            handle_output(output, &mut events, &mut links, &mut headings);
        }

        // Write to String buffer.
        let mut as_html = String::new();
        html::push_html(&mut as_html, events.into_iter());

        ParsedMarkdown {
            html: as_html,
            headings,
            links,
        }
    }
}

fn handle_output<'a>(
    output: Option<Vec<Output<'a>>>,
    events: &mut Vec<Event<'a>>,
    links: &mut Vec<Link>,
    headings: &mut Vec<Heading>,
) {
    if let Some(output) = output {
        output.into_iter().for_each(|result| match result {
            Output::Event(ev) => events.push(ev),
            Output::Link(link) => links.push(link),
            Output::Heading(heading) => headings.push(heading),
            _ => {}
        });
    }
}
