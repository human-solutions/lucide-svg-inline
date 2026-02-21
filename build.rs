use std::fs;
use std::path::Path;

fn main() {
    let icons_dir = Path::new("icons");
    println!("cargo:rerun-if-changed=icons");

    let mut names: Vec<String> = fs::read_dir(icons_dir)
        .expect("failed to read icons/ directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().into_string().ok()?;
            name.strip_suffix(".svg").map(|s| s.to_string())
        })
        .collect();
    names.sort();

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir).join("bundled.rs");

    let mut code = String::new();
    code.push_str("pub(crate) fn bundled_svg(name: &str) -> Option<&'static str> {\n");
    code.push_str("    match name {\n");
    for name in &names {
        code.push_str(&format!(
            "        \"{name}\" => Some(include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/icons/{name}.svg\"))),\n",
        ));
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");
    code.push_str(
        "pub(crate) const BUNDLED_VERSION: &str = include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/icons/VERSION\"));\n",
    );

    fs::write(&out_path, &code).expect("failed to write bundled.rs");
}
