use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use bunt::termcolor::{ColorChoice, StandardStream};
use crossbeam_channel::bounded;

use crate::config::Config;
use crate::livereload_server::LivereloadServer;
use crate::preview_server::PreviewServer;
use crate::site::Site;
use crate::watcher::Watcher;
use crate::{broken_links_checker, docs_finder, Result};

pub struct ServeCommand {}

#[derive(Default)]
pub struct ServeOptions {
    pub port: Option<u16>,
}

impl ServeCommand {
    pub fn run(options: ServeOptions, config: Config) -> Result<()> {
        let mut stdout = if config.color_enabled() {
            StandardStream::stdout(ColorChoice::Auto)
        } else {
            StandardStream::stdout(ColorChoice::Never)
        };
        let root = docs_finder::find(&config);

        let site = Arc::new(Mutex::new(Site::in_memory(config.clone())));
        let c_site = Arc::clone(&site);

        bunt::writeln!(stdout, "{$bold}{$blue}Docgen | Serve{/$}{/$}")?;
        println!("Starting development server...\n");

        // Do initial build ---------------------------

        let start = Instant::now();
        site.lock().unwrap().build(config.clone(), &root).unwrap();

        if let Err(e) = broken_links_checker::check(&root, &site.lock().unwrap()) {
            bunt::writeln!(stdout, "{$bold}{$yellow}WARNING{/$}{/$}")?;
            println!("{}", e);
        }

        let duration = start.elapsed();

        // Watcher ------------------------------------

        let (watch_snd, watch_rcv) = bounded(128);
        let watcher = Watcher::new(vec![config.docs_dir().to_path_buf()], watch_snd);
        thread::Builder::new()
            .name("watcher".into())
            .spawn(move || watcher.run())
            .unwrap();

        // Live Reload --------------------------------

        let (reload_send, reload_rcv) = bounded(128);
        let livereload_server = LivereloadServer::new(config.livereload_addr(), reload_rcv);
        thread::Builder::new()
            .name("livereload".into())
            .spawn(move || livereload_server.run())
            .unwrap();

        // Preview Server -----------------------------

        let mut addr = config.addr();
        addr.set_port(options.port.unwrap_or_else(|| config.addr().port()));

        let http_server = PreviewServer::new(
            addr,
            c_site,
            config.color_enabled(),
            config.base_path().to_owned(),
        );
        thread::Builder::new()
            .name("http-server".into())
            .spawn(move || http_server.run())
            .unwrap();

        // Listen for updates on from the watcher, rebuild the site,
        // and inform the websocket listeners.

        for (path, msg) in watch_rcv {
            bunt::writeln!(stdout, "    File {$bold}{}{/$} {}.", path.display(), msg)?;

            let mut site_write = site.lock().unwrap();
            site_write.reset().unwrap();
            let start = Instant::now();
            let root = docs_finder::find(&config);
            site_write.rebuild(config.clone(), &root).unwrap();
            let duration = start.elapsed();
            drop(site_write);

            bunt::writeln!(stdout, "    Site rebuilt in {$bold}{:?}{/$}\n", duration)?;

            if let Err(e) = broken_links_checker::check(&root, &site.lock().unwrap()) {
                bunt::writeln!(stdout, "{$bold}{$yellow}WARNING{/$}{/$}")?;
                println!("{}", e);
            }

            reload_send.send(()).unwrap();
        }

        Ok(())
    }
}
