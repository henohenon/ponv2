# pon v2 設計

2026-10-13。Zed で Markdown を書くときの、画像と新規ページの手間を消す道具。
ブログ型（ページごとにフォルダを持ち、画像もそこに置く）と、メモ型（md を直下に平置き、画像は `images/<名前>/`）の両方で使う。

旧 pon から引き継ぐのは **名前とロゴ（`assets/`）だけ**。コード・構成・作法は引き継がない。

## 方針

- **速さを最優先**。押してから結果まで 1 桁〜十数 ms を狙う。途中で pwsh・PowerShell・ターミナルの窓を立ち上げない。
- **Rust、単一 exe `pon`、コンソール無し（windows subsystem）**。
  - 起動時に `AttachConsole(ATTACH_PARENT_PROCESS)` を試み、ターミナルから呼ばれたときだけ標準出力に結果を書く。
  - 失敗の通知は常に小さなメッセージ窓（アイコンは標準の「×」ではなく色違いの pon、`assets/icon-error.*`。警告音も鳴らない）。成功は無言。
- **Zed のタスクからしか呼ばれない前提**（Stream Deck からも同じ exe）。人が CLI から打つ用途の UX は作らない。
- 入力は Zed がタスクに渡す環境変数 `ZED_FILE` `ZED_ROW` `ZED_COLUMN` `ZED_SELECTED_TEXT` と、サブコマンドの引数だけ。

## サブコマンド

| コマンド | 役目 |
|---|---|
| `pon paste` | クリップボードの画像を置き場に保存し、本文にリンクを入れる |
| `pon rename` | `![](新,旧)` の行で、画像を改名して置き場へ動かし、参照を直す |
| `pon open [名前] [--today] [--project DIR]` | 名前からページを開く。無ければ作る |

### 共通：プロジェクトと設定

1. 起点は `--project` → `ZED_FILE` のフォルダ → 作業ディレクトリ、の順で最初にある物。
2. そこから上へ辿って最初の `pon.toml` を設定とし、そのフォルダをプロジェクトのルートとする。
3. 無ければ止まる（`paste` と `rename` だけは「画像は md と同じフォルダ」を既定として動く）。

### 設定 `pon.toml`

```toml
[images]
# 画像の置き場。md のフォルダからの相対。{stem} は md のファイル名（拡張子なし）。
# 省略時は md と同じフォルダ。
dir = "images/{stem}"

# ページの規則。上から順に名前へ当て、最初に一致した物を使う。
# match は正規表現、path はプロジェクトのルートからの相対、$1.. は捕捉。
[[pages]]
match = '^(\d{4}-\d{2}-\d{2})$'
path = "!/diary/$1/!.md"
template = """
---
title: $1
---
"""

[[pages]]
match = '^([a-z0-9-]+)$'
path = "!/$1/!.md"
template = """
---
title: $1
---
"""
```

メモ型は `[images] dir = "images/{stem}"` と `[[pages]] match = '^(.+)$' / path = "$1.md"` の 2 つで済む。

### `pon paste`

1. クリップボードから画像を取る（PNG があればそれ、無ければ DIB を PNG に変換）。無ければ止まる。
2. 名前を決める。`ZED_SELECTED_TEXT` が空でなく、使える文字（英数字 `-` `_`）だけならそれ。そうでなければ `image` の連番（`image.png`、`image_1.png`、…）。
3. 置き場（設定の `images.dir`、無ければ md のフォルダ）に `<名前>.png` で保存。フォルダが無ければ作る。同名があれば止まる。
4. 本文を書き換える。`ZED_FILE` をディスク上で開き、選択を名前に使ったときはその範囲（`ZED_ROW`/`ZED_COLUMN` から選択テキストの長さ分）を置き換え、そうでなければカーソル位置（選択の先頭）に、`![](<md のフォルダからの相対パス>)` を入れて保存する。Zed 側はタスクの `save: current` で実行前に保存しているので衝突しない。
   - `ZED_COLUMN` は UTF-8 のバイト位置（1 始まり）。実装時に日本語の行で確かめる。

### `pon rename`

先に作った TypeScript 試作の仕様を写す。

