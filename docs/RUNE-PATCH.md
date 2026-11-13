# The Rune fork

Upstream Rune does not take pull requests, and dusk pins a fork of `dev` based on `6ba947e5`. Two Wayland layer-surface bugs block dusk there, so they are carried in a fork:

- Fork: [rodneyosodo/rune](https://github.com/rodneyosodo/rune)
- Upstream tracking: [anomalyco/rune#623](https://github.com/anomalyco/rune/issues/623) (focus) and [anomalyco/rune#319](https://github.com/anomalyco/rune/issues/319) (keymap fd)
- Branch at dusk's revision: `fix/wayland-layer-keyboard`, commit `c2487b99`, based on `dev` `6ba947e5`, tag `dusk-6ba947e5-focus-keymap`
- The previous pin is kept on `fix/wayland-layer-keyboard-781e2b6` (`841b58ae`, tag `dusk-781e2b6-focus-keymap`)

`Cargo.toml` pins the commit directly:

```toml
rune = { git = "https://github.com/rodneyosodo/rune", rev = "c2487b991f354769681f051b4805a6b4693152b8", ... }
```

There is no `vendor/` copy in the repo. A clean build fetches that one git dependency; `Cargo.lock` records the commit. For an offline build, run `cargo vendor` and point `.cargo/config.toml` at the result.

Rune's text stack pulls Parley and Fontique with the `system` feature, which probes Fontconfig through `pkg-config`. Linux builds therefore need the Fontconfig development files and `pkg-config` in addition to a C toolchain.

## The fixes

### 1. Read the keymap from the start

Hyprland passes `wl_keyboard.keymap` as a shared, read-only open file description whose offset can already be at the end (Hyprland sends it that way, and another client may have read it). Rune read it with `read_to_string`, got an empty string, and failed to parse a keymap:

```
xkbcommon: ERROR: [XKB-822] Failed to parse input xkb string
```

With `state.xkb` empty, every key on a layer surface was dropped. The fix reads at offset 0 without moving the shared offset, with `pread`:

```rust
use std::os::unix::fs::FileExt;
let file = std::fs::File::from(fd);
if size <= 16 * 1024 * 1024 {
    let mut buf = vec![0u8; size as usize];
    if file.read_exact_at(&mut buf, 0).is_ok() { ... }
}
```

`seek(0)` would also fix the read, but the fd is one shared open file description, so seeking moves the offset for every other client too.

Evidence: the same fd returns 0 bytes with `read` and 35572 bytes with `mmap` or after `lseek(fd, 0, SEEK_SET)`; `xkbcli compile-keymap` accepts the map.

### 2. Report layer-surface focus

A layer surface is not a toplevel, so winit never sends `WindowEvent::Focused` for it. Rune only set its internal `keyboard_focus` flag, and `ConfiguredOverlay::on_focus` never ran, so an overlay could not close when the user moved to another window. The fix adds `Input::Focus(bool)`, pushes it when the flag changes, and routes it to `Runtime::window_focus` in `overlay/draw.rs`.

## Bumping Rune

1. Update the fork branch (rebase on the new `dev`, or cherry-pick the two hunks).
2. Move the tag and push it, or pin the new commit hash.
3. Update the `rev` in `Cargo.toml`, run `cargo update -p rune`, and commit `Cargo.lock`.
4. Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `dusk check`, and review the headless stills.
5. Update the commit and tag in this file.

If upstream fixes both bugs, drop the fork and pin the upstream revision directly. The release workflow already embeds `cargo vendor` output in the source tarball for this reason.
