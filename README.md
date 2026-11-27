# VIctory-9060XT16

A performance- and visual-focused fork of [Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)
(itself downstream of [VIctory](https://github.com/Marvisak/VIctory) and
[avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX)),
a compatibility layer that runs an Xbox Series title natively on Windows from
files the user supplies.

## Target & Philosophy

This fork combines **Belobug's AMD/RDNA4 performance base** with **avatarmanz's
console-accurate visual fixes**, specifically tuned and profiled for
**mid-range 6-core/12-thread CPUs + 32 GB RAM + AMD Radeon RX 9060 XT 16 GB (RDNA 4)**:

- **Goal:** A smooth, stable **30 fps** without ray tracing on **32 GB RAM + RDNA 4 (16 GB VRAM)** while keeping visuals as close to console as possible.
- **Data-driven optimization:** Every release is profiled on real hardware using `VICTORY_PROFILE`, the live HUD (`VICTORY_PANEL=1`), and `scripts\extract-perf.ps1`.
- **Best of both worlds:** Regularly pulls performance PRs from `Belobug/VIctory-Belobug-AMD` and visual/accuracy PRs from `avatarmanz/VIctory-7900XTX`, adding targeted CPU/D3D12 pool optimizations on top.

**This repository contains no game files, code or assets.** You need your own
copy of the game. It is not affiliated with or endorsed by the game's developer
or publisher.

## Reference Hardware & Measured Progress

Measured on Windows 11 with **AMD Ryzen 5 5500 (6c/12t Zen 3)**, **32 GB DDR4**,
and **AMD Radeon RX 9060 XT 16 GB (RDNA 4)**, Series S 1080p profile, ray tracing off:

| Build | Key Changes | Prison / Window Scene |
|---|---|---|
| `v0.0.1-alpha` | Base fork + initial RDNA4 crash fixes | ~9 fps (~110 ms/frame), `SUB` 95–111 ms, `MakeResident` stalls (~1.25 s/s) |
| `v0.0.4-alpha` | `VICTORY_WAIT_RESIDENT=textures`, `VICTORY_PRESSURE_PHYSICAL=1`, pooled `HostList::upload()`, `VICTORY_LANE_CORE=0`, `VICTORY_THIN_THREADS=4` | ~12 fps (84 ms/frame), `MakeResident` stalls eliminated, `SUB` 46.6 ms |
| `v0.0.5-alpha` | 11 upstream perf/fix PRs (`AddressHash`, `early-frame-load`, `read-gaps`, `dstorage` workers/priority, XMA priority) + raised scratch/upload pools & allocator retire cap | 11.0 fps (91 ms, **max spike cut 148 → 99 ms**), D3D12 kernel churn (`win32u.dll`) cut **-46.6%** (`500 → 267` samples), VRAM `9.5 GiB` |
| `v0.0.6-alpha` | Integrated `avatarmanz` visual fixes (`VICTORY_AMBIENT_REFLECT`, live TV/phone textures, BC7 Series S fix, detile retry, `-SkipIntro`) + Belobug O(n²) fixes (`#440`, `#437`, `#434`) + `SPARE_BATCH` pool `16 → 128` | Identified `VICTORY_PORT_CONSTANTS_ORDER=1` clamping 1,505 indirect draws individually (-2 fps regression) |
| `v0.0.7-alpha` | Disabled `VICTORY_PORT_CONSTANTS_ORDER=0` by default (opt-in for anti-flicker), clean PowerShell `-SaveLog` output, full `VIctory-9060XT16` identity | Current build |

## What Is Different in `VIctory-9060XT16`

- **Eliminates D3D12 kernel allocation/free thrashing on 32 GB hosts:**
  - Pooled `HostList::upload()` buffers by power-of-two bucket (`64 KiB`–`16 MiB`, capped at `256 MiB`, trimmed under physical-RAM pressure).
  - Raised scratch chunk pool (`384 MiB`), indirect batch UAV buffer pool (`128` buffers, trimmed under pressure), idle host-list pool (`320`), and allocator retirement cap (`4096` resets).
- **6-core/12-thread CPU scheduling defaults (`scripts\play.ps1`):**
  - `VICTORY_THIN_THREADS=4`, `VICTORY_LANE_CORE=0`, `VICTORY_DSTORAGE_WORKERS=4`, `VICTORY_DSTORAGE_PRIORITY=below`, `VICTORY_WAIT_RESIDENT=textures`, `VICTORY_PRESSURE_PHYSICAL=1`, `VICTORY_LIBRARY_STORE_LATER=1`, `VICTORY_SIZE_OWN_HANDLE=1`.
- **Console-accurate visual fixes without tanking frame rate:**
  - `VICTORY_AMBIENT_REFLECT=1.5` (bounds missed reflection rays indoors so eyes, wet skin and floors do not glow with open-sky light).
  - Live in-place texture tracking (`VICTORY_LIVE_TEXTURES`) for TV/phone video planes, BC7 mode 4/5 rotation fix for Series S textures, incomplete detile retry, and RDNA2 shader lowering fixes.
- **Built-in profiling & telemetry workflow:**
  - `scripts\play.ps1` supports `-Console series-s|series-x`, `-SkipIntro`, `-Panel`, and `-SaveLog`.
  - Press **`F9`** in-game anytime to append a compact memory/VRAM/switch snapshot to `victory-perf-snapshot.txt`.
  - Run `scripts\extract-perf.ps1` after a run to summarize key frame telemetry into `perf-summary.txt`.

## Useful Switches

### Renderer Patches

| Switch | What it does |
|---|---|
| `VICTORY_PORT_CONSTANTS=1` | The title's 174 MB constants heap is read by the card where the title writes it: no uploads, no fills |
| `VICTORY_PORT_CONSTANTS_ORDER=1` | Clamps in-place indirect draws individually after shader writes (reduces tree/lamp flicker, costs ~2 fps; off by default in `play.ps1`) |
| `VICTORY_PORT_DESCRIPTORS=1` | Experimental descriptor-table write hook (off by default) |
| `VICTORY_PORT_CBV=1` | Translated shaders read constant buffers in place as raw bytes |
| `VICTORY_PORT_GUARD=1` | Shared write-guard lock with per-block claims and parallel refresh planning |
| `VICTORY_PORT_JOURNAL=1` | Splits descriptor journal locks into 64 parts and requests heap records at title submit |

### Other Switches

| Switch | What it does |
|---|---|
| `--console series-s` | Series S 1080p profile (recommended on 32 GB RAM / RX 9060 XT) |
| `VICTORY_OUTPUT=1920x1080` | Title renders to 1080p output (Series X profile) |
| `VICTORY_UPSCALER=7` | Selects the title's FidelityFX upscaler (`6` = title's own TAA/TSR, `7` = FidelityFX; helps reduce dithered hair/window noise on Series S) |
| `VICTORY_AMBIENT_REFLECT=1.5` | Bounds missed reflection rays indoors by local ambient light (fixes glowing/blue eyes when RT is off; `0` disables) |
| `VICTORY_SKIP_BINK=1` | Skips one-shot intro/logo movies (`scripts\play.ps1 -SkipIntro`) |
| `VICTORY_FRAME_INTERVAL_MS=16.667` | Lets the title go above 30 fps (menus and videos reach 60) |
| `VICTORY_AMBIENT_SETTLE` | How fast ambient light settles after walking indoors (default `250,3`; `0` off) |
| `VICTORY_TEXTURE_STORE=1` | Keeps converted textures on disk between runs |
| `VICTORY_KBM_GTA=1` | Optional GTA V PC-style keyboard and mouse mapping (`F2` or `` ` `` toggles mouse-look; tune with `VICTORY_MOUSE_SENS` / `VICTORY_MOUSE_INVERT_Y=1`) |
| `VICTORY_D3D12_ADAPTER` | Selects a specific GPU by DXGI index or adapter name substring |
| `VICTORY_FSR_4K` | PC FidelityFX 4K upscaling (`quality`, `balanced`, `performance`; requires `VICTORY_FIDELITYFX_DIR`) |

## How to Run It

You need Windows 10/11 x64, a CPU with AVX2, a Direct3D 12 GPU with Shader Model
6.6 and Mesh Shaders, Rust (pinned toolchain), MSVC build tools, and your own
dump of the title.

```powershell
git clone https://github.com/lfmalta96/VIctory-9060XT16
cd VIctory-9060XT16
copy victory.example.toml victory.toml      # set your paths outside this folder
cargo build --locked --profile diag -p victory --features diagnostics
scripts\play.ps1 -Exe <path of your copy's executable> -Console series-s -SkipIntro -Panel -SaveLog
```

The launcher requests light DRED by default with `-Build diag` (requires
`--features diagnostics`). For a release binary built with that feature, pass
`-Build release -More @{VICTORY_D3D12_DRED='light'}`.

Use `scripts\play.ps1 -Safe` to disable experimental fast paths when diagnosing
an issue, and `-RayTracing` to enable the title's ray-traced indirect light.

Before the first run (optional, speeds up shader/pipeline warmup):

```powershell
target\diag\victory.exe shaders <the shader archives>
target\diag\victory.exe warm --used
```

More commands, debugging switches and upstream documentation:
[docs/README-fxceless.md](docs/README-fxceless.md),
[docs/findings.md](docs/findings.md), and [docs/replay.md](docs/replay.md).

## Credits

This project stands on the shoulders of the entire VIctory community. Full git
history and commit authorship are preserved:

- **[Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)**:
  direct upstream base, AMD/RDNA4 optimizations, ambient light stand-in,
  pipeline workers, residency management, and community PRs.
- **[avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX)**:
  visual accuracy fixes (`VICTORY_AMBIENT_REFLECT`, live TV/phone video texture
  tracking, BC7 Series S fix, detile retry, RDNA2 shader fixes, `-SkipIntro`,
  and renderer port ordering).
- **[VIctory](https://github.com/Marvisak/VIctory)** by Marvisak and
  contributors: the original loader, runtime services, Direct3D 12 backend, and
  shader recompiler.
- **[DexNom/VIctory](https://github.com/DexNom/VIctory)**: PC FidelityFX FSR
  4.1.2 / 3.1 bridge, ray tracing fixes, cube gradients, and sparse textures.
- **[fxceless/victory-fork](https://github.com/fxceless/victory-fork)**: the
  earlier fork carrying foundational performance and compatibility PRs.
- **[myokex/victory-amd](https://github.com/myokex/victory-amd)** (with Spexivus,
  joroyt, tightfall, and Joshua Odom), **[FQY55kai/ZJOU-VICTORY](https://github.com/FQY55kai/ZJOU-VICTORY)**,
  **Geckomacabre**, and **cxy0072019-sketch**: memory guards, title-memory reads,
  and texture read-back.
- **[XCL](https://github.com/ivall/xcl)** (GPL-2.0-only): referenced for
  understanding title behavior; no code from XCL is included.

## License

GPL-3.0-or-later, as VIctory. See [LICENSE](LICENSE) and
[docs/asset-policy.md](docs/asset-policy.md).
