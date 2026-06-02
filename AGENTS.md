# Repository Guidelines

## Project Structure

Rust workspace with five crates for a Xiangqi (Chinese Chess) application:

```
crates/
├── chess-core/     # Pure rules engine: board, move generation, game adjudication
├── chess-ai/       # Built-in search/eval + UCI engine integration (Pikafish)
├── chess-net/      # LAN/relay protocol, E2E encryption, WebSocket over TLS
├── chess-app/      # Bevy GUI front-end (binary: chess)
└── chess-relay/    # Public relay server (binary: chess-relay)
```

Other directories: `assets/` (fonts, textures), `engines/` (external UCI binaries, not committed), `certs/` (dev TLS certs, git-ignored).

## Build, Test, and Development Commands

| Command | Purpose |
|---------|---------|
| `cargo build --workspace` | Build all crates (debug) |
| `cargo build --release -p chess-app` | Release client binary |
| `cargo build --release -p chess-relay` | Release relay binary |
| `cargo test --workspace` | Run all unit + integration tests |
| `cargo clippy --workspace` | Lint (must pass without warnings) |
| `cargo fmt --all` | Format all source files |
| `./certs/gen-dev-cert.sh` | Generate self-signed TLS certs for local testing |

## Coding Style & Naming Conventions

- **Edition**: Rust 2021. **Formatting**: standard `rustfmt` — run `cargo fmt --all` before committing.
- **Naming**: `snake_case` (functions/variables), `PascalCase` (types/traits), `SCREAMING_SNAKE_CASE` (constants).
- Each module file starts with a `//!` doc comment explaining its purpose.
- Use `thiserror` for library error types; `anyhow` only in binary entry points.

## Testing Guidelines

- Unit tests live in `#[cfg(test)] mod tests` blocks; integration tests in `crates/*/tests/`.
- Name tests descriptively: `start_position_has_44_legal_moves`, `relay_pairs_and_forwards_moves_end_to_end`.
- `chess-core` validates with perft against published reference values.
- `chess-relay` runs real TLS end-to-end tests on ephemeral ports.
- Always run `cargo test --workspace` before submitting changes.

## Commit & Pull Request Guidelines

Commits follow **Conventional Commits**:

```
type(scope): concise description
```

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`.  
Scopes match crate names: `core`, `ai`, `net`, `app`, `fonts`.

Pull requests should:
- Pass `cargo test --workspace` and `cargo clippy --workspace`.
- Include a brief explanation of *why* the change is needed.
- Reference related issues when applicable.

## Configuration

- Copy `relay.toml.example` → `relay.toml` for the server; `client.toml.example` → `client.toml` for the client.
- Environment variables (`PIKAFISH_PATH`, `CHESS_RELAY_HOST`, etc.) override file-based config — see example files for details.
