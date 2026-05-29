# External UCI Engines

Place engine binaries here and point the app at them via environment variables.

## Pikafish (recommended, MIT-licensed)

Pikafish is the strongest open-source Xiangqi engine (a Stockfish derivative)
and is **MIT-licensed**. Its NNUE weights file is distributed separately and has
its own redistribution terms, so neither the binary nor the weights are bundled
in this repository.

```
engines/
├── pikafish            # (or pikafish.exe on Windows) — the engine binary
└── pikafish.nnue       # the NNUE weights file
```

Obtain official release binaries for Windows/Linux/macOS from the Pikafish
project releases, place them here, then:

```bash
export PIKAFISH_PATH=./engines/pikafish
export PIKAFISH_EVAL=./engines/pikafish.nnue
cargo run --release -p chess-app
```

The app auto-detects these variables at startup. If unset or the engine fails to
launch, the built-in Rust engine is used automatically (see crate `chess-ai`).

## Any other UCI Xiangqi engine

The integration (`chess_ai::UciEngine`) speaks standard UCI
(`uci`/`isready`/`position fen …`/`go movetime …`/`bestmove`), so any
MIT/Apache-licensed UCI engine works — set `PIKAFISH_PATH` to its binary.
Do **not** use GPL-v3 engines, per the project's licence constraint.
