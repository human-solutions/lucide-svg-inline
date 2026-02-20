//! Build-time codegen that generates a typed `Icon` enum from Lucide SVG files.
//!
//! Call [`generate()`] from your `build.rs` to read an `lucide-icons.toml` manifest,
//! transform SVGs according to [`SvgDefaults`], and write generated Rust code
//! to `OUT_DIR`. Icons are inlined via `include_str!` enabling dead code elimination.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use heck::ToUpperCamelCase;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Configuration for the code generator.
pub struct Config {
    /// Path to the directory containing Lucide SVG files.
    pub svg_dir: PathBuf,

    /// Path to the TOML manifest listing which icons to include.
    pub manifest: PathBuf,

    /// Build-time SVG defaults applied when generating the static SVG strings.
    pub svg_defaults: SvgDefaults,
}

/// Controls how SVG markup is transformed at build time.
/// These are baked into the `include_str!` output — zero runtime cost.
pub struct SvgDefaults {
    /// Remove `width` and `height` attributes from the `<svg>` element,
    /// keeping only `viewBox`. Default: `true`.
    pub strip_dimensions: bool,

    /// CSS class(es) to inject on every `<svg>` element. Default: `None`.
    pub default_class: Option<String>,

    /// Override the `stroke-width` attribute. Lucide default is `2`. Default: `None`.
    pub stroke_width: Option<f32>,

    /// Override the `stroke` (color) attribute. Default: `None`.
    pub stroke: Option<String>,

    /// Override the `stroke-linecap` attribute. Default: `None`.
    pub stroke_linecap: Option<String>,

    /// Override the `stroke-linejoin` attribute. Default: `None`.
    pub stroke_linejoin: Option<String>,

    /// Override the `fill` attribute. Default: `None`.
    pub fill: Option<String>,
}

impl Default for SvgDefaults {
    fn default() -> Self {
        Self {
            strip_dimensions: true,
            default_class: None,
            stroke_width: None,
            stroke: None,
            stroke_linecap: None,
            stroke_linejoin: None,
            fill: None,
        }
    }
}

/// Errors that can occur during code generation.
#[derive(Debug)]
pub enum Error {
    /// The SVG directory does not exist.
    SvgDirNotFound(PathBuf),
    /// The manifest file could not be read.
    ManifestReadError(PathBuf, std::io::Error),
    /// The manifest file contains invalid TOML.
    ManifestParseError(String),
    /// The manifest is missing the `icons` key or it is not an array.
    ManifestMissingIcons,
    /// An icon name in the manifest is empty.
    EmptyIconName,
    /// An SVG file listed in the manifest was not found.
    IconNotFound { name: String, expected_path: PathBuf },
    /// Failed to read an SVG file.
    SvgReadError(PathBuf, std::io::Error),
    /// Failed to write an output file.
    WriteError(PathBuf, std::io::Error),
    /// The SVG file has no opening `<svg` tag.
    InvalidSvg(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SvgDirNotFound(p) => write!(f, "SVG directory not found: {}", p.display()),
            Error::ManifestReadError(p, e) => {
                write!(f, "failed to read manifest {}: {e}", p.display())
            }
            Error::ManifestParseError(e) => write!(f, "failed to parse manifest TOML: {e}"),
            Error::ManifestMissingIcons => {
                write!(f, "manifest missing `icons` key (expected an array of strings)")
            }
            Error::EmptyIconName => write!(f, "manifest contains an empty icon name"),
            Error::IconNotFound {
                name,
                expected_path,
            } => write!(
                f,
                "icon '{name}' not found at {}",
                expected_path.display()
            ),
            Error::SvgReadError(p, e) => write!(f, "failed to read SVG {}: {e}", p.display()),
            Error::WriteError(p, e) => write!(f, "failed to write {}: {e}", p.display()),
            Error::InvalidSvg(name) => write!(f, "SVG '{name}' has no opening <svg> tag"),
        }
    }
}

impl std::error::Error for Error {}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Generate the `Icon` enum source file in `OUT_DIR`.
///
/// Call this from `build.rs`. Emits `cargo:rerun-if-changed` directives
/// for the manifest and each individual SVG file referenced in it.
pub fn generate(config: Config) -> Result<(), Error> {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));

    // Emit rerun-if-changed for the manifest itself
    println!("cargo:rerun-if-changed={}", config.manifest.display());

    // Parse manifest early so we can emit per-file directives
    let icon_names = parse_manifest(&config.manifest)?;
    for name in &icon_names {
        println!(
            "cargo:rerun-if-changed={}",
            config.svg_dir.join(format!("{name}.svg")).display()
        );
    }

    generate_to_inner(&config, &icon_names, &out_dir)
}

