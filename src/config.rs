use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use http::Uri;
use serde::{Deserialize, Serialize};

use crate::address::get_safe_addr;
use crate::navigation::Link;
use crate::site::BuildMode;
use crate::{Error, Result};

#[derive(Debug, Clone, Deserialize)]
struct DocgenYaml {
    title: String,
    subtitle: Option<String>,
    port: Option<u16>,
    logo: Option<PathBuf>,
    navigation: Option<Vec<Navigation>>,
    footer: Option<Footer>,
    edit_root: Option<String>,
    base_path: Option<String>,
    docs_dir: Option<String>,
    base_url: Option<String>,
}

impl DocgenYaml {
    fn find(root: &Path) -> Option<PathBuf> {
        if root.join("docgen.yaml").exists() {
            Some(root.join("docgen.yaml"))
        } else if root.join("docgen.yml").exists() {
            Some(root.join("docgen.yml"))
        } else {
            None
        }
    }

    /// Runs checks that validate the values of provided in the Yaml file
    fn validate(&mut self, project_root: &Path) -> Result<()> {
        // Get the root doc path
        // We don't validate if it exists or not because the rest
        // of the application is responsible for assuming that "docs"
        // is the default, and checking whether or not the directory
        // exists.
        let docs_dir_path = self.docs_dir(project_root);

        // Validate logo exists
        if let Some(p) = &self.logo {
            let location = docs_dir_path.join("_include").join(p);
            if !location.exists() {
                return Err(Error::new(format!(
                    "Could not find logo specified in docgen.yaml at {}.\n\
                     The logo path should be relative to the _include directory.",
                    location.display()
                )));
            }
        }

        // Validate edit root
        if let Some(edit_root) = &self.edit_root {
            Uri::try_from(edit_root)
                .map_err(|x| Error::new(format!("Invalid edit root url. Error: {:?}", x)))?;
        }

        // Validate navigation paths exist
        // Validate navigation wildcards recursively
        fn validate_level(
            nav: &Navigation,
            config: &DocgenYaml,
            project_root: &Path,
        ) -> Result<()> {
            let doc_path = config.docs_dir(project_root).join(&nav.path);
            if !doc_path.exists() {
                return Err(Error::new(format!(
                    "Could not find file specified in navigation at {}. Fix the path or run docgen nav to regenerate navigation.",
                    doc_path.display()
                )));
            }

            if let Some(children) = &nav.children {
                match children {
                    NavChildren::WildCard(pattern) => {
                        if pattern != "*" {
                            return Err(Error::new(format!(
                                "Invalid pattern for navigation children. \
                                 Found '{}', expected \"*\" or a list of child pages",
                                pattern
                            )));
                        }
                    }
                    NavChildren::List(navs) => {
                        for nav in navs {
                            validate_level(nav, config, project_root)?;
                        }
                    }
                }
            }

            Ok(())
        }

        if let Some(navs) = &self.navigation {
            for nav in navs {
                validate_level(nav, self, &project_root)?;
            }
        }

        // Validate base path
        if let Some(path) = &mut self.base_path {
            let uri: Uri = path.parse().map_err(|_| {
                Error::new(format!(
                    "base_path was not valid absolute URI path. Got `{}`",
                    path
                ))
            })?;

            if !uri.path().starts_with("/") {
                return Err(Error::new(format!(
                    "Base path must be an absolute path. Got `{}`.",
                    path
                )));
            }

            if !path.ends_with("/") {
                path.push('/');
            }
        }

        Ok(())
    }

