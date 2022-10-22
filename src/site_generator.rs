use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use elasticlunr::Index;
use once_cell::sync::OnceCell;
use rayon::prelude::*;
use serde::Serialize;
use syntect::highlighting::ThemeSet;
use syntect::html::{css_for_theme_with_class_style, ClassStyle};
use walkdir::WalkDir;

use crate::config::Config;
use crate::navigation::{Link, Navigation};
use crate::site::{BuildMode, SiteBackend};
use crate::Directory;
use crate::{Error, Result};

static INCLUDE_DIR: &str = "_include";
static HEAD_FILE: &str = "_head.html";
static THEME_SET: OnceCell<ThemeSet> = OnceCell::new();

macro_rules! export_file {
    ($self: expr, $data: expr, $filename: expr, $dir: expr) => {
        $self
            .site
            .add_file(&$self.config.out_dir().join($dir).join($filename), $data)
            .map_err(|e| {
                Error::io(
                    e,
                    format!("Could not write {} to {} directory", $filename, $dir),
                )
            })?;
    };
}

pub struct SiteGenerator<'a, T: SiteBackend> {
    config: Config,
    root: Directory,
    site: Box<&'a T>,
    timestamp: String,
}

impl<'a, T: SiteBackend> SiteGenerator<'a, T> {
    pub fn new(site: &'a T) -> Self {
        let start = SystemTime::now();

        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        SiteGenerator {
            root: site.root(),
            site: Box::new(site),
            config: site.config().clone(),
            timestamp: format!("{}", since_the_epoch.as_secs()),
        }
    }

    pub fn run(&self) -> Result<()> {
        let nav_builder = Navigation::new(&self.config);
        let navigation = nav_builder.build_for(&self.root);

        let head_include = self.read_head_include()?;

        self.build_includes()?;
        self.build_assets()?;
        self.build_directory(&self.root, &navigation, head_include.as_deref())?;
        self.build_search_index(&self.root)?;

        Ok(())
    }

    fn read_head_include(&self) -> Result<Option<String>> {
        let custom_head = self.config.docs_dir().join(INCLUDE_DIR).join(HEAD_FILE);

        if custom_head.exists() {
            let content = fs::read_to_string(custom_head)
                .map_err(|e| Error::io(e, "Could not read custom head include file"))?;

            Ok(Some(content))
        } else {
            Ok(None)
        }
    }