/// Generate the `Icon` enum source file to a specific directory.
///
/// This is the testable core — [`generate()`] is a thin wrapper around this.
pub fn generate_to(config: &Config, out_dir: &Path) -> Result<(), Error> {
    if !config.svg_dir.is_dir() {
        return Err(Error::SvgDirNotFound(config.svg_dir.clone()));
    }

    let icon_names = parse_manifest(&config.manifest)?;
    generate_to_inner(config, &icon_names, out_dir)
}

fn generate_to_inner(
    config: &Config,
    icon_names: &[String],
    out_dir: &Path,
) -> Result<(), Error> {
    if !config.svg_dir.is_dir() {
        return Err(Error::SvgDirNotFound(config.svg_dir.clone()));
    }

    // Create output SVG directory
    let svg_out_dir = out_dir.join("svgs");
    fs::create_dir_all(&svg_out_dir)
        .map_err(|e| Error::WriteError(svg_out_dir.clone(), e))?;

    // Process each icon: read SVG, apply defaults, write to OUT_DIR
    let mut icons: Vec<IconEntry> = Vec::with_capacity(icon_names.len());
    for name in icon_names {
        let src_path = config.svg_dir.join(format!("{name}.svg"));
        if !src_path.is_file() {
            return Err(Error::IconNotFound {
                name: name.clone(),
                expected_path: src_path,
            });
        }

        let svg_content = fs::read_to_string(&src_path)
            .map_err(|e| Error::SvgReadError(src_path.clone(), e))?;

        let transformed = apply_defaults(&svg_content, &config.svg_defaults, name)?;

        let out_path = svg_out_dir.join(format!("{name}.svg"));
        write_if_changed(&out_path, &transformed)?;

        let variant = kebab_to_variant(name);
        icons.push(IconEntry {
            name: name.clone(),
            variant,
        });
    }

    // Generate and write Rust source
    let rust_code = generate_rust_code(&icons);
    let icon_rs_path = out_dir.join("icon.rs");
    write_if_changed(&icon_rs_path, &rust_code)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

struct IconEntry {
    name: String,
    variant: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Write a file only if its content has changed, preserving mtime otherwise.
/// This prevents downstream recompilation when `include_str!` files are unchanged.
fn write_if_changed(path: &Path, content: &str) -> Result<(), Error> {
    if path.is_file() {
        if let Ok(existing) = fs::read_to_string(path) {
            if existing == content {
                return Ok(());
            }
        }
    }
    fs::write(path, content).map_err(|e| Error::WriteError(path.to_path_buf(), e))
}

// ---------------------------------------------------------------------------
// Manifest parsing
// ---------------------------------------------------------------------------

fn parse_manifest(path: &Path) -> Result<Vec<String>, Error> {
    let content =
        fs::read_to_string(path).map_err(|e| Error::ManifestReadError(path.to_path_buf(), e))?;

    let table: toml::Table = content
        .parse()
        .map_err(|e: toml::de::Error| Error::ManifestParseError(e.to_string()))?;

    let icons_value = table.get("icons").ok_or(Error::ManifestMissingIcons)?;

    let icons_array = icons_value
        .as_array()
        .ok_or(Error::ManifestMissingIcons)?;

    let mut names = BTreeSet::new();
    for val in icons_array {
        let name = val
            .as_str()
            .ok_or(Error::ManifestMissingIcons)?
            .to_string();
        if name.is_empty() {
            return Err(Error::EmptyIconName);
        }
        names.insert(name);
    }

    Ok(names.into_iter().collect())
}

// ---------------------------------------------------------------------------
// Name conversion
// ---------------------------------------------------------------------------

fn kebab_to_variant(name: &str) -> String {
    // Handle leading digits: "2fa" -> "two-fa" -> "TwoFa"
    let processed = if name.starts_with(|c: char| c.is_ascii_digit()) {
        let mut chars = name.chars();
        let digit = chars.next().unwrap();
        let word = match digit {
            '0' => "zero",
            '1' => "one",
            '2' => "two",
            '3' => "three",
            '4' => "four",
            '5' => "five",
            '6' => "six",
            '7' => "seven",
            '8' => "eight",
            '9' => "nine",
            _ => unreachable!(),
        };
        let rest: String = chars.collect();
        if rest.is_empty() {
            word.to_string()
        } else if rest.starts_with('-') {
            format!("{word}{rest}")
        } else {
            format!("{word}-{rest}")
        }
    } else {
        name.to_string()
    };

    processed.to_upper_camel_case()
}

// ---------------------------------------------------------------------------
// SVG parsing and manipulation
// ---------------------------------------------------------------------------

/// Find the end of an opening tag, handling quoted attribute values.
fn find_tag_end(s: &str) -> Option<usize> {
    let mut in_quote: Option<char> = None;
    for (i, c) in s.char_indices() {
        match in_quote {
            Some(q) if c == q => in_quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => in_quote = Some(c),
            None if c == '>' => return Some(i),
            _ => {}
        }
    }
    None
}

/// Parse the opening `<svg ...>` tag and return (attributes, rest_of_svg).
/// `rest_of_svg` starts from the `>` that closes the opening tag.
fn parse_svg_opening(svg: &str) -> Option<(Vec<(String, String)>, &str)> {
    let svg_start = svg.find("<svg")?;
    let after_tag = &svg[svg_start + 4..];
    let tag_end = find_tag_end(after_tag)?;
    let attr_str = &after_tag[..tag_end];
    let rest = &svg[svg_start + 4 + tag_end..]; // from '>' onward

    let attrs = parse_attributes(attr_str);
    Some((attrs, rest))
}

/// Parse HTML/XML attributes from a string like ` foo="bar" baz="qux"`.
fn parse_attributes(s: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    let mut remaining = s.trim();

    while !remaining.is_empty() {
        // Skip whitespace
        remaining = remaining.trim_start();
        if remaining.is_empty() {
            break;
        }

        // Find '='
        let eq_pos = match remaining.find('=') {
            Some(p) => p,
            None => break,
        };

        let attr_name = remaining[..eq_pos].trim().to_string();
        remaining = remaining[eq_pos + 1..].trim_start();

        // Expect quote
        if remaining.is_empty() {
            break;
        }

        let quote_char = remaining.as_bytes()[0] as char;
        if quote_char != '"' && quote_char != '\'' {
            break;
        }

        remaining = &remaining[1..];

        // Find closing quote
        let close_pos = match remaining.find(quote_char) {
            Some(p) => p,
            None => break,
        };

        let attr_value = remaining[..close_pos].to_string();
        remaining = &remaining[close_pos + 1..];

        attrs.push((attr_name, attr_value));
    }

    attrs
}

/// Reconstruct an SVG string from attributes and the rest of the content.
fn reconstruct_svg(attrs: &[(String, String)], rest: &str) -> String {
    let mut result = String::from("<svg");
    for (name, value) in attrs {
        result.push_str(&format!(" {name}=\"{value}\""));
    }
    result.push_str(rest);
    result
}

// ---------------------------------------------------------------------------
// Build-time transforms
// ---------------------------------------------------------------------------

fn apply_defaults(svg: &str, defaults: &SvgDefaults, name: &str) -> Result<String, Error> {
    let (mut attrs, rest) = parse_svg_opening(svg)
        .ok_or_else(|| Error::InvalidSvg(name.to_string()))?;

    if defaults.strip_dimensions {
        attrs.retain(|(n, _)| n != "width" && n != "height");
    }

    if let Some(ref class) = defaults.default_class {
        set_or_add_attr(&mut attrs, "class", class);
    }

    if let Some(sw) = defaults.stroke_width {
        // Format without trailing zeros
        let val = format_f32(sw);
        set_or_add_attr(&mut attrs, "stroke-width", &val);
    }

    if let Some(ref stroke) = defaults.stroke {
        set_or_add_attr(&mut attrs, "stroke", stroke);
    }

    if let Some(ref fill) = defaults.fill {
        set_or_add_attr(&mut attrs, "fill", fill);
    }

    if let Some(ref lc) = defaults.stroke_linecap {
        set_or_add_attr(&mut attrs, "stroke-linecap", lc);
    }

    if let Some(ref lj) = defaults.stroke_linejoin {
        set_or_add_attr(&mut attrs, "stroke-linejoin", lj);
    }

    Ok(reconstruct_svg(&attrs, rest))
}

fn set_or_add_attr(attrs: &mut Vec<(String, String)>, name: &str, value: &str) {
    if let Some(attr) = attrs.iter_mut().find(|(n, _)| n == name) {
        attr.1 = value.to_string();
    } else {
        attrs.push((name.to_string(), value.to_string()));
    }
}

fn format_f32(v: f32) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i32)
    } else {
        format!("{v}")
    }
}

