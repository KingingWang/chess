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

The shipped files are subset to just the glyphs the game needs (≈90 KB each),
extracted from the system `.ttc` collections:

```bash
# 1. extract the SC face from the .ttc into a standalone .otf
python3 - <<'PY'
from fontTools.ttLib import TTCollection
for src, out in [("/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc","cjk-full.otf"),
                 ("/usr/share/fonts/opentype/noto/NotoSerifCJK-Bold.ttc","cjk-bold-full.otf")]:
    for f in TTCollection(src).fonts:
        if "Noto Serif CJK SC" in (f["name"].getDebugName(1) or ""):
            f.save(out); break
PY

# 2. subset to the characters used by the UI + pieces
CHARS='中国象棋本地双人机对战创建主房间加入局域网联红黑方走棋你的回合等待对手思考中胜负和将死困毙欠行长认输求提楚河汉界漢难度简单普通大师返回菜单新对局帅仕相马车炮兵士象馬車砲卒退出开始'
pyftsubset cjk-full.otf      --text="$CHARS" --unicodes="U+0020-007E,U+3000-303F,U+FF00-FF60" --output-file=cjk.otf
pyftsubset cjk-bold-full.otf --text="$CHARS" --unicodes="U+0020-007E,U+3000-303F,U+FF00-FF60" --output-file=cjk-bold.otf
```
