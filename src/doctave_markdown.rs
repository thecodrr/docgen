use pulldown_cmark::{html, CodeBlockKind, CowStr, Event, LinkType, Options, Parser, Tag};
use regex::Regex;
use url::{ParseError, Url};

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Clone)]
pub struct Markdown {
    pub as_html: String,
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Heading {
    pub title: String,
    pub anchor: String,
    pub level: u32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Link {
    pub title: String,
    pub url: UrlType,
}

#[derive(Debug, PartialEq, Clone)]
pub enum UrlType {
    Local(PathBuf),
    Remote(Url),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Callout {
    kind: CalloutKind,
    title: Option<String>,
}

impl Callout {
    fn build(kind: &str) -> Result<Self, &'static str> {
        let kind = CalloutKind::try_from(kind)?;
        let title = Some(kind.to_string());

        Ok(Callout { kind, title })
    }

    fn build_with_title(kind: &str, raw_title: &str) -> Result<Self, &'static str> {
        let title;
        let kind = CalloutKind::try_from(kind)?;

        if raw_title == "" {
            title = None;
        } else {
            title = Some(raw_title.to_owned());
        }

        Ok(Callout { kind, title })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum CalloutKind {
    Info,
    Success,
    Warning,
    Error,
}

impl TryFrom<&str> for CalloutKind {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, &'static str> {
        match value {
            "info" => Ok(CalloutKind::Info),
            "notice" => Ok(CalloutKind::Info),
            "success" => Ok(CalloutKind::Success),
            "warning" => Ok(CalloutKind::Warning),
            "warn" => Ok(CalloutKind::Warning),
            "error" => Ok(CalloutKind::Error),
            _ => Err("Unknown callout kind"),
        }
    }
}

impl fmt::Display for CalloutKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CalloutKind::Info => write!(f, "info"),
            CalloutKind::Success => write!(f, "success"),
            CalloutKind::Warning => write!(f, "warning"),
            CalloutKind::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ParseOptions {
    /// Changes the root URL for any links that point to the current domain.
    pub url_root: String,
    pub link_rewrite_rules: HashMap<String, String>,
    pub url_params: HashMap<String, String>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        ParseOptions {
            url_root: String::from("/"),
            link_rewrite_rules: HashMap::new(),
            url_params: HashMap::new(),
        }
    }
}

pub fn parse(input: &str, opts: Option<ParseOptions>) -> Markdown {
    let parse_opts = opts.unwrap_or(ParseOptions::default());

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_TABLES);

    let mut headings = vec![];
    let mut links = vec![];
    let mut active_callout = None;
    let mut current_link = None;
    let mut current_heading: Option<Heading> = None;

    let mut parser = Parser::new_ext(input, options).into_iter().peekable();

