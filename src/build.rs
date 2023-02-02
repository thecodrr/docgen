use std::time::Instant;

use bunt::termcolor::{ColorChoice, StandardStream};

use crate::config::Config;
use crate::site::{BuildMode, Site};
use crate::Result;

pub struct BuildCommand {}

impl BuildCommand {
    pub fn run(config: Config) -> Result<()> {
        let mut stdout = if config.color_enabled() {
            StandardStream::stdout(ColorChoice::Auto)
        } else {
            StandardStream::stdout(ColorChoice::Never)
        };

        let root = crate::docs_finder::find(&config);
        let mut site = Site::disk_backed(config.clone());

        let target_dir = config.out_dir();

        bunt::writeln!(stdout, "{$bold}{$blue}Docgen | Build{/$}{/$}")?;

        if let BuildMode::Release = config.build_mode() {
            bunt::writeln!(
                stdout,
                "Building site into {$bold}{}{/$} in {$bold}release mode{/$}\n",
                target_dir.display(),
            )?;
        } else {
            bunt::writeln!(
                stdout,
                "Building site into {$bold}{}{/$}\n",
                target_dir.display()
            )?;
        }

        let start = Instant::now();
        let result = site.build(config.clone(), &root);
        let duration = start.elapsed();

        if result.is_ok() {
            bunt::writeln!(stdout, "Site built in {$bold}{:?}{/$}\n", duration)?;

            let dead_links_result = crate::broken_links_checker::check(&root, &site);
            if dead_links_result.is_err() && config.allow_failed_checks() {
                bunt::writeln!(stdout, "{$bold}{$yellow}WARNING{/$}{/$}")?;
                bunt::writeln!(stdout, "{}", dead_links_result.unwrap_err())?;
                Ok(())
            } else {
                dead_links_result
            }
        } else {
            result
        }
    }
}
