use lucide_svg_inline::{Config, SvgDefaults};
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn make_config(_out_dir: &std::path::Path) -> Config {
    Config {
        svg_dir: fixtures_dir().join("svgs"),
        manifest: fixtures_dir().join("lucide-icons.toml"),
        svg_defaults: SvgDefaults::default(),
    }
}

// ---------------------------------------------------------------------------
// Full pipeline
// ---------------------------------------------------------------------------

#[test]
fn test_generate_creates_icon_rs_and_svgs() {
    let tmp = tempfile::tempdir().unwrap();
    let config = make_config(tmp.path());

    lucide_svg_inline::generate_to(&config, tmp.path()).unwrap();

    // icon.rs should exist
    let icon_rs = std::fs::read_to_string(tmp.path().join("icon.rs")).unwrap();
    assert!(icon_rs.contains("pub enum Icon"));
    assert!(icon_rs.contains("ArrowRight"));
    assert!(icon_rs.contains("CircleCheck"));
    assert!(icon_rs.contains("Heart"));
    assert!(icon_rs.contains("X,"));

    // SVG files should exist in svgs/
    assert!(tmp.path().join("svgs/heart.svg").is_file());
    assert!(tmp.path().join("svgs/arrow-right.svg").is_file());
    assert!(tmp.path().join("svgs/x.svg").is_file());
    assert!(tmp.path().join("svgs/circle-check.svg").is_file());
}

#[test]
fn test_generate_default_strip_dimensions() {
    let tmp = tempfile::tempdir().unwrap();
    let config = make_config(tmp.path());

    lucide_svg_inline::generate_to(&config, tmp.path()).unwrap();

    let svg = std::fs::read_to_string(tmp.path().join("svgs/heart.svg")).unwrap();
    // strip_dimensions defaults to true
    assert!(!svg.contains(r#" width=""#));
    assert!(!svg.contains(r#" height=""#));
    assert!(svg.contains("viewBox="));
}

#[test]
fn test_generate_with_all_transforms() {
    let tmp = tempfile::tempdir().unwrap();
    let config = Config {
        svg_dir: fixtures_dir().join("svgs"),
        manifest: fixtures_dir().join("lucide-icons.toml"),
        svg_defaults: SvgDefaults {
            strip_dimensions: true,
            default_class: Some("icon lucide".into()),
            stroke_width: Some(1.75),
            stroke: Some("red".into()),
            fill: Some("blue".into()),
            stroke_linecap: Some("butt".into()),
            stroke_linejoin: Some("miter".into()),
        },
    };

    lucide_svg_inline::generate_to(&config, tmp.path()).unwrap();

    let svg = std::fs::read_to_string(tmp.path().join("svgs/heart.svg")).unwrap();
    assert!(svg.contains(r#"class="icon lucide""#));
    assert!(svg.contains(r#"stroke-width="1.75""#));
    assert!(svg.contains(r#"stroke="red""#));
    assert!(svg.contains(r#"fill="blue""#));
    assert!(svg.contains(r#"stroke-linecap="butt""#));
    assert!(svg.contains(r#"stroke-linejoin="miter""#));
    assert!(!svg.contains(r#" width="24""#));
}

#[test]
fn test_generate_preserve_dimensions() {
    let tmp = tempfile::tempdir().unwrap();
    let config = Config {
        svg_dir: fixtures_dir().join("svgs"),
        manifest: fixtures_dir().join("lucide-icons.toml"),
        svg_defaults: SvgDefaults {
            strip_dimensions: false,
            ..Default::default()
        },
    };

    lucide_svg_inline::generate_to(&config, tmp.path()).unwrap();

    let svg = std::fs::read_to_string(tmp.path().join("svgs/heart.svg")).unwrap();
    assert!(svg.contains(r#"width="24""#));
    assert!(svg.contains(r#"height="24""#));
}

#[test]
fn test_generated_code_contains_render_support() {
    let tmp = tempfile::tempdir().unwrap();
    let config = make_config(tmp.path());

    lucide_svg_inline::generate_to(&config, tmp.path()).unwrap();

    let icon_rs = std::fs::read_to_string(tmp.path().join("icon.rs")).unwrap();
    assert!(icon_rs.contains("pub struct RenderOptions"));
    assert!(icon_rs.contains("fn render_svg("));
    assert!(icon_rs.contains("pub fn render("));
    assert!(icon_rs.contains("pub fn name("));
    assert!(icon_rs.contains("pub fn all("));
    assert!(icon_rs.contains("impl std::fmt::Display for Icon"));
}

#[test]
fn test_generated_code_svg_method_uses_include_str() {
    let tmp = tempfile::tempdir().unwrap();
    let config = make_config(tmp.path());

    lucide_svg_inline::generate_to(&config, tmp.path()).unwrap();

    let icon_rs = std::fs::read_to_string(tmp.path().join("icon.rs")).unwrap();
    assert!(icon_rs.contains(r#"include_str!(concat!(env!("OUT_DIR"), "/svgs/heart.svg"))"#));
    assert!(icon_rs.contains(r#"include_str!(concat!(env!("OUT_DIR"), "/svgs/arrow-right.svg"))"#));
}

#[test]
fn test_generated_name_method() {
    let tmp = tempfile::tempdir().unwrap();
    let config = make_config(tmp.path());

    lucide_svg_inline::generate_to(&config, tmp.path()).unwrap();

    let icon_rs = std::fs::read_to_string(tmp.path().join("icon.rs")).unwrap();
    assert!(icon_rs.contains(r#"Self::Heart => "heart""#));
    assert!(icon_rs.contains(r#"Self::ArrowRight => "arrow-right""#));
    assert!(icon_rs.contains(r#"Self::X => "x""#));
}

// ---------------------------------------------------------------------------
// Error cases
// ---------------------------------------------------------------------------

#[test]
fn test_error_icon_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = tmp.path().join("lucide-icons.toml");
    std::fs::write(&manifest, r#"icons = ["nonexistent"]"#).unwrap();

    let config = Config {
        svg_dir: fixtures_dir().join("svgs"),
        manifest,
        svg_defaults: SvgDefaults::default(),
    };

    let err = lucide_svg_inline::generate_to(&config, tmp.path()).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("nonexistent"), "Error should mention the icon name: {msg}");
}

#[test]
fn test_error_svg_dir_not_found() {
    let tmp = tempfile::tempdir().unwrap();

    let config = Config {
        svg_dir: PathBuf::from("/nonexistent/path"),
        manifest: fixtures_dir().join("lucide-icons.toml"),
        svg_defaults: SvgDefaults::default(),
    };

    let err = lucide_svg_inline::generate_to(&config, tmp.path()).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("not found"), "Error should say not found: {msg}");
}

#[test]
fn test_error_malformed_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = tmp.path().join("lucide-icons.toml");
    std::fs::write(&manifest, "{{{{bad toml").unwrap();

    let config = Config {
        svg_dir: fixtures_dir().join("svgs"),
        manifest,
        svg_defaults: SvgDefaults::default(),
    };

    let err = lucide_svg_inline::generate_to(&config, tmp.path()).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("parse"), "Error should mention parsing: {msg}");
}

#[test]
fn test_error_missing_icons_key() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = tmp.path().join("lucide-icons.toml");
    std::fs::write(&manifest, "other_key = true").unwrap();

    let config = Config {
        svg_dir: fixtures_dir().join("svgs"),
        manifest,
        svg_defaults: SvgDefaults::default(),
    };

    let err = lucide_svg_inline::generate_to(&config, tmp.path()).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("icons"), "Error should mention icons key: {msg}");
}
