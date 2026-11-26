# Extracts the latest steady-state performance blocks from the newest
# victory-run-*.log into perf-summary.txt for quick sharing and analysis.
param(
    [string]$LogFile,
    [int]$TailLines = 180
)
$root = Split-Path -Parent $PSScriptRoot
if (-not $LogFile) {
    $latest = Get-ChildItem -Path $root -Filter "victory-run-*.log" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $latest) {
        Write-Host "Nenhum victory-run-*.log encontrado em $root. Rode play.ps1 com -SaveLog primeiro."
        exit 1
    }
    $LogFile = $latest.FullName
}
$patterns = @(
    'fps,',
    'VICTORY_ASYNC_SUBMIT',
    "the card's clock",
    'preparing per frame',
    'VICTORY_THIN',
    'VICTORY_LAND',
    'descriptor records the title copied',
    'our passes over copies',
    'shader writes looked for on the CPU',
    'new memory waited for',
    'new card memory waited for',
    'host pipeline library',
    'dropped .* without a host pipeline',
    'perf snapshot appended'
)
$regex = ($patterns -join '|')
$matched = Select-String -Path $LogFile -Pattern $regex | Select-Object -Last $TailLines | ForEach-Object { $_.Line }
$outPath = Join-Path $root "perf-summary.txt"
$header = @(
    "=== Perf summary extracted from $(Split-Path -Leaf $LogFile) ($(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')) ===",
    ""
)
($header + $matched) | Set-Content -Path $outPath -Encoding UTF8
Write-Host "Resumo salvo em: $outPath ($($matched.Count) linhas)"