    let mut events = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            // Mermaid JS code block tranformations
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(inner))) => {
                let lang = inner.split(' ').next().unwrap();

                if lang == "mermaid" {
                    events.push(Event::Html(CowStr::Borrowed("<div class=\"mermaid\">\n")));
                } else if lang == "math" {
                    events.push(Event::Html(CowStr::Borrowed("<div class=\"math\">\n")));
                } else {
                    events.push(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(inner))));
                }
            }
            Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(inner))) => {
                let lang = inner.split(' ').next().unwrap();
                if lang == "mermaid" || lang == "math" {
                    events.push(Event::Html(CowStr::Borrowed("</div>")));
                } else {
                    events.push(Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(inner))));
                }
            }
            Event::Code(ref text) => {
                if let Some(heading) = &mut current_heading {
                    if heading.anchor.len() != 0 {
                        heading.anchor.push('-');
                    }

                    heading
                        .anchor
                        .push_str(&text.clone().trim().to_lowercase().replace(" ", "-"));

                    heading.title.push_str(&text);
                }
                events.push(event);
            }

            // Link rewrites
            Event::Start(Tag::Link(link_type, url, title)) => {
                let (link_type, url, title) = rewrite_link(link_type, url, title, &parse_opts);

                let url = if !parse_opts.url_params.is_empty() && is_in_local_domain(&url) {
                    append_parameters(url, &parse_opts)
                } else {
                    url
                };

                if link_type == LinkType::Inline {
                    if let Ok(valid_url) = Url::parse(&url.clone())
                        .map(|u| UrlType::Remote(u))
                        .or_else(|e| match e {
                            ParseError::EmptyHost | ParseError::RelativeUrlWithoutBase => {
                                Ok(UrlType::Local(PathBuf::from(url.clone().into_string())))
                            }
                            e => Err(e),
                        })
                        .map_err(|l| l)
                    {
                        current_link = Some(Link {
                            title: title.clone().to_string(),
                            url: valid_url,
                        });
                    }
                }
                events.push(Event::Start(Tag::Link(link_type, url, title)));
            }

            Event::End(Tag::Link(link_type, url, title)) => {
                if current_link.is_some() {
                    links.push(current_link.take().unwrap())
                }

                events.push(Event::End(Tag::Link(link_type, url, title)));
            }

            // Image link rewrites
            Event::Start(Tag::Image(link_type, url, title)) => {
                let (link_type, url, title) = rewrite_link(link_type, url, title, &parse_opts);

                events.push(Event::Start(Tag::Image(link_type, url, title)));
            }

            // Apply heading anchor tags
            Event::Start(Tag::Heading(level @ 1..=6)) => {
                current_heading = Some(Heading {
                    level: level,
                    anchor: String::new(),
                    title: String::new(),
                });

                events.push(event);
            }

            Event::End(Tag::Heading(_)) => {
                let closed_heading = current_heading.take().unwrap();

                let header_start = events
                    .iter_mut()
                    .rev()
                    .find(|tag| match tag {
                        Event::Start(Tag::Heading(_)) => true,
                        _ => false,
                    })
                    .unwrap();

                *header_start = Event::Html(CowStr::from(format!(
                    "<h{} id=\"{}\">",
                    closed_heading.level, closed_heading.anchor
                )));

                headings.push(closed_heading);
                events.push(event);
            }

            Event::Start(Tag::Paragraph) => {
                if let Some(next_event) = parser.peek() {
                    match next_event {
                        Event::Text(text) => {
                            if !is_callout_start(&text) && !is_callout_end(&text) {
                                events.push(event);
                            }
                        }
                        _ => events.push(event),
                    }
                } else {
                    events.push(event)
                }
            }

            Event::End(Tag::Paragraph) => {
                if let Some(Event::Start(Tag::Paragraph)) = events.last() {
                    events.pop();
                } else if is_callout_close_event(events.last()) {
                    // Skip
                } else {
                    events.push(event);
                }
            }

            Event::Text(text) => {
                let text = convert_emojis(&text);

                if let Some(link) = &mut current_link {
                    // We are in the middle of parsing a link. Push the title.
                    link.title.push_str(&text);
                }

                if let Some(heading) = &mut current_heading {
                    if heading.anchor.len() != 0 {
                        heading.anchor.push('-');
                    }

                    heading
                        .anchor
                        .push_str(&text.clone().trim().to_lowercase().replace(" ", "-"));

                    heading.title.push_str(&text);
                }

                if active_callout.is_some() && is_callout_end(&text) {
                    active_callout = None;
                    if Some(&Event::End(Tag::Paragraph)) != events.last() {
                        events.push(Event::End(Tag::Paragraph));
                    }
                    events.push(Event::Html(CowStr::from(format!("</div>"))));
                    if Some(&Event::SoftBreak) == parser.peek() {
                        events.push(Event::Start(Tag::Paragraph));
                    }
                } else if active_callout.is_none() && is_callout_start(&text) {
                    if let Some(callout) = parse_callout(&text) {
                        active_callout = Some(callout.clone());

                        if let Some(title) = callout.title {
                            events.push(Event::Html(CowStr::from(format!(
                                "<div class=\"callout {}\"><p class=\"callout-title\">{}</p>",
                                callout.kind, title
                            ))));
                        } else {
                            events.push(Event::Html(CowStr::from(format!(
                                "<div class=\"callout {}\">",
                                callout.kind
                            ))));
                        }
                    } else {
                        events.push(Event::Text(text.into()));
                    }

                    events.push(Event::Start(Tag::Paragraph));
                } else {
                    events.push(Event::Text(text.into()));
                }
            }
            _ => events.push(event),
        };
    }

    // Write to String buffer.
    let mut as_html = String::new();
    html::push_html(&mut as_html, events.into_iter());

    let mut allowed_div_classes = HashSet::new();
    // Mermaid JS and math blocks
    allowed_div_classes.insert("mermaid");
    allowed_div_classes.insert("math");
    // Callout-specific
    allowed_div_classes.insert("callout");
    allowed_div_classes.insert("callout-title");
    allowed_div_classes.insert("info");
    allowed_div_classes.insert("success");
    allowed_div_classes.insert("warning");
    allowed_div_classes.insert("error");

    let mut allowed_classes = HashMap::new();
    allowed_classes.insert("div", allowed_div_classes);

    let safe_html = ammonia::Builder::new()
        .link_rel(None)
        .add_tags(&["h1"])
        .add_tag_attributes("h1", &["id"])
        .add_tags(&["h2"])
        .add_tag_attributes("h2", &["id"])
        .add_tags(&["h3"])
        .add_tag_attributes("h3", &["id"])
        .add_tags(&["h4"])
        .add_tag_attributes("h4", &["id"])
        .add_tags(&["h5"])
        .add_tag_attributes("h5", &["id"])
        .add_tags(&["h6"])
        .add_tag_attributes("h6", &["id"])
        .add_tags(&["code"])
        .add_tag_attributes("code", &["class"])
        .add_tags(&["p"])
        .add_tag_attributes("p", &["class"])
        .add_tags(&["input"])
        .add_tag_attribute_values("input", "disabled", &[""])
        .add_tag_attribute_values("input", "type", &["checkbox"])
        .add_tag_attribute_values("input", "checked", &[""])
        .allowed_classes(allowed_classes)
        .add_clean_content_tags(&["form", "script", "style"])
        .clean(&*as_html)
        .to_string();

    Markdown {
        as_html: safe_html,
        links,
        headings,
    }
}