// ---------------------------------------------------------------------------
// Code generation
// ---------------------------------------------------------------------------

fn generate_rust_code(icons: &[IconEntry]) -> String {
    let mut code = String::with_capacity(4096);

    // Header
    code.push_str("// Generated by lucide-svg-inline — do not edit\n\n");

    // Icon enum
    code.push_str("#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]\n");
    code.push_str("pub enum Icon {\n");
    for icon in icons {
        code.push_str(&format!("    {},\n", icon.variant));
    }
    code.push_str("}\n\n");

    // RenderOptions struct
    code.push_str(
        "\
#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    pub size: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub stroke_width: Option<f32>,
    pub stroke: Option<String>,
    pub fill: Option<String>,
    pub class: Option<String>,
    pub extra_attrs: Vec<(String, String)>,
}
\n",
    );

    // impl Icon
    code.push_str("impl Icon {\n");

    // svg() method
    code.push_str("    pub fn svg(&self) -> &'static str {\n");
    code.push_str("        match self {\n");
    for icon in icons {
        code.push_str(&format!(
            "            Self::{} => include_str!(concat!(env!(\"OUT_DIR\"), \"/svgs/{}.svg\")),\n",
            icon.variant, icon.name
        ));
    }
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // render() method
    code.push_str("    pub fn render(&self, opts: &RenderOptions) -> String {\n");
    code.push_str("        render_svg(self.svg(), opts)\n");
    code.push_str("    }\n\n");

    // name() method
    code.push_str("    pub fn name(&self) -> &'static str {\n");
    code.push_str("        match self {\n");
    for icon in icons {
        code.push_str(&format!(
            "            Self::{} => \"{}\",\n",
            icon.variant, icon.name
        ));
    }
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // all() method
    code.push_str("    pub fn all() -> &'static [Icon] {\n");
    code.push_str("        &[\n");
    for icon in icons {
        code.push_str(&format!("            Icon::{},\n", icon.variant));
    }
    code.push_str("        ]\n");
    code.push_str("    }\n");

    code.push_str("}\n\n");

    // Display impl
    code.push_str("impl std::fmt::Display for Icon {\n");
    code.push_str(
        "    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n",
    );
    code.push_str("        f.write_str(self.name())\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Self-contained render_svg helper
    code.push_str(&generate_render_fn());

    code
}

