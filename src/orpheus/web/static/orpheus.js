// Orpheus web UI — vanilla JS; no framework.
// All dynamic DOM construction uses createElement/textContent — no innerHTML
// assignment with user-derived data.

const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));

function el(tag, props = {}, children = []) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (v == null) continue;
    if (k === "class") node.className = v;
    else if (k === "text") node.textContent = v;
    else if (k === "dataset") Object.assign(node.dataset, v);
    else if (k in node) node[k] = v;
    else node.setAttribute(k, v);
  }
  for (const child of [].concat(children)) {
    if (child == null) continue;
    node.appendChild(typeof child === "string" ? document.createTextNode(child) : child);
  }
  return node;
}

function clear(node) { while (node.firstChild) node.removeChild(node.firstChild); }

// ---- tabs --------------------------------------------------------------

function switchTab(name) {
  $$("[role=tab]").forEach(btn => {
    const active = btn.dataset.tab === name;
    btn.classList.toggle("active", active);
    btn.setAttribute("aria-selected", String(active));
  });
  $$(".panel").forEach(p => {
    p.classList.toggle("active", p.dataset.panel === name);
  });
}

$$("[role=tab]").forEach(btn => btn.addEventListener("click", () => switchTab(btn.dataset.tab)));

// ---- dropzones ---------------------------------------------------------

function wireDropzone(zoneId, inputId, listId) {
  const zone = $("#" + zoneId);
  const input = $("#" + inputId);
  const list = $("#" + listId);

  zone.addEventListener("click", e => { if (e.target.tagName !== "LABEL") input.click(); });
  zone.addEventListener("dragover", e => { e.preventDefault(); zone.classList.add("dragover"); });
  zone.addEventListener("dragleave", () => zone.classList.remove("dragover"));
  zone.addEventListener("drop", e => {
    e.preventDefault();
    zone.classList.remove("dragover");
    const dt = new DataTransfer();
    for (const f of e.dataTransfer.files) dt.items.add(f);
    input.files = dt.files;
    renderFileList(input, list);
  });
  input.addEventListener("change", () => renderFileList(input, list));
}

function renderFileList(input, list) {
  clear(list);
  for (const f of input.files) {
    list.appendChild(el("li", { text: `${f.name}   ${formatBytes(f.size)}` }));
  }
}

function formatBytes(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
  return `${(n / 1024 / 1024).toFixed(1)} MiB`;
}

wireDropzone("dropzone", "scan-files", "file-list");
wireDropzone("extract-dropzone", "extract-file", "extract-file-list");

// ---- scan --------------------------------------------------------------

$("#scan-form").addEventListener("submit", async e => {
  e.preventDefault();
  const status = $("#scan-status");
  const files = $("#scan-files").files;
  if (!files.length) { setStatus(status, "no files selected", "err"); return; }

  const fd = new FormData();
  for (const f of files) fd.append("wallet", f);
  fd.append("passwords", $("#scan-passwords").value);
  fd.append("provider", $("#scan-provider").value);

  setStatus(status, "descending…");
  try {
    const r = await fetch("/api/scan", { method: "POST", body: fd });
    const body = await r.json();
    if (!r.ok) throw new Error(body.error || r.statusText);
    renderResults(body.results);
    switchTab("results");
    setStatus(status, `returned with ${body.results.length} wallet(s)`, "ok");
  } catch (err) {
    setStatus(status, err.message, "err");
  }
});

// ---- extract -----------------------------------------------------------

$("#extract-form").addEventListener("submit", async e => {
  e.preventDefault();
  const status = $("#extract-status");
  const files = $("#extract-file").files;
  if (!files.length) { setStatus(status, "select a wallet file", "err"); return; }

  const fd = new FormData();
  fd.append("wallet", files[0]);
  fd.append("passwords", $("#extract-passwords").value);
  fd.append("provider", "mock");

  setStatus(status, "opening…");
  try {
    const r = await fetch("/api/scan", { method: "POST", body: fd });
    const body = await r.json();
    if (!r.ok) throw new Error(body.error || r.statusText);
    renderResults(body.results);
    switchTab("results");
    setStatus(status, "done", "ok");
  } catch (err) {
    setStatus(status, err.message, "err");
  }
});

// ---- mnemonic ----------------------------------------------------------

$("#mnemonic-type").addEventListener("change", e => {
  $("#wordlist-field").hidden = e.target.value !== "blockchain";
});

