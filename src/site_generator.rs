use std::ffi::OsStr;
use std::fs;
use std::sync::mpsc::channel;
use std::time::{SystemTime, UNIX_EPOCH};

use elasticlunr::Index;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::config::Config;
use crate::navigation::{Link, Navigation};
use crate::site::{BuildMode, SiteBackend};
use crate::Document;
use crate::{Error, Result};

static INCLUDE_DIR: &str = "_include";
static HEAD_FILE: &str = "_head.html";
static LIGHT_SYNTAX_THEME_FILE: &str = "light.css";
static DARK_SYNTAX_THEME_FILE: &str = "dark.css";

lazy_static! {
    static ref DEBUG_SCRIPT: String = {
        let code = r#"document.addEventListener('load', function () {
        // Don't reset scrolling on livereload
        window.addEventListener('scroll', function () {
            localStorage.setItem('docgen-scrollPosition', window.scrollY);
        
            dragRightMenu();
        }, false);

        if (localStorage.getItem('docgen-scrollPosition') !== null)
            window.scrollTo(0, localStorage.getItem('docgen-scrollPosition'));
    
        document.getElementById('menu-toggle-switch').addEventListener('change', function (e) {
            disableScrollifMenuOpen();
        });
    }, false);"#
            .as_bytes()
            .to_vec();

        let mut minified_code = vec![];
        minify_js::minify(minify_js::TopLevelMode::Global, code, &mut minified_code).unwrap();

        String::from_utf8(minified_code).unwrap()
    };
}

#[derive(PartialEq, Debug)]
enum AssetScope {
    App,
    #[cfg(feature = "katex")]
    Math,
    Diagram,
    Code,
    Debug,
    Ignore,
}

#[derive(Debug)]
struct Asset {
    id: String,
    scope: AssetScope,
    path: String,
}

pub struct SiteGenerator<'a> {
    config: Config,
    root: &'a Vec<Document>,
    timestamp: String,
    scripts: Vec<Asset>,
    stylesheets: Vec<Asset>,
}

impl<'a> SiteGenerator<'a> {
    pub fn new(config: Config, root: &'a Vec<Document>) -> Self {
        let start = SystemTime::now();

        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        SiteGenerator {
            root,
            config,
            timestamp: format!("{}", since_the_epoch.as_secs()),
            scripts: vec![],
            stylesheets: vec![],
        }
    }

