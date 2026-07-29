# Suggested trust catalog

**Rule:** empty until the principal curates content. Agents never invent CA PEMs.

## Drop-in (Timothy)

1. Add PEM files under `roots/` (optional).
2. Edit `catalog.json` entries:

```json
{
  "id": "au-example-root",
  "label": "Example AU community root",
  "jurisdiction": "AU",
  "kind": "pem_root",
  "material": "",
  "material_path": "roots/example.pem",
  "enabled_by_default": false,
  "notes": "Disabled until you enable in Trust UI",
  "source_url": null,
  "license": null
}
```

3. Package/desktop copies this folder into app resources when packaging scripts are extended (T1).
4. Runtime: user imports via Trust UI → live `webizen/trust_store.json`.
