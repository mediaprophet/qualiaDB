#!/usr/bin/env python3
"""
Build the QualiaDB Docs Hypermedia Asset Showcase Catalog & Stream Mirror.

Scans:
  - C:\\github\\eco-intent\\<species-slug>  -> github.com/intenteco/<species-slug>
  - C:\\github\\poiesis\\<thing-slug>       -> github.com/qualia-poiesis/<thing-slug>

Ensures each asset folder is a git repo with its canonical GitHub origin remote URL configured,
stages streamable .10d / .q42 / .m64 / .ttl / preview files under docs/assets-stream/<org>/<slug>/,
and writes docs/data/assets-catalog.json for docs/assets-showcase.html.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any

ECO_ROOT = Path(r"C:\github\eco-intent")
POIESIS_ROOT = Path(r"C:\github\poiesis")
DOCS_ROOT = Path(r"C:\github\qualiaDB\docs")
STREAM_ROOT = DOCS_ROOT / "assets-stream"
CATALOG_PATH = DOCS_ROOT / "data" / "assets-catalog.json"

SKIP_SLUGS = {
    "devdocs",
    "groundcover-pipeline",
    "game-demo",
    "webizen-os-shell",
    "website",
}

FLORA_GENERA = {
    "acacia", "anigozanthos", "atriplex", "austrostipa", "banksia", "bulbine",
    "bursaria", "callistemon", "chrysocephalum", "corymbia", "dichondra",
    "dicksonia", "eucalyptus", "hypochaeris", "lolium", "lomandra",
    "microlaena", "microseris", "plantago", "poa", "rytidosperma",
    "taraxacum", "themeda", "trifolium", "wahlenbergia", "xanthorrhoea",
}

FAUNA_GENERA = {
    "corvus", "dacelo", "iridomyrmex", "macropus", "ornithorhynchus",
    "phascolarctos", "varanus", "vombatus", "wallabia",
}


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def read_json_safe(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8", errors="replace"))
    except Exception:
        return {}


def parse_manifest_yaml_basic(path: Path) -> dict[str, Any]:
    """Lightweight parser for top-level scalar fields and lists in manifest.q42.yaml."""
    if not path.is_file():
        return {}
    text = path.read_text(encoding="utf-8", errors="replace")
    out: dict[str, Any] = {}
    for key in ("name", "slug", "title", "brief", "did", "domain", "epistemic_zone", "embodiment_mode", "quality_profile", "landing_page", "licence_policy"):
        m = re.search(rf'^{key}:\s*"?([^"\r\n]+)"?\s*$', text, re.MULTILINE)
        if m:
            out[key] = m.group(1).strip()

    coords_m = re.search(r"^coordinates_10d:\s*\[([^\]]+)\]", text, re.MULTILINE)
    if coords_m:
        try:
            out["coordinates_10d"] = [float(x.strip()) for x in coords_m.group(1).split(",")]
        except Exception:
            pass

    dims: dict[str, float] = {}
    for dkey in ("length_m", "width_m", "height_m", "curb_mass_kg", "dry_mass_kg", "canopy_spread_m", "root_depth_m"):
        dm = re.search(rf"^\s+{dkey}:\s*([0-9.+-eE]+)", text, re.MULTILINE)
        if dm:
            try:
                dims[dkey] = float(dm.group(1))
            except Exception:
                pass
    if dims:
        out["dimensions_si"] = dims

    # Count components / organs and animation_clips
    comp_Section = re.findall(r"^\s+-\s+id:\s*\"?([a-zA-Z0-9_.-]+)\"?\s*\n\s+did:", text, re.MULTILINE)
    if comp_Section:
        out["component_ids"] = comp_Section

    clip_matches = re.findall(
        r"^\s+-\s+id:\s*\"?([a-zA-Z0-9_.-]+)\"?\s*\n\s+label:\s*\"([^\"]+)\"\s*\n\s+duration_s:\s*([0-9.]+)",
        text,
        re.MULTILINE,
    )
    if clip_matches:
        out["animation_clips"] = [
            {"id": cid, "label": clabel, "duration_s": float(cdur)}
            for cid, clabel, cdur in clip_matches
        ]

    strata_matches = re.findall(
        r"^\s+-\s+stratum:\s*\"?(L[0-6][^\"]*)\"?\s*\n\s+scale_m:\s*\"?([^\r\n\"]+)\"?",
        text,
        re.MULTILINE,
    )
    if strata_matches:
        out["strata"] = [{"stratum": s, "scale_m": sc} for s, sc in strata_matches]

    return out


def ensure_git_repo_and_remote(repo_dir: Path, org: str, slug: str) -> dict[str, Any]:
    canonical_remote = f"https://github.com/{org}/{slug}.git"
    git_dir = repo_dir / ".gitignore"
    if not (repo_dir / ".git").exists():
        subprocess.run(["git", "init"], cwd=str(repo_dir), capture_output=True, text=True)

    rem_proc = subprocess.run(
        ["git", "remote", "get-url", "origin"],
        cwd=str(repo_dir),
        capture_output=True,
        text=True,
    )
    remote_url = rem_proc.stdout.strip() if rem_proc.returncode == 0 else ""
    if not remote_url:
        subprocess.run(
            ["git", "remote", "add", "origin", canonical_remote],
            cwd=str(repo_dir),
            capture_output=True,
            text=True,
        )
        remote_url = canonical_remote

    head_proc = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"],
        cwd=str(repo_dir),
        capture_output=True,
        text=True,
    )
    head_commit = head_proc.stdout.strip() if head_proc.returncode == 0 else None

    branch_proc = subprocess.run(
        ["git", "branch", "--show-current"],
        cwd=str(repo_dir),
        capture_output=True,
        text=True,
    )
    branch = branch_proc.stdout.strip() if branch_proc.returncode == 0 else "main"
    if not branch:
        branch = "main"

    return {
        "org": org,
        "repo_url": f"https://github.com/{org}/{slug}",
        "git_remote": remote_url,
        "default_branch": "main" if head_commit else branch,
        "head_commit": head_commit,
    }


def classify_asset(org: str, slug: str, title: str, brief: str) -> tuple[str, str]:
    if org == "intenteco":
        genus = slug.split("-")[0].lower()
        if slug == "homo-sapiens":
            return ("Biosphere · Human Anatomy & Physiology", "Human / CCF-HRA")
        if genus in FAUNA_GENERA:
            return ("Biosphere · Native Fauna", "Fauna")
        if genus in FLORA_GENERA:
            if genus in ("eucalyptus", "acacia", "banksia", "corymbia", "callistemon", "dicksonia", "bursaria", "xanthorrhoea"):
                return ("Biosphere · Canopy Trees, Shrubs & Ferns", "Flora · Canopy & Shrub")
            return ("Biosphere · Groundcover, Grasses & Forbs", "Flora · Groundcover & Forb")
        return ("Biosphere · Living Species", "Biosphere")
    else:
        if any(w in slug for w in ("rover", "car", "campervan", "trailer", "narrowboat")):
            return ("Made-World · Vehicles, Trailers & Watercraft", "Vehicles & Mobility")
        if any(w in slug for w in ("solar", "windpump", "comms-mast", "growing-station")):
            return ("Made-World · Off-Grid Energy, Water & Comms Infrastructure", "Infrastructure")
        return ("Made-World · Civic Buildings, Workshops & Timber Structures", "Buildings & Civic")


def humanize_slug(slug: str, org: str) -> str:
    parts = slug.split("-")
    if org == "intenteco" and len(parts) >= 2:
        return f"{parts[0].capitalize()} {' '.join(parts[1:])}"
    return " ".join(p.upper() if p in ("4wd",) else p.capitalize() for p in parts)


def find_review_receipt(repo_dir: Path) -> dict[str, Any]:
    direct = repo_dir / "previews" / "review-receipt.json"
    if direct.is_file():
        return read_json_safe(direct)
    for sub in sorted((repo_dir / "previews").glob("*/review-receipt.json")):
        return read_json_safe(sub)
    return {}


def process_asset(repo_dir: Path, org: str) -> dict[str, Any]:
    slug = repo_dir.name
    git_info = ensure_git_repo_and_remote(repo_dir, org, slug)

    synth_proj = read_json_safe(repo_dir / "synth.project.json")
    manifest_yaml = parse_manifest_yaml_basic(repo_dir / "manifest.q42.yaml")
    dist_receipt = read_json_safe(repo_dir / "dist" / "receipt.json")
    verify_outputs = read_json_safe(repo_dir / "dist" / "verify-outputs.json")
    review_receipt = find_review_receipt(repo_dir)

    title = (
        synth_proj.get("title")
        or manifest_yaml.get("title")
        or humanize_slug(slug, org)
    )
    brief = (
        synth_proj.get("brief")
        or manifest_yaml.get("brief")
        or dist_receipt.get("note")
        or f"{title} — Holographic 10D v2 & Hypermedia (.hmc) digital twin repository."
    )
    did = (
        synth_proj.get("did")
        or manifest_yaml.get("did")
        or (f"did:q42:taxon:{slug.replace('-', '_')}" if org == "intenteco" else f"did:q42:design:{slug.replace('-', '_')}")
    )
    work_packet = (
        synth_proj.get("work_packet")
        or dist_receipt.get("work_packet")
        or ("ECO-WP" if org == "intenteco" else "POIESIS-WP")
    )
    category_full, category_short = classify_asset(org, slug, title, brief)

    # Projection / geometry metrics
    proj = (
        review_receipt.get("projection")
        or (dist_receipt.get("native_review") or {}).get("projection")
        or {}
    )
    vertices = proj.get("vertex_count", 0)
    triangles = proj.get("triangle_count", 0)
    materials_count = 0
    bbox_diag_m = None

    for chk in verify_outputs.get("format_checks", []):
        if chk.get("format") == "10d":
            m = chk.get("metrics") or {}
            vertices = vertices or m.get("vertices", 0)
            triangles = triangles or m.get("triangles", 0)
            materials_count = max(materials_count, m.get("materials", 0))
            if m.get("bbox_diagonal_m"):
                bbox_diag_m = m.get("bbox_diagonal_m")

    bounds_min = proj.get("bounds_min", [-1.0, 0.0, -1.0])
    bounds_max = proj.get("bounds_max", [1.0, 2.0, 1.0])
    camera_target = proj.get("camera_target", [0.0, 0.85, 0.0])
    camera_distance = proj.get("camera_distance", 5.5)

    # Collect dist files and stage streamable subset into docs/assets-stream/<org>/<slug>/
    asset_stream_dir = STREAM_ROOT / org / slug
    asset_stream_dir.mkdir(parents=True, exist_ok=True)

    dist_dir = repo_dir / "dist"
    lods_dir = dist_dir / "lods"

    # 1. Identify all .10d variants
    base_10d = dist_dir / f"{slug}.10d"
    pbr_10d = dist_dir / f"{slug}-pbr.10d"
    lod_10ds = sorted(lods_dir.glob("*.10d")) if lods_dir.is_dir() else []

    ten_d_variants: list[dict[str, Any]] = []

    def add_10d_variant(fpath: Path, rel_repo_path: str, label: str, variant_id: str, stage_max_bytes: int = 1_600_000) -> None:
        if not fpath.is_file():
            return
        fsize = fpath.stat().st_size
        fsha = sha256_file(fpath)
        staged_rel = None
        if fsize <= stage_max_bytes:
            dest = asset_stream_dir / fpath.name
            shutil.copy2(fpath, dest)
            staged_rel = f"assets-stream/{org}/{slug}/{fpath.name}"
        ten_d_variants.append({
            "id": variant_id,
            "label": label,
            "filename": fpath.name,
            "repo_path": rel_repo_path,
            "bytes": fsize,
            "sha256": fsha,
            "stream_url": staged_rel,
            "raw_github_url": f"https://raw.githubusercontent.com/{org}/{slug}/main/{rel_repo_path}",
            "jsdelivr_url": f"https://cdn.jsdelivr.net/gh/{org}/{slug}@main/{rel_repo_path}",
        })

    # Always stage base .10d if <= 1.5 MB
    if base_10d.is_file():
        add_10d_variant(base_10d, f"dist/{base_10d.name}", "Canonical 10D v2 Master", "master", stage_max_bytes=1_500_000)

    # Check LODs (clean names first, or hashed streaming LODs)
    seen_lod_sizes: set[int] = set()
    for lpath in lod_10ds:
        lname = lpath.name
        lsize = lpath.stat().st_size
        # Avoid duplicate hashed copies of identical size in the same lods folder
        if lsize in seen_lod_sizes and re.search(r"-[0-9a-f]{12}\.10d$", lname):
            continue
        seen_lod_sizes.add(lsize)
        if "lod0" in lname:
            v_id, v_lbl = "lod0", "LOD 0 (High Detail)"
        elif "aboveground" in lname:
            v_id, v_lbl = "aboveground", "Above-Ground Canopy LOD"
        elif "lod1" in lname:
            v_id, v_lbl = "lod1", "LOD 1 (Mid Streaming)"
        elif "lod2" in lname:
            v_id, v_lbl = "lod2", "LOD 2 (Fast Horizon)"
        elif "rhizosphere" in lname:
            v_id, v_lbl = "rhizosphere", "Subterranean Rhizosphere & Mycorrhiza"
        elif "xray" in lname:
            v_id, v_lbl = "xray", "Anatomical X-Ray Manifold"
        elif "streaming" in lname:
            v_id, v_lbl = "streaming", "Streaming Cluster LOD"
        else:
            v_id, v_lbl = lname.replace(".10d", ""), lname
        # Stage all LODs up to 1.1 MB so large trees have LOD0/LOD1/LOD2 + Rhizosphere ready to stream!
        add_10d_variant(lpath, f"dist/lods/{lname}", v_lbl, v_id, stage_max_bytes=1_100_000)

    if pbr_10d.is_file():
        add_10d_variant(pbr_10d, f"dist/{pbr_10d.name}", "Film / Production16 PBR 10D", "pbr", stage_max_bytes=0)

    # Ensure at least one .10d is staged locally in assets-stream
    staged_variants = [v for v in ten_d_variants if v.get("stream_url")]
    if not staged_variants and ten_d_variants:
        # Force stage the smallest available .10d
        smallest_v = min(ten_d_variants, key=lambda v: v["bytes"])
        src_p = repo_dir / smallest_v["repo_path"]
        dest_p = asset_stream_dir / src_p.name
        shutil.copy2(src_p, dest_p)
        smallest_v["stream_url"] = f"assets-stream/{org}/{slug}/{src_p.name}"
        staged_variants = [smallest_v]

    # Pick default stream variant (prefer master if staged, else highest-detail staged LOD)
    default_10d = next((v for v in ten_d_variants if v["id"] == "master" and v.get("stream_url")), None)
    if not default_10d and staged_variants:
        default_10d = staged_variants[0]

    # 2. Stage .q42 semantic volumes
    q42_files: list[dict[str, Any]] = []
    for qpath in sorted(dist_dir.glob("*.q42")):
        dest = asset_stream_dir / qpath.name
        shutil.copy2(qpath, dest)
        q42_files.append({
            "filename": qpath.name,
            "repo_path": f"dist/{qpath.name}",
            "bytes": qpath.stat().st_size,
            "sha256": sha256_file(qpath),
            "stream_url": f"assets-stream/{org}/{slug}/{qpath.name}",
            "raw_github_url": f"https://raw.githubusercontent.com/{org}/{slug}/main/dist/{qpath.name}",
        })

    # 3. Stage .m64 matter container (only ~16 KB)
    m64_files: list[dict[str, Any]] = []
    for mpath in sorted(dist_dir.glob("*.m64")):
        dest = asset_stream_dir / mpath.name
        shutil.copy2(mpath, dest)
        m64_files.append({
            "filename": mpath.name,
            "repo_path": f"dist/{mpath.name}",
            "bytes": mpath.stat().st_size,
            "sha256": sha256_file(mpath),
            "stream_url": f"assets-stream/{org}/{slug}/{mpath.name}",
        })

    # 4. Record .emf and .qac and .hmc containers
    emf_files: list[dict[str, Any]] = []
    for epath in sorted(dist_dir.glob("*.emf")):
        emf_files.append({
            "filename": epath.name,
            "repo_path": f"dist/{epath.name}",
            "bytes": epath.stat().st_size,
            "sha256": sha256_file(epath),
            "raw_github_url": f"https://raw.githubusercontent.com/{org}/{slug}/main/dist/{epath.name}",
        })

    qac_files: list[dict[str, Any]] = []
    for apath in sorted(dist_dir.glob("*.qac")):
        qac_files.append({
            "filename": apath.name,
            "repo_path": f"dist/{apath.name}",
            "bytes": apath.stat().st_size,
            "sha256": sha256_file(apath),
            "raw_github_url": f"https://raw.githubusercontent.com/{org}/{slug}/main/dist/{apath.name}",
        })

    hmc_info = None
    hmc_path = dist_dir / f"{slug}.hmc"
    if hmc_path.is_file():
        hmc_sha = (dist_receipt.get("hmc") or {}).get("sha256") or sha256_file(hmc_path)
        hmc_info = {
            "filename": hmc_path.name,
            "repo_path": f"dist/{hmc_path.name}",
            "bytes": hmc_path.stat().st_size,
            "sha256": hmc_sha,
            "raw_github_url": f"https://raw.githubusercontent.com/{org}/{slug}/main/dist/{hmc_path.name}",
            "jsdelivr_url": f"https://cdn.jsdelivr.net/gh/{org}/{slug}@main/dist/{hmc_path.name}",
        }

    # 5. Stage primary ontology .ttl for 3D RDF Holograph Theatre streaming
    ont_dir = repo_dir / "ontology"
    ttl_info = None
    for cand_name in ("taxon.ttl", "artefact.ttl", "index.ttl"):
        tpath = ont_dir / cand_name
        if tpath.is_file():
            dest = asset_stream_dir / tpath.name
            shutil.copy2(tpath, dest)
            ttl_info = {
                "filename": tpath.name,
                "repo_path": f"ontology/{tpath.name}",
                "bytes": tpath.stat().st_size,
                "stream_url": f"assets-stream/{org}/{slug}/{tpath.name}",
                "raw_github_url": f"https://raw.githubusercontent.com/{org}/{slug}/main/ontology/{tpath.name}",
            }
            break

    # 6. Stage turnaround preview image & spectral slice
    preview_dir = repo_dir / "previews"
    preview_img_url = None
    for p_cand in ("turnaround.png", "explorer-turnaround.png", "three-quarter-view-assumed.png"):
        ppath = preview_dir / p_cand
        if ppath.is_file():
            dest = asset_stream_dir / ppath.name
            shutil.copy2(ppath, dest)
            preview_img_url = f"assets-stream/{org}/{slug}/{ppath.name}"
            break

    spectral_img_url = None
    for s_cand in ("spectral-slice.png", "homo-sapiens-bioelectric-slice.png"):
        spath = preview_dir / s_cand
        if spath.is_file():
            dest = asset_stream_dir / spath.name
            shutil.copy2(spath, dest)
            spectral_img_url = f"assets-stream/{org}/{slug}/{spath.name}"
            break

    # Source .vibe recipe metadata
    vibe_path = repo_dir / "source" / f"{slug}.vibe"
    if not vibe_path.is_file():
        vibe_path = repo_dir / "recipes" / f"{slug}.vibe"
    vibe_bytes = vibe_path.stat().st_size if vibe_path.is_file() else 0

    actuation_path = repo_dir / "source" / "actuation.vibe"
    actuation_bytes = actuation_path.stat().st_size if actuation_path.is_file() else 0

    return {
        "slug": slug,
        "org": org,
        "domain": "biosphere" if org == "intenteco" else "made_world",
        "domain_label": "IntentEco · Biosphere (SPC)" if org == "intenteco" else "Qualia Poiesis · Made-World",
        "category": category_full,
        "category_short": category_short,
        "title": title,
        "brief": brief,
        "did": did,
        "work_packet": work_packet,
        "quality_profile": synth_proj.get("quality_profile") or manifest_yaml.get("quality_profile") or "production16",
        "licence_policy": manifest_yaml.get("licence_policy") or ("SocialPermissiveCommons" if org == "intenteco" else "PermissiveCommons"),
        "epistemic_zone": manifest_yaml.get("epistemic_zone") or "reality",
        "embodiment_mode": manifest_yaml.get("embodiment_mode") or "holographic_10d_v2",
        "coordinates_10d": manifest_yaml.get("coordinates_10d") or [1.0, 2.0, 3.0, -35.2809, 149.1300, 580.0, 1732060800.0, 0.95, 0.99, 0.64],
        "dimensions_si": manifest_yaml.get("dimensions_si") or {},
        "animation_clips": manifest_yaml.get("animation_clips") or [],
        "component_count": len(manifest_yaml.get("component_ids") or []),
        "strata_count": len(manifest_yaml.get("strata") or []) or 7,
        "repository": git_info,
        "geometry": {
            "vertices": vertices,
            "triangles": triangles,
            "materials": materials_count,
            "bbox_diagonal_m": bbox_diag_m,
            "bounds_min": bounds_min,
            "bounds_max": bounds_max,
            "camera_target": camera_target,
            "camera_distance": camera_distance,
        },
        "default_stream_url": default_10d["stream_url"] if default_10d else None,
        "default_variant_id": default_10d["id"] if default_10d else None,
        "ten_d_variants": ten_d_variants,
        "q42_volumes": q42_files,
        "m64_containers": m64_files,
        "emf_fields": emf_files,
        "qac_acoustics": qac_files,
        "hmc_container": hmc_info,
        "ontology_ttl": ttl_info,
        "preview_image": preview_img_url,
        "spectral_image": spectral_img_url,
        "sources": {
            "vibe_bytes": vibe_bytes,
            "actuation_bytes": actuation_bytes,
        },
    }


def main() -> None:
    STREAM_ROOT.mkdir(parents=True, exist_ok=True)
    CATALOG_PATH.parent.mkdir(parents=True, exist_ok=True)

    assets: list[dict[str, Any]] = []

    for root, org in ((ECO_ROOT, "intenteco"), (POIESIS_ROOT, "qualia-poiesis")):
        if not root.is_dir():
            continue
        for sub in sorted(root.iterdir()):
            if not sub.is_dir() or sub.name in SKIP_SLUGS:
                continue
            if not (sub / "dist").is_dir():
                continue
            print(f"[{org}] Processing {sub.name}...")
            asset = process_asset(sub, org)
            assets.append(asset)

    eco_count = sum(1 for a in assets if a["org"] == "intenteco")
    poiesis_count = sum(1 for a in assets if a["org"] == "qualia-poiesis")
    total_hmc_bytes = sum((a["hmc_container"] or {}).get("bytes", 0) for a in assets)
    total_verts = sum(a["geometry"]["vertices"] for a in assets)
    total_tris = sum(a["geometry"]["triangles"] for a in assets)

    catalog = {
        "schema": "qualiadb.hypermedia-assets-catalog/v1",
        "version": "0.0.41",
        "generated_at": "2026-11-23T13:30:00+11:00",
        "summary": {
            "total_assets": len(assets),
            "intenteco_biosphere_count": eco_count,
            "qualia_poiesis_made_world_count": poiesis_count,
            "total_hmc_bytes": total_hmc_bytes,
            "total_vertices": total_verts,
            "total_triangles": total_tris,
            "organizations": [
                {
                    "id": "intenteco",
                    "name": "IntentEco — Biosphere & Living Species Commons",
                    "github_url": "https://github.com/intenteco",
                    "local_root": "C:\\github\\eco-intent\\<species-slug>",
                    "licence": "Social Permissive Commons (SPC) / CC BY-NC-ND 4.0",
                    "asset_count": eco_count,
                },
                {
                    "id": "qualia-poiesis",
                    "name": "Qualia Poiesis — Made-World Artefacts & Civic Infrastructure",
                    "github_url": "https://github.com/qualia-poiesis",
                    "local_root": "C:\\github\\poiesis\\<thing-slug>",
                    "licence": "Permissive Commons / CC BY-NC-ND 4.0",
                    "asset_count": poiesis_count,
                },
            ],
        },
        "assets": assets,
    }

    CATALOG_PATH.write_text(json.dumps(catalog, indent=2), encoding="utf-8")
    print(f"Wrote {len(assets)} assets ({eco_count} intenteco + {poiesis_count} qualia-poiesis) to {CATALOG_PATH}")


if __name__ == "__main__":
    main()
