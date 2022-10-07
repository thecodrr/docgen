use pulldown_cmark::{html, CodeBlockKind, CowStr, Event, LinkType, Options, Parser, Tag};
use regex::Regex;
use url::{ParseError, Url};

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::fmt;
use std::path::{Path, PathBuf};

macro_rules! html {
    ($($arg:tt)*) => {{
        Event::Html(CowStr::from(format!($($arg)*)))
    }};
}

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
    pub is_tab: bool,
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
pub struct TabGroup {
    idx: usize,
    tabs: Vec<Tab>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Tab {
    id: String,
    title: String,
    is_active: bool,
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
    pub url_params: Vec<(String, String)>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        ParseOptions {
            url_root: String::from("/"),
            link_rewrite_rules: HashMap::new(),
            url_params: vec![],
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
    let mut active_tabgroup: Option<TabGroup> = None;
    let mut current_link = None;
    let mut current_heading: Option<Heading> = None;
    let mut current_tab: Option<Tab> = None;

    let mut parser = Parser::new_ext(input, options).into_iter().peekable();

    let mut events = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::Rule => {
                close_tabgroup(&mut events, &mut active_tabgroup, &mut current_tab);
            }
            // Mermaid JS code block tranformations
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(inner))) => {
                let lang = inner.split(' ').next().unwrap();

                if lang == "mermaid" {
                    events.push(html!("<div class=\"mermaid\">\n"));
                } else if lang == "math" {
                    events.push(html!("<div class=\"math\">\n"));
                } else {
                    events.push(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(inner))));
                }
            }
            Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(inner))) => {
                let lang = inner.split(' ').next().unwrap();
                if lang == "mermaid" || lang == "math" {
                    events.push(html!("</div>"));
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
                let mut url_parts = url.split("/").skip(2);
                let tab_id = url_parts.next();

                if current_heading.is_some()
                    && link_type == LinkType::Inline
                    && url.starts_with("#/tab/")
                    && tab_id.is_some()
                {
                    let mut is_active = false;
                    if let Some(heading) = &mut current_heading {
                        heading.is_tab = true;
                        if active_tabgroup.is_none() {
                            active_tabgroup = Some(TabGroup {
                                tabs: vec![],
                                idx: events.len(),
                            });
                            events.push(html!("<div class=\"tabgroup\">"));
                            events.push(html!("!!TABS_PLACEHOLDER!!"));
                            is_active = true;
                        }
                        if current_tab.is_some() {
                            events.push(html!("</div>"));
                        }
                        current_tab = Some(Tab {
                            id: tab_id.unwrap().to_string(),
                            title: "".to_string(),
                            is_active,
                        });
                        events.push(html!(
                            "<div class=\"tab-panel {}\" data-tab-id=\"{}\">",
                            if is_active { "active" } else { "" },
                            tab_id.unwrap()
                        ));
                    }
                } else {
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
                    level,
                    anchor: String::new(),
                    title: String::new(),
                    is_tab: false,
                });

                events.push(event);
            }

            Event::End(Tag::Heading(_)) => {
                let closed_heading = current_heading.take().unwrap();

                if !closed_heading.is_tab {
                    let header_start = events
                        .iter_mut()
                        .rev()
                        .find(|tag| match tag {
                            Event::Start(Tag::Heading(_)) => true,
                            _ => false,
                        })
                        .unwrap();
                    *header_start = html!(
                        "<h{} id=\"{}\">",
                        closed_heading.level,
                        closed_heading.anchor
                    );

                    if current_tab.is_none() {
                        headings.push(closed_heading);
                    }
                    events.push(event);
                } else {
                    if let Some(tab) = &mut current_tab {
                        tab.title = closed_heading.title
                    }

                    active_tabgroup
                        .as_mut()
                        .unwrap()
                        .tabs
                        .push(current_tab.clone().unwrap());

                    events.iter_mut().rev().position(|tag| match tag {
                        Event::Start(Tag::Heading(_)) => {
                            *tag = html!("");
                            true
                        }
                        Event::Html(_) => false,
                        _ => {
                            *tag = html!("");
                            false
                        }
                    });
                }
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
                    events.push(html!("</div>"));
                    if Some(&Event::SoftBreak) == parser.peek() {
                        events.push(Event::Start(Tag::Paragraph));
                    }
                } else if active_callout.is_none() && is_callout_start(&text) {
                    if let Some(callout) = parse_callout(&text) {
                        active_callout = Some(callout.clone());

                        if let Some(title) = callout.title {
                            events.push(html!(
                                "<div class=\"callout {}\"><p class=\"callout-title\">{}</p>",
                                callout.kind,
                                title
                            ));
                        } else {
                            events.push(html!("<div class=\"callout {}\">", callout.kind));
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

    close_tabgroup(&mut events, &mut active_tabgroup, &mut current_tab);

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
    // Tabs specific
    allowed_div_classes.insert("tabgroup");
    allowed_div_classes.insert("tab");
    allowed_div_classes.insert("tab-panel");
    allowed_div_classes.insert("active");

    let mut allowed_label_classes = HashSet::new();
    allowed_label_classes.insert("active");

    let mut allowed_classes = HashMap::new();
    allowed_classes.insert("div", allowed_div_classes);
    allowed_classes.insert("label", allowed_label_classes);

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
        .add_tags(&["label"])
        .add_tag_attributes("label", &["id", "role"])
        .add_tags(&["input"])
        .add_tag_attribute_values("input", "disabled", &[""])
        .add_tag_attribute_values("input", "type", &["checkbox"])
        .add_tag_attribute_values("input", "checked", &[""])
        .allowed_classes(allowed_classes)
        .add_tag_attributes("div", &["id", "data-tab-title", "data-tab-id"])
        .add_tag_attributes("ul", &["role"])
        .add_tag_attributes("li", &["role"])
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

fn close_tabgroup(events: &mut Vec<Event>, tabgroup: &mut Option<TabGroup>, tab: &mut Option<Tab>) {
    if let Some(group) = tabgroup {
        if tab.is_some() {
            events.push(html!("</div>"));
        }

        let idx = group.idx + 1;
        let mut tablist = Vec::new();

        tablist.push(html!("<ul class=\"tab-list\" role=\"tablist\">"));
        group.tabs.iter().for_each(|tab| {
            tablist.push(html!("<li class= role=\"presentation\">"));
            tablist.push(html!(
                "<label class=\"{}\" id=\"{}\" title=\"{}\" role=\"tab\">{}</a>",
                if tab.is_active { "active" } else { "" },
                tab.id,
                tab.title,
                tab.title
            ));
            tablist.push(html!("</li>"));
        });
        tablist.push(html!("</ul>"));

        events.splice(idx..idx + 1, tablist);

        *tabgroup = None;
        *tab = None;
        events.push(html!("</div>"));
    }
}

// fn generate_tabgroup_id() -> String {
//     thread_rng()
//         .sample_iter(&Alphanumeric)
//         .take(30)
//         .map(char::from)
//         .collect()
// }

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
    event == Some(&html!("</div>"))
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
