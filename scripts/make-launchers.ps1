# Stream Deck 用のショートカット（.lnk）を launchers\ に作る。
#
# Stream Deck の「System → Open」は path 欄に引数を続けて書いても渡してくれないので、
# 引数をショートカットに持たせ、ボタンからはショートカットを指す。
# ボタンの App/File 欄には、参照ボタンで選ばず、.lnk のフルパスを手で打つこと
# （参照で選ぶと exe 本体に解決されて引数が落ちる）。
#
# 一覧はリポジトリ直下の launchers.json（Git には入れない）に書く。無ければ下の例を使う。
#   [ { "name": "notes-today", "args": "open --today --project D:\\notes" }, ... ]
#
# 使い方:  powershell -NoProfile -File scripts\make-launchers.ps1

$exe = (Get-Command pon -ErrorAction Stop).Source
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$out = Join-Path $root "launchers"
New-Item -ItemType Directory -Force $out | Out-Null

$listFile = Join-Path $root "launchers.json"
if (Test-Path $listFile) {
    $buttons = Get-Content -Raw -Encoding UTF8 $listFile | ConvertFrom-Json
} else {
    $buttons = @(
        [pscustomobject]@{ name = "notes-today"; args = "open --today --project D:\notes" },
        [pscustomobject]@{ name = "notes-name";  args = "open --project D:\notes" }
    )
}

$shell = New-Object -ComObject WScript.Shell
foreach ($b in $buttons) {
    $path = Join-Path $out ("pon-" + $b.name + ".lnk")
    $lnk = $shell.CreateShortcut($path)
    $lnk.TargetPath = $exe
    $lnk.Arguments = $b.args
    $lnk.WorkingDirectory = Split-Path $exe
    $lnk.IconLocation = "$exe,0"
    $lnk.Save()
    Write-Output $path
}