1. 対象の行は `ZED_SELECTED_TEXT` があればその中の全行、無ければ `ZED_ROW` の行。
2. 行から `![alt](a,b)` を拾う（カンマ前後の空白は無視）。無ければ止まる。
3. `a` と `b` のうち、md のフォルダから見て存在する方が旧名、無い方が新名。両方ある／両方無い → 止まる。
4. 新名の拡張子は省略可。書けば旧名と同じでなければ止まる。使える文字は英数字 `-` `_`。
5. 新しい置き場は設定の `images.dir`。`<置き場>/<新名>` が既にあれば止まる。
6. 全部の検査を通してから、ファイルを動かし、md の中で旧名を指す参照（`](旧)` `](./旧)` `](旧 "cap")`）をすべて新しい相対パスに直す。改行コードは保つ。

### `pon open`

1. **名前**：引数 → `--today`（今日の日付 `YYYY-MM-DD`）→ `ZED_SELECTED_TEXT` → 入力窓、の順で最初にある物。入力窓は Rust から直接出す Win32 の小窓で、今の前面の窓（Zed）に所有させて前に出す。キャンセルなら止まる。
2. **プロジェクト**：上の共通規則。
3. **規則**：`[[pages]]` を上から当て、最初の一致を採る。無ければ止まる。
4. **作る**：`path` のファイルが無ければ途中のフォルダごと作り、`template`（`$1..` を展開）を書く。あれば何もしない。パスがプロジェクトの外に出るなら止まる。
5. **開く**：`zed <path>` を呼ぶ（`cli_default_open_behavior: existing_window` で既存の窓に入る）。

## Zed への登録

全体設定（`%APPDATA%\Zed`）に書く。プロジェクトごとの `.zed/tasks.json` は置かない。

`tasks.json`：4 つとも `shell` を `cmd` にする（pwsh は起動に 190 ms、cmd は 8 ms）。`save: "current"`、`reveal: "never"`、`hide: "always"`。失敗は exe がメッセージ窓を出すのでターミナルを見せる必要がない。

| タスク | コマンド | キー（`keymap.json`） |
|---|---|---|
| pon: 貼る | `pon paste` | `ctrl-alt-v` |
| pon: 改名 | `pon rename` | `ctrl-alt-r` |
| pon: 今日 | `pon open --today` | `ctrl-alt-t` |
| pon: 名前 | `pon open` | `ctrl-alt-n` |

## Stream Deck

System → Open の path 欄に exe と引数を直接書く。`.vbs` も `.lnk` も要らない。コンソール無しの exe なので窓は出ない。

- 今日：`pon.exe open --today --project D:\notes`
- 名前：`pon.exe open --project D:\notes`（入力窓が出る）
- 決まったページ：`… open ideas --project D:\notes`

アイコンは `assets/icon-streamdeck*.png`。

## ビルドと配置

- `cargo install --path .` で cargo の bin フォルダに入る。PATH 済みなのでタスクは `pon` と書くだけ。
- release はサイズと起動を優先（`opt-level`、`lto`、`panic = "abort"`、`strip`）。
- exe のアイコンに `assets/icon.ico` を埋める。失敗窓のアイコン `assets/icon-error.ico` も ID 2 で埋める（`icon-error.svg` は `icon.svg` の色違いで、小さく出るので余白を詰めてある。ico は 16〜256px の PNG 入り）。

## 止まる条件（まとめ）

何も変えずに止まり、メッセージ窓で理由を出す。

- 設定が無い（`open`）／規則に当たらない／パスがプロジェクトの外
- クリップボードに画像が無い（`paste`）
- 行に `![](新,旧)` が無い／両方ある／両方無い／拡張子が違う／使えない文字／保存先に同名がある
- 名前が空、入力窓をキャンセル

## テスト

- 純粋な部分（行の解析、旧名と新名の決定、本文の書き換え、設定の読み込み、規則の当て方、パスの組み立て、選択範囲の置き換え）は単体テスト。TypeScript 試作のテスト 14 件はここに移す。
- ファイル操作は一時フォルダで実ファイルを使って確かめる。
- クリップボード・入力窓・`zed` CLI は薄い殻にして、手で確かめる。

## 範囲外

- md 以外の設定ファイルから画像を指している場合の書き換え。
- 人向けの CLI（ヘルプの体裁、対話）。
- Zed 本体の `ctrl+v` 貼り付けの置き換え（共存する。そちらで貼った物は `rename` で直す）。
