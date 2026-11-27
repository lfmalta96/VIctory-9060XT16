# Starts the title with the settings this fork is played with (README.md, "How to run it").
#   scripts\play.ps1 -Exe <path of your own copy's executable>
# -Safe         without the newer, faster ways (the first thing to try when something is wrong)
# -RayTracing   the title's own ray-traced indirect light: lit interiors, about a third slower
# -Fsr4K        PC FidelityFX at 3840x2160 (quality, balanced or performance): FSR 4.1.2 where AMD's SDK offers it for the card, FSR 3.1 otherwise
# -SkipIntro    the logos and the intro movie are skipped (VICTORY_SKIP_BINK=1): faster to the menus
# -Build        target\diag (the default, the build all measurements are of) or target\release
#               Light DRED is requested by default for diag and requires --features diagnostics.
# -Console      series-x (default) or series-s (1080p profile, faster on 32 GB / mid-range GPUs)
# -Panel        show the live on-screen performance panel (VICTORY_PANEL=1; F3 toggles it)
# -SaveLog      save console output to victory-run-YYYYMMDD-HHMMSS.log in the Victory folder
# -More         other settings: -More @{VICTORY_OUTPUT='1920x1080'}
# Tip: press F9 in-game anytime to append a compact snapshot to victory-perf-snapshot.txt.
# Run it from a normal window, not as administrator: the driver then keeps its shader cache elsewhere.
param(
    [Parameter(Mandatory)][string]$Exe,
    [switch]$Safe,
    [switch]$RayTracing,
    [ValidateSet('quality', 'balanced', 'performance')][string]$Fsr4K,
    [switch]$SkipIntro,
    [ValidateSet('diag', 'release')][string]$Build = 'diag',
    [ValidateSet('series-x', 'series-s')][string]$Console = 'series-x',
    [switch]$Panel,
    [switch]$SaveLog,
    [hashtable]$More = @{}
)
$settings = @{
    VICTORY_TEXTURE_NO_WAIT = '1'; VICTORY_RELAY = '2'; VICTORY_WIDE_BACK = '1'; VICTORY_CARD_REHASH_RUNS = '1'
    VICTORY_JOIN_COPIES = '1'; VICTORY_INDIRECT_WINDOW = '1'; VICTORY_RELAY_COPY = '1'; VICTORY_CARD_STEPS = '2'
    VICTORY_INDIRECT_BATCH = '2'; VICTORY_MAP_THREADS = '1'; VICTORY_TITLE_PRIORITIES = '0'; VICTORY_CARD_AWAKE = '1'
    VICTORY_SHOW_AFTER = '1'; VICTORY_AMBIENT = '0.8'; VICTORY_AMBIENT_REFLECT = '1.5'; VICTORY_RESIDENCY = '1'
    # 32 GB RAM + RDNA4 switches: wait on residency only for textures (not plain buffers),
    # track physical-RAM pressure (avoids system pagefile false triggers), and avoid blocking
    # pipeline workers during library disk saves.
    VICTORY_WAIT_RESIDENT = 'textures'
    VICTORY_PRESSURE_PHYSICAL = '1'
    VICTORY_LIBRARY_STORE_LATER = '1'
    # AMD's compiler makes the compute pipelines it crashed on (a buffer's size through a handle of its own).
    VICTORY_SIZE_OWN_HANDLE = '1'
}
if ($Build -eq 'diag') { $settings.VICTORY_D3D12_DRED = 'light' }
if ($Safe) {
    $settings.VICTORY_UNDER_WIDE = '0'; $settings.VICTORY_LAND_ECHO = '0'
} else {
    $settings.VICTORY_THIN = '1'; $settings.VICTORY_JOIN_PARTS = '8'; $settings.VICTORY_UPLOAD_AHEAD = '1'
    # Best balance on 6-core/12-thread CPUs (Ryzen 5 5500): 5 list workers and no exclusive lane core.
    $settings.VICTORY_THIN_THREADS = '5'
    $settings.VICTORY_LANE_CORE = '0'
    # Fast DirectStorage decompression + smooth texture streaming on 6-core/12-thread CPUs (Ryzen 5 5500).
    $settings.VICTORY_DSTORAGE_WORKERS = '10'
    $settings.VICTORY_DSTORAGE_PRIORITY = 'normal'
    $settings.VICTORY_TEXTURE_STAGE_MB = '384'
    $settings.VICTORY_EARLY_TEXTURES_MS = '30'
    $settings.VICTORY_TEXTURE_STAGE_MS = '40'
    $settings.VICTORY_BAKE_TEXTURES_MS = '300'
    # Renderer patches 1, 2, 5, 6 and 7 + GPU-written readback skip (WC + >=128 KiB cached) + changed-only uploads.
    $settings.VICTORY_PORT_CONSTANTS = '1'
    $settings.VICTORY_PORT_DESCRIPTORS = '1'
    $settings.VICTORY_WC_NO_BACK = '2'
    $settings.VICTORY_SEND_CHANGED = '1'
    $settings.VICTORY_WC_KEEP_STORES = '1'
    # Keep indirect batching enabled by default (set to '1' via -More to reduce tree/lamp flicker at ~2 fps cost).
    $settings.VICTORY_PORT_CONSTANTS_ORDER = '0'
    $settings.VICTORY_PORT_CBV = '1'
    $settings.VICTORY_PORT_GUARD = '1'
    $settings.VICTORY_PORT_JOURNAL = '1'
    # Offload >=128 KiB buffer comparisons to GPU (sweet spot with VICTORY_WC_NO_BACK=2).
    $settings.VICTORY_CARD_BUFFERS_MIN_KB = '128'
}
if ($RayTracing) { $settings.VICTORY_BVH_BUILDER = '1' }
if ($Fsr4K) { $settings.VICTORY_FSR_4K = $Fsr4K.ToLowerInvariant() }
if ($Panel) { $settings.VICTORY_PANEL = '1' }
if ($SkipIntro) { $settings.VICTORY_SKIP_BINK = '1' }
foreach ($k in $More.Keys) { $settings[$k] = $More[$k] }

