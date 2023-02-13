use crate::config::Footer;
use crate::markdown::extensions::toc::Heading;
use crate::navigation::Link;
use crate::site::BuildMode;

static LIGHT_MODE_SVG_DATA: &str = "M10 2a1 1 0 011 1v1a1 1 0 11-2 0V3a1 1 0 011-1zm4 8a4 4 0 11-8 0 4 4 0 018 0zm-.464 4.95l.707.707a1 1 0 001.414-1.414l-.707-.707a1 1 0 00-1.414 1.414zm2.12-10.607a1 1 0 010 1.414l-.706.707a1 1 0 11-1.414-1.414l.707-.707a1 1 0 011.414 0zM17 11a1 1 0 100-2h-1a1 1 0 100 2h1zm-7 4a1 1 0 011 1v1a1 1 0 11-2 0v-1a1 1 0 011-1zM5.05 6.464A1 1 0 106.465 5.05l-.708-.707a1 1 0 00-1.414 1.414l.707.707zm1.414 8.486l-.707.707a1 1 0 01-1.414-1.414l.707-.707a1 1 0 011.414 1.414zM4 11a1 1 0 100-2H3a1 1 0 000 2h1z";

static DARK_MODE_SVG_DATA: &str =
    "M17.293 13.293A8 8 0 016.707 2.707a8.001 8.001 0 1010.586 10.586z";

markup::define! {
    Page<'a>(
    content: &'a String,
    headings: &'a Vec<Heading>,
    navigation: &'a String,
    custom_head: Option<&'a str>,
    page_title: &'a str,
    build_mode: BuildMode,
    init_script: &'a String,
    dev_script: &'a String,
    header: &'a String,
    footer: &'a Option<Footer>,
    head_links: String,
    foot_links: String,
    edit_link: Option<String>,
    livereload_script_path: Option<&'a str>,
    livereload_port: Option<&'a str>) {
        @markup::doctype()
        html[lang="en"] {
            head {
                meta[charset="utf-8"];

                title { @page_title }

                meta[name="description",content="Documentation for ".to_string() + page_title];

                meta[name="viewport",content="width=device-width, initial-scale=1"];

                @markup::raw(head_links)

                script {
                    {markup::raw(init_script)}
                }

                @if let Some(custom_head) = custom_head {
                    @markup::raw(custom_head)
                }
            }

            body.preload {
                label[for="menu-toggle-switch", class="menu-toggle-button"] {
                    "☰"
                }
                input[type="checkbox", id="menu-toggle-switch", value='0'];

                .page {
                    @markup::raw(header)

                    div[class="container"] {
                        div[class="sidebar-left"] {
                            @markup::raw(navigation)
                        }

                        div[class="docgen-content"] {
                            @markup::raw(content)
                        }

                        div[class="sidebar-right"] {
                            @if let Some(edit_link) = edit_link {
                                a[class="edit-link", href=edit_link] {
                                    {"Edit this page"}
                                }
                            }

                            div[class="page-nav", id="page-nav"] {
                                p[class="page-nav-header"] {
                                    {"On this page"}
                                }

                                ul {
                                    @for heading in headings.iter() {
                                        li[class=format!("page-nav-level-{}", heading.level)] {
                                            a[href=format!("#{}", heading.anchor)] {
                                                {&heading.title}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                @if let Some(footer) = footer {
                    footer {
                        div.groups {
                            @if let Some(groups) = &footer.groups {
                                @for group in groups.iter() {
                                    div {
                                        div.title {
                                            {&group.title}
                                        }

                                        ul {
                                            @for link in &group.links {
                                                li {
                                                    a[href=&link.href, target=if link.external.unwrap_or(false) { "_blank" } else { "_self"} ] {
                                                        {&link.title}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        @if let Some(copyright) = &footer.copyright {
                            div.copyright {
                                {copyright}
                            }
                        }
                    }
                }

                @markup::raw(foot_links)

                @if let BuildMode::Dev = build_mode {
                    script[id="livereloadjs", type="text/javascript", async="true", defer="true", src=livereload_script_path, {"data-port"}=livereload_port] {
                    }

                    script {
                        {markup::raw(dev_script)}
                    }
                }
            }
        }
    }


    PageHeader<'a>(logo: Option<&'a str>, base_path: &'a str, project_title: &'a str, project_subtitle: &'a str) {
        .header {
            .logo {
                @if let Some(logo) = logo {
                    a[href=base_path] {
                        img[src=format!("{}{}", base_path, logo), alt=format!("{} logo", project_title)];
                    }
                }

                h2[class="project-title"] {
                    a[href=base_path] {
                        {project_title}
                    }
                    span[class="project-subtitle"] {
                        {project_subtitle}
                    }
                }
            }

            .search {
                form[id="search-form"] {
                    input[type="text", id="search-box", autocomplete="off", placeholder="Search..."];
                    span[class="search-icon"] {
                        "S"
                    }
                    ul[id="search-results"] {}
                }
            }

            div[class="header-dummy-right"] {}
        }
    }

    SideNavigation<'a>(navigation: &'a [Link]) {
        nav[class="site-nav"] {
            ul.tree {
                @for link in navigation.iter() {
                    @NavigationLink { link: &link }
                }
            }
        }


        span[id="light-dark-mode-switch"] {
            svg[xmlns="http://www.w3.org/2000/svg", id="dark-mode-icon", viewBox="0 0 20 20", fill="currentColor"] {
                path[d=DARK_MODE_SVG_DATA]{}
            }


            svg[xmlns="http://www.w3.org/2000/svg", id="light-mode-icon", viewBox="0 0 20 20", fill="currentColor"] {
                path[{"fill-rule"}="evenodd", d=LIGHT_MODE_SVG_DATA, {"clip-rule"}="evenodd"]{}
            }
        }
    }


    NavigationLink<'a>(link: &'a Link) {

            @if link.children.len() > 0 {
                li.nested {
                    details {
                        summary {
                            span {
                                @link.title
                            }
                        }

                        ul {
                            @for link in link.children.iter() {
                                @NavigationLink { link: &link }
                            }
                        }
                    }
                }
            } else {
                li {
                    a[href={&link.path}] {
                        @link.title
                    }
                }
            }
    }
}
