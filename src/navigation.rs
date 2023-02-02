use crate::config::{Config, DirIncludeRule, NavRule};
use crate::Document;
use serde::Serialize;

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

pub struct Navigation<'a> {
    config: &'a Config,
}

impl<'a> Navigation<'a> {
    pub fn new(config: &'a Config) -> Self {
        Navigation { config }
    }

    /// Builds a navigation tree given a root directory
    pub fn build_for(&self, docs: &[Document]) -> Vec<Link> {
        match &self.config.navigation() {
            None => self.links(docs, false),
            Some(nav) => self.customize(&nav, &self.links(docs, true)),
        }
    }

    /// Build a nested hierarchy from a flat list of documents
    ///
    /// TODO I don't like recursive algorithms. Is there a way to represent
    /// the navigation without nesting?
    pub fn links(&self, docs: &[Document], include_root_readme: bool) -> Vec<Link> {
        let base_path = self.config.base_path();
        // This algorithm starts from bottom up and collects all the documents
        // under a specific subdirectory and stores it temporarily inside a
        // vector. This goes on until we reach the root of any top directory
        // wherein, we take the collected entires and add them as children to
        // the directory link.
        let mut directories = HashMap::new();
        let index_file_name = OsStr::new("index.html");
        directories.insert(String::from(base_path), vec![]);

        for doc in docs {
            let uri_path = &doc.uri_path;
            let html_path = &doc.html_path;
            let title = &doc.title;
            let parent_path = doc.parent.display().to_string();

            let is_root_readme = html_path.file_name() == Some(index_file_name);
            let is_top_most = uri_path == base_path;

            let mut link = Link {
                title: title.to_string(),
                path: uri_path.to_string(),
                children: vec![],
                index: doc.index,
            };

            if is_top_most && is_root_readme {
                if include_root_readme {
                    directories
                        .entry(parent_path)
                        .or_insert(vec![])
                        .insert(0, link);
                }
            } else if is_root_readme {
                let children = directories.entry(uri_path.to_string()).or_insert(vec![]);
                children.sort_by(|a, b| a.index.cmp(&b.index));

                link.children.append(children);
                directories.entry(parent_path).or_insert(vec![]).push(link);
            } else {
                directories.entry(parent_path).or_insert(vec![]).push(link);
            }
        }

        directories.remove(&String::from(base_path)).unwrap()
    }

    /// Customizes the navigation tree given some rules provided through the
    /// docgen.yaml config.
    ///
    /// Note that the config validates that any files/directories referenced
    /// in the rules already exist, which is why we can reasonably confidently
    /// unwrap some Nones here. The only case they would trip is if the files
    /// got removed between the validation and building these rules, which is
    /// a _very_ small window.
    ///
    /// Note that in the case where an explicit path is provided, the link is
    /// not necessarily a direct child of its parent. It could be that links
    /// under a directory actually point to a parent's sibling, or to somewhere
    /// else in the tree.
    pub fn customize(&self, rules: &[NavRule], default: &[Link]) -> Vec<Link> {
        let mut links = vec![];

        let root_path_rule = NavRule::File(PathBuf::from("/"));
        for rule in rules {
            let rule = if rule
                .is_default_readme_rule(&self.config.project_root(), &self.config.docs_dir())
            {
                // If we're building navigation for the default readme file, we should
                // use a different path as the rule will contain "/README.md", while the
                // rest of the program expects it to be "/"
                &root_path_rule
            } else {
                rule
            };

            match rule {
                NavRule::File(path) => links.push(
                    self.find_matching_link(path, &default)
                        .expect("No matching link found"),
                ),
                NavRule::Dir(path, dir_rule) => {
                    let mut index_link = self
                        .find_matching_link(path, &default)
                        .expect("No matching link found");

                    match dir_rule {
                        // Don't include any children
                        None => {
                            index_link.children.truncate(0);
                            links.push(index_link);
                        }
                        // Include all children
                        Some(DirIncludeRule::WildCard) => links.push(index_link),
                        // Include only links that match the description
                        Some(DirIncludeRule::Explicit(nested_rules)) => {
                            let children = self.customize(nested_rules, &default);
                            index_link.children = children;
                            links.push(index_link);
                        }
                    }
                }
            }
        }

        links
    }

