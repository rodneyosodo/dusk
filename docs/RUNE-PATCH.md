# Rune

dusk pins upstream Rune at a commit on `dev`, not a fork:

- Upstream: [anomalyco/rune](https://github.com/anomalyco/rune)
- Commit: `1ba38527` (`dev`, 23 November 2026)

The two Wayland layer-surface bugs dusk used to carry are fixed upstream:

- Layer surfaces report keyboard focus: [anomalyco/rune#623](https://github.com/anomalyco/rune/issues/623), fixed in #1222 (`1dc3b333`)
- The keymap fd is read from offset 0 without moving the shared offset: [anomalyco/rune#319](https://github.com/anomalyco/rune/issues/319), fixed in #1451 (`1e641412`)

The fork (`rodneyosodo/rune`) and its tags `dusk-781e2b6-focus-keymap` and
`dusk-6ba947e5-focus-keymap` are kept for history but are no longer used.

`Cargo.toml` pins the upstream commit:

```toml
rune = { git = "https://github.com/anomalyco/rune", rev = "1ba38527f375b5444141dec55af1b387b56316d6", ... }
```

## Vendor patches

Rune patches several crates with its own vendored copies. A dependency's `[patch.crates-io]` does
not apply to the app that depends on it, so dusk repeats the ones it needs:

- `parley_engine`: the text stack asks for its `line-dictionaries` feature.
- `winit` and `sctk-adwaita`: the Wayland layer host calls winit's content-corner-radius extension.
- `accesskit_windows` and `accesskit_macos`: the accessibility backends Rune builds against.

`wgpu-hal` is not repeated: its vendored version is not selected in dusk's graph.

There is no `vendor/` copy in this repo. A clean build fetches the Rune git dependency and the
patched crates from that same revision; `Cargo.lock` records them. For an offline build, run
`cargo vendor` and point `.cargo/config.toml` at the result.

Rune's text stack pulls Parley and Fontique with the `system` feature. Dusk also enables Fontique's
`fontconfig-dlopen` feature, so builds do not need Fontconfig development files; the library is
loaded at run time, which keeps Fontconfig a normal runtime dependency.

## Bumping Rune

1. Pick a new upstream `dev` commit.
2. Update the `rev` in the `rune` dependency and in every `[patch.crates-io]` entry together.
3. Run `cargo update -p rune`, then `cargo test`, `cargo clippy --all-targets -- -D warnings`,
   `dusk check`, and review the headless stills.
4. Update the commit here and in `README.md`.

If upstream starts publishing a consumable package, drop the repeated patches and pin that
release.
