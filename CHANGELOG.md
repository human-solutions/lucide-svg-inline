# Changelog

## 0.2.0

- Vendor all ~1555 Lucide v0.475.0 SVGs in the repo (`icons/` directory)
- Make `svg_dir` optional — defaults to bundled icons, no external download needed
- Add `ICONS.md` with the full icon list (names, categories, tags)
- Add `scripts/update-icons.sh` to regenerate vendored icons and `ICONS.md` from any Lucide release

## 0.1.0

- Initial release with build-time codegen for typed `Icon` enum
- `include_str!`-based SVG inlining with dead code elimination
- `SvgDefaults` for build-time transforms (strip dimensions, class, stroke-width, etc.)
- `RenderOptions` for runtime per-use overrides (size, stroke, fill, class, extra attrs)
