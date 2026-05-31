# Bundled fonts

The UI and piece glyphs (帅 / 将 / 车 …) are rendered with subset copies of
**Noto Serif CJK SC** (Regular + Bold), chosen for a 国风 (classical Chinese)
serif feel.

| File          | Source face                       | License      |
|---------------|-----------------------------------|--------------|
| `cjk.otf`     | Noto Serif CJK SC — Regular       | SIL OFL 1.1  |
| `cjk-bold.otf`| Noto Serif CJK SC — Bold          | SIL OFL 1.1  |

These are **fonts, not artwork**: the SIL Open Font License (OFL 1.1) permits
free use, embedding, and redistribution, so bundling them does not conflict with
the project's "original UI artwork only" rule (which concerns board/piece/UI
*images*, all of which are still drawn from primitives in code).

## Reproducing the subset

The shipped files are subset to just the characters the game actually uses,
auto-discovered from every string literal under `crates/`. After adding or
changing on-screen text, rerun:

```bash
python3 scripts/subset_fonts.py
```

The script requires `pyftsubset` (from the `fonttools` Python package) and the
system `NotoSerifCJK-Regular.ttc` / `NotoSerifCJK-Bold.ttc` collections (Debian/
Ubuntu: `fonts-noto-cjk`). It verifies every collected character is present in
both produced fonts before exiting.
