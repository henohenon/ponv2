# Stream Deck 用のショートカット（.lnk）を launchers\ に作る。
#
# Stream Deck の「System → Open」は path 欄に引数を続けて書いても渡してくれないので、
# 引数をショートカットに持たせ、ボタンからはショートカットを指す。
# ボタンの App/File 欄には、参照ボタンで選ばず、.lnk のフルパスを手で打つこと
# （参照で選ぶと exe 本体に解決されて引数が落ちる）。
#
# 使い方:  powershell -NoProfile -File scripts\make-launchers.ps1
# 直したいときは下の $buttons を編集して、もう一度実行する。

$exe = "pon.exe"
$out = Join-Path $PSScriptRoot "..\launchers"
New-Item -ItemType Directory -Force $out | Out-Null
$out = (Resolve-Path $out).Path

$buttons = @(
    @{ name = "notes-today";       args = "open --today --project D:\notes" },
    @{ name = "notes-name";        args = "open --project D:\notes" },
    @{ name = "blog-today";  args = "open --today --project D:\blog" },
    @{ name = "blog-name";   args = "open --project D:\blog" }
)

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
