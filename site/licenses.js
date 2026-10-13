// Lizenzseite: liest licenses.json (erzeugt von scripts/licenses.py) und baut die Liste.
(() => {
  const de = document.documentElement.lang === "de";
  const T = de
    ? {
        count: (n) => `${n} Komponenten`,
        shown: (n, all) => `${n} von ${all} Komponenten`,
        none: "Keine Komponente gefunden.",
        kinds: { all: "Alle", rust: "Rust", js: "JavaScript", font: "Schriften", vendored: "Eingebunden" },
        by: "Von",
        source: "Quellcode",
        standard: "Das Paket enthält keine eigene Lizenzdatei. Angezeigt wird der Standardtext seiner Lizenz.",
        error: "Die Liste konnte nicht geladen werden.",
      }
    : {
        count: (n) => `${n} components`,
        shown: (n, all) => `${n} of ${all} components`,
        none: "No component found.",
        kinds: { all: "All", rust: "Rust", js: "JavaScript", font: "Fonts", vendored: "Bundled" },
        by: "By",
        source: "Source code",
        standard: "This package ships no license file of its own. Shown is the standard text of its license.",
        error: "The list could not be loaded.",
      };

  const list = document.getElementById("list");
  const search = document.getElementById("search");
  const filters = document.getElementById("filters");
  const status = document.getElementById("status");
  let data = null;
  let kind = "all";

  const el = (tag, props = {}, ...children) => {
    const node = Object.assign(document.createElement(tag), props);
    node.append(...children.filter((c) => c != null));
    return node;
  };

  function item(c) {
    const details = el("details", { className: "item" });
    const summary = el(
      "summary",
      {},
      el("span", { className: "name" }, c.name),
      el("span", { className: "ver" }, c.version),
      el("span", { className: "spacer" }),
      el("span", { className: "lic" }, c.license),
    );
    details.append(summary);
    // Inhalt erst beim Aufklappen bauen: hält die Seite mit vielen Einträgen schnell.
    details.addEventListener(
      "toggle",
      () => {
        if (!details.open || details.dataset.ready) return;
        details.dataset.ready = "1";
        const meta = el("p", { className: "meta" });
        if (c.authors.length) meta.append(`${T.by} ${c.authors.join(", ")} · `);
        meta.append(el("a", { href: c.url, rel: "noopener" }, T.source));
        const body = el("div", { className: "body" }, meta);
        if (c.standardText) body.append(el("p", { className: "note" }, T.standard));
        for (const key of c.texts) body.append(el("pre", {}, data.texts[key]));
        details.append(body);
      },
    );
    return details;
  }

  function render() {
    const q = search.value.trim().toLowerCase();
    const rows = data.components.filter(
      (c) =>
        (kind === "all" || c.kind === kind) &&
        (!q || c.name.toLowerCase().includes(q) || c.license.toLowerCase().includes(q)),
    );
    list.replaceChildren(...rows.map(item));
    if (!rows.length) list.append(el("p", { className: "empty" }, T.none));
    status.textContent =
      rows.length === data.components.length ? T.count(rows.length) : T.shown(rows.length, data.components.length);
  }

  function chips() {
    const counts = { all: data.components.length };
    for (const c of data.components) counts[c.kind] = (counts[c.kind] || 0) + 1;
    filters.replaceChildren(
      ...Object.keys(T.kinds)
        .filter((k) => counts[k])
        .map((k) => {
          const b = el("button", { type: "button" }, T.kinds[k], el("span", { className: "n" }, String(counts[k])));
          b.setAttribute("aria-pressed", String(k === kind));
          b.addEventListener("click", () => {
            kind = k;
            for (const x of filters.children) x.setAttribute("aria-pressed", String(x === b));
            render();
          });
          return b;
        }),
    );
  }

  fetch("licenses.json")
    .then((r) => r.json())
    .then((d) => {
      data = d;
      chips();
      render();
      search.addEventListener("input", render);
    })
    .catch(() => (status.textContent = T.error));
})();