/// Rewrites the link by either setting a different root path, or by
/// swapping the whole URL if there is a matching rule in the rewrite
/// rules.
fn rewrite_link<'a>(
    link_type: LinkType,
    url: CowStr<'a>,
    title: CowStr<'a>,
    parse_opts: &'a ParseOptions,
) -> (LinkType, CowStr<'a>, CowStr<'a>) {
    if let Some(matching_link) = parse_opts
        .link_rewrite_rules
        .get(&url.clone().into_string())
    {
        (link_type, matching_link.as_str().into(), title)
    } else if Path::new(&url.clone().into_string()).starts_with("/") {
        let mut rewritten = parse_opts.url_root.trim_end_matches('/').to_string();
        rewritten.push_str(&url.to_string());

        (link_type, rewritten.into(), title)
    } else {
        (link_type, url, title)
    }
}

fn append_parameters<'a>(url: CowStr<'a>, parse_opts: &'a ParseOptions) -> CowStr<'a> {
    let mut appended = url.into_string();
    appended.push_str("?");

    let mut position = 0;
    let length = parse_opts.url_params.len();

    for (key, value) in &parse_opts.url_params {
        appended.push_str(key);
        appended.push_str("=");
        appended.push_str(value);

        position += 1;
        if position != length {
            appended.push_str("&");
        }
    }

    appended.into()
}

fn is_in_local_domain(url_string: &str) -> bool {
    match Url::parse(url_string) {
        Ok(url) => url.host().is_none(),
        Err(url::ParseError::RelativeUrlWithoutBase) => true,
        Err(url::ParseError::EmptyHost) => true,
        Err(_) => false,
    }
}

lazy_static! {
    static ref CALLOUT_PATTERN_START: Regex =
        Regex::new(r"^\{%\s*(?P<type>\w+)\s*(?P<title>.*)\s*%\}$").unwrap();
    static ref CALLOUT_PATTERN_END: Regex = Regex::new(r"\{%\s*end\s*%\}").unwrap();
}

fn is_callout_start(text: &str) -> bool {
    parse_callout(text).is_some()
}

