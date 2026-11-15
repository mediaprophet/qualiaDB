(async function () {
  const host = document.getElementById("receipt-host");
  if (!host) return;

  const files = [
    "data/integrity/schemaorg-30-current-https.receipt.json",
    "data/integrity/oewn-subset.receipt.json",
    "data/integrity/oewn-strip-expected.receipt.json",
    "data/integrity/princeton-empty-lex.receipt.json",
    "data/integrity/datasources-matrix.json",
  ];

  function badgeClass(status) {
    const s = String(status || "").toUpperCase();
    if (s === "PASS" || s === "RDF_IMPORT_VERIFY") return "pass";
    if (s === "EXPECTED_DIVERGENCE" || s === "WARN") return "warn";
    return "fail";
  }

  for (const path of files) {
    try {
      const res = await fetch(path);
      if (!res.ok) throw new Error(String(res.status));
      const data = await res.json();
      const el = document.createElement("article");
      el.className = "receipt";
      const title =
        data.pair ||
        data.title ||
        (data.source_root ? "Datasources matrix" : path);
      const status =
        data.status ||
        (Array.isArray(data.entries)
          ? `${data.entries.length} entries`
          : "loaded");
      el.innerHTML = `
        <div class="badge ${badgeClass(data.status)}">${status}</div>
        <strong>${title}</strong>
        <div style="font-size:0.9rem;margin-top:0.35rem;opacity:0.9">
          ${data.summary || data.isomorphism || data.note || path}
        </div>
      `;
      host.appendChild(el);
    } catch (err) {
      const el = document.createElement("article");
      el.className = "receipt";
      el.innerHTML = `<div class="badge warn">pending</div><strong>${path}</strong>
        <div style="font-size:0.9rem;margin-top:0.35rem">Receipt not generated yet (${err.message}).</div>`;
      host.appendChild(el);
    }
  }
})();
