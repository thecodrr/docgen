use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
};

use pulldown_cmark::{html, Event, Options, Parser, Tag};

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
        task_list::Tasklist,
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
    pub preview: String,
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
    pub blocks: HashSet<String>,
}

impl Default for ParsedMarkdown {
    fn default() -> Self {
        ParsedMarkdown {
            html: String::new(),
            preview: String::new(),
            headings: vec![],
            links: vec![],
            blocks: HashSet::new(),
        }
    }
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
            Box::new(Tasklist),
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
        let mut parser = Parser::new_ext(input, Options::all()).into_iter();

        let mut events: Vec<Event> = Vec::new();
        let mut parsed = ParsedMarkdown::default();
        let mut extract_preview = false;

        while let Some(ev) = &mut parser.borrow_mut().next() {
            if let Event::Text(text) = ev {
                for extension in &self.text_processors {
                    *text = extension.process_text(text)
                }

                if extract_preview {
                    parsed.preview = text.to_string();
                    extract_preview = false;
                }
            }

            if let Event::Start(Tag::Paragraph) = ev {
                extract_preview = parsed.preview.len() <= 0;
            }

            let mut handled = false;
            for extension in &mut self.extensions {
                let (output, is_handled) = extension.process_event(&mut events, &ev);

                handle_output(output, &mut events, &mut parsed);

                if is_handled {
                    handled = true;
                    break;
                }
            }

            if !handled {
                events.push(ev.to_owned());
            }
        }

        for extension in &mut self.extensions {
            let output = extension.end_of_doc(&mut events);
            handle_output(output, &mut events, &mut parsed);
        }

        // Write to String buffer.
        html::push_html(&mut parsed.html, events.into_iter());

        parsed
    }
}

#[inline]
fn handle_output<'a>(
    output: Option<Vec<Output<'a>>>,
    events: &mut Vec<Event<'a>>,
    parsed: &mut ParsedMarkdown,
) {
    if let Some(output) = output {
        output.into_iter().for_each(|result| match result {
            Output::Event(ev) => events.push(ev),
            Output::Link(link) => parsed.links.push(link),
            Output::Heading(heading) => parsed.headings.push(heading),
            Output::Block(block) => {
                parsed.blocks.insert(block.to_string());
            }
            _ => {}
        });
    }
}