    /// Copies over all custom includes from the _includes directory
    fn build_includes(&self) -> Result<()> {
        let custom_assets_dir = self.config.docs_dir().join(INCLUDE_DIR);

        for asset in WalkDir::new(&custom_assets_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter(|e| e.path().file_name() != Some(OsStr::new(HEAD_FILE)))
        {
            let stripped_path = asset
                .path()
                .strip_prefix(&custom_assets_dir)
                .expect("asset directory was not parent of found asset");

            let destination = self.config.out_dir().join(stripped_path);

            self.site.copy_file(asset.path(), &destination)?;
        }

        Ok(())
    }

    /// Builds fixed assets required by Docgen
    fn build_assets(&self) -> Result<()> {
        let ts = THEME_SET.get_or_init(|| ThemeSet::load_defaults());

        // create dark color scheme css
        let dark_theme =
            css_for_theme_with_class_style(&ts.themes["Solarized (dark)"], ClassStyle::Spaced)
                .unwrap();
        let light_theme =
            css_for_theme_with_class_style(&ts.themes["InspiredGitHub"], ClassStyle::Spaced)
                .unwrap();

        export_file!(self, crate::MERMAID_JS.into(), "mermaid.js", "assets");
        export_file!(self, crate::ELASTIC_LUNR.into(), "elasticlunr.js", "assets");

        if let BuildMode::Dev = self.config.build_mode() {
            // Livereload only in debug mode
            export_file!(self, crate::LIVERELOAD_JS.into(), "livereload.js", "assets");
        }

        export_file!(self, crate::KATEX_JS.into(), "katex.js", "assets");
        export_file!(self, crate::APP_JS.into(), "docgen-app.js", "assets");

        // Add fonts
        for font in crate::KATEX_FONTS
            .entries()
            .iter()
            .filter_map(|f| f.as_file())
        {
            self.site
                .add_file(
                    &self
                        .config
                        .out_dir()
                        .join("assets")
                        .join("katex-fonts")
                        .join(font.path().file_name().unwrap()),
                    Vec::from(font.contents()),
                )
                .map_err(|e| Error::io(e, "Could not write katex fonts to assets directory"))?;
        }

        export_file!(self, crate::NORMALIZE_CSS.into(), "normalize.css", "assets");
        export_file!(self, crate::KATEX_CSS.into(), "katex.css", "assets");
        export_file!(
            self,
            light_theme.into(),
            "syntect-theme-light.css",
            "assets"
        );
        export_file!(self, dark_theme.into(), "syntect-theme-dark.css", "assets");

        let mut data = serde_json::Map::new();
        data.insert(
            "theme_main".to_string(),
            serde_json::Value::String(self.config.main_color().to_css_string()),
        );
        data.insert(
            "theme_main_dark".to_string(),
            serde_json::Value::String(self.config.main_color_dark().to_css_string()),
        );

        let mut out = Vec::new();

        crate::HANDLEBARS
            .render_to_write("style.css", &data, &mut out)
            .map_err(|e| Error::handlebars(e, "Could not write custom style sheet"))?;

        export_file!(self, out.into(), "docgen-style.css", "assets");

        Ok(())
    }

    fn build_directory(
        &self,
        dir: &Directory,
        nav: &[Link],
        head_include: Option<&str>,
    ) -> Result<()> {
        let results: Result<Vec<()>> = dir
            .docs
            .par_iter()
            .map(|doc| {
                let page_title = if doc.uri_path() == "/" {
                    self.config.title().to_string()
                } else {
                    doc.title().to_string()
                };

                let data = TemplateData {
                    content: doc.html().to_string(),
                    headings: doc
                        .headings()
                        .iter()
                        .map(|heading| {
                            let mut map = BTreeMap::new();
                            map.insert("title", heading.title.clone());
                            map.insert("anchor", heading.anchor.clone());
                            map.insert("level", heading.level.to_string());

                            map
                        })
                        .collect::<Vec<_>>(),
                    navigation: &nav,
                    current_path: doc.uri_path(),
                    project_title: self.config.title().to_string(),
                    logo: self.config.logo().map(|l| l.to_string()),
                    build_mode: self.config.build_mode().to_string(),
                    base_path: self.config.base_path().to_owned(),
                    livereload_port: self.config.livereload_addr().port().to_string(),
                    timestamp: &self.timestamp,
                    page_title,
                    head_include,
                };

                let mut out = Vec::new();

                crate::HANDLEBARS
                    .render_to_write("page", &data, &mut out)
                    .map_err(|e| Error::handlebars(e, "Could not render template"))?;

                self.site
                    .add_file(&doc.destination(self.config.out_dir()), out.into())?;

                Ok(())
            })
            .collect();
        let _ok = results?;

        dir.dirs
            .par_iter()
            .map(|d| self.build_directory(&d, &nav, head_include))
            .collect()
    }

    fn build_search_index(&self, root: &Directory) -> Result<()> {
        let mut index = Index::new(&["title", "uri", "body"]);

        self.build_search_index_for_dir(root, &mut index);

        {
            self.site
                .add_file(
                    &self.config.out_dir().join("search_index.json"),
                    index.to_json().as_bytes().into(),
                )
                .map_err(|e| Error::io(e, "Could not create search index"))
        }
    }

    fn build_search_index_for_dir(&self, root: &Directory, index: &mut Index) {
        for doc in &root.docs {
            index.add_doc(
                &doc.id.to_string(),
                &[
                    &doc.title(),
                    &doc.uri_path().as_str(),
                    doc.markdown_section(),
                ],
            );
        }
        for dir in &root.dirs {
            self.build_search_index_for_dir(&dir, index);
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateData<'a> {
    pub content: String,
    pub headings: Vec<BTreeMap<&'static str, String>>,
    pub navigation: &'a [Link],
    pub head_include: Option<&'a str>,
    pub current_path: String,
    pub page_title: String,
    pub base_path: String,
    pub livereload_port: String,
    pub logo: Option<String>,
    pub project_title: String,
    pub build_mode: String,
    pub timestamp: &'a str,
}
