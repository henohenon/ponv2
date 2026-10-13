<div align="center">
  <img src="assets/icon.svg" alt="pon icon" width="96">
</div>

# pon

Clipboard image → Markdown paste helper for Zed on Windows. Successor of [pon](https://github.com/henohenon/pon), rewritten in Rust.

## Commands

| Command | Key | What it does |
|---|---|---|
| `pon paste` | `ctrl-alt-v` | Saves the clipboard image into the configured folder and inserts `![](…)` at the cursor. Selected text becomes the file name; otherwise `image.png`, `image_1.png`, … |
| `pon rename` | `ctrl-alt-r` | On a line like `![](neko,image.png)` (new,old — either order, extension optional), renames/moves the image and rewrites every reference in the file |
| `pon open --today` | `ctrl-alt-t` | Opens (or creates) today's page |
| `pon open [name]` | `ctrl-alt-n` | Opens (or creates) a page by name. Without a name, uses the selection or asks in a small input box |

`pon` is a windowless exe meant to be called from [Zed tasks](https://zed.dev/docs/tasks) and Stream Deck. Errors show a message box; success is silent.

## Active file detection

Zed passes `ZED_FILE`, `ZED_ROW`, `ZED_COLUMN` and `ZED_SELECTED_TEXT` to tasks, so pon no longer reads Zed's state DB like v1 did.

Zed only provides these for files inside a folder opened as a project. A file opened on its own gets no variables (and Zed's own image paste does not work there either), so `pon open` always opens the project folder together with the file.

## Zed setup

`%APPDATA%\Zed\tasks.json` (one entry per command; `paste` shown):

```jsonc
[
  {
    "label": "pon: paste",
    "command": "pon",
    "args": ["paste"],
    "shell": { "program": "cmd" },  // cmd starts in ~8 ms, pwsh in ~190 ms
    "save": "current",
    "reveal": "never",
    "hide": "always"
  }
]
```

`%APPDATA%\Zed\keymap.json`:

```jsonc
[
  {
    "context": "Workspace",
    "bindings": {
      "ctrl-alt-v": ["task::Spawn", { "task_name": "pon: paste" }],
      "ctrl-alt-r": ["task::Spawn", { "task_name": "pon: rename" }],
      "ctrl-alt-t": ["task::Spawn", { "task_name": "pon: today" }],
      "ctrl-alt-n": ["task::Spawn", { "task_name": "pon: open" }]
    }
  }
]
```

## Config

`pon.toml` at the root of each notes folder. pon walks up from the current file to find it.

```toml
# Images go next to the page unless set. {stem} = the page's file name without extension.
[images]
dir = "images/{stem}"

# Pages: first matching rule wins. `match` is a regex, `$1` etc. are its captures.
[[pages]]
match = '^(\d{4}-\d{2}-\d{2})$'
path = "diary/$1.md"
template = "# $1\n"

[[pages]]
match = '^(.+)$'
path = "$1.md"
```

## Stream Deck

The built-in **System → Open** action does not pass arguments typed after the path, so give it a shortcut instead. [scripts/make-launchers.ps1](scripts/make-launchers.ps1) creates `.lnk` files with the arguments baked in (edit the list at the top), e.g. `pon open --today --project D:\notes`. Type the `.lnk` path into the field; picking it with the browse button resolves it to the exe and drops the arguments.

## Logs

Every run appends one line to `%LOCALAPPDATA%\pon\pon-v2.log` (selection is recorded as its length only).

## Building

```
cargo install --path .
```

Requires Rust (edition 2024) and the MSVC toolchain on Windows. Design notes: [docs/spec.md](docs/spec.md).
