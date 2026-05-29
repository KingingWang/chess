# 中国象棋 · Xiangqi (Rust + Bevy)

A cross-platform Chinese Chess game built with **Rust** and the **Bevy** game
engine, with a clean ECS architecture that decouples rules, rendering, AI, and
networking. Supports **local 2-player**, **human-vs-AI**, and **LAN online**
play.

## Workspace layout

```
chess/
├── crates/
│   ├── chess-core   # Rules engine: board, full move generation, all
│   │                #  competition rules, FEN/ICCS, result adjudication.
│   │                #  Pure logic, no I/O — 15 unit tests + perft(1..4).
│   ├── chess-ai     # Opponent AI: external UCI engine (Pikafish) integration
│   │                #  + built-in alpha-beta/quiescence fallback. Fully async.
│   ├── chess-net    # LAN networking over TCP (Tokio): JSON-lines protocol for
│   │                #  move / resign / draw, with handshake + color assignment.
│   └── chess-app    # Bevy front-end (bin `chess`): ECS systems wiring the
│                    #  above, bevy_ui menus/HUD, non-blocking AI & net bridges.
├── assets/          # Game assets (see assets/README.md for the art pipeline).
├── engines/         # Drop external UCI engine binaries here (see below).
└── docs/            # Architecture & rules notes.
```

## Building & running

```bash
# Run the game (desktop: Windows / Linux / macOS)
cargo run --release -p chess-app      # or: cargo run --release --bin chess

# Run the full test suite
cargo test --workspace

# Validate move generation to depth 4 (3,290,240 nodes)
cargo test -p chess-core --release -- --ignored perft_deep
```

## Game modes

Choose from the main menu:

* **Local 2-Player** — both sides on one screen.
* **Vs Computer (AI)** — you play Red; the engine plays Black.
* **Host LAN Game** — binds `0.0.0.0:9696` (override with `CHESS_BIND`).
* **Join LAN Game** — connects to `127.0.0.1:9696` (override with `CHESS_ADDR`,
  e.g. `CHESS_ADDR=192.168.1.50:9696`).

In-game: click a piece, then click a destination. Legal targets are
highlighted. Buttons: New Game / Resign / Offer Draw / Main Menu.

## AI engine

The AI is licence-clean (MIT/Apache only) and has two backends:

1. **Pikafish (recommended)** — the strongest MIT-licensed Xiangqi engine,
   driven over the standard **UCI** protocol. This is the path to the project's
   strength target (≥2600 ELO @ 3 s on an i7-12700K), met by Pikafish + its
   NNUE. Enable it via environment variables:

   ```bash
   export PIKAFISH_PATH=./engines/pikafish          # engine binary
   export PIKAFISH_EVAL=./engines/pikafish.nnue      # NNUE weights
   cargo run --release -p chess-app
   ```

   See `engines/README.md` for how to obtain platform binaries. Pikafish is
   **not** bundled (its NNUE weights have their own redistribution terms);
   the integration is fully implemented and used automatically when present.

2. **Built-in fallback** — a pure-Rust alpha-beta + quiescence search
   (`chess-ai::search`). It is correct and club-strength, used automatically
   when no external engine is configured or it fails to launch, so the game is
   always playable out of the box. It is **not** a 2600-ELO NNUE engine — that
   bar is delegated to Pikafish per the spec's "fall back to Pikafish" rule.

Both backends run off the render thread (the built-in via `spawn_blocking`, the
UCI engine via async process I/O) and never stall the frame loop.

## Honest scope notes

This repository delivers a complete, tested **software** foundation. Three
deliverables in the brief require human/external resources and are therefore
documented rather than fabricated:

* **Original UI artwork (Figma/PSD).** Cannot be authored by code. The renderer
  draws the board/pieces from primitives and is structured so original textures
  (PNG/SVG) drop in cleanly — see `assets/README.md` for the spec and pipeline.
* **A trained ≥2600-ELO NNUE network.** Training such a net is a large GPU/data
  effort; the spec's own fallback ("integrate Pikafish") is implemented.
* **Prebuilt platform AI binaries.** Pikafish binaries are redistributed under
  its own terms; `engines/README.md` explains fetching/placing them.

See `docs/ARCHITECTURE.md` and `docs/RULES.md` for details.

## Licence

This project's source: **MIT OR Apache-2.0** (see workspace `Cargo.toml`).
External engines retain their own licences.
