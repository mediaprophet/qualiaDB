# Bio / Medical / Histopathology CV — Native Implementation Map

**Branch:** `0.0.28`  
**Library:** `crates/qualia-vision/src/bio/`  
**Rule:** pure Rust; no Python product path; OpenCV/HistomicsTK/QuPath are **algorithm references**, not ABI.

## Status legend

| Tag | Meaning |
|-----|---------|
| **Present** | Native code + tests |
| **Partial** | Lite / simplified native version |
| **Queued** | Catalogue only; implement next wave |
| **Gated** | Needs Apache/MIT weights or clinical corpus |
| **Policy** | PHI / non-diagnosis / consent |

## Layout

```text
bio/
  histopathology/   Reinhard, Macenko, OD, background, SNMF-lite
  morphology/       watershed, Voronoi-Otsu, top-hat, nucleus features, OD index
  radiomics/        first-order, GLCM, shape 2D/3D lite
  medical/          HU window, MIP, isotropic NN, spectral unmix
  dicom_lite/       tag parse (LE explicit), anonymize, SUV formula
  tracking/         Crocker–Grier link, particle centroids
```

## HistomicsTK / stain

| Technique | Status | Module |
|-----------|--------|--------|
| Reinhard color normalization | Present | `histopathology/reinhard_normalize` |
| Macenko deconvolution | Present | `histopathology/macenko_deconvolution` |
| SNMF unmixing | Partial (lite multiplicative) | `histopathology/snmf_unmix_lite` |
| Background intensity sampling | Present | `histopathology/background_intensity_sample` |
| Optical density | Present | `histopathology/optical_density` |

## OpenSlide / WSI

| Technique | Status | Notes |
|-----------|--------|-------|
| Multi-res pyramid WSI tile | Queued | Need .svs reader; not in this wave |
| Tile filters during read | Partial | Use `cv/` filters on tiles once WSI lands |

## QuPath-class

| Technique | Status | Module |
|-----------|--------|--------|
| Pixel classifier RF/ANN | Gated | Interactive / weights |
| Cell detection watershed | Partial | `morphology/watershed_markers` + Voronoi-Otsu |
| Subcellular morphometrics | Partial | `morphology/nucleus_features` |
| Positive cell OD index | Present | `morphology/positive_od_threshold` |

## Cellpose / StarDist / SAM / deep

| Technique | Status |
|-----------|--------|
| Cellpose gradient tracking, Omnipose, StarDist, SplineDist, MicroSAM | **Gated** — algorithm stubs later; weights |
| Eikonal distance | Queued |

## ImageJ / TrackMate / Napari

| Technique | Status | Module |
|-----------|--------|--------|
| Extended-minima watershed | Partial | `extended_minima` + `watershed_markers` |
| Morphological top-hat (RMP-like) | Present | `morphological_tophat` |
| LAP / Crocker–Grier tracking | Partial | `tracking/crocker_grier_link` |
| Multi-dim raycasting | Queued | Renderer path |

## PathML / Ilastik / PlantSeg / Ultrack

| Technique | Status |
|-----------|--------|
| ABMIL, HIPPO, Multicut, Autocontext RF, Carving, ILP tracking | Queued / Gated |
| Voronoi-Otsu labeling | Partial Present | `voronoi_otsu_label` |

## ITK / radiomics / registration

| Technique | Status | Module |
|-----------|--------|--------|
| HU windowing | Present | `medical/hu_window` |
| Isotropic NN resample | Present | `medical/isotropic_resample_nn` |
| MIP | Present | `medical/mip_project` |
| Spectral unmixing | Present | `medical/spectral_unmix_nnls` |
| GLCM / first-order / shape | Present | `radiomics/*` |
| B-spline / SyN / Fast Marching / BET | Queued | |
| N4 bias | Queued | |

## DICOM

| Technique | Status | Module |
|-----------|--------|--------|
| Basic LE explicit tag parse | Partial | `dicom_lite/parse_dicom_tags_basic` |
| PHI anonymize tag map | Present | `dicom_lite/anonymize_tag_map` |
| SUV formula | Present | `dicom_lite/suv_from_activity` |
| Full decompress / NIfTI / RTStruct / PACS | Queued | |

## Deep behavioral / calcium / federated

DeepLabCut, SLEAP, CaImAn CNMF, FedAvg, TotalSegmentator, MONAI, etc. → **Gated** or **Queued** (weights + policy).

## Non-claims

- Not a full OpenSlide/QuPath/Cellpose replacement in one ship.
- Clinical outputs = **proposals**; non-diagnosis.
- PHI handling fails closed; do not store raw DICOM PHI in quins.
