#!/usr/bin/env python3
"""Erzeugt site/licenses.json: alle Fremdkomponenten, die im Launcher ausgeliefert werden.

- Rust: Abhängigkeiten von `pinecone-app` (nur normale Abhängigkeiten, keine Build- oder
  Dev-Abhängigkeiten) für Windows und Linux, Quellen aus dem Cargo-Cache.
- JavaScript: Produktions-Abhängigkeiten aus package.json samt deren Abhängigkeiten, dazu die
  Svelte-Laufzeit (wird in die Oberfläche kompiliert).
- Eingebunden: MC Skin Editor (ui/vendor).

Lizenztexte werden aus den Dateien der Pakete gelesen und nach Inhalt zusammengefasst.

Aufruf im Repo-Wurzelverzeichnis: python3 scripts/licenses.py
"""

import hashlib
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "site", "licenses.json")
TARGETS = ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"]
LICENSE_FILE = re.compile(r"^(licen[sc]e|copying|notice|unlicense)([-_.].*)?$", re.I)
MAX_TEXT = 64 * 1024

texts: dict[str, str] = {}


def add_text(text: str) -> str:
    text = text.replace("\r\n", "\n").strip()[:MAX_TEXT]
    key = hashlib.sha1(text.encode()).hexdigest()[:12]
    texts[key] = text
    return key


def license_texts(directory: str, extra: str | None = None) -> list[str]:
    files = []
    if extra:
        files.append(os.path.join(directory, extra))
    try:
        names = sorted(os.listdir(directory))
    except OSError:
        names = []
    for name in names:
        path = os.path.join(directory, name)
        if LICENSE_FILE.match(name) and os.path.isfile(path) and path not in files:
            files.append(path)
    keys = []
    for path in files:
        try:
            with open(path, encoding="utf-8", errors="replace") as f:
                content = f.read()
        except OSError:
            continue
        if content.strip():
            key = add_text(content)
            if key not in keys:
                keys.append(key)
    return keys


def normalize(expr: str | None) -> str:
    if not expr:
        return "Unbekannt"
    expr = expr.replace(" / ", " OR ").replace("/", " OR ")
    return re.sub(r"\s+", " ", expr).strip()


def clean_url(url: str | None) -> str | None:
    if not url:
        return None
    url = url.strip()
    url = re.sub(r"^git\+", "", url)
    url = re.sub(r"\.git$", "", url)
    url = re.sub(r"^git://", "https://", url)
    url = re.sub(r"^ssh://git@", "https://", url)
    return url if url.startswith("http") else None


def rust() -> list[dict]:
    pkgs = {}
    for target in TARGETS:
        meta = json.loads(
            subprocess.check_output(
                ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", target],
                cwd=ROOT,
            )
        )
        by_id = {p["id"]: p for p in meta["packages"]}
        nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
        root = next(p["id"] for p in meta["packages"] if p["name"] == "pinecone-app")
        seen, stack = set(), [root]
        while stack:
            pid = stack.pop()
            if pid in seen:
                continue
            seen.add(pid)
            for dep in nodes[pid]["deps"]:
                if any(k["kind"] is None for k in dep["dep_kinds"]):
                    stack.append(dep["pkg"])
        for pid in seen:
            p = by_id[pid]
            if p["source"] is not None:
                pkgs[(p["name"], p["version"])] = p
    out = []
    for p in pkgs.values():
        directory = os.path.dirname(p["manifest_path"])
        out.append(
            {
                "name": p["name"],
                "version": p["version"],
                "license": normalize(p.get("license")),
                "url": clean_url(p.get("repository")) or clean_url(p.get("homepage"))
                or f"https://crates.io/crates/{p['name']}",
                "authors": [re.sub(r"\s*<[^>]*>", "", a) for a in p.get("authors") or []],
                "kind": "rust",
                "texts": license_texts(directory, p.get("license_file")),
            }
        )
    return out


def npm_package(directory: str) -> dict | None:
    try:
        with open(os.path.join(directory, "package.json"), encoding="utf-8") as f:
            return json.load(f)
    except OSError:
        return None


def npm_dir(name: str, parent: str) -> str | None:
    """Node-Auflösung: node_modules vom Paketordner aufwärts bis zur Wurzel."""
    d = parent
    while True:
        cand = os.path.join(d, "node_modules", name)
        if os.path.isfile(os.path.join(cand, "package.json")):
            return cand
        if os.path.abspath(d) == ROOT:
            return None
        d = os.path.dirname(d)


