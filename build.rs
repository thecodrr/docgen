use std::{
    collections::HashMap,
    fs::{self, create_dir, read_to_string, remove_dir_all},
    path::Path,
};

use css_minify::optimizations::{Level, Minifier};
use seahash::hash;
use syntect::{
    highlighting::ThemeSet,
    html::{css_for_theme_with_class_style, ClassStyle},
};
use walkdir::WalkDir;

// macro_rules! p {
//     ($($tokens: tt)*) => {
//         println!("cargo:warning={}", format!($($tokens)*))
//     }
// }

fn main() {
    let dist_dir = Path::new("./dist");
    let mut assets_map = HashMap::new();

    if dist_dir.exists() {
        remove_dir_all(dist_dir).unwrap();
    }
    create_dir(dist_dir).unwrap();

    compile_assets(&mut assets_map, &dist_dir);
    compile_syntect_styles(&mut assets_map, &dist_dir);
    compile_fonts(&dist_dir);

    let assets_map_json = serde_json::to_string(&assets_map).unwrap();
    fs::write(Path::new("./dist/assets_map.json"), assets_map_json).unwrap();

    println!("cargo:rerun-if-changed={}", "./assets/");
}

fn compile_assets(assets_map: &mut HashMap<String, String>, dest_dir: &Path) {
    let assets_dir = Path::new("./assets");

    WalkDir::new(assets_dir)
        .follow_links(true)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .for_each(|entry| {
            if entry.file_type().is_dir() {
                return;
            }

            let mut source = read_to_string(entry.path()).unwrap();

            let filename = entry.file_name().to_string_lossy().to_string();

            if filename.ends_with(".js") && !filename.ends_with(".min.js") {
                let mut out = Vec::new();
                minify_js::minify(
                    minify_js::TopLevelMode::Module,
                    source.into_bytes(),
                    &mut out,
                )
                .expect(format!("Failed to minify {}", entry.path().display()).as_str());
                source = String::from_utf8(out).unwrap();
            } else if filename.ends_with(".css") && !filename.ends_with(".min.css") {
                source = Minifier::default()
                    .minify(source.as_str(), Level::Three)
                    .unwrap();
            }

            let dest = write_hashed_file(
                entry.path().file_name().unwrap().to_str().unwrap(),
                source,
                dest_dir,
            );

            assets_map.insert(
                entry
                    .path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
                dest,
            );
        });
}

fn compile_syntect_styles(assets_map: &mut HashMap<String, String>, dest_dir: &Path) {
    let ts = ThemeSet::load_defaults();

    // create dark color scheme css
    let dark_theme = Minifier::default()
        .minify(
            css_for_theme_with_class_style(&ts.themes["Solarized (dark)"], ClassStyle::Spaced)
                .unwrap()
                .as_str(),
            Level::Three,
        )
        .unwrap();

    let light_theme = Minifier::default()
        .minify(
            css_for_theme_with_class_style(&ts.themes["InspiredGitHub"], ClassStyle::Spaced)
                .unwrap()
                .as_str(),
            Level::Three,
        )
        .unwrap();

    let light_theme_dest = write_hashed_file("syntect_light_theme.css", light_theme, dest_dir);

    let dark_theme_dest = write_hashed_file("syntect_dark_theme.css", dark_theme, dest_dir);

    assets_map.insert("syntect_light_theme.css".to_string(), light_theme_dest);

    assets_map.insert("syntect_dark_theme.css".to_string(), dark_theme_dest);
}

fn compile_fonts(dest_dir: &Path) {
    let fonts_dir = Path::new("./assets/fonts/");
    let dest_fonts_dir = dest_dir.join("./fonts/");

    if !dest_fonts_dir.exists() {
        create_dir(&dest_fonts_dir).unwrap();
    }

    WalkDir::new(fonts_dir)
        .follow_links(true)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .for_each(|entry| {
            if entry.file_type().is_dir() {
                return;
            }
            let filename = entry.path().file_name().unwrap().to_str().unwrap();
            fs::copy(entry.path(), dest_fonts_dir.join(filename)).unwrap();
        });
}

fn write_hashed_file(filename: &str, contents: String, dest: &Path) -> String {
    let path = Path::new(filename);
    let hashed_path = path.with_extension(format!(
        "{:x}.{}",
        hash(contents.as_bytes()),
        path.extension().unwrap().to_str().unwrap()
    ));
    let hashed_filename = Path::new(hashed_path.file_name().unwrap());

    fs::write(dest.join(Path::new(hashed_filename)), contents).unwrap();

    hashed_filename.to_string_lossy().to_string()
}
