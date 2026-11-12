# Release checklist

## Before publishing

- Check the name is free: `paru -Ss '^dusk$'`, the AUR, crates.io, and a general search.
- Update the version in `Cargo.toml`, `packaging/PKGBUILD`, and `man/dusk.1`.
- Run the full check set:
  ```sh
  cargo fmt --package dusk -- --check
  cargo test --all-targets
  cargo clippy --all-targets -- -D warnings
  cargo run -- check
  cargo test theme::tests::every_builtin_theme_parses_without_problems
  cargo test theme::tests::text_holds_contrast_on_the_worst_wallpaper
  ```
- Review stills for every scene at 1920x1080 and 3840x2160, portrait 800x1000, `--scale 2`, light
  and dark, with and without blur. See `docs/TESTING.md`.
- Walk the manual matrix in `docs/TESTING.md`. Use `--dry-run` for every action.
- Smoke-test Sway and Niri: `dusk doctor`, focused-monitor placement, logout, lock/suspend.
- Re-record `docs/dusk.mp4` if the entrance, selection or armed state changed:
  ```sh
  target/release/dusk --script events.json --video docs/dusk.mp4 --fps 60 --size 1920x1080
  ```
- Run `cargo build --release`; `DUSK_TIMING=1` prints launch phases.
- Tag `vX.Y.Z` and push the tag.

## The release workflow

`.github/workflows/release.yml` runs on a `v*` tag, or on demand with an existing tag:

1. **verify** checks that the tag matches `Cargo.toml`, then runs format, tests, clippy,
   `dusk check`, the theme test, and the contrast test.
2. **build** builds natively on `ubuntu-latest` (x86_64) and `ubuntu-24.04-arm` (aarch64) with
   `--locked`, packaging the binary, man page, completions, README, theming guide and licence.
3. **release** makes a source tarball with `git archive`, runs `cargo vendor` inside it so the AUR
   package can build offline, writes `SHA256SUMS`, and publishes the GitHub Release.
4. **aur** is optional, gated on the repository variable `AUR_ENABLED=true` and the secret
   `AUR_SSH_PRIVATE_KEY`. It regenerates the AUR package from the published source tarball and
   pushes `.SRCINFO`.

A clean local build fetches the pinned Rune commit from the fork; release runners need network.
The release source tarball bundles `cargo vendor`, so the AUR package itself builds `--offline`.

### AUR setup

The AUR package must exist before the first automated push. Add the variable and secret, and make
sure the package's default branch is `master`.

### Re-running a release

If a job fails, fix the commit, move the tag, and push it again. To release an existing tag, run
the workflow manually with the tag as the input. The release step replaces assets on a re-run.

## Install check on a clean machine

- `paru -S dusk`, or `cargo install --git ...`.
- One keybind and no config.
- `dusk doctor` shows the compositor, layer-shell, blur rule, locker and logind capabilities.
- `dusk --dry-run` opens and closes with the keybind; a second launch closes the first.
- `dusk init`, edit the accent, `dusk check`, `dusk preview` reloads both the config and theme.
