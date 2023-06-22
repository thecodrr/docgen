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
pub mod docs_finder;
mod error;
mod frontmatter;
mod init;
mod livereload_server;
pub mod markdown;
mod nav;
pub mod navigation;
mod page_template;
mod preview_server;
#[allow(dead_code, unused_variables)]
mod serve;
mod site;
mod site_generator;
mod watcher;

use std::collections::{BTreeMap, HashMap};
use std::fs::{self};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub use build::BuildCommand;
pub use config::Config;
pub use error::Error;
pub use init::InitCommand;
use markdown::extensions::toc::Heading;
use markdown::parser::{MarkdownParser, ParseOptions, ParsedMarkdown};
pub use nav::NavigationCommand;
pub use serve::{ServeCommand, ServeOptions};
pub use site::BuildMode;

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
}

pub type Result<T> = std::result::Result<T, error::Error>;

use std::sync::atomic::AtomicU32;

static DOCUMENT_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub index: u32,
    pub id: u32,
    /// The relative path in the docs folder to the file
    path: PathBuf,
    parent: PathBuf,

    /// The path to the HTML file on disk that will be generated
    html_path: PathBuf,

    /// The URI path to this file.
    ///
    /// E.g: /foo/bar.html => /foo/bar
    uri_path: String,

    raw: String,
    markdown: ParsedMarkdown,
    frontmatter: BTreeMap<String, String>,
    base_path: String,
    title: String,
    description: String,

    last_modified: SystemTime,
}

impl Document {
    /// Loads a document from disk and parses it.
    ///
    /// Must be provided both the absolute path to the file, and the relative
    /// path inside the docs directory to the original file.
    fn load(absolute_path: &Path, relative_docs_path: &Path, base_path: &str) -> Self {
        let raw = fs::read_to_string(absolute_path).unwrap();
        let metadata = fs::metadata(absolute_path).unwrap();
        let frontmatter =
            frontmatter::parse(&raw).expect("TODO: Print an error when frontmatter is busted");

        Document::new(
            relative_docs_path,
            raw,
            frontmatter,
            base_path,
            metadata.modified().unwrap_or_else(|_| SystemTime::now()),
        )
    }

    /// Creates a new document from its raw components
    pub fn new(
        path: &Path,
        raw: String,
        frontmatter: BTreeMap<String, String>,
        base_path: &str,
        last_modified: SystemTime,
    ) -> Self {
        let is_root = path.ends_with("README.md");
        let html_path = if is_root {
            path.with_file_name("index.html")
        } else {
            path.with_extension("html")
        };
        let uri_path = format!("{}{}", base_path, Link::path_to_uri(&html_path));

        let parent = Path::new(&uri_path)
            .parent()
            .map(|s| {
                if s.ends_with("/") {
                    s.to_path_buf()
                } else {
                    s.join("")
                }
            })
            .unwrap_or_else(|| Path::new("/").to_path_buf());

        let markdown_options = {
            let mut opts = ParseOptions::default();
            opts.url_root = base_path.to_owned();
            opts
        };

        let mut parser = MarkdownParser::new(Some(markdown_options));
        let markdown = parser.parse(frontmatter::without(&raw));

        let title = frontmatter
            .get("title")
            .map(|t| t.as_ref())
            .or_else(|| {
                if markdown.headings.len() > 0 {
                    Some(markdown.headings[0].title.as_str())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| path.file_stem().unwrap().to_str().unwrap())
            .to_string();

        let description = frontmatter
            .get("description")
            .map(|t| t.to_owned())
            .or_else(|| Some("Documentation for ".to_owned() + &title))
            .unwrap();

        Document {
            index: frontmatter
                .get("index")
                .and_then(|idx| idx.parse::<u32>().ok())
                .unwrap_or(u32::MAX),
            id: DOCUMENT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            path: path.to_path_buf(),
            base_path: base_path.to_owned(),
            raw,
            markdown,
            frontmatter,
            html_path,
            uri_path,
            description,
            title,
            parent,
            last_modified,
        }
    }

    fn src(&self) -> String {
        let is_root = self.path.ends_with("README.md");
        if is_root {
            self.path.parent().unwrap().to_string_lossy().to_string()
        } else {
            self.path.to_string_lossy().to_string()
        }
    }

    fn original_path(&self) -> &Path {
        &self.path
    }

    /// Destination path, given an output directory
    fn destination(&self, out: &Path) -> PathBuf {
        out.join(&self.html_path)
    }

    fn preview(&self) -> &String {
        &self.markdown.preview
    }

    fn headings(&self) -> &Vec<Heading> {
        &self.markdown.headings
    }

    fn outgoing_links(&self) -> &[markdown::extensions::link_rewriter::Link] {
        &self.markdown.links
    }

    fn html(&self) -> &String {
        &self.markdown.html
    }
}
