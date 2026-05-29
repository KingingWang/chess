# Implemented Rules (《中国象棋竞赛规则》)

## Movement

* **General/King (将/帅)** — one orthogonal step, confined to the 3×3 palace.
* **Advisor (士/仕)** — one diagonal step, confined to the palace.
* **Elephant (象/相)** — exactly two diagonal steps; **may not cross the river**;
  blocked if the midpoint ("象眼") is occupied — *塞象眼*.
* **Horse (马)** — one orthogonal then one diagonal; blocked if the orthogonal
  "leg" square is occupied — *蹩马腿*.
* **Chariot (车)** — any distance orthogonally until blocked.
* **Cannon (炮)** — moves like a chariot when not capturing; captures by jumping
  **exactly one** intervening screen piece ("炮架").
* **Pawn (兵/卒)** — one step forward; after crossing the river it may also step
  sideways; never backward.

## Special prohibitions

* **Flying General (白脸将)** — the two generals may not face each other on an
  open file; any move exposing this is illegal.
* A move may not leave one's own general in check (full legality filtering).

## Game end

* **Checkmate (将死)** — side to move is in check with no legal move → loss.
* **Stalemate (困毙)** — side to move has no legal move (not in check) → **loss**
  (Xiangqi differs from Western chess here).
* **Resignation / draw agreement** — explicit results.
* **Threefold repetition** — draw when neither side is forcing.
* **Perpetual check (长将)** — when a position recurs reached by one side always
  checking, that side must yield → loss.

## Scope notes on repetition adjudication

The clear, unambiguous **perpetual check (长将)** case is implemented, plus a
threefold-repetition draw fallback and a no-capture inactivity draw. The full
official adjudication of **长捉 / 一将一杀 / 一将一闲** (distinguishing chase,
idle, and kill in mixed repetitions) is intentionally out of scope; these are
notoriously intricate and rarely needed for casual/AI play. When an external
engine (Pikafish) is driving, it applies its own repetition rules during search.
