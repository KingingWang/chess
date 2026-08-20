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
├── engines/         # Pikafish engine binaries + NNUE (git-ignored; embedded
│   │                #  into release builds — see engines/README.md).
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

### Headless soak test

The app can play itself for hours without a human, to shake out crashes:

```bash
CHESS_STRESS=1 CHESS_STRESS_SECS=10800 RUST_LOG=info cargo run -p chess-app
```

A random-legal-move Red plays full games against the AI Black with analysis
mode toggling between games, occasional undoes, and automatic restarts; it
logs a heartbeat per minute plus per-game results and exits cleanly when the
budget runs out (a panic shows up as a dead process instead).
`CHESS_STRESS_GAMES=N` caps the number of games. The same binary also supports
`CHESS_SHOT`/`CHESS_SCENE` self-screenshots (see `crates/chess-app/src/devshot.rs`).

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

The AI has two backends:

1. **Pikafish (bundled in release builds)** — the strongest open-source
   Xiangqi engine, driven over the standard **UCI** protocol. Release
   binaries **embed** the Pikafish executable + NNUE weights for the target
   platform (see `engines/README.md` and `crates/chess-app/build.rs`), so a
   player who downloads the game gets master-strength AI with zero setup. On
   first launch the engine is extracted to a per-user cache dir and run as a
   child process. During development, place the files under `engines/` (they
   are picked up automatically) or point at a custom build:

   ```bash
   export PIKAFISH_PATH=./engines/macos-arm64/pikafish   # engine binary
   export PIKAFISH_EVAL=./engines/pikafish.nnue           # NNUE weights
   cargo run --release -p chess-app
   ```

   Licensing: the Pikafish binary is **GPL-3.0** (it runs as a separate
   process; distributions that include it must ship `engines/Copying.txt` and
   point to its source), and the NNUE weights are **non-commercial use only**
   (`engines/NNUE-License.md`). See `engines/README.md` for details.

2. **Built-in fallback** — a pure-Rust alpha-beta + quiescence search
   (`chess-ai::search`). It is correct and club-strength, used automatically
   when no external engine is found or it fails to launch, so the game is
   always playable out of the box.

Both backends run off the render thread (the built-in via `spawn_blocking`, the
UCI engine via async process I/O) and never stall the frame loop.

**Move variety**: the opening book (`chess-ai::book`) samples weighted-randomly
among main lines, so every game starts differently (Easy and Extreme skip the
book — Easy for playfulness, Extreme so every move is engine-computed). The
built-in engine additionally picks uniformly among root moves within a small
score window of the best (±80/40/20 cp for Easy/Medium/Hard; Master and
Extreme are always full-strength best-move). Extreme is the top tier: 10 s
per move, all CPU cores, and a 256 MB hash for Pikafish.

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
