# Architecture

## ECS decoupling (Bevy)

The `chess-app` crate is the only Bevy-aware crate. It keeps the four concerns
the brief calls out cleanly separated:

| Concern     | Owner                         | Bevy touchpoint                         |
|-------------|-------------------------------|-----------------------------------------|
| Rules/logic | `chess-core` (`CoreGame` res) | mutated only via `moves::apply_local_move` / input |
| Rendering   | `board_view`, `ui`            | `Update` systems + `RenderDirty` flag   |
| AI          | `chess-ai` via `ai_bridge`    | Tokio task → `crossbeam` channel → system |
| Networking  | `chess-net` via `net_bridge`  | Tokio task ↔ `crossbeam` channels        |

### Async without blocking the render thread

A single multi-threaded **Tokio runtime** lives in the `AsyncRuntime` resource.

* **AI**: when it is the engine's turn, `ai_bridge::request_ai_move` spawns a
  task that computes the move (`Ai::best_move`) and sends it back over a bounded
  channel. The built-in search runs inside `spawn_blocking` so the CPU-bound
  work never occupies an async worker or the Bevy schedule.
  `ai_bridge::poll_ai_move` applies the result on a later frame.
* **Networking**: `net_bridge::start_net` spawns a task that owns the
  `chess_net::Session`. It drains an outbound command channel (move/resign/draw)
  and forwards inbound peer messages to an event channel that
  `net_bridge::poll_net_events` reads each frame.

Because Bevy systems only ever `try_recv()` on lock-free channels, the frame
loop is never blocked regardless of search time or socket latency.

### State flow

`AppState::Menu` ⇄ `AppState::InGame`. `OnEnter`/`OnExit` systems build and tear
down the menu, board, and HUD. Game systems are gated by
`run_if(in_state(InGame))` and `.chain()`ed so input → AI → net → status →
redraw run in a deterministic order each frame.

## Move authority & validation

`CoreGame.game` is the single source of truth. Every move — local input, AI, or
peer — funnels through `chess_core::Game::make_move`, which **rejects illegal
moves**. Network peers are therefore never trusted blindly: a malformed/illegal
remote move is dropped locally.

## Rules engine (`chess-core`)

* `Board` — 9×10 placement + side to move; pseudo-legal & legal move generation;
  attack/check detection; FEN I/O.
* `Game` — history, undo, repetition tracking, and result adjudication
  (checkmate, stalemate-as-loss, threefold repetition draw, perpetual check).

Validated by 15 unit tests plus `perft` leaf-node counts matching published
reference values: 44 / 1920 / 79666 / 3290240 for depths 1–4.

## AI engine (`chess-ai`)

* `uci::UciEngine` — async driver for an external UCI engine (Pikafish).
* `search` — negamax + alpha-beta + quiescence, iterative deepening, MVV-LVA
  ordering, wall-clock budgeted; the licence-clean built-in fallback.
* `Ai` — unifies both behind one async `best_move`, auto-falling-back to the
  built-in engine if the UCI engine is unavailable.

## Networking (`chess-net`)

* `protocol` — newline-delimited JSON `Message`s.
* `connection` — async line-framed `Connection`, `Server`, `connect`.
* `session` — handshake + color assignment → `Session`.
