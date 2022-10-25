use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use elasticlunr::Index;
use rayon::prelude::*;
use serde::Serialize;
use walkdir::WalkDir;

use crate::config::{Config, Themes};
use crate::navigation::{Link, Navigation};
use crate::site::{BuildMode, SiteBackend};
use crate::{Directory, Document};
use crate::{Error, Result};

static INCLUDE_DIR: &str = "_include";
static HEAD_FILE: &str = "_head.html";

macro_rules! export_asset {
    ($self: expr, $filename: expr, $dir: expr, $scope: expr) => {{
        let dest_filename = crate::ASSETS_MAP.get($filename).unwrap();
        let data = crate::ASSETS
            .get_file(dest_filename)
            .expect("Failed to get")
            .contents();

        $self
            .site
            .add_file(
                &$self.config.out_dir().join($dir).join(dest_filename),
                data.to_vec(),
            )
            .map_err(|e| {
                Error::io(
                    e,
                    format!("Could not write {} to {} directory", $filename, $dir),
                )
            })?;
        Asset {
            path: format!("{}/{}", $dir, dest_filename),
            scope: $scope,
        }
    }};
}

#[derive(PartialEq)]
enum AssetScope {
    App,
    Math,
    Diagram,
    Code,
    Debug,
    Ignore,
}

struct Asset {
    scope: AssetScope,
    path: String,
}

pub struct SiteGenerator<'a, T: SiteBackend> {
    config: Config,
    root: Directory,
    site: Box<&'a T>,
    timestamp: String,
    scripts: Vec<Asset>,
    stylesheets: Vec<Asset>,
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
            scripts: vec![],
            stylesheets: vec![],
        }
    }

    pub fn run(&mut self) -> Result<()> {
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
    fn build_assets(&mut self) -> Result<()> {
        self.scripts.push(export_asset!(
            self,
            "mermaid.min.js",
            "assets",
            AssetScope::Diagram
        ));

        self.scripts.push(export_asset!(
            self,
            "elasticlunr.min.js",
            "assets",
            AssetScope::App
        ));

        if let BuildMode::Dev = self.config.build_mode() {
            // Livereload only in debug mode
            self.scripts.push(export_asset!(
                self,
                "livereload.min.js",
                "assets",
                AssetScope::Debug
            ));
        }

        self.scripts.push(export_asset!(
            self,
            "katex.min.js",
            "assets",
            AssetScope::Math
        ));

        self.scripts
            .push(export_asset!(self, "app.js", "assets", AssetScope::App));

        // Add fonts
        for font in crate::ASSETS
            .get_dir("fonts")
            .unwrap()
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
                        .join("fonts")
                        .join(font.path().file_name().unwrap()),
                    Vec::from(font.contents()),
                )
                .map_err(|e| Error::io(e, "Could not write katex fonts to assets directory"))?;
        }

        self.stylesheets.push(export_asset!(
            self,
            "normalize.css",
            "assets",
            AssetScope::App
        ));

        self.stylesheets.push(export_asset!(
            self,
            "katex.min.css",
            "assets",
            AssetScope::Math
        ));

        self.stylesheets.push(export_asset!(
            self,
            "syntect_light_theme.css",
            "assets",
            AssetScope::Code
        ));

        self.stylesheets.push(export_asset!(
            self,
            "syntect_dark_theme.css",
            "assets",
            AssetScope::Ignore
        ));

        self.stylesheets
            .push(export_asset!(self, "style.css", "assets", AssetScope::App));

        // let mut data = serde_json::Map::new();
        // data.insert(
        //     "theme_main".to_string(),
        //     serde_json::Value::String(self.config.main_color().to_css_string()),
        // );
        // data.insert(
        //     "theme_main_dark".to_string(),
        //     serde_json::Value::String(self.config.main_color_dark().to_css_string()),
        // );

        // let mut out = Vec::new();

        // crate::HANDLEBARS
        //     .render_to_write("style.css", &data, &mut out)
        //     .map_err(|e| Error::handlebars(e, "Could not write custom style sheet"))?;

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
                    footer: self.build_footer(&doc),
                    header: self.build_header(&doc),
                    themes: self.config.themes(),

                    syntect_dark_theme: crate::ASSETS_MAP
                        .get("syntect_dark_theme.css")
                        .unwrap()
                        .to_string(),
                    syntect_light_theme: crate::ASSETS_MAP
                        .get("syntect_light_theme.css")
                        .unwrap()
                        .to_string(),
                };

                let mut out = Vec::new();

                crate::HANDLEBARS
                    .render_to_write("page", &data, &mut out)
                    .map_err(|e| Error::handlebars(e, "Could not render template"))?;

                if let BuildMode::Release = self.config.build_mode() {
                    out = minify_html::minify(
                        &mut out,
                        &minify_html::Cfg {
                            keep_closing_tags: true,
                            keep_comments: false,
                            keep_html_and_head_opening_tags: true,
                            minify_css: true,
                            minify_js: true,
                            remove_processing_instructions: true,
                            remove_bangs: true,
                            keep_spaces_between_attributes: false,
                            do_not_minify_doctype: false,
                            ensure_spec_compliant_unquoted_attribute_values: false,
                        },
                    );
                }

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
        let mut index = Index::new(&["title", "uri", "body", "preview"], Some(vec!["body"]));

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
                    doc.preview(),
                ],
            );
        }
        for dir in &root.dirs {
            self.build_search_index_for_dir(&dir, index);
        }
    }

    fn build_header(&self, doc: &Document) -> String {
        compile_assets(&self.stylesheets, doc, &|asset: &Asset| {
            format!(
                "<link rel=\"stylesheet\" type=\"text/css\" href=\"{}{}\">",
                self.config.base_path(),
                asset.path
            )
        })
    }

    fn build_footer(&self, doc: &Document) -> String {
        compile_assets(&self.scripts, doc, &|asset: &Asset| {
            format!(
                "<script async defer type=\"text/javascript\" src=\"{}{}\"></script>",
                self.config.base_path(),
                asset.path
            )
        })
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
    pub header: String,
    pub footer: String,
    pub themes: &'a Themes,

    pub syntect_light_theme: String,
    pub syntect_dark_theme: String,
}

fn compile_assets(
    assets: &Vec<Asset>,
    doc: &Document,
    generate_html: &dyn Fn(&Asset) -> String,
) -> String {
    assets
        .into_iter()
        .filter_map(|script| {
            let include = match script.scope {
                AssetScope::Debug | AssetScope::App => true,
                AssetScope::Code => doc.markdown.blocks.contains("code"),
                AssetScope::Diagram => doc.markdown.blocks.contains("diagram"),
                AssetScope::Math => doc.markdown.blocks.contains("math"),
                _ => false,
            };

            if include {
                Some(script)
            } else {
                None
            }
        })
        .map(|asset| generate_html(asset))
        .collect::<Vec<String>>()
        .join("\n")
}
