# Testing dusk

## Automated

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --package dusk -- --check
```

The unit tests cover config parsing and merging, duplicate and reserved keys, theme chains and look-only enforcement, ordering, confirmation timing (a repeat or a second press inside 250 ms is ignored), capability filtering (`yes`, `challenge`, `no`, `na`, `inhibited`), colour parsing, and Hyprland IPC parsing.

## Headless scenes

Run one still per state. `--scene` settles the entrance; add `--at` for a time after a script.

```sh
for scene in idle selected armed problem inhibited many halo mono reduced light dark; do
  cargo run -- --scene "$scene" --snap "/tmp/dusk-$scene.png" --size 1920x1080
done
```

Portrait, 4K and scale:

```sh
cargo run -- --scene idle --snap /tmp/portrait.png --size 800x1000
cargo run -- --scene many --snap /tmp/4k.png --size 3840x2160
cargo run -- --scene many --snap /tmp/portrait-many.png --size 800x1000
cargo run -- --scene idle --snap /tmp/2x.png --size 1920x1080 --scale 2
cargo run -- --scene idle --snap /tmp/narrow.png --size 640x320
```

A five-card row shrinks to fit a short screen while the labels stay legible; below that it becomes a
scrolling list. The many-action portrait should stay a grid.

Motion frames:

```sh
cat >/tmp/events.json <<'JSON'
{"events": [
  {"t": 0.1, "type": "key", "name": "Right"},
  {"t": 0.35, "type": "key", "name": "Right"}
]}
JSON
cargo run -- --script /tmp/events.json --frames 30 --out /tmp/frames
```

`--tree /tmp/tree.json` asserts structure without looking at pixels. `--snap` is the visual review path. Golden images are intentionally not committed; Rune's headless output can vary by GPU.

## Manual matrix before a release

Run each with a real Hyprland session, but always with `--dry-run` or `preview`:

- two monitors with mixed scale;
- a monitor unplugged while open;
- Caps Lock and a non-US layout;
- launch during a fullscreen window;
- double launch (second closes the first);
- a launch bind that shares a key with an action (`Super+S` must not trigger Suspend);
- idle lock and suspend while open: `system::tests` starts a private bus with a mock
  `org.freedesktop.login1` and covers `Lock`, `LockedHint` and `PrepareForSleep`. Keep the manual
  check for a real lock and suspend/resume. A `busctl emit` from another connection does not
  satisfy zbus's sender match, which is why the mock owns the name.
- a hung process (watchdog removes the surface);
- a polkit prompt (it must appear after the menu is gone);
- an active block inhibitor (the menu stays and names the blocker);
- pointer, touch, background click, Esc, Tab, arrows, Home/End, and mnemonics.
- Sway and Niri: `dusk doctor` detects the compositor and focused output, the logout action uses `swaymsg exit` / `niri msg action quit --skip-confirmation`, and a `--dry-run` menu opens on the focused monitor. These paths have unit-tested parsers but need a live session check before a release.

## Preview

Run `dusk preview` with a config that selects a file theme (for example `theme "foo"` in
`~/.config/dusk/config.kdl` with `~/.config/dusk/themes/foo.kdl`). Editing either file must print
`preview reloaded` and redraw the menu; built-in themes are embedded and have no file to watch.

## Safety rule

Tests, previews and development only use `--dry-run`, `preview`, or headless runs. No automated test may reach a real power action. The action command is only spawned from `Mode::Normal`.