$("#mnemonic-form").addEventListener("submit", async e => {
  e.preventDefault();
  const status = $("#mnemonic-status");
  const decoded = $("#mnemonic-decoded");
  const phrase = $("#mnemonic-phrase").value.trim();
  if (!phrase) { setStatus(status, "paste a phrase", "err"); return; }
  decoded.hidden = true;
  clear(decoded);

  const payload = {
    phrase,
    type: $("#mnemonic-type").value,
    passphrase: $("#mnemonic-passphrase").value,
    gap_limit: parseInt($("#mnemonic-gap").value, 10) || 20,
    wordlist: $("#mnemonic-wordlist").value.trim(),
  };

  setStatus(status, "deriving…");
  try {
    const r = await fetch("/api/mnemonic", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    const body = await r.json();
    if (!r.ok) throw new Error(body.error || r.statusText);

    if (body.decoded) {
      decoded.appendChild(el("h4", {
        text: `${body.decoded.version} — ${body.decoded.word_count} words`,
      }));
      decoded.appendChild(el("p", { class: "password", text: body.decoded.password }));
      decoded.appendChild(el("p", {
        style: "margin:0.5rem 0 0;color:var(--fg-faint);font-size:0.8rem;",
        text: "This password unlocks the blockchain.com wallet.aes.json payload.",
      }));
      decoded.hidden = false;
      setStatus(status, "decoded", "ok");
    } else if (body.keys) {
      renderResults([{
        source_file: "(mnemonic)",
        source_type: "bip39",
        key_count: body.keys.length,
        total_balance_sat: 0,
        keys: body.keys,
      }]);
      switchTab("results");
      setStatus(status, `${body.keys.length} keys derived`, "ok");
    }
  } catch (err) {
    setStatus(status, err.message, "err");
  }
});

// ---- demo --------------------------------------------------------------

$("#demo-btn").addEventListener("click", async () => {
  try {
    const r = await fetch("/api/demo", { method: "POST" });
    const body = await r.json();
    if (!r.ok) throw new Error(body.error || r.statusText);
    renderResults(body.results);
    switchTab("results");
  } catch (err) {
    alert(`demo failed: ${err.message}`);
  }
});

// ---- render ------------------------------------------------------------

function renderResults(results) {
  const body = $("#results-body");
  const lede = $("#results-lede");
  clear(body);

  if (!results || !results.length) {
    lede.textContent = "No wallets matched any extractor.";
    body.appendChild(el("p", { class: "empty", text: "silence in the hall" }));
    return;
  }

  const totalKeys = results.reduce((s, r) => s + r.key_count, 0);
  const totalBal = results.reduce((s, r) => s + r.total_balance_sat, 0);
  const withValue = results.filter(r => r.total_balance_sat > 0).length;

  clear(lede);
  lede.appendChild(document.createTextNode("Sorted through "));
  lede.appendChild(el("strong", { text: String(results.length) }));
  lede.appendChild(document.createTextNode(" wallet file(s), extracted "));
  lede.appendChild(el("strong", { text: String(totalKeys) }));
  lede.appendChild(document.createTextNode(" key(s). Keep anything showing bronze."));

  const summary = el("div", { class: "results-summary" }, [
    statDL("wallets", String(results.length)),
    statDL("keys", String(totalKeys)),
    statDL("with value", String(withValue), true),
    statDL("recovered", `${(totalBal / 1e8).toFixed(8)} BTC`, true),
  ]);
  body.appendChild(summary);

  for (const r of results) {
    const group = el("section", { class: "wallet-group" });
    const head = el("header", { class: "wallet-head" }, [
      el("h3", { title: r.source_file, text: shortenPath(r.source_file) }),
      el("div", {
        class: "meta",
        text: `${r.source_type} · ${r.key_count} keys · ${(r.total_balance_sat / 1e8).toFixed(8)} BTC`,
      }),
    ]);
    group.appendChild(head);

    if (r.error) {
      group.appendChild(el("p", {
        style: "color:var(--rust);",
        text: `error: ${r.error}`,
      }));
    } else if (r.keys.length) {
      group.appendChild(renderKeyTable(r.keys));
    }
    body.appendChild(group);
  }
}

function statDL(term, value, hit) {
  return el("dl", {}, [
    el("dt", { text: term }),
    el("dd", { class: hit ? "hit" : null, text: value }),
  ]);
}

function renderKeyTable(keys) {
  const wrap = el("div", { style: "overflow-x:auto;" });
  const table = el("table", { class: "keys" });
  const thead = el("thead", {}, [
    el("tr", {}, [
      el("th", { text: "path" }),
      el("th", { text: "address" }),
      el("th", { style: "text-align:right", text: "BTC" }),
      el("th", { style: "text-align:right", text: "txs" }),
      el("th", { text: "WIF" }),
    ]),
  ]);
  table.appendChild(thead);
  const tbody = el("tbody");
  const sorted = keys.slice().sort((a, b) => (b.balance_sat || 0) - (a.balance_sat || 0));
  for (const k of sorted) {
    const tr = el("tr", { class: k.balance_sat ? "hit" : null }, [
      el("td", { class: "path", text: k.derivation_path || "—" }),
      el("td", { class: "addr", text: k.address_compressed || "" }),
      el("td", { class: "btc", text: ((k.balance_sat || 0) / 1e8).toFixed(8) }),
      el("td", { class: "btc", text: String(k.tx_count || 0) }),
      el("td", { class: "wif", text: truncate(k.wif, 20) }),
    ]);
    tbody.appendChild(tr);
  }
  table.appendChild(tbody);
  wrap.appendChild(table);
  return wrap;
}

function shortenPath(p) {
  if (!p) return "";
  if (p.length <= 60) return p;
  const parts = p.split("/");
  if (parts.length < 3) return p;
  return `…/${parts.slice(-2).join("/")}`;
}

function truncate(s, n) {
  if (!s) return "";
  if (s.length <= n) return s;
  const h = Math.floor(n / 2);
  return `${s.slice(0, h)}…${s.slice(-(n - h - 1))}`;
}

function setStatus(el_, text, kind) {
  el_.textContent = text;
  el_.classList.remove("ok", "err");
  if (kind) el_.classList.add(kind);
}
