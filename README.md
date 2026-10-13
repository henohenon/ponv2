<img src="assets/icon.svg" alt="pon" width="96" align="right">

# pon

Zed で Markdown を書くときの、画像とページの手間を消す道具。設計は [docs/spec.md](docs/spec.md)。

- `pon paste` — クリップボードの画像を置き場に保存して、本文にリンクを入れる
- `pon rename` — `![](新,旧)` の行で、画像を改名して置き場へ動かし、参照を直す
- `pon open [名前] [--today] [--project DIR]` — 名前からページを開く。無ければ作る

Zed のタスク（`%APPDATA%\Zed\tasks.json`）とキー（`keymap.json`）から呼ぶ。
プロジェクトごとの置き場と規則は、そのルートの `pon.toml`。

```bash
cargo install --path .
```