    pub fn run<T: SiteBackend>(&mut self, site: &mut T) -> Result<()> {
        let nav_builder = Navigation::new(&self.config);
        let navigation = nav_builder.build_for(&self.root);

        let head_include = self.read_head_include()?;

        self.build_includes(site)?;
        self.build_assets(site)?;
        self.build_directory(self.root, &navigation, head_include.as_deref(), site)?;
        self.build_search_index(&self.root, site)?;

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
    fn build_includes<T: SiteBackend>(&mut self, site: &mut T) -> Result<()> {
        let custom_assets_dir = self.config.docs_dir().join(INCLUDE_DIR);

        for asset in WalkDir::new(&custom_assets_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter(|e| e.path().file_name() != Some(OsStr::new(HEAD_FILE)))
            .filter(|e| e.path().file_name() != Some(OsStr::new(DARK_SYNTAX_THEME_FILE)))
            .filter(|e| e.path().file_name() != Some(OsStr::new(LIGHT_SYNTAX_THEME_FILE)))
        {
            let stripped_path = asset
                .path()
                .strip_prefix(&custom_assets_dir)
                .expect("asset directory was not parent of found asset");

            let destination = self.config.out_dir().join(stripped_path);

            site.copy_file(asset.path(), &destination)?;
        }

        Ok(())
    }

    /// Builds fixed assets required by Docgen
    fn build_assets<T: SiteBackend>(&mut self, site: &mut T) -> Result<()> {
        self.scripts
            .push(self.export_asset(site, "mermaid.min.js", "assets", AssetScope::Diagram));

        self.scripts
            .push(self.export_asset(site, "elasticlunr.min.js", "assets", AssetScope::App));

        self.scripts
            .push(self.export_asset(site, "app.js", "assets", AssetScope::App));

        self.stylesheets
            .push(self.export_asset(site, "normalize.css", "assets", AssetScope::App));

        #[cfg(feature = "katex")]
        {
            self.stylesheets.push(self.export_asset(
                site,
                "katex.min.css",
                "assets",
                AssetScope::Math,
            ));

            // Add fonts
            for font in crate::ASSETS
                .get_dir("fonts")
                .unwrap()
                .entries()
                .iter()
                .filter_map(|f| f.as_file())
            {
                let asset_path = self
                    .config
                    .out_dir()
                    .join("assets")
                    .join("fonts")
                    .join(font.path().file_name().unwrap());

                if !asset_path.exists() {
                    site.add_file(&asset_path, &Vec::from(font.contents()))
                        .map_err(|e| {
                            Error::io(e, "Could not write katex fonts to assets directory")
                        })?;
                }
            }
        }

        let custom_light_theme = self
            .config
            .docs_dir()
            .join(INCLUDE_DIR)
            .join(LIGHT_SYNTAX_THEME_FILE);

        if custom_light_theme.exists() {
            self.stylesheets.push(self.export_file(
                site,
                crate::ASSETS_MAP.get("light.css").unwrap(),
                "assets",
                fs::read(custom_light_theme)?.as_slice(),
                AssetScope::Code,
            ));
        } else {
            self.stylesheets
                .push(self.export_asset(site, "light.css", "assets", AssetScope::App));
        }

        let custom_dark_theme = self
            .config
            .docs_dir()
            .join(INCLUDE_DIR)
            .join(DARK_SYNTAX_THEME_FILE);

        if custom_dark_theme.exists() {
            self.stylesheets.push(self.export_file(
                site,
                crate::ASSETS_MAP.get("dark.css").unwrap(),
                "assets",
                fs::read(custom_dark_theme)?.as_slice(),
                AssetScope::Code,
            ));
        } else {
            self.stylesheets
                .push(self.export_asset(site, "dark.css", "assets", AssetScope::App));
        }

        self.stylesheets
            .push(self.export_asset(site, "style.css", "assets", AssetScope::App));

        Ok(())
    }

    fn build_directory<T: SiteBackend>(
        &self,
        docs: &Vec<Document>,
        nav: &[Link],
        head_include: Option<&str>,
        site: &mut T,
    ) -> Result<()> {
        let side_navigation = crate::page_template::SideNavigation { navigation: nav }.to_string();
        let header = crate::page_template::PageHeader {
            base_path: self.config.base_path(),
            logo: self.config.logo(),
            project_title: self.config.title(),
            project_subtitle: self.config.subtitle(),
        }
        .to_string();
        let init_script = self.init_script();
        let livereload_script_path = if let BuildMode::Dev = self.config.build_mode() {
            let asset = self.export_asset(site, "livereload.min.js", "assets", AssetScope::Debug);
            Some(format!("{}{}", self.config.base_path(), asset.path))
        } else {
            None
        };
        let livereload_port = if let BuildMode::Dev = self.config.build_mode() {
            Some(self.config.livereload_addr().port().to_string())
        } else {
            None
        };

        let (sender, receiver) = channel();

        docs.par_iter().for_each_with(sender, |sender, doc| {
            let page_title = if doc.uri_path == "/" {
                self.config.title()
            } else {
                &doc.title
            };

            let data = crate::page_template::Page {
                content: doc.html(),
                headings: doc.headings(),
                build_mode: self.config.build_mode(),
                page_title,

                edit_link: self.config.build_edit_link(&doc.path),

                head_links: self.build_header(&doc),
                foot_links: self.build_footer(&doc),

                footer: self.config.footer(),

                custom_head: head_include,
                header: &header,
                navigation: &side_navigation,
                init_script: &init_script,
                dev_script: &DEBUG_SCRIPT,
                livereload_script_path: livereload_script_path.as_deref(),
                livereload_port: livereload_port.as_deref(),
            }
            .to_string();

            sender
                .send((doc.destination(self.config.out_dir()), data.into_bytes()))
                .unwrap();
        });

        receiver.iter().for_each(|(dest, content)| {
            site.add_file(&dest, &content).unwrap();
        });

        Ok(())
    }

    fn build_search_index<T: SiteBackend>(&self, root: &Vec<Document>, site: &mut T) -> Result<()> {
        let mut index = Index::new(&["title", "uri", "body", "preview"], Some(vec!["body"]));

        self.build_search_index_for_dir(root, &mut index);

        {
            site.add_file(
                &self.config.out_dir().join("search_index.json"),
                &index.to_json().as_bytes().into(),
            )
            .map_err(|e| Error::io(e, "Could not create search index"))
        }
    }

    fn build_search_index_for_dir(&self, docs: &Vec<Document>, index: &mut Index) {
        for doc in docs {
            index.add_doc(
                &doc.id.to_string(),
                &[&doc.title, &doc.uri_path, doc.html(), doc.preview()],
            );
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
                "<script id=\"{}\" async defer type=\"text/javascript\" src=\"{}{}\"></script>",
                asset.id,
                self.config.base_path(),
                asset.path
            )
        })
    }

