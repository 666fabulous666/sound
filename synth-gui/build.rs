use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("preset_defaults.rs");

    // Find all .json files in assets/presets
    let presets_dir = Path::new("../assets/presets");

    let mut entries: Vec<_> = fs::read_dir(presets_dir)
        .unwrap_or_else(|_| panic!("Failed to read directory: {:?}", presets_dir))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()?.to_str()? == "json" {
                let file_name = path.file_stem()?.to_str()?.to_string();
                // Convert file_name to display name (replace underscores with spaces, title case)
                let display_name = file_name
                    .split('_')
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => {
                                first.to_uppercase().chain(chars).collect()
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                Some((display_name, path))
            } else {
                None
            }
        })
        .collect();

    // Sort by display name for consistent ordering
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    // Generate the Rust code
    let mut code = String::from("/// Built-in instrument presets (auto-generated from assets/presets/*.json)\n");
    code.push_str("pub const PRESET_DEFAULTS: &[(&str, &str)] = &[\n");

    for (display_name, path) in &entries {
        let relative_path = path
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        code.push_str(&format!(
            "    (\"{}\", include_str!(\"{}\")),\n",
            display_name, relative_path
        ));
    }

    code.push_str("];\n");

    fs::write(&dest_path, code).unwrap();

    // Tell Cargo to rerun if the presets directory changes
    println!("cargo:rerun-if-changed=../assets/presets");
}