fn generate_render_fn() -> String {
    r##"fn render_svg(svg: &str, opts: &RenderOptions) -> String {
    let svg_start = match svg.find("<svg") {
        Some(p) => p,
        None => return svg.to_string(),
    };
    let after_tag = &svg[svg_start + 4..];
    let tag_end = match find_render_tag_end(after_tag) {
        Some(p) => p,
        None => return svg.to_string(),
    };
    let attr_str = &after_tag[..tag_end];
    let rest = &svg[svg_start + 4 + tag_end..];

    let mut attrs = parse_render_attrs(attr_str);

    // Apply size
    let w = opts.width.or(opts.size);
    let h = opts.height.or(opts.size);
    if let Some(w) = w {
        set_render_attr(&mut attrs, "width", &w.to_string());
    }
    if let Some(h) = h {
        set_render_attr(&mut attrs, "height", &h.to_string());
    }

    if let Some(sw) = opts.stroke_width {
        let val = if sw.fract() == 0.0 {
            format!("{}", sw as i32)
        } else {
            format!("{sw}")
        };
        set_render_attr(&mut attrs, "stroke-width", &val);
    }

    if let Some(ref stroke) = opts.stroke {
        set_render_attr(&mut attrs, "stroke", stroke);
    }

    if let Some(ref fill) = opts.fill {
        set_render_attr(&mut attrs, "fill", fill);
    }

    if let Some(ref class) = opts.class {
        set_render_attr(&mut attrs, "class", class);
    }

    for (name, value) in &opts.extra_attrs {
        set_render_attr(&mut attrs, name, value);
    }

    let mut result = String::from("<svg");
    for (name, value) in &attrs {
        result.push(' ');
        result.push_str(name);
        result.push_str("=\"");
        result.push_str(value);
        result.push('"');
    }
    result.push_str(rest);
    result
}