    /// Matches a path provided in a NavRule to a Link. Recursively searches through
    /// the link children to find a match.
    fn find_matching_link(&self, path: &Path, links: &[Link]) -> Option<Link> {
        let mut without_docs_part = path.components();
        let _ = without_docs_part.next();
        let doc_path = Link::path_to_uri(without_docs_part.as_path());

        let search_result = links.iter().find(|link| {
            let link_path = link.path.strip_prefix(self.config.base_path()).unwrap();
            link_path.trim_end_matches("/") == doc_path
        });

        match search_result {
            Some(link) => Some(link.clone()),
            None => {
                let recursive_results = links
                    .iter()
                    .flat_map(|l| self.find_matching_link(path, &l.children))
                    .collect::<Vec<_>>();

                // _Should_ only be one match, if any
                return recursive_results.get(0).map(|l| l.clone());
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Link {
    pub path: String,
    pub title: String,
    pub children: Vec<Link>,
    pub index: u32,
}

impl Link {
    pub fn path_to_uri(path: &Path) -> String {
        let mut tmp = path.to_owned();

        // Default to stripping .html extensions
        if tmp.file_name() == Some(OsStr::new("index.html")) {
            tmp.pop();
            tmp = tmp.join("")
        } else {
            tmp.set_extension("");
        }

        let path_with_forward_slash = if cfg!(windows) {
            let mut p = OsString::new();

            for (i, component) in tmp.components().enumerate() {
                if i > 0 {
                    p.push("/");
                }
                p.push(component);
            }
            p
        } else {
            tmp.as_os_str().to_os_string()
        };

        path_with_forward_slash
            .to_str()
            .unwrap()
            .trim_start_matches("/")
            .to_string()
    }

    pub fn path_to_uri_with_extension(path: &Path) -> String {
        let mut tmp = path.to_owned();

        // Default to stripping .html extensions
        if tmp.file_name() == Some(OsStr::new("index.html")) {
            tmp.set_file_name("");
        }

        let path_with_forward_slash = if cfg!(windows) {
            let mut p = OsString::new();

            for (i, component) in tmp.components().enumerate() {
                if i > 0 {
                    p.push("/");
                }
                p.push(component);
            }
            p
        } else {
            tmp.as_os_str().to_os_string()
        };

        path_with_forward_slash
            .to_str()
            .unwrap()
            .trim_start_matches("/")
            .to_string()
    }
}

#[cfg(test)]
mod test {
    use insta::assert_debug_snapshot;
    use rayon::slice::ParallelSliceMut;

    use super::*;
    use std::collections::BTreeMap;
    use std::path::Path;

    use crate::docs_finder::document_sort;
    use crate::Document;

    fn page(path: &str, name: &str, base_path: Option<&str>) -> Document {
        let mut frontmatter = BTreeMap::new();
        frontmatter.insert("title".to_string(), name.to_string());

        Document::new(
            Path::new(path),
            "Not important".to_string(),
            frontmatter,
            base_path.unwrap_or("/"),
        )
    }

    fn config(yaml: Option<&str>) -> Config {
        let conf = yaml.unwrap_or("---\ntitle: My project\n");

        Config::from_yaml_str(&Path::new("project"), conf).unwrap()
    }

    #[test]
    fn path_to_uri_test() {
        #[cfg(target_os = "windows")]
        {
            assert_eq!(
                Link::path_to_uri(&Path::new("docs\\windows\\hello\\world.html")),
                String::from("docs/windows/hello/world")
            );
        }

        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(
                Link::path_to_uri(&Path::new("docs/windows/hello/world.html")),
                String::from("docs/windows/hello/world")
            );

            assert_eq!(
                Link::path_to_uri(&Path::new("/child/child_of_child/index.html")),
                String::from("child/child_of_child/")
            );

            assert_eq!(
                Link::path_to_uri(&Path::new("child/index.html")),
                String::from("child/")
            );

            // child/index.html
        }
    }

    #[test]
    fn path_to_uri_with_extension_test() {
        #[cfg(target_os = "windows")]
        {
            assert_eq!(
                Link::path_to_uri_with_extension(&Path::new("docs\\windows\\hello\\world.html")),
                String::from("docs/windows/hello/world.html")
            );
        }

        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(
                Link::path_to_uri_with_extension(&Path::new("docs/windows/hello/world.html")),
                String::from("docs/windows/hello/world.html")
            );
        }
    }

    #[test]
    fn basic_navigation() {
        let config = config(None);
        let mut docs = vec![
            page("README.md", "Getting Started", None),
            page("child/child_of_child/README.md", "Child of Child", None),
            page("child/README.md", "Nested Root", None),
            page("1.one.md", "One", None),
            page("2.two.md", "Two", None),
            page("4.four.md", "Four", None),
            page("child/3.three.md", "Three", None),
            page("child/5.five.md", "Five", None),
            page("child/child_of_child/8.eight.md", "Eight", None),
            page("child/child_of_child/7.seven.md", "Seven", None),
            page("child/child_of_child/ASASSA.md", "Child of Child", None),
            page("child/child_of_child/6.six.md", "Six", None),
        ];
        docs.par_sort_by(document_sort);

        insta::with_settings!({
            description => "Basic navigation",
            omit_expression => true // do not include the default expression
        }, {
            let navigation = Navigation::new(&config);
            let result = navigation.build_for(&docs);
            assert_debug_snapshot!(result);
        });
    }

    #[test]
    fn sorting_alphanumerically() {
        let config = config(None);
        let mut docs = vec![
            page("README.md", "Getting Started", None),
            page("001.md", "bb", None),
            page("002.md", "11", None),
            page("child/README.md", "Index", None),
            page("child/001.md", "BB", None),
            page("child/002.md", "22", None),
            page("child/003.md", "AA", None),
            page("child/004.md", "11", None),
            page("child2/README.md", "Index", None),
            page("child2/001.md", "123", None),
            page("child2/002.md", "aa", None),
            page("child2/003.md", "cc", None),
            page("child2/004.md", "bb", None),
        ];
        docs.par_sort_by(document_sort);

        insta::with_settings!({
            description => "Sort alphanumerically",
            omit_expression => true // do not include the default expression
        }, {
            let navigation = Navigation::new(&config);
            let result = navigation.build_for(&docs);
            assert_debug_snapshot!(result);
        });
    }

    #[test]
    fn manual_menu_simple() {
        let mut docs = vec![
            page("README.md", "Getting Started", None),
            page("one.md", "One", None),
            page("two.md", "Two", None),
            page("child/README.md", "Nested Root", None),
            page("child/three.md", "Three", None),
        ];
        docs.par_sort_by(document_sort);

        let rules = vec![
            NavRule::File(PathBuf::from("docs/one.md")),
            NavRule::Dir(PathBuf::from("docs/child"), Some(DirIncludeRule::WildCard)),
        ];

        insta::with_settings!({
            description => "Manual menu simple",
            omit_expression => true // do not include the default expression
        }, {
            let config = config(None);
            let navigation = Navigation::new(&config);
            let links = navigation.build_for(&docs);
            let result = navigation.customize(&rules, &links);
            assert_debug_snapshot!(result);
        });
    }

    #[test]
    fn manual_menu_nested() {
        let mut docs = vec![
            page("README.md", "Getting Started", None),
            page("one.md", "One", None),
            page("two.md", "Two", None),
            page("child/README.md", "Nested Root", None),
            page("child/three.md", "Three", None),
            page("child/nested/README.md", "Nested Root", None),
            page("child/nested/four.md", "Four", None),
        ];
        docs.par_sort_by(document_sort);

        let rules = vec![
            NavRule::File(PathBuf::from("docs").join("one.md")),
            NavRule::Dir(
                PathBuf::from("docs").join("child"),
                Some(DirIncludeRule::Explicit(vec![NavRule::Dir(
                    PathBuf::from("docs").join("child").join("nested"),
                    Some(DirIncludeRule::Explicit(vec![NavRule::File(
                        PathBuf::from("docs")
                            .join("child")
                            .join("nested")
                            .join("four.md"),
                    )])),
                )])),
            ),
        ];

        insta::with_settings!({
            description => "Manual menu nested",
            omit_expression => true // do not include the default expression
        }, {
            let config = config(None);
            let navigation = Navigation::new(&config);
            let links = navigation.build_for(&docs);
            let result = navigation.customize(&rules, &links);
            assert_debug_snapshot!(result);
        });
    }

    #[test]
    fn manual_menu_file_from_nested_directory() {
        let mut docs = vec![
            page("README.md", "Getting Started", None),
            page("child/README.md", "Nested Root", None),
            page("child/three.md", "Three", None),
        ];
        docs.par_sort_by(document_sort);

        let rules = vec![NavRule::File(
            PathBuf::from("docs").join("child").join("three.md"),
        )];

        insta::with_settings!({
            description => "Manual menu file from nested directory",
            omit_expression => true // do not include the default expression
        }, {
            let config = config(None);
            let navigation = Navigation::new(&config);
            let links = navigation.build_for(&docs);
            let result = navigation.customize(&rules, &links);
            assert_debug_snapshot!(result);
        });
    }

    #[test]
    fn manual_menu_file_from_parent_directory() {
        let mut docs = vec![
            page("README.md", "Getting Started", None),
            page("one.md", "One", None),
            page("child/README.md", "Nested Root", None),
        ];
        docs.par_sort_by(document_sort);

        let rules = vec![NavRule::Dir(
            PathBuf::from("docs").join("child"),
            Some(DirIncludeRule::Explicit(vec![NavRule::File(
                PathBuf::from("docs").join("one.md"),
            )])),
        )];

        insta::with_settings!({
            description => "Manual menu file from parent directory",
            omit_expression => true // do not include the default expression
        }, {
            let config = config(None);
            let navigation = Navigation::new(&config);
            let links = navigation.build_for(&docs);
            let result = navigation.customize(&rules, &links);
            assert_debug_snapshot!(result);
        });
    }

    #[test]
    fn build_with_base_path() {
        let config = config(Some(indoc! {"
        ---
        title: Not in the root
        base_path: /example
        "}));

        let mut docs = vec![
            page("README.md", "Getting Started", Some(config.base_path())),
            page("one.md", "One", Some(config.base_path())),
            page("two.md", "Two", Some(config.base_path())),
            page("child/README.md", "Nested Root", Some(config.base_path())),
            page("child/three.md", "Three", Some(config.base_path())),
        ];
        docs.par_sort_by(document_sort);

        insta::with_settings!({
            description => "With base path",
            omit_expression => true // do not include the default expression
        }, {
            let navigation = Navigation::new(&config);
            let result = navigation.build_for(&docs);
            assert_debug_snapshot!(result);
        });
    }
}