    fn init_script(&self) -> String {
        let init_script = format!(
            r#"var DOCGEN_TIMESTAMP = "{}";
    var BASE_PATH = "{}";

    document.addEventListener("DOMContentLoaded", function() {{
        const link = document.querySelector(`.site-nav a[href="${{document.location.pathname}}"]`);
        if (link) {{
            link.classList.add("active")
            const detailsElement = link.closest("details");
            if (detailsElement) {{
                detailsElement.setAttribute("open", true);
            }}
        }}
    }});

    function setColor() {{
        const color = localStorage.getItem("docgen-color");
    
        if (color === "dark") {{
          document.documentElement.classList.remove("light");
          document.documentElement.classList.add("dark");
        }} else {{
          document.documentElement.classList.remove("dark");
          document.documentElement.classList.add("light");
        }}
      }}
      
      setColor();"#,
            &self.timestamp,
            self.config.base_path(),
        )
        .as_bytes()
        .to_vec();

        let mut minified_init_script = vec![];
        minify_js::minify(
            minify_js::TopLevelMode::Global,
            init_script,
            &mut minified_init_script,
        )
        .unwrap();

        String::from_utf8(minified_init_script).unwrap()
    }

    fn export_asset<T: SiteBackend>(
        &self,
        site: &mut T,
        filename: &str,
        dir: &str,
        scope: AssetScope,
    ) -> Asset {
        let dest_filename = crate::ASSETS_MAP.get(filename).unwrap();
        let asset = Asset {
            path: format!("{}/{}", dir, dest_filename),
            scope,
            id: filename.to_string(),
        };
        let export_path = self.config.out_dir().join(dir).join(dest_filename);

        if export_path.exists() && !site.in_memory() {
            asset
        } else {
            let data = crate::ASSETS
                .get_file(dest_filename)
                .expect("Failed to get")
                .contents();

            site.add_file(&export_path, &data.to_vec())
                .map_err(|e| {
                    Error::io(
                        e,
                        format!("Could not write {} to {} directory", filename, dir),
                    )
                })
                .unwrap();

            asset
        }
    }

    fn export_file<T: SiteBackend>(
        &self,
        site: &mut T,
        filename: &str,
        dir: &str,
        data: &[u8],
        scope: AssetScope,
    ) -> Asset {
        let asset = Asset {
            path: format!("{}/{}", dir, filename),
            scope,
            id: filename.to_string(),
        };
        let export_path = self.config.out_dir().join(dir).join(filename);

        if export_path.exists() && !site.in_memory() {
            asset
        } else {
            site.add_file(&export_path, &data.to_vec())
                .map_err(|e| {
                    Error::io(
                        e,
                        format!("Could not write {} to {} directory", filename, dir),
                    )
                })
                .unwrap();

            asset
        }
    }
}

fn compile_assets(
    assets: &Vec<Asset>,
    doc: &Document,
    generate_html: &dyn Fn(&Asset) -> String,
) -> String {
    assets
        .iter()
        .filter_map(|script| {
            let include = match script.scope {
                AssetScope::Debug | AssetScope::App => true,
                AssetScope::Code => doc.markdown.blocks.contains("code"),
                AssetScope::Diagram => doc.markdown.blocks.contains("diagram"),
                #[cfg(feature = "katex")]
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