fn is_callout_end(text: &str) -> bool {
    CALLOUT_PATTERN_END.is_match(text)
}

fn is_callout_close_event(event: Option<&Event>) -> bool {
    event == Some(&Event::Html(CowStr::from(format!("</div>"))))
}

fn parse_callout(text: &str) -> Option<Callout> {
    if let Some(captures) = CALLOUT_PATTERN_START.captures(&text.trim_end()) {
        match (captures.name("type"), captures.name("title")) {
            (Some(callout_type), None) => Callout::build(callout_type.as_str()).ok(),
            (Some(callout_type), Some(title)) => {
                Callout::build_with_title(callout_type.as_str(), title.as_str().trim()).ok()
            }
            _ => None,
        }
    } else {
        None
    }
}

fn convert_emojis(input: &str) -> String {
    let mut acc = String::with_capacity(input.len());
    let mut parsing_emoji = false;
    let mut emoji_identifier = String::new();

    for c in input.chars() {
        match (c, parsing_emoji) {
            (':', false) => parsing_emoji = true,
            (':', true) => {
                if let Some(emoji) = emojis::lookup(&emoji_identifier) {
                    acc.push_str(emoji.as_str());
                } else {
                    acc.push(':');
                    acc.push_str(&emoji_identifier);
                    acc.push(':');
                }

                parsing_emoji = false;
                emoji_identifier.truncate(0);
            }
            (_, true) => emoji_identifier.push(c),
            (_, false) => acc.push(c),
        }
    }

    if parsing_emoji {
        acc.push(':');
        acc.push_str(&emoji_identifier);
    }

    acc
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parses_a_markdown_doc() {
        let input = indoc! {"
        # My heading

        Some content

        ## Some other heading
        "};

        let Markdown {
            as_html,
            headings,
            links: _,
        } = parse(&input, None);

        assert_eq!(
            as_html,
            indoc! {"
                <h1 id=\"my-heading\">My heading</h1>
                <p>Some content</p>
                <h2 id=\"some-other-heading\">Some other heading</h2>
            "}
        );

        assert_eq!(
            headings,
            vec![
                Heading {
                    title: "My heading".to_string(),
                    anchor: "my-heading".to_string(),
                    level: 1,
                },
                Heading {
                    title: "Some other heading".to_string(),
                    anchor: "some-other-heading".to_string(),
                    level: 2,
                }
            ]
        );
    }

    #[test]
    fn optionally_rewrites_link_root_path() {
        let input = indoc! {"
        [an link](/foo/bar)
        "};

        let Markdown {
            as_html,
            headings: _headings,
            links: _,
        } = parse(&input, None);

        assert_eq!(
            as_html,
            indoc! {"
                <p><a href=\"/foo/bar\">an link</a></p>
            "}
        );

        let mut options = ParseOptions::default();
        options.url_root = "/other/root".to_owned();

        let Markdown {
            as_html,
            headings: _headings,
            links: _,
        } = parse(&input, Some(options));

        assert_eq!(
            as_html,
            indoc! {"
                <p><a href=\"/other/root/foo/bar\">an link</a></p>
            "}
        );
    }

    #[test]
    fn does_not_rewrite_non_absolute_urls() {
        let input = indoc! {"
        [an link](https://www.google.com)
        "};

        let mut options = ParseOptions::default();
        options.url_root = "/other/root".to_owned();

        let Markdown {
            as_html,
            headings: _headings,
            links: _,
        } = parse(&input, Some(options));
        assert_eq!(
            as_html,
            indoc! {"
                <p><a href=\"https://www.google.com\">an link</a></p>
            "}
        );

        let input = indoc! {"
        [an link](relative/link)
        "};

        let mut options = ParseOptions::default();
        options.url_root = "/other/root".to_owned();

        let Markdown {
            as_html,
            headings: _headings,
            links: _,
        } = parse(&input, Some(options));

        assert_eq!(
            as_html,
            indoc! {"
                <p><a href=\"relative/link\">an link</a></p>
            "}
        );
    }

    #[test]
    fn rewrites_any_image_that_has_an_explicit_rewrite_mapping() {
        let input = indoc! {"
        ![an image](/assets/cat.jpg)
        "};

        let mut options = ParseOptions::default();

        options.link_rewrite_rules.insert(
            "/assets/cat.jpg".to_owned(),
            "https://example.com/cat.jpg".to_owned(),
        );

        let Markdown {
            as_html,
            headings: _headings,
            links: _,
        } = parse(&input, Some(options));

        assert_eq!(
            as_html,
            indoc! {"
                <p><img src=\"https://example.com/cat.jpg\" alt=\"an image\"></p>
            "}
        );
    }

    #[test]
    fn rewrites_any_link_that_has_an_explicit_rewrite_mapping() {
        let input = indoc! {"
        [an document](/assets/plans.pdf)
        "};

        let mut options = ParseOptions::default();

        options.link_rewrite_rules.insert(
            "/assets/plans.pdf".to_owned(),
            "https://example.com/plans.pdf".to_owned(),
        );

        let Markdown {
            as_html,
            headings: _headings,
            links: _,
        } = parse(&input, Some(options));

        assert_eq!(
            as_html,
            indoc! {"
                <p><a href=\"https://example.com/plans.pdf\">an document</a></p>
            "}
        );
    }

    #[test]
    fn appends_parameters_to_the_end_of_urls() {
        let input = indoc! {"
        [an link](relative/link)
        "};

        let mut options = ParseOptions::default();

        options
            .url_params
            .insert("base".to_owned(), "123".to_owned());

        let Markdown {
            as_html,
            headings: _headings,
            links: _,
        } = parse(&input, Some(options));

        assert_eq!(
            as_html,
            indoc! {"
                <p><a href=\"relative/link?base=123\">an link</a></p>
            "}
        );
    }

    #[test]
    fn appends_multiple_parameters_to_the_end_of_urls() {
        let input = indoc! {"
        [an link](relative/link)
        "};

        let mut options = ParseOptions::default();

        options
            .url_params
            .insert("bases".to_owned(), "are".to_owned());
        options
            .url_params
            .insert("belong".to_owned(), "tous".to_owned());

        let Markdown {
            as_html,
            headings: _headings,
            links: _,
        } = parse(&input, Some(options));

        assert!(as_html.contains("bases=are"));
        assert!(as_html.contains("belong=tous"));
        assert!(as_html.contains("&amp;"));
    }

    #[test]
    fn appends_multiple_parameters_to_the_end_of_absolute_urls() {
        let input = indoc! {"
        [an link](/absolute/link)
        "};

        let mut options = ParseOptions::default();

        options
            .url_params
            .insert("base".to_owned(), "123".to_owned());

        let Markdown {
            as_html,
            headings: _headings,
            links: _,
        } = parse(&input, Some(options));

        assert_eq!(
            as_html,
            indoc! {"
                <p><a href=\"/absolute/link?base=123\">an link</a></p>
            "}
        );
    }

    #[test]
    fn does_not_append_params_to_urls_with_a_specific_domain() {
        let input = indoc! {"
        [an link](http://www.example.com/)
        "};

        let mut options = ParseOptions::default();

        options
            .url_params
            .insert("bases".to_owned(), "are".to_owned());
        options
            .url_params
            .insert("belong".to_owned(), "tous".to_owned());

        let Markdown {
            as_html,
            headings: _headings,
            links: _,
        } = parse(&input, Some(options));

        assert_eq!(
            as_html,
            indoc! {"
                <p><a href=\"http://www.example.com/\">an link</a></p>
            "}
        );
    }

    #[test]
    fn sanitizes_input() {
        let input = indoc! {"
        <script>
        alert('I break you');
        </script>
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _,
        } = parse(&input, Some(options));

        assert_eq!(as_html, "\n");
    }

    #[test]
    fn allows_mermaid_blocks() {
        let input = indoc! {"
        ```mermaid
        graph TD;
            A-->B;
            A-->C;
        ```
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _,
        } = parse(&input, Some(options));

        assert_eq!(
            as_html,
            indoc! {"
        <div class=\"mermaid\">
        graph TD;
            A--&gt;B;
            A--&gt;C;
        </div>"}
        );
    }

    #[test]
    fn allows_code_blocks() {
        let input = indoc! {"
        ```ruby
        1 + 1
        ```
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _,
        } = parse(&input, Some(options));

        assert_eq!(
            as_html,
            indoc! {"
        <pre><code class=\"language-ruby\">1 + 1
        </code></pre>
 "}
        );
    }

    #[test]
    fn gathers_a_list_of_links_on_the_page() {
        let input = indoc! {"
        [foo](/bar)

        [Example](https://www.example.com)
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html: _as_html,
            headings: _headings,
            links,
        } = parse(&input, Some(options));

        assert_eq!(
            links,
            vec![
                Link {
                    title: "foo".to_string(),
                    url: UrlType::Local("/bar".into())
                },
                Link {
                    title: "Example".to_string(),
                    url: UrlType::Remote(Url::parse("https://www.example.com").unwrap())
                }
            ]
        );
    }

    #[test]
    fn gathers_the_internal_text_of_a_link() {
        let input = indoc! {"
        [**BOLD**](/bar)
        [![AltText](/src/foo)](/bar)
        ## [AnHeader](/bar)
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html: _as_html,
            headings: _headings,
            links,
        } = parse(&input, Some(options));

        assert_eq!(
            links,
            vec![
                Link {
                    title: "BOLD".to_string(),
                    url: UrlType::Local("/bar".into())
                },
                Link {
                    title: "AltText".to_string(),
                    url: UrlType::Local("/bar".into())
                },
                Link {
                    title: "AnHeader".to_string(),
                    url: UrlType::Local("/bar".into())
                }
            ]
        );
    }

    #[test]
    fn detects_emojis() {
        let input = indoc! {"
        I am :grinning:.
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _links,
        } = parse(&input, Some(options));

        assert_eq!(as_html, "<p>I am 😀.</p>\n");
    }

    #[test]
    fn detects_emojis_in_links() {
        let input = indoc! {"
        [:grinning:](/foo)
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _links,
        } = parse(&input, Some(options));

        assert_eq!(as_html, "<p><a href=\"/foo\">😀</a></p>\n");
    }

    #[test]
    fn leaves_the_emoji_identifier_alone_if_it_is_not_recognised() {
        let input = indoc! {"
        Look at this :idonotexist:
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _links,
        } = parse(&input, Some(options));

        assert_eq!(as_html, "<p>Look at this :idonotexist:</p>\n");
    }

    #[test]
    fn ignores_identifiers_that_do_not_end() {
        let input = indoc! {"
        Look at this :stop
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _links,
        } = parse(&input, Some(options));

        assert_eq!(as_html, "<p>Look at this :stop</p>\n");
    }

    #[test]
    fn ignores_identifiers_that_do_not_end_with_whitespace() {
        let input = indoc! {"
        Look at this :stop MORE
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _links,
        } = parse(&input, Some(options));

        assert_eq!(as_html, "<p>Look at this :stop MORE</p>\n");
    }

    fn assert_matches(actual: &str, expected: &str) {
        if actual.trim().replace("\n", "").replace(" ", "")
            != expected.trim().replace("\n", "").replace(" ", "")
        {
            assert!(
                false,
                "Expected and actual did not match:\n == ACTUAL ==============\n{}\n == EXPECTED ============\n{}",
                actual,
                expected)
        }
    }

    #[test]
    fn supports_callout_blocks() {
        let kinds = [
            ("info", "info"),
            ("notice", "info"),
            ("success", "success"),
            ("warn", "warning"),
            ("warning", "warning"),
            ("error", "error"),
        ];

        for (kind, css) in kinds {
            let input = formatdoc! {"
        {{% {} An Note %}}

        The content

        More content

        {{% end %}}
        ", kind};

            let options = ParseOptions::default();

            let Markdown {
                as_html,
                headings: _headings,
                links: _links,
            } = parse(&input, Some(options));

            let expected = formatdoc! {"
        <div class=\"callout {}\"><p class=\"callout-title\">An Note</p>
        <p>The content</p>
        <p>More content</p>
        </div>", css};

            assert_eq!(as_html, expected);
        }
    }

    #[test]
    fn callouts_dont_need_space_after_starting() {
        let input = indoc! {"
        {% warning An Note %}
        The content
        {% end %}
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _links,
        } = parse(&input, Some(options));

        let expected = indoc! {"
        <div class=\"callout warning\"><p class=\"callout-title\">An Note</p>
        <p>The content
        </p></div>"};

        assert_matches(&as_html, &expected);
    }

    #[test]
    fn callouts_can_have_stuff_after_it() {
        let input = indoc! {"
        {% warning An Note %}
        The content
        {% end %}

        Moar
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _links,
        } = parse(&input, Some(options));

        let expected = indoc! {"
        <div class=\"callout warning\">
            <p class=\"callout-title\">An Note</p>
            <p>The content</p>
        </div>
        <p>Moar</p>
        "};

        assert_matches(&as_html, &expected);
    }

    #[test]
    fn callouts_cannot_be_nested() {
        let input = indoc! {"
        {% warning An Warning %}
        {% info An Info %}
        The content
        {% end %}
        More content
        {% end %}
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _links,
        } = parse(&input, Some(options));

        let expected = indoc! {"
        <div class=\"callout warning\">
            <p class=\"callout-title\">An Warning</p>
            <p>
                {% info An Info %}
                The content
            </p>
        </div>
        <p>
            More content
            {% end %}
        </p>"
        };

        assert_matches(&as_html, &expected);
    }

    #[test]
    fn callouts_can_contain_images() {
        let input = indoc! {"
        {% info An Info %}
        ![an pic](/cat.jpg)
        {% end %}
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _links,
        } = parse(&input, Some(options));

        let expected = indoc! {"
        <div class=\"callout info\">
            <p class=\"callout-title\">An Info</p>
            <p><img src=\"/cat.jpg\" alt=\"an pic\"></p>
        </div>"};

        assert_matches(&as_html, &expected);
    }

    #[test]
    fn supports_github_style_markdown_checkboxes() {
        let input = indoc! {"
        * [ ] Incomplete
        * [x] Complete
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _links,
        } = parse(&input, Some(options));

        let expected = indoc! {"
        <ul>
            <li>
                <input disabled=\"\" type=\"checkbox\">
                Incomplete
            </li>
            <li>
                <input disabled=\"\" type=\"checkbox\" checked=\"\">
                Complete
            </li>
        </ul>

        "};

        assert_matches(&as_html, &expected);
    }

    #[test]
    fn does_not_allow_random_forms() {
        let input = indoc! {"
        <form>
          <label for=\"ufname\">First name:</label><br>
          <input type=\"text\" id=\"fname\" name=\"fname\"><br>
          <label for=\"lname\">Last name:</label><br>
          <input type=\"text\" id=\"lname\" name=\"lname\">
        </form>
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _links,
        } = parse(&input, Some(options));

        let expected = "";

        assert_matches(&as_html, &expected);
    }

    #[test]
    fn it_detects_math_blocks() {
        let input = indoc! {"
        ```math
        % \\f is defined as #1f(#2) using the macro
        \\f\\relax{x} = \\int_{-\\infty}^\\infty
            \\f\\hat\\xi\\,e^{2 \\pi i \\xi x}
            \\,d\\xi
        ```
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html,
            headings: _headings,
            links: _links,
        } = parse(&input, Some(options));

        // Not a regular language block
        assert!(as_html.contains("class=\"math\""));
        // Contains the contents of the match block
        assert!(as_html.contains("f is defined as"));
    }

    #[test]
    fn code_blocks_in_headings_included_in_heading_titles() {
        // https://github.com/Doctave/doctave/issues/15
        let input = indoc! {"
        # Foo `bar` baz
        "};

        let options = ParseOptions::default();

        let Markdown {
            as_html: _as_html,
            links: _links,
            headings,
        } = parse(&input, Some(options));

        let link = headings.get(0).unwrap();

        assert!(
            link.title == "Foo bar baz",
            "Incorrect title. Expected \"Foo bar baz\", got \"{}\"",
            link.title
        );
    }
}