fn find_render_tag_end(s: &str) -> Option<usize> {
    let mut in_quote: Option<char> = None;
    for (i, c) in s.char_indices() {
        match in_quote {
            Some(q) if c == q => in_quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => in_quote = Some(c),
            None if c == '>' => return Some(i),
            _ => {}
        }
    }
    None
}

fn parse_render_attrs(s: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    let mut remaining = s.trim();
    while !remaining.is_empty() {
        remaining = remaining.trim_start();
        if remaining.is_empty() {
            break;
        }
        let eq_pos = match remaining.find('=') {
            Some(p) => p,
            None => break,
        };
        let attr_name = remaining[..eq_pos].trim().to_string();
        remaining = remaining[eq_pos + 1..].trim_start();
        if remaining.is_empty() {
            break;
        }
        let quote_char = remaining.as_bytes()[0] as char;
        if quote_char != '"' && quote_char != '\'' {
            break;
        }
        remaining = &remaining[1..];
        let close_pos = match remaining.find(quote_char) {
            Some(p) => p,
            None => break,
        };
        let attr_value = remaining[..close_pos].to_string();
        remaining = &remaining[close_pos + 1..];
        attrs.push((attr_name, attr_value));
    }
    attrs
}

fn set_render_attr(attrs: &mut Vec<(String, String)>, name: &str, value: &str) {
    if let Some(attr) = attrs.iter_mut().find(|(n, _)| n == name) {
        attr.1 = value.to_string();
    } else {
        attrs.push((name.to_string(), value.to_string()));
    }
}
"##
    .to_string()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Name conversion --

    #[test]
    fn test_kebab_to_variant_simple() {
        assert_eq!(kebab_to_variant("heart"), "Heart");
    }

    #[test]
    fn test_kebab_to_variant_multi_word() {
        assert_eq!(kebab_to_variant("arrow-right"), "ArrowRight");
    }

    #[test]
    fn test_kebab_to_variant_single_char() {
        assert_eq!(kebab_to_variant("x"), "X");
    }

    #[test]
    fn test_kebab_to_variant_leading_digit() {
        assert_eq!(kebab_to_variant("2fa"), "TwoFa");
    }

    #[test]
    fn test_kebab_to_variant_leading_digit_with_dash() {
        assert_eq!(kebab_to_variant("3d-cube"), "ThreeDCube");
    }

    // -- SVG parsing --

    #[test]
    fn test_parse_svg_opening_single_line() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><path d="M1 1"/></svg>"#;
        let (attrs, rest) = parse_svg_opening(svg).unwrap();
        assert_eq!(attrs.len(), 3);
        assert_eq!(attrs[0], ("xmlns".into(), "http://www.w3.org/2000/svg".into()));
        assert_eq!(attrs[1], ("width".into(), "24".into()));
        assert_eq!(attrs[2], ("height".into(), "24".into()));
        assert!(rest.starts_with('>'));
    }

    #[test]
    fn test_parse_svg_opening_multiline() {
        let svg = "<svg\n  xmlns=\"http://www.w3.org/2000/svg\"\n  width=\"24\"\n  height=\"24\"\n>\n  <path d=\"M1 1\"/>\n</svg>";
        let (attrs, _rest) = parse_svg_opening(svg).unwrap();
        assert_eq!(attrs.len(), 3);
        assert_eq!(attrs[1].0, "width");
    }

    #[test]
    fn test_parse_attributes_with_inner_gt() {
        // Attribute values should not contain '>', but tag end detection should handle quotes
        let s = r#" foo="a>b" bar="c""#;
        let attrs = parse_attributes(s);
        assert_eq!(attrs.len(), 2);
        assert_eq!(attrs[0].1, "a>b");
    }

    #[test]
    fn test_find_tag_end_with_quotes() {
        let s = r#" foo="a>b" bar="c">"#;
        let pos = find_tag_end(s).unwrap();
        assert_eq!(s.as_bytes()[pos], b'>');
        assert_eq!(pos, s.len() - 1);
    }

    // -- Build-time transforms --

    #[test]
    fn test_apply_defaults_strip_dimensions() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M1 1"/></svg>"#;
        let defaults = SvgDefaults {
            strip_dimensions: true,
            ..Default::default()
        };
        let result = apply_defaults(svg, &defaults, "test").unwrap();
        assert!(!result.contains("width="));
        assert!(!result.contains("height="));
        assert!(result.contains("viewBox="));
    }

    #[test]
    fn test_apply_defaults_inject_class() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><path/></svg>"#;
        let defaults = SvgDefaults {
            strip_dimensions: false,
            default_class: Some("icon lucide".into()),
            ..Default::default()
        };
        let result = apply_defaults(svg, &defaults, "test").unwrap();
        assert!(result.contains(r#"class="icon lucide""#));
    }

    #[test]
    fn test_apply_defaults_replace_stroke_width() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" stroke-width="2"><path/></svg>"#;
        let defaults = SvgDefaults {
            strip_dimensions: false,
            stroke_width: Some(1.75),
            ..Default::default()
        };
        let result = apply_defaults(svg, &defaults, "test").unwrap();
        assert!(result.contains(r#"stroke-width="1.75""#));
        assert!(!result.contains(r#"stroke-width="2""#));
    }

    #[test]
    fn test_apply_defaults_noop() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><path/></svg>"#;
        let defaults = SvgDefaults {
            strip_dimensions: false,
            ..Default::default()
        };
        let result = apply_defaults(svg, &defaults, "test").unwrap();
        assert!(result.contains(r#"width="24""#));
        assert!(result.contains(r#"height="24""#));
    }

    // -- Manifest parsing --

    #[test]
    fn test_parse_manifest_valid() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = dir.path().join("lucide-icons.toml");
        fs::write(&manifest, r#"icons = ["heart", "arrow-right", "x"]"#).unwrap();

        let names = parse_manifest(&manifest).unwrap();
        // BTreeSet sorts them
        assert_eq!(names, vec!["arrow-right", "heart", "x"]);
    }

    #[test]
    fn test_parse_manifest_missing_icons_key() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = dir.path().join("lucide-icons.toml");
        fs::write(&manifest, "something_else = true").unwrap();

        let err = parse_manifest(&manifest).unwrap_err();
        assert!(matches!(err, Error::ManifestMissingIcons));
    }

    #[test]
    fn test_parse_manifest_empty_array() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = dir.path().join("lucide-icons.toml");
        fs::write(&manifest, "icons = []").unwrap();

        let names = parse_manifest(&manifest).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn test_parse_manifest_malformed() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = dir.path().join("lucide-icons.toml");
        fs::write(&manifest, "{{{{invalid").unwrap();

        let err = parse_manifest(&manifest).unwrap_err();
        assert!(matches!(err, Error::ManifestParseError(_)));
    }

    #[test]
    fn test_parse_manifest_empty_name() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = dir.path().join("lucide-icons.toml");
        fs::write(&manifest, r#"icons = ["heart", ""]"#).unwrap();

        let err = parse_manifest(&manifest).unwrap_err();
        assert!(matches!(err, Error::EmptyIconName));
    }

    #[test]
    fn test_parse_manifest_deduplicates() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = dir.path().join("lucide-icons.toml");
        fs::write(&manifest, r#"icons = ["heart", "heart", "x"]"#).unwrap();

        let names = parse_manifest(&manifest).unwrap();
        assert_eq!(names, vec!["heart", "x"]);
    }

    // -- Reconstruct SVG --

    #[test]
    fn test_reconstruct_svg() {
        let attrs = vec![
            ("xmlns".to_string(), "http://www.w3.org/2000/svg".to_string()),
            ("viewBox".to_string(), "0 0 24 24".to_string()),
        ];
        let rest = "><path d=\"M1 1\"/></svg>";
        let result = reconstruct_svg(&attrs, rest);
        assert_eq!(
            result,
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M1 1"/></svg>"#
        );
    }

    // -- Format f32 --

    #[test]
    fn test_format_f32_integer() {
        assert_eq!(format_f32(2.0), "2");
    }

    #[test]
    fn test_format_f32_decimal() {
        assert_eq!(format_f32(1.75), "1.75");
    }
}
