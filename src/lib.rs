#[deny(clippy::all)]
#[cfg(test)]
#[macro_use]
extern crate indoc;

#[macro_use]
extern crate lazy_static;

pub mod address;
mod broken_links_checker;
mod build;
pub mod config;
mod docs_finder;
mod error;
mod frontmatter;
mod init;
mod livereload_server;
pub mod markdown;
mod navigation;
mod preview_server;
#[allow(dead_code, unused_variables)]
mod serve;
mod site;
mod site_generator;
mod watcher;

use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub use build::BuildCommand;
pub use config::Config;
pub use error::Error;
pub use init::InitCommand;
use markdown::extensions::toc::Heading;
use markdown::parser::{MarkdownParser, ParseOptions, ParsedMarkdown};
pub use serve::{ServeCommand, ServeOptions};
pub use site::BuildMode;

use handlebars::Handlebars;
use include_dir::{include_dir, Dir};
use navigation::Link;

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/dist/");

lazy_static! {
    pub static ref ASSETS_MAP: HashMap<String, String> = {
        let assets_map = ASSETS
            .get_file("assets_map.json")
            .unwrap()
            .contents_utf8()
            .unwrap();

        serde_json::from_str::<HashMap<String, String>>(assets_map).unwrap()
    };
    pub static ref HANDLEBARS: Handlebars<'static> = {
        let mut handlebars = Handlebars::new();

        handlebars
            .register_template_string("page", include_str!("../templates/page.html"))
            .unwrap();
        handlebars
            .register_template_string("navigation", include_str!("../templates/navigation.html"))
            .unwrap();
        handlebars
            .register_template_string(
                "nested_navigation",
                include_str!("../templates/nested_navigation.html"),
            )
            .unwrap();

        handlebars
    };
}

pub type Result<T> = std::result::Result<T, error::Error>;

#[derive(Debug, Clone)]
pub struct Directory {
    path: PathBuf,
    docs: Vec<Document>,
    dirs: Vec<Directory>,
}

impl Directory {
    fn path(&self) -> &Path {
        &self.path
    }

    fn index(&self) -> &Document {
        &self
            .docs
            .iter()
            .find(|d| d.original_file_name() == Some(OsStr::new("README.md")))
            .expect("No index file found for directory")
    }

    fn links(&self, include_root_readme: bool) -> Vec<Link> {
        let mut links = self
            .docs
            .iter()
            .map(|d| Link {
                title: d.title().to_owned(),
                path: d.uri_path(),
                children: vec![],
            })
            // Filter out the index for each sub-link, but not the default/README file
            .filter(|l| {
                l.path != self.index().uri_path()
                    || (l.path == "/".to_string() && include_root_readme)
            })
            .collect::<Vec<_>>();

        let mut children = self
            .dirs
            .iter()
            .map(|d| Link {
                title: d.index().title().to_owned(),
                path: d.index().uri_path(),
                children: d.links(include_root_readme),
            })
            .collect::<Vec<_>>();

        links.append(&mut children);
        links.sort_by(|a, b| alphanumeric_sort::compare_str(&a.title, &b.title));

        links
    }
}

use std::sync::atomic::AtomicU32;

static DOCUMENT_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone, PartialEq)]
struct Document {
    pub id: u32,
    /// The relative path in the docs folder to the file
    path: PathBuf,
    rename: Option<String>,
    raw: String,
    markdown: ParsedMarkdown,
    frontmatter: BTreeMap<String, String>,
    base_path: String,
}

impl Document {
    /// Loads a document from disk and parses it.
    ///
    /// Must be provided both the absolute path to the file, and the relative
    /// path inside the docs directory to the original file.
    fn load(absolute_path: &Path, relative_docs_path: &Path, base_path: &str) -> Self {
        let raw = fs::read_to_string(absolute_path).unwrap();
        let frontmatter =
            frontmatter::parse(&raw).expect("TODO: Print an error when frontmatter is busted");

        Document::new(relative_docs_path, raw, frontmatter, base_path)
    }

    /// Creates a new document from its raw components
    fn new(
        path: &Path,
        raw: String,
        frontmatter: BTreeMap<String, String>,
        base_path: &str,
    ) -> Self {
        let rename = if path.ends_with("README.md") {
            Some("index".to_string())
        } else {
            None
        };

        let markdown_options = {
            let mut opts = ParseOptions::default();
            opts.url_root = base_path.to_owned();
            opts
        };

        let mut parser = MarkdownParser::new(Some(markdown_options));
        let markdown = parser.parse(frontmatter::without(&raw));

        Document {
            id: DOCUMENT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            path: path.to_path_buf(),
            base_path: base_path.to_owned(),
            raw,
            markdown,
            rename,
            frontmatter,
        }
    }

    fn original_file_name(&self) -> Option<&OsStr> {
        self.path.file_name()
    }

    fn original_path(&self) -> &Path {
        &self.path
    }

    /// Destination path, given an output directory
    fn destination(&self, out: &Path) -> PathBuf {
        out.join(self.html_path())
    }

    /// The path to the HTML file on disk that will be generated
    fn html_path(&self) -> PathBuf {
        // TODO(Nik): Refactor this mess to be readable
        match self.rename {
            None => self.path.with_file_name(&format!(
                "{}.html",
                self.path.file_stem().unwrap().to_str().unwrap()
            )),
            Some(ref rename) => self.path.with_file_name(&format!("{}.html", rename)),
        }
    }

    /// The URI path to this file.
    ///
    /// E.g: /foo/bar.html => /foo/bar
    fn uri_path(&self) -> String {
        format!("{}{}", self.base_path, Link::path_to_uri(&self.html_path()))
    }

    fn markdown_section(&self) -> &str {
        frontmatter::without(&self.raw)
    }

    fn preview(&self) -> &str {
        let raw = frontmatter::without(&self.raw);
        let to = if raw.len() > 100 { 100 } else { raw.len() };
        &raw[0..to]
    }

    fn headings(&self) -> &[Heading] {
        &self.markdown.headings
    }

    fn outgoing_links(&self) -> &[markdown::extensions::link_rewriter::Link] {
        &self.markdown.links
    }

    fn html(&self) -> &str {
        &self.markdown.html
    }

    fn title(&self) -> &str {
        self.frontmatter
            .get("title")
            .map(|t| t.as_ref())
            .unwrap_or_else(|| self.path.file_stem().unwrap().to_str().unwrap())
    }
}
