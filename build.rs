use std::{
    collections::HashMap,
    fs::{self, create_dir, read_to_string, remove_dir_all},
    path::Path,
};

use seahash::hash;
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

    compile_assets(&mut assets_map, dist_dir);
    compile_fonts(dist_dir);

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