def authors_of(pkg: dict) -> list[str]:
    a = pkg.get("author")
    if isinstance(a, dict):
        a = a.get("name")
    return [re.sub(r"\s*[<(][^>)]*[>)]", "", a).strip()] if isinstance(a, str) and a else []


SVELTE_RUNTIME = ["esm-env", "clsx"]


def javascript() -> list[dict]:
    root_pkg = npm_package(ROOT)
    # Die Svelte-Laufzeit samt der Pakete, die sie im Browser importiert.
    runtime = [("svelte", ROOT)] + [(n, os.path.join(ROOT, "node_modules", "svelte")) for n in SVELTE_RUNTIME]
    queue = [(name, ROOT) for name in root_pkg.get("dependencies", {})] + runtime
    found: dict[str, dict] = {}
    while queue:
        name, parent = queue.pop()
        directory = npm_dir(name, parent)
        # Typdefinitionen landen nicht im Build.
        if not directory or directory in found or name.startswith("@types/"):
            continue
        pkg = npm_package(directory)
        repo = pkg.get("repository")
        if isinstance(repo, dict):
            repo = repo.get("url")
        if isinstance(repo, str) and re.match(r"^[\w.-]+/[\w.-]+$", repo):
            repo = f"https://github.com/{repo}"
        lic = pkg.get("license")
        if isinstance(lic, dict):
            lic = lic.get("type")
        found[directory] = {
            "name": pkg["name"],
            "version": pkg.get("version", ""),
            "license": normalize(lic),
            "url": clean_url(repo) or clean_url(pkg.get("homepage"))
            or f"https://www.npmjs.com/package/{pkg['name']}",
            "authors": authors_of(pkg),
            "kind": "font" if pkg["name"].startswith("@fontsource/") else "js",
            "texts": license_texts(directory),
        }
        # Von Svelte nur die Laufzeit (oben), nicht den Compiler.
        if name != "svelte":
            queue += [(dep, directory) for dep in pkg.get("dependencies", {})]
    return list(found.values())


def vendored() -> list[dict]:
    directory = os.path.join(ROOT, "ui", "vendor", "mc-skin-editor")
    return [
        {
            "name": "MC Skin Editor",
            "version": "1.3.0",
            "license": "Apache-2.0",
            "url": "https://github.com/eheikes/mc-skin-editor",
            "authors": ["Eric Heikes"],
            "kind": "vendored",
            "texts": license_texts(directory),
        }
    ]


MIT_BODY = """Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE."""

# Erkennungsmerkmale der Volltexte, die andere Pakete mitbringen.
CANONICAL = {
    "Apache-2.0": r"^\s*Apache License\s+Version 2\.0, January 2004",
    "MPL-2.0": r"^\s*Mozilla Public License,? [Vv]ersion 2\.0",
    "BSD-3-Clause": r"Redistributions of source code must retain.*Neither the name",
}


def fallback(component: dict) -> None:
    """Pakete ohne Lizenzdatei bekommen den Standardtext ihrer Lizenz (erste passende Wahl)."""
    for spdx in re.findall(r"[A-Za-z0-9.-]+", component["license"]):
        if spdx == "MIT":
            holders = ", ".join(component["authors"]) or f"the {component['name']} authors"
            component["texts"] = [add_text(f"MIT License\n\nCopyright (c) {holders}\n\n{MIT_BODY}")]
        elif spdx in CANONICAL:
            pattern = re.compile(CANONICAL[spdx], re.S)
            key = next((k for k, t in sorted(texts.items()) if pattern.search(t)), None)
            if key:
                component["texts"] = [key]
        if component["texts"]:
            component["standardText"] = True
            return


def main() -> None:
    components = rust() + javascript() + vendored()
    for c in components:
        if not c["texts"]:
            fallback(c)
    components.sort(key=lambda c: (c["name"].lower().lstrip("@"), c["version"]))
    missing = [c["name"] for c in components if not c["texts"]]
    used = {k for c in components for k in c["texts"]}
    data = {
        "components": components,
        "texts": {k: v for k, v in sorted(texts.items()) if k in used},
    }
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, separators=(",", ":"))
    size = os.path.getsize(OUT) // 1024
    print(f"{len(components)} Komponenten, {len(data['texts'])} Lizenztexte, {size} KB -> {OUT}")
    if missing:
        print(f"Ohne Lizenzdatei ({len(missing)}): {', '.join(sorted(set(missing)))}", file=sys.stderr)


if __name__ == "__main__":
    main()
