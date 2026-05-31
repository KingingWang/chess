#!/usr/bin/env python3
"""Re-subset assets/fonts/cjk{,-bold}.otf to exactly the characters used by the
UI string literals under crates/. Run after introducing new on-screen text.

Requires `pyftsubset` and the Noto Serif CJK source .ttc collections.
"""
from __future__ import annotations
import os, re, subprocess, sys, tempfile
from pathlib import Path
from fontTools.ttLib import TTCollection

ROOT = Path(__file__).resolve().parent.parent
SRC = {
    "regular": "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
    "bold":    "/usr/share/fonts/opentype/noto/NotoSerifCJK-Bold.ttc",
}
OUT = {
    "regular": ROOT / "assets/fonts/cjk.otf",
    "bold":    ROOT / "assets/fonts/cjk-bold.otf",
}
# Always-included Unicode ranges: ASCII, CJK symbols, halfwidth/fullwidth forms.
RANGES = "U+0020-007E,U+3000-303F,U+FF00-FF60"

def collect_chars() -> set[str]:
    chars: set[str] = set()
    pat = re.compile(r'"((?:[^"\\]|\\.)*)"')
    for path in (ROOT / "crates").rglob("*.rs"):
        text = path.read_text(encoding="utf-8", errors="ignore")
        for m in pat.finditer(text):
            for ch in m.group(1):
                # Anything non-ASCII goes in --text; ASCII covered by RANGES.
                if ord(ch) > 0x7E:
                    chars.add(ch)
    return chars

def extract_sc(ttc_path: str, out_path: Path) -> None:
    for f in TTCollection(ttc_path).fonts:
        name = f["name"].getDebugName(1) or ""
        if "Noto Serif CJK SC" in name:
            f.save(str(out_path))
            return
    raise RuntimeError(f"SC face not found in {ttc_path}")

def main() -> int:
    chars = collect_chars()
    print(f"collected {len(chars)} distinct non-ASCII chars from string literals")
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        for variant, ttc in SRC.items():
            full = tmp / f"{variant}-full.otf"
            extract_sc(ttc, full)
            out = OUT[variant]
            out.parent.mkdir(parents=True, exist_ok=True)
            cmd = [
                "pyftsubset", str(full),
                f"--text={''.join(sorted(chars))}",
                f"--unicodes={RANGES}",
                f"--output-file={out}",
            ]
            subprocess.run(cmd, check=True)
            size = out.stat().st_size
            print(f"  {out.relative_to(ROOT)}  {size:,} bytes")
    # Verify every collected char is in the produced cmap.
    from fontTools.ttLib import TTFont
    for variant, path in OUT.items():
        cmap = TTFont(str(path), lazy=True).getBestCmap()
        miss = [c for c in chars if ord(c) not in cmap]
        if miss:
            print(f"FAIL {variant}: missing {len(miss)}: {''.join(miss)}", file=sys.stderr)
            return 1
    print("ok: every collected char is present in both fonts")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
