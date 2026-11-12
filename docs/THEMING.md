# Theming dusk

A theme is a small KDL file that changes looks, never behaviour. Put it in `~/.config/dusk/themes/<name>.kdl` and select it from your config:

```kdl
theme "aurora"
```

`theme` also takes a built-in name (`dusk`, `halo`, `mono`) or a path. A theme may `extends` a built-in base; later settings win.

```kdl
extends "dusk"
layout "grid"

tokens {
    accent "#ffb86b"
    card "#ffffff/10"
    radius 20
}

tokens "light" {
    dim "#ffffff/52"
    text "#241d2e"
}
```

A theme may set `extends`, `layout`, `tokens` and `tokens "light"` only. `action`, `hide`, `order` and `appearance` belong in `config.kdl`; putting them in a theme is reported by `dusk check` and ignored. `config.kdl` may also override `tokens` and `tokens "light"`; its values are applied after the theme.

## Colour syntax

| Form             | Example                                           | Meaning                                |
| ---------------- | ------------------------------------------------- | -------------------------------------- |
| hex              | `"#d38be0"`, `"#d38be0cc"`                        | sRGB, optional `aa` alpha              |
| hex with slash   | `"#d38be0/80"`                                    | 80% alpha                              |
| named            | `"white"`, `"black"`, `"transparent"`             |                                        |
| named with slash | `"white/40"`                                      | 40% alpha                              |
| token reference  | `"accent"`, `"accent/25"`, `"card-border/60"`     | another token                          |
| rgb              | `"rgb(211 139 224)"` or `"rgb(83% 55% 88% / 40)"` | channels are 0–255 or percentages      |
| oklch            | `"oklch(0.72 0.16 320 / 0.8)"`                    | lightness, chroma, hue, optional alpha |

A slash value is a percentage (`40` = 40%) unless it contains a decimal point (`0.4` = 40%). Token references may point forwards, so `card "accent/20"` works even if `accent` is defined below it.

## Tokens

### Colour

| Token             | Default (dusk) | Used for                                                                    |
| ----------------- | -------------- | --------------------------------------------------------------------------- |
| `accent`          | `#c79bff`      | custom actions, centre glow                                                 |
| `dim`             | `#07060c/62`   | full-screen veil behind the menu (thickened when Hyprland has no blur rule) |
| `backdrop-top`    | `#1a1433/70`   | top of the backdrop gradient                                                |
| `backdrop-bottom` | `#2b1220/66`   | bottom of the backdrop gradient                                             |
| `bloom-a`         | `#5b4bff/30`   | soft glow in the top-left corner                                            |
| `bloom-b`         | `#ff6a4d/22`   | soft glow in the bottom-right corner                                        |
| `card`            | `#ffffff/6`    | unselected card glass                                                       |
| `card-border`     | `#ffffff/12`   | card hairline (brighter at the top) and glass sheen                         |
| `shadow`          | `#000000/55`   | card, highlight and vignette shadow                                         |
| `icon`            | `#ffffff/96`   | icon ink                                                                    |
| `text`            | `#f6f2fb`      | clock and selected label                                                    |
| `text-dim`        | `#ffffff/58`   | date, unselected label, key hints, problem line                             |
| `selected-text`   | `#0d0a12`      | reserved for text on a solid accent                                         |
| `warning`         | `#ff8b6b`      | armed action and blocker line                                               |
| `lock`            | `#8ec8ff`      | Lock orb and highlight                                                      |
| `suspend`         | `#8be3d2`      | Suspend and Hibernate highlight                                             |
| `logout`          | `#d99af0`      | Log out highlight                                                           |
| `reboot`          | `#ffc46b`      | Reboot highlight                                                            |
| `shutdown`        | `#ff7b6b`      | Shut down highlight                                                         |

Every theme is checked with and without compositor blur. Keep `card` opaque enough to hold text contrast on a bright wallpaper; the default dim is deliberately strong.

### Geometry

| Token          | Default | Meaning                                               |
| -------------- | ------- | ----------------------------------------------------- |
| `radius`       | `28`    | card corner radius (continuous corners)               |
| `card-width`   | `150`   | card width in points                                  |
| `card-height`  | `170`   | card height (`halo` uses a square)                    |
| `gap`          | `18`    | space between cards or rows                           |
| `icon-size`    | `26`    | icon glyph                                            |
| `orb-size`     | `62`    | tinted disc behind each icon; `0` draws the icon bare |
| `label-size`   | `14.5`  | label point size                                      |
| `label-weight` | `560`   | variable-font weight                                  |
| `key-size`     | `10.5`  | keycap point size                                     |
| `padding`      | `16`    | inside a card                                         |

Cards shrink to fit the screen; the board wraps to more rows in `row`, or to a grid. Portrait screens and narrow windows choose fewer columns. A `list` always uses one column and scrolls when it outgrows the screen. Labels wrap to two lines and then truncate; they never change a card's size.

### Motion

| Token           | Default | Meaning                                |
| --------------- | ------- | -------------------------------------- |
| `enter-ms`      | `260`   | entrance duration                      |
| `exit-ms`       | `120`   | exit duration                          |
| `stagger-ms`    | `32`    | delay between cards entering           |
| `spring-bounce` | `0.06`  | selection highlight bounce (0 is calm) |

Under the system Reduce Motion setting, the entrance becomes a 150 ms fade with no travel or blur.

### Special

| Token     | Default  | Meaning                                                |
| --------- | -------- | ------------------------------------------------------ |
| `compact` | `#false` | icon-only pill, as in `halo`                           |
| `mono`    | `#false` | monospace labels and always-visible keys, as in `mono` |
| `header`  | `#true`  | the time and date above the cards                      |

These are looks, not layouts; `halo` also sets a larger radius and small dim, and `mono` sets `layout "list"` and transparent cards.

## Built-in themes

| Id     | Character                                                               |
| ------ | ----------------------------------------------------------------------- |
| `dusk` | glass cards with glowing icon orbs, a clock, and an indigo-to-ember sky |
| `halo` | a compact icon-only pill at screen centre with little dimming           |
| `mono` | a monospace list with visible keys and no glass                         |

Read the sources in [`themes/`](../themes) for complete examples. The `dusk` theme is the reference; `halo` and `mono` are short `extends` files.

## For coding agents

1. Load the theme and config with `dusk check`; it reports file and line for every problem.
2. Render a still: `dusk --scene idle --snap out.png --size 1920x1080`.
3. To review motion, write a script and render frames:
   ```json
   { "events": [{ "t": 0.1, "type": "key", "name": "Right" }] }
   ```
   ```sh
   dusk --script events.json --frames 30 --out frames/
   ```
4. Check both `--theme light` and `--theme dark` (or the `light`/`dark` scenes), `--size 1920x1080` and `--size 3840x2160`, `--scale 2`, portrait `--size 800x1000`, and a long list (`--scene many`).
5. Never use a real action in an automated test. `--dry-run`, `preview` and headless runs only print.
6. Keep themes look-only. If a change needs a command, it belongs in `config.kdl`.