$victory = Join-Path $PSScriptRoot "..\target\$Build\victory.exe"
if (-not (Test-Path $victory)) { Write-Host "No $victory. Build it first (README.md)."; exit 1 }
if (-not (Test-Path $Exe)) { Write-Host "No $Exe."; exit 1 }
# (Other settings left over in this window go; where your files are stays.)
$paths = 'VICTORY_CONFIG', 'VICTORY_GAME_DIR', 'VICTORY_CACHE_DIR', 'VICTORY_DATA_DIR', 'VICTORY_FIDELITYFX_DIR'
$inherited = @(Get-ChildItem Env:VICTORY_* | Where-Object { $_.Name -notin $paths })
foreach ($entry in $inherited) { Remove-Item "Env:$($entry.Name)" }
foreach ($k in $settings.Keys) { Set-Item "Env:$k" $settings[$k] }
$discarded = $inherited | Where-Object {
    $_.Name -notin $More.Keys -and -not $_.Value.Equals([Environment]::GetEnvironmentVariable($_.Name))
} | Sort-Object Name
if ($discarded) {
    Write-Host "Discarded inherited values for $($discarded.Name -join ', '). Use -More to pass them explicitly."
}
$Exe = (Resolve-Path $Exe).Path
$root = Join-Path $PSScriptRoot '..'
Push-Location $root   # victory.toml is looked for here
try {
    if ($SaveLog) {
        $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
        $logFile = Join-Path $root "victory-run-$stamp.log"
        Write-Host "Saving full log to $logFile (press F9 in-game for instant perf snapshot; run scripts\extract-perf.ps1 after)"
        & $victory run $Exe --execute --console $Console 2>&1 | ForEach-Object { "$_" } | Tee-Object -FilePath $logFile
    } else {
        & $victory run $Exe --execute --console $Console
    }
} finally { Pop-Location }
