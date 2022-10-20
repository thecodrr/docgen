use std::{collections::HashMap, path::PathBuf};

use pulldown_cmark::{CowStr, Event, LinkType, Tag};
use url::{ParseError, Url};

use crate::markdown::extension::{Extension, Output};

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

pub struct LinkRewriter {
    pub url_root: String,
    pub link_rewrite_rules: HashMap<String, String>,
    pub url_params: Vec<(String, String)>,
    pub current_link: Option<Link>,
}

impl Extension for LinkRewriter {
    fn process_event<'a>(
        &mut self,
        _events: &mut Vec<Event<'a>>,
        event: &Event<'a>,
    ) -> (Option<Vec<Output<'a>>>, bool) {
        match event.to_owned() {
            Event::Start(Tag::Image(link_type, url, title)) => {
                let url = self.rewrite_link(url);
                return (
                    Some(vec![Output::Event(Event::Start(Tag::Image(
                        link_type,
                        CowStr::from(url),
                        title,
                    )))]),
                    true,
                );
            }
            Event::Start(Tag::Link(link_type, url, title)) => {
                let rewritten_url = self.rewrite_link(url);
                let url = if !self.url_params.is_empty() && is_in_local_domain(&rewritten_url) {
                    append_parameters(rewritten_url, &self.url_params)
                } else {
                    rewritten_url
                };
                let str_url = url.to_owned();

                if link_type == LinkType::Inline {
                    if let Ok(valid_url) = Url::parse(&url)
                        .map(|u| UrlType::Remote(u))
                        .or_else(|e| match e {
                            ParseError::EmptyHost | ParseError::RelativeUrlWithoutBase => {
                                Ok(UrlType::Local(PathBuf::from(url)))
                            }
                            e => Err(e),
                        })
                        .map_err(|l| l)
                    {
                        self.current_link = Some(Link {
                            title: title.clone().to_string(),
                            url: valid_url,
                        });
                    }
                }

                return (
                    Some(vec![Output::Event(Event::Start(Tag::Link(
                        link_type,
                        CowStr::from(str_url),
                        title,
                    )))]),
                    true,
                );
            }
            Event::End(Tag::Link(link_type, url, title)) => {
                let mut output: Vec<Output> = vec![];

                if self.current_link.is_some() {
                    output.push(Output::Link(self.current_link.take().unwrap()));
                }

                output.push(Output::Event(Event::End(Tag::Link(link_type, url, title))));

                return (Some(output), true);
            }
            Event::Text(text) => {
                if let Some(link) = &mut self.current_link {
                    link.title.push_str(&text);
                }
            }
            _ => {}
        }
        (None, false)
    }
}

impl LinkRewriter {
    /// Rewrites the link by either setting a different root path, or by
    /// swapping the whole URL if there is a matching rule in the rewrite
    /// rules.
    fn rewrite_link(&self, url: CowStr) -> String {
        if let Some(matching_link) = self.link_rewrite_rules.get(&url.clone().into_string()) {
            matching_link.to_owned()
        } else if url.starts_with("/") {
            format!("{}{}", self.url_root.trim_end_matches('/'), url)
        } else {
            url.to_string()
        }
    }
}

fn append_parameters<'a>(url: String, url_params: &'a Vec<(String, String)>) -> String {
    let mut appended = url;
    appended.push_str("?");

    let mut position = 0;
    let length = url_params.len();

    for (key, value) in url_params {
        appended.push_str(key);
        appended.push_str("=");
        appended.push_str(value);

        position += 1;
        if position != length {
            appended.push_str("&");
        }
    }

    appended
}

fn is_in_local_domain(url_string: &str) -> bool {
    match Url::parse(url_string) {
        Ok(url) => url.host().is_none(),
        Err(url::ParseError::RelativeUrlWithoutBase) => true,
        Err(url::ParseError::EmptyHost) => true,
        Err(_) => false,
    }
}
