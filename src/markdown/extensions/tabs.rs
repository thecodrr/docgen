use pulldown_cmark::{CowStr, Event, LinkType, Tag};

use crate::markdown::extension::{Extension, Output};

#[derive(Debug, Clone)]
pub struct Tab {
    id: String,
    title: String,
    is_active: bool,
}

pub struct TabGroup {
    index: usize,
    tabs: Vec<Tab>,
}

pub struct Tabs {
    pub current_tabgroup: Option<TabGroup>,
    pub current_tab: Option<Tab>,
}

impl Extension for Tabs {
    fn process_event<'a>(
        &mut self,
        events: &mut Vec<Event<'a>>,
        event: &Event<'a>,
    ) -> (Option<Vec<Output<'a>>>, bool) {
        match event {
            Event::Start(Tag::Link(link_type, url, _title)) => {
                if *link_type == LinkType::Inline && url.starts_with("#/tab/") {
                    let mut url_parts = url.split("/").skip(2);
                    let tab_id = url_parts.next();

                    if tab_id.is_none() {
                        return (None, false);
                    }

                    let mut output: Vec<Output> = vec![];
                    if self.current_tabgroup.is_none() {
                        self.current_tabgroup = Some(TabGroup {
                            index: events.len(),
                            tabs: vec![],
                        });

                        output.push(Output::Event(html!("<div class=\"tabgroup\">")));
                        output.push(Output::Event(html!("<tabstrip/>")));
                    }

                    let is_active = self
                        .current_tabgroup
                        .as_ref()
                        .map_or(false, |f| f.tabs.len() == 0);

                    if !is_active {
                        output.push(Output::Event(html!("</div>")));
                    }

                    self.current_tab = Some(Tab {
                        id: tab_id.unwrap().to_string(),
                        title: String::new(),
                        is_active,
                    });

                    output.push(Output::Event(html!(
                        "<div class=\"tab-panel {}\" data-tab-id=\"{}\">",
                        if is_active { "active" } else { "" },
                        tab_id.unwrap()
                    )));

                    return (Some(output), true);
                }
            }
            Event::Rule => {
                if self.current_tabgroup.is_some() {
                    self.current_tab = None;
                    return (
                        Some(close_tabgroup(
                            events,
                            &mut self.current_tabgroup.take().unwrap(),
                        )),
                        true,
                    );
                }
            }
            Event::End(Tag::Heading(_)) => {
                if self.current_tabgroup.is_some() && self.current_tab.is_some() {
                    self.current_tabgroup
                        .as_mut()
                        .unwrap()
                        .tabs
                        .push(self.current_tab.clone().unwrap());
                    self.current_tab = None;
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
                    return (None, true);
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(tab) = &mut self.current_tab {
                    tab.title.push_str(&text);
                }
            }
            _ => {}
        }
        (None, false)
    }

    fn end_of_doc<'a>(&mut self, events: &mut Vec<Event<'a>>) -> Option<Vec<Output<'a>>> {
        if self.current_tabgroup.is_some() {
            self.current_tab = None;
            return Some(close_tabgroup(
                events,
                &mut self.current_tabgroup.take().unwrap(),
            ));
        }
        None
    }
}

fn close_tabgroup<'a>(events: &mut Vec<Event>, tabgroup: &mut TabGroup) -> Vec<Output<'a>> {
    let mut output: Vec<Output> = vec![];
    output.push(Output::Event(html!("</div>")));

    let idx = tabgroup.index + 1;
    let mut tablist = Vec::new();

    tablist.push(html!("<ul class=\"tab-list\" role=\"tablist\">"));
    tabgroup.tabs.iter().for_each(|tab| {
        tablist.push(html!("<li class= role=\"presentation\">"));
        tablist.push(html!(
            "<label class=\"{}\" id=\"{}\" title=\"{}\" role=\"tab\">{}</label>",
            if tab.is_active { "active" } else { "" },
            tab.id,
            tab.title,
            tab.title
        ));
        tablist.push(html!("</li>"));
    });
    tablist.push(html!("</ul>"));

    events.splice(idx..idx + 1, tablist);

    output.push(Output::Event(html!("</div>")));

    output
}
