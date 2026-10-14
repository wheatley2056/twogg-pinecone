#!/usr/bin/env python3
"""Kontrastprüfung der Theme-Tokens (AK-38.3): Jeder Text-Token erreicht auf seinen
Hintergrund-Tokens in allen Themes mindestens 4,5:1. Läuft im CI; Exit-Code 1 bei Verstößen."""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CSS = ROOT / "ui" / "src" / "app.css"
# Wählbare Akzentfarben; `--accent-fg` wird daraus berechnet (ui/src/lib/settings.svelte.ts).
SETTINGS = ROOT / "ui" / "src" / "lib" / "settings.svelte.ts"

# Text-Token -> Hintergründe, auf denen er vorkommt.
PAIRS = {
    "--text": ["--bg", "--side", "--stage", "--bar", "--surface", "--surface-2", "--surface-active"],
    "--text-soft": ["--bg", "--side", "--surface", "--surface-2"],
    "--text-muted": ["--bg", "--side", "--surface", "--surface-2"],
    "--accent-fg": ["--bg", "--side", "--surface"],
    "--nadel": ["--bg", "--surface"],
    "--danger": ["--bg", "--surface"],
    "--ink": ["--accent"],
}
MIN = 4.5


def themes(css: str) -> dict[str, dict[str, str]]:
    blocks = re.findall(r"(:root(?:\[data-theme=\"(\w+)\"\])?)\s*\{([^}]*)\}", css)
    base: dict[str, str] = {}
    out: dict[str, dict[str, str]] = {}
    for _, name, body in blocks:
        values = dict(re.findall(r"(--[\w-]+):\s*(#[0-9a-fA-F]{6})\b", body))
        if not name:
            base.update(values)
            out["rinde"] = base
        else:
            out[name] = {**base, **values}
    return out


def shade(hex_color: str, t: float) -> str:
    """Wie `shade` in settings.svelte.ts: mit Weiß (t > 0) oder Schwarz (t < 0) mischen."""
    target, k = (255 if t > 0 else 0), abs(t)
    return "#" + "".join(
        f"{round(int(hex_color[i : i + 2], 16) * (1 - k) + target * k):02x}" for i in (1, 3, 5)
    )


def luminance(hex_color: str) -> float:
    def channel(c: int) -> float:
        c = c / 255
        return c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4

    r, g, b = (int(hex_color[i : i + 2], 16) for i in (1, 3, 5))
    return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)


def ratio(a: str, b: str) -> float:
    la, lb = sorted((luminance(a), luminance(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def main() -> int:
    failures = []
    accents = re.findall(r'color: "(#[0-9a-fA-F]{6})"', SETTINGS.read_text(encoding="utf-8"))
    if not accents:
        print("Keine Akzentfarben in settings.svelte.ts gefunden")
        return 1
    for (theme, base), accent in (
        (t, a) for t in themes(CSS.read_text(encoding="utf-8")).items() for a in accents
    ):
        tokens = {
            **base,
            "--accent": accent,
            "--accent-fg": shade(accent, -0.5) if theme == "winterland" else accent,
        }
        for fg, bgs in PAIRS.items():
            for bg in bgs:
                if fg not in tokens or bg not in tokens:
                    failures.append(f"{theme}: {fg} oder {bg} fehlt")
                    continue
                r = ratio(tokens[fg], tokens[bg])
                if r < MIN:
                    failures.append(
                        f"{theme}/{accent}: {fg} {tokens[fg]} auf {bg} {tokens[bg]} = {r:.2f}:1"
                    )
    for f in failures:
        print(f"Kontrast zu gering: {f}")
    if not failures:
        print("Kontrast ok: alle Text-Tokens erreichen 4,5:1.")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
