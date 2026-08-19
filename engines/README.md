# Bundled Pikafish Engine

The release build of `chess` embeds the [Pikafish](https://github.com/official-pikafish/Pikafish)
engine (the strongest open-source Xiangqi engine, a Stockfish derivative) so
players get master-strength AI out of the box with zero setup.

## Layout

```text
engines/
├── pikafish.nnue               # NNUE weights, shared by all platforms (~51 MB)
├── macos-arm64/pikafish        # macOS Apple Silicon
├── linux-x86_64/pikafish       # Linux x86_64 (SSE4.1/POPCNT build)
├── windows-x86_64/pikafish.exe # Windows x86_64
├── Copying.txt                 # GPL-3.0 (the engine's licence)
├── NNUE-License.md             # weights licence (non-commercial use only)
└── PIKAFISH-VERSION            # which upstream release these files came from
```

Everything except this README and `.gitkeep` is git-ignored (the binaries +
weights are large and have their own licences; see below). Download official
builds from <https://github.com/official-pikafish/Pikafish/releases> and the
network from <https://pikafish.org> and place them as above.

## How it is used

* **Debug builds** (`cargo run -p chess-app`): `src/engine_bundle.rs` finds the
  engine on disk — `PIKAFISH_PATH`/`PIKAFISH_EVAL` env vars first, then this
  `engines/` directory (via the executable path or the working directory).
* **Release builds**: `build.rs` embeds the platform binary + `pikafish.nnue`
  into the `chess` executable with `include_bytes!`. On first launch the bytes
  are extracted once to a per-user cache dir (e.g. `~/Library/Caches/chess/…`
  on macOS) and executed from there — Pikafish is a separate UCI *process*, so
  it must exist as a real file.
* If no engine is found (e.g. an unsupported platform), the app silently falls
  back to the built-in Rust engine, so the game is always playable.

`PIKAFISH_PATH` / `PIKAFISH_EVAL` always override the bundled copy, which is
handy for testing newer upstream builds.

## Licensing — read before redistributing

* **Pikafish binary: GPL-3.0** (see `Copying.txt`). It runs as a separate
  child process communicating over the UCI text protocol; the chess app itself
  is not a derivative work, but any distribution that includes the Pikafish
  binary must also include the GPL-3.0 text and a written offer / pointer to
  its source (<https://github.com/official-pikafish/Pikafish>, release noted in
  `PIKAFISH-VERSION`).
* **NNUE weights: non-commercial use only** (see `NNUE-License.md`). Do not
  ship the weights in any commercial product without permission from the
  Pikafish team.
