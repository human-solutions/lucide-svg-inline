# lucide-svg-inline

A build-time codegen crate that generates a typed Rust `Icon` enum from [Lucide](https://lucide.dev/) SVG files. Icons are inlined as `&'static str` via `include_str!`, enabling dead code elimination — only icons you actually reference end up in your binary.

All ~1500 Lucide icons are bundled with the crate — no need to vendor SVGs yourself.

## Quick start

Add to your `Cargo.toml`:

```toml
[build-dependencies]
lucide-svg-inline = "0.1"
```

Create an `lucide-icons.toml` manifest:

```toml
icons = [
    "arrow-right",
    "heart",
    "x",
]
```

Add a `build.rs`:

```rust
fn main() {
    lucide_svg_inline::generate(lucide_svg_inline::Config {
        svg_dir: None, // use bundled Lucide icons
        manifest: "lucide-icons.toml".into(),
        svg_defaults: lucide_svg_inline::SvgDefaults {
            strip_dimensions: true,
            default_class: Some("icon lucide".into()),
            stroke_width: Some(1.75),
            ..Default::default()
        },
    })
    .expect("Failed to generate icon definitions");
}
```

Include the generated code in your project:

```rust
// src/icon.rs
include!(concat!(env!("OUT_DIR"), "/icon.rs"));
```

Use the icons:

```rust
use crate::icon::{Icon, RenderOptions};

// Zero-cost static SVG string
let svg = Icon::Heart.svg();

// Runtime customization
let big_heart = Icon::Heart.render(&RenderOptions {
    size: Some(48),
    stroke: Some("red".into()),
    ..Default::default()
});

// Accessibility attributes
let close_btn = Icon::X.render(&RenderOptions {
    size: Some(16),
    extra_attrs: vec![
        ("aria-label".into(), "Close".into()),
        ("role".into(), "img".into()),
    ],
    ..Default::default()
});
```

## Custom SVG directory

To use your own SVG files instead of the bundled ones, set `svg_dir`:

```rust
lucide_svg_inline::Config {
    svg_dir: Some("vendor/lucide-icons".into()),
    manifest: "lucide-icons.toml".into(),
    svg_defaults: Default::default(),
}
```

## `SvgDefaults` options

All build-time transforms are baked into the `include_str!` output — zero runtime cost.

| Field | Default | Description |
|---|---|---|
| `strip_dimensions` | `true` | Remove `width`/`height`, keep `viewBox` |
| `default_class` | `None` | CSS class(es) for all icons |
| `stroke_width` | `None` | Override stroke width (Lucide default: `2`) |
| `stroke` | `None` | Override stroke color (Lucide default: `currentColor`) |
| `fill` | `None` | Override fill (Lucide default: `none`) |
| `stroke_linecap` | `None` | Override linecap (Lucide default: `round`) |
| `stroke_linejoin` | `None` | Override linejoin (Lucide default: `round`) |

## `RenderOptions`

Runtime per-use overrides via the `render()` method:

| Field | Type | Description |
|---|---|---|
| `size` | `Option<u32>` | Sets both `width` and `height` |
| `width` | `Option<u32>` | Sets `width` (overrides `size`) |
| `height` | `Option<u32>` | Sets `height` (overrides `size`) |
| `stroke_width` | `Option<f32>` | Override stroke width |
| `stroke` | `Option<String>` | Override stroke color |
| `fill` | `Option<String>` | Override fill |
| `class` | `Option<String>` | Override CSS class |
| `extra_attrs` | `Vec<(String, String)>` | Additional attributes |

## Bundled icon version

```rust
// In build.rs — check which Lucide version is bundled
println!("Bundled Lucide version: {}", lucide_svg_inline::bundled_version());
```

## License

MIT
