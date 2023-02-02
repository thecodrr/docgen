use crate::config::Config;
use crate::site_generator::SiteGenerator;
use crate::Document;
use crate::{Error, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq)]
/// Describes the mode we should build the site in, meaning
/// which assets we want to include/exclude for development.
pub enum BuildMode {
    Dev,
    Release,
}

impl std::fmt::Display for BuildMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildMode::Dev => write!(f, "dev"),
            BuildMode::Release => write!(f, "release"),
        }
    }
}

#[derive(Debug, Clone)]
/// The main handle to a site. Generic over a backend implementation.
/// Currently has InMemory and DiskBacked backends, used for serve and build respectively.
///
/// When `build` is called on this struct, the backend is populated by the
/// `SiteGenerator`.
pub struct Site<B: SiteBackend> {
    pub backend: B,
    pub config: Config,
}

impl Site<InMemorySite> {
    pub fn in_memory(config: Config) -> Site<InMemorySite> {
        Site {
            backend: InMemorySite::new(config.clone()),
            config,
        }
    }
}

impl Site<DiskBackedSite> {
    pub fn disk_backed(config: Config) -> Site<DiskBackedSite> {
        Site {
            backend: DiskBackedSite::new(config.clone()),
            config,
        }
    }
}

impl<B: SiteBackend> Site<B> {
    pub fn reset(&mut self) -> Result<()> {
        self.backend.reset()
    }

    pub fn build(&mut self, config: Config, root: &Vec<Document>) -> Result<()> {
        self.backend.build(config, root)
    }

    pub fn rebuild(&mut self, config: Config, root: &Vec<Document>) -> Result<()> {
        self.backend.reset()?;
        self.backend.build(config, root)
    }
}

pub trait SiteBackend: Send + Sync {
    fn config(&self) -> &Config;
    /// Adds the rendered content for a given path
    fn add_file(&mut self, path: &Path, content: &Vec<u8>) -> std::io::Result<()>;
    fn copy_file(&mut self, from: &Path, to: &Path) -> std::io::Result<()>;
    /// Reads the rendered output of the specified path
    fn read_path(&self, path: &Path) -> Option<Vec<u8>>;
    /// Says if we have rendered the specified file
    fn has_file(&self, path: &Path) -> bool;
    /// Clears the rendered output, and reloads the documentation from disk into memory
    fn reset(&mut self) -> Result<()>;
    /// Renders the loaded documentation into memory
    fn build(&mut self, config: Config, root: &Vec<Document>) -> Result<()>;
    fn list_files(&self) -> Vec<PathBuf>;
    fn in_memory(&self) -> bool;
}

#[derive(Debug)]
pub struct InMemorySite {
    config: Config,
    rendered: HashMap<PathBuf, Vec<u8>>,
}

impl InMemorySite {
    pub fn new(config: Config) -> Self {
        InMemorySite {
            rendered: HashMap::new(),
            config,
        }
    }
}

impl SiteBackend for InMemorySite {
    fn in_memory(&self) -> bool {
        true
    }

    fn config(&self) -> &Config {
        &self.config
    }

    fn add_file(&mut self, path: &Path, html: &Vec<u8>) -> std::io::Result<()> {
        // let mut content = self.content.write().unwrap();

        let path = path.strip_prefix(self.config.out_dir()).unwrap();

        self.rendered.insert(path.to_owned(), html.to_vec());
        Ok(())
    }

    fn copy_file(&mut self, from: &Path, to: &Path) -> std::io::Result<()> {
        let content = fs::read(from)?;
        self.add_file(to, &content)
    }

    fn read_path(&self, path: &Path) -> Option<Vec<u8>> {
        self.rendered.get(path).map(|s| s.clone())
    }

    fn has_file(&self, path: &Path) -> bool {
        self.rendered.contains_key(path)
    }

    fn reset(&mut self) -> Result<()> {
        self.rendered = HashMap::new();
        Ok(())
    }

    fn build(&mut self, config: Config, root: &Vec<Document>) -> Result<()> {
        let mut generator = SiteGenerator::new(config, root);
        generator.run(self)
    }

    fn list_files(&self) -> Vec<PathBuf> {
        self.rendered
            .keys()
            .map(|p| p.to_owned())
            .collect::<Vec<_>>()
    }
}

pub struct DiskBackedSite {
    config: Config,
}

impl DiskBackedSite {
    pub fn new(config: Config) -> Self {
        DiskBackedSite { config }
    }

    pub fn create_dir(&self) -> Result<()> {
        fs::create_dir(&self.config.out_dir()).map_err(|e| {
            Error::io(
                e,
                format!(
                    "Could not create site directory in {}",
                    self.config.out_dir().display()
                ),
            )
        })
    }

    pub fn delete_dir(&self) -> Result<()> {
        if self.config.out_dir().exists() {
            fs::remove_dir_all(&self.config.out_dir()).map_err(|e| {
                Error::io(
                    e,
                    format!(
                        "Could not clear site directory in {}",
                        self.config.out_dir().display()
                    ),
                )
            })?
        }

        Ok(())
    }
}

impl SiteBackend for DiskBackedSite {
    fn in_memory(&self) -> bool {
        false
    }

    fn config(&self) -> &Config {
        &self.config
    }

    fn add_file(&mut self, path: &Path, content: &Vec<u8>) -> std::io::Result<()> {
        fs::create_dir_all(
            self.config
                .out_dir()
                .join(path.parent().expect("Path had no parent directory")),
        )?;

        fs::write(self.config.out_dir().join(path), &content)?;

        Ok(())
    }

    fn copy_file(&mut self, from: &Path, to: &Path) -> std::io::Result<()> {
        fs::create_dir_all(
            self.config
                .out_dir()
                .join(to.parent().expect("Path had no parent directory")),
        )?;

        fs::copy(from, to).map(|_| ())
    }

    fn read_path(&self, path: &Path) -> Option<Vec<u8>> {
        if self.config.out_dir().join(path).exists() {
            Some(fs::read(self.config.out_dir().join(path)).unwrap())
        } else {
            None
        }
    }

    fn has_file(&self, path: &Path) -> bool {
        self.config.out_dir().join(path).exists()
    }

    fn reset(&mut self) -> Result<()> {
        self.delete_dir()?;
        self.create_dir()?;

        Ok(())
    }

    fn build(&mut self, config: Config, root: &Vec<Document>) -> Result<()> {
        let mut generator = SiteGenerator::new(config, root);
        generator.run(self)
    }

    fn list_files(&self) -> Vec<PathBuf> {
        walkdir::WalkDir::new(self.config.out_dir())
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.path().to_owned())
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn you_can_add_a_file_and_read_it_back() {
        let path = Path::new("/workspace/site/index.html");
        let content = "An Content";

        let config = Config::from_yaml_str(Path::new("/workspace"), "---\ntitle: Title").unwrap();

        let mut site = InMemorySite::new(config);

        site.add_file(&path, &content.into()).unwrap();

        let uri = Path::new("index.html");

        assert_eq!(site.read_path(uri).unwrap(), content.as_bytes());
        assert!(site.has_file(uri));
    }
}