    fn docs_dir(&self, project_root: &Path) -> PathBuf {
        let to_join = match &self.docs_dir {
            Some(docs_dir) => docs_dir.clone(),
            None => "docs".to_string(),
        };

        let doc_root_path = project_root.join(to_join);
        doc_root_path
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Navigation {
    pub path: PathBuf,
    pub children: Option<NavChildren>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Footer {
    pub groups: Option<Vec<FooterGroup>>,
    pub copyright: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FooterGroup {
    pub title: String,
    pub links: Vec<FooterLink>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FooterLink {
    pub href: String,
    pub title: String,
    pub external: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum NavChildren {
    WildCard(String),
    List(Vec<Navigation>),
}

// static DEFAULT_THEME_COLOR: &str = "#445282";

#[derive(Debug, Clone, Serialize)]
pub struct Themes {
    pub light: HashMap<String, String>,
    pub dark: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NavRule {
    File(PathBuf),
    Dir(PathBuf, Option<DirIncludeRule>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DirIncludeRule {
    WildCard,
    Explicit(Vec<NavRule>),
}

impl NavRule {
    fn from_yaml_input(input: Vec<Navigation>) -> Vec<NavRule> {
        let mut rules = vec![];
        for item in input {
            if item.children.is_some() {
                let dir_rules = Self::build_directory_rules(&item);
                rules.push(dir_rules);
            } else {
                rules.push(NavRule::File(item.path.clone()));
            }
        }
        rules
    }

    fn build_directory_rules(dir: &Navigation) -> NavRule {
        match &dir.children {
            None => NavRule::Dir(dir.path.clone(), None),
            Some(NavChildren::WildCard(_)) => {
                NavRule::Dir(dir.path.clone(), Some(DirIncludeRule::WildCard))
            }
            Some(NavChildren::List(paths)) => NavRule::Dir(
                dir.path.clone(),
                Some(DirIncludeRule::Explicit(
                    paths
                        .iter()
                        .map(|p| {
                            if p.children.is_some() {
                                Self::build_directory_rules(p)
                            } else {
                                NavRule::File(p.path.clone())
                            }
                        })
                        .collect::<Vec<_>>(),
                )),
            ),
        }
    }

    pub fn is_default_readme_rule(&self, root_dir: &Path, docs_dir: &Path) -> bool {
        let my_path = match self {
            NavRule::File(path) => path,
            NavRule::Dir(_, _) => return false,
        };

        root_dir.join(my_path) == docs_dir.join("README.md")
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    color: bool,
    allow_failed_checks: bool,
    project_root: PathBuf,
    out_dir: PathBuf,
    docs_dir: PathBuf,
    base_path: String,
    base_url: Option<String>,
    edit_root: Option<String>,
    title: String,
    subtitle: String,
    logo: Option<String>,
    navigation: Option<Vec<NavRule>>,
    build_mode: BuildMode,
    preview_addr: SocketAddr,
    livereload_addr: SocketAddr,
    footer: Option<Footer>,
}

impl Config {
    pub fn load(project_root: &Path, skip_validation: bool) -> Result<Self> {
        let path = DocgenYaml::find(&project_root)
            .ok_or(Error::new("Could not find docgen.yaml in project"))?;

        let yaml =
            fs::read_to_string(path).map_err(|_| Error::new("Could not read docgen.yaml file"))?;

        Config::from_yaml_str(project_root, &yaml, skip_validation)
    }

    pub fn from_yaml_str(project_root: &Path, yaml: &str, skip_validation: bool) -> Result<Self> {
        let mut docgen_yaml: DocgenYaml = serde_yaml::from_str(yaml)
            .map_err(|e| Error::yaml(e, "Could not parse docgen.yaml"))?;

        if !skip_validation {
            docgen_yaml.validate(project_root)?;
        }

        let preview_addr = get_safe_addr("127.0.0.1", docgen_yaml.port.unwrap_or_else(|| 4001))
            .expect("Failed to get address for preview server.");
        let livereload_addr = get_safe_addr("127.0.0.1", 35729)
            .expect("Failed to get address for live reload server.");

        let config = Config {
            color: true,
            allow_failed_checks: false,
            project_root: project_root.to_path_buf(),
            out_dir: project_root.join("site"),
            docs_dir: docgen_yaml.docs_dir(project_root),
            base_path: docgen_yaml.base_path.unwrap_or(String::from("/")),
            title: docgen_yaml.title,
            subtitle: docgen_yaml.subtitle.unwrap_or(String::from("DOCS")),
            edit_root: docgen_yaml.edit_root,
            footer: docgen_yaml.footer,
            logo: docgen_yaml
                .logo
                .map(|p| Link::path_to_uri_with_extension(&p))
                .map(|p| p.as_str().trim_start_matches("/").to_owned()),
            navigation: docgen_yaml.navigation.map(|n| NavRule::from_yaml_input(n)),
            preview_addr,
            livereload_addr,
            build_mode: BuildMode::Dev,
            base_url: docgen_yaml.base_url,
        };

        Ok(config)
    }

    /// The title of the project
    pub fn base_url(&self) -> &Option<String> {
        &self.base_url
    }

    /// The title of the project
    pub fn footer(&self) -> &Option<Footer> {
        &self.footer
    }

    /// The title of the project
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The title of the project
    pub fn subtitle(&self) -> &str {
        &self.subtitle
    }

    /// The root directory of the project - the folder containing the docgen.yaml file.
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// The directory the HTML will get built into
    pub fn out_dir(&self) -> &Path {
        &self.out_dir
    }

    /// The directory that contains all the Markdown documentation
    pub fn docs_dir(&self) -> &Path {
        &self.docs_dir
    }

    /// The directory that contains all the Markdown documentation
    #[inline]
    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// Rules that set the site navigation structure
    pub fn navigation(&self) -> Option<&[NavRule]> {
        self.navigation.as_deref()
    }

    /// Port to serve the development server on
    pub fn addr(&self) -> SocketAddr {
        self.preview_addr
    }

    /// Port to serve the livereload server on
    pub fn livereload_addr(&self) -> SocketAddr {
        self.livereload_addr
    }

    pub fn color_enabled(&self) -> bool {
        self.color
    }

    pub fn allow_failed_checks(&self) -> bool {
        self.allow_failed_checks
    }

    pub fn disable_colors(&mut self) {
        self.color = false
    }

    pub fn set_allow_failed_checks(&mut self) {
        self.allow_failed_checks = true
    }

    pub fn build_mode(&self) -> BuildMode {
        self.build_mode
    }

    pub fn set_build_mode(&mut self, mode: BuildMode) {
        self.build_mode = mode;
    }

    /// URI path to a logo that will show up at the top left next to the title
    pub fn logo(&self) -> Option<&str> {
        self.logo.as_deref()
    }

    /// URI path to a logo that will show up at the top left next to the title
    pub fn build_edit_link(&self, doc_path: &PathBuf) -> Option<String> {
        if let Some(edit_root) = &self.edit_root {
            return Some(
                Path::new(edit_root)
                    .join(self.docs_dir.file_name().unwrap())
                    .join(doc_path)
                    .as_os_str()
                    .to_string_lossy()
                    .to_string(),
            );
        }
        None
    }
}

pub fn project_root() -> Option<PathBuf> {
    let mut current_dir = std::env::current_dir().expect("Unable to determine current directory");

    loop {
        // If we are in the root dir, just return it
        if current_dir.join("docgen.yaml").exists() || current_dir.join("docgen.yml").exists() {
            return Some(current_dir);
        }

        if let Some(parent) = current_dir.parent() {
            current_dir = parent.to_path_buf();
        } else {
            return None;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    extern crate indoc;

    #[test]
    fn validate_logo() {
        let yaml = indoc! {"
            ---
            title: The Title
            logo: i-do-not-exist.png
        "};

        let error = Config::from_yaml_str(Path::new(""), yaml, false).unwrap_err();

        assert!(
            format!("{}", error).contains("Could not find logo specified in docgen.yaml"),
            "Error message was: {}",
            error
        );
    }

    #[test]
    fn validate_base_path() {
        let yaml = indoc! {"
            ---
            title: The Title
            base_path: not/absolute
        "};

        let error = Config::from_yaml_str(Path::new(""), yaml, false).unwrap_err();

        println!("{:?}", error);

        assert!(
            format!("{}", error)
                .contains("base_path was not valid absolute URI path. Got `not/absolute`"),
            "Got incorrect error message: {}",
            error
        );
    }

    #[test]
    fn validate_base_path_ends_with_slash() {
        let yaml = indoc! {"
            ---
            title: The Title
            base_path: /docs
        "};

        let config = Config::from_yaml_str(Path::new(""), yaml, false).unwrap();

        assert_eq!(config.base_path(), "/docs/");
    }

    #[test]
    fn validate_default_base_path() {
        let yaml = indoc! {"
            ---
            title: The Title
        "};

        let config = Config::from_yaml_str(Path::new(""), yaml, false).unwrap();

        assert_eq!(config.base_path(), "/");
    }

    #[test]
    fn validate_navigation_wildcard() {
        let yaml = indoc! {"
            ---
            title: The Title
            navigation:
              - path: docs/tutorial.md
                children: not-wildcard
        "};

        let error = Config::from_yaml_str(Path::new(""), yaml, false).unwrap_err();

        assert!(
            format!("{}", error).contains(
                "Invalid pattern for navigation children. \
                Found 'not-wildcard', expected \"*\" or a list of child pages"
            ),
            "Error message was: {}",
            error
        );
    }

    #[test]
    fn convert_navigation_input_to_rules_file() {
        let input = vec![Navigation {
            path: PathBuf::from("docs").join("README.md"),
            children: None,
        }];

        assert_eq!(
            NavRule::from_yaml_input(input),
            vec![NavRule::File(PathBuf::from("docs").join("README.md"))]
        );
    }

    #[test]
    fn convert_navigation_input_to_rules_directory_no_children() {
        let input = vec![Navigation {
            path: PathBuf::from("docs").join("features"), // TODO: Make not rely on our docs
            children: None,
        }];

        assert_eq!(
            NavRule::from_yaml_input(input),
            vec![NavRule::Dir(PathBuf::from("docs").join("features"), None)]
        );
    }

    #[test]
    fn convert_navigation_input_to_rules_directory_wildcard_children() {
        let input = vec![Navigation {
            path: PathBuf::from("docs").join("features"), // TODO: Make not rely on our docs
            children: Some(NavChildren::WildCard(String::from("*"))),
        }];

        assert_eq!(
            NavRule::from_yaml_input(input),
            vec![NavRule::Dir(
                PathBuf::from("docs").join("features"),
                Some(DirIncludeRule::WildCard)
            )]
        );
    }

    #[test]
    fn convert_navigation_input_to_rules_directory_explicit_children() {
        let input = vec![Navigation {
            path: PathBuf::from("docs").join("features"), // TODO: Make not rely on our docs
            children: Some(NavChildren::List(vec![Navigation {
                path: PathBuf::from("docs").join("features").join("markdown.md"),
                children: None,
            }])),
        }];

        assert_eq!(
            NavRule::from_yaml_input(input),
            vec![NavRule::Dir(
                PathBuf::from("docs").join("features"),
                Some(DirIncludeRule::Explicit(vec![NavRule::File(
                    PathBuf::from("docs").join("features").join("markdown.md")
                )]))
            )]
        );
    }
}
