/**
 * dicom-overlay.js
 *
 * Browser-side DICOM slice loading and anatomy overlay for Qualia Anatomy.
 * Uses dicom-parser (CDN) — no Qualia engine changes required.
 *
 * Phase 1: 2D slice → Babylon.js billboard plane aligned to focused organ (heuristic).
 * Phase 2 (future): true patient-space registration via transform matrix from host/graph.
 */

const DicomOverlay = (() => {
  /** @type {import("@babylonjs/core") | null} */
  let babylon = null;
  let getScene = () => null;
  let organMap = null;

  let overlayMesh = null;
  let dynamicTexture = null;
  let slices = [];
  let activeSliceIndex = 0;
  let opacity = 0.72;
  let visible = true;
  let placement = { offsetX: 0, offsetY: 0, offsetZ: 0.28, scale: 0.85, rotationY: 0 };
  let mappedOrgan = null;
  let lastMeta = null;

  const DEFAULT_PLACEMENT = {
    "Heart": { offsetX: 0, offsetY: 0.05, offsetZ: 0.32, scale: 0.9, rotationY: 0 },
    "Lung": { offsetX: 0, offsetY: 0.08, offsetZ: 0.3, scale: 1.05, rotationY: 0 },
    "Liver": { offsetX: 0.12, offsetY: -0.02, offsetZ: 0.22, scale: 0.95, rotationY: -0.35 },
    "Brain (Allen)": { offsetX: 0, offsetY: 0.18, offsetZ: 0.12, scale: 1.0, rotationY: 0 },
    "Kidney (Left)": { offsetX: -0.14, offsetY: -0.06, offsetZ: 0.18, scale: 0.75, rotationY: 0.2 },
    "Kidney (Right)": { offsetX: 0.14, offsetY: -0.06, offsetZ: 0.18, scale: 0.75, rotationY: -0.2 },
    "Pancreas": { offsetX: 0.04, offsetY: -0.04, offsetZ: 0.2, scale: 0.7, rotationY: -0.15 },
    "Spleen": { offsetX: -0.1, offsetY: 0, offsetZ: 0.2, scale: 0.65, rotationY: 0.15 },
    "Small Intestine": { offsetX: 0, offsetY: -0.1, offsetZ: 0.2, scale: 0.9, rotationY: 0 },
    "Prostate": { offsetX: 0, offsetY: -0.14, offsetZ: 0.16, scale: 0.55, rotationY: 0 },
    "Uterus": { offsetX: 0, offsetY: -0.1, offsetZ: 0.2, scale: 0.7, rotationY: 0 },
    "Ovary (Left)": { offsetX: -0.1, offsetY: -0.12, offsetZ: 0.18, scale: 0.45, rotationY: 0 },
    "Ovary (Right)": { offsetX: 0.1, offsetY: -0.12, offsetZ: 0.18, scale: 0.45, rotationY: 0 },
    "Skin": { offsetX: 0, offsetY: 0, offsetZ: 0.42, scale: 1.35, rotationY: 0 },
    "Pelvis": { offsetX: 0, offsetY: -0.12, offsetZ: 0.16, scale: 1.0, rotationY: 0 },
    "Knee (Left)": { offsetX: -0.12, offsetY: -0.22, offsetZ: 0.14, scale: 0.55, rotationY: 0.1 },
    "Knee (Right)": { offsetX: 0.12, offsetY: -0.22, offsetZ: 0.14, scale: 0.55, rotationY: -0.1 },
    "Blood Vasculature": { offsetX: 0, offsetY: 0.02, offsetZ: 0.3, scale: 1.15, rotationY: 0 },
    "Eye (Left)": { offsetX: -0.08, offsetY: 0.12, offsetZ: 0.24, scale: 0.35, rotationY: 0.15 },
    "Eye (Right)": { offsetX: 0.08, offsetY: 0.12, offsetZ: 0.24, scale: 0.35, rotationY: -0.15 },
    "Spinal Cord": { offsetX: 0, offsetY: 0, offsetZ: -0.08, scale: 0.9, rotationY: 0 },
    "Thymus": { offsetX: 0, offsetY: 0.1, offsetZ: 0.22, scale: 0.4, rotationY: 0 },
    "Lymph Node": { offsetX: -0.08, offsetY: 0.04, offsetZ: 0.2, scale: 0.35, rotationY: 0 },
    "Large Intestine": { offsetX: 0, offsetY: -0.08, offsetZ: 0.18, scale: 0.95, rotationY: 0 },
    "Urinary Bladder": { offsetX: 0, offsetY: -0.14, offsetZ: 0.15, scale: 0.5, rotationY: 0 }
  };

  function ensureParser() {
    if (typeof window.dicomParser === "undefined") {
      throw new Error("dicom-parser is not loaded");
    }
  }

  function tagValue(dataSet, tag, fallback = "") {
    return dataSet.string(tag) || fallback;
  }

  function numericTag(dataSet, tag, fallback = 0) {
    const raw = dataSet.string(tag);
    if (raw == null || raw === "") return fallback;
    const parsed = Number(raw);
    return Number.isFinite(parsed) ? parsed : fallback;
  }

  function normalizeToken(value) {
    return String(value || "").toLowerCase().replace(/[^a-z0-9]+/g, " ").trim();
  }

  async function loadOrganMap() {
    try {
      const response = await fetch("./Knowledge/dicom-organ-map.json");
      if (response.ok) {
        organMap = await response.json();
      }
    } catch (err) {
      console.warn("[DicomOverlay] Could not load dicom-organ-map.json", err);
    }
  }

  function inferOrganFromTags(meta) {
    const haystack = normalizeToken(
      [meta.bodyPartExamined, meta.seriesDescription, meta.studyDescription, meta.protocolName]
        .filter(Boolean)
        .join(" ")
    );

    if (!organMap?.tagMatchers) return null;

    for (const matcher of organMap.tagMatchers) {
      const tokens = matcher.tokens || [];
      if (tokens.some(token => haystack.includes(normalizeToken(token)))) {
        return matcher.organ || null;
      }
    }
    return null;
  }

  function getWindowedPixelArray(dataSet, byteArray) {
    const rows = dataSet.uint16("x00280010") || 0;
    const columns = dataSet.uint16("x00280011") || 0;
    if (!rows || !columns) {
      throw new Error("DICOM missing Rows/Columns");
    }

    const bitsAllocated = dataSet.uint16("x00280100") || 8;
    const pixelRepresentation = dataSet.uint16("x00280103") || 0;
    const slope = numericTag(dataSet, "x00281053", 1);
    const intercept = numericTag(dataSet, "x00281052", 0);
    const windowCenter = numericTag(dataSet, "x00281050", null);
    const windowWidth = numericTag(dataSet, "x00281051", null);

    const element = dataSet.elements.x7fe00010;
    if (!element) {
      throw new Error("DICOM has no Pixel Data (7FE0,0010)");
    }

    let samples;
    if (bitsAllocated === 8) {
      samples = new Uint8Array(byteArray.buffer, element.dataOffset, element.length);
    } else if (bitsAllocated === 16) {
      if (pixelRepresentation === 1) {
        samples = new Int16Array(byteArray.buffer, element.dataOffset, element.length / 2);
      } else {
        samples = new Uint16Array(byteArray.buffer, element.dataOffset, element.length / 2);
      }
    } else {
      throw new Error(`Unsupported Bits Allocated: ${bitsAllocated}`);
    }

    let min = Infinity;
    let max = -Infinity;
    for (let i = 0; i < samples.length; i++) {
      const hounsfield = samples[i] * slope + intercept;
      if (hounsfield < min) min = hounsfield;
      if (hounsfield > max) max = hounsfield;
    }

    let wc = windowCenter;
    let ww = windowWidth;
    if (wc == null || ww == null) {
      wc = (min + max) / 2;
      ww = Math.max(1, max - min);
    }

    const photometric = tagValue(dataSet, "x00280004", "MONOCHROME2").toUpperCase();
    const invert = photometric === "MONOCHROME1";

    const lower = wc - ww / 2;
    const upper = wc + ww / 2;
    const out = new Uint8ClampedArray(rows * columns * 4);

    for (let i = 0; i < rows * columns; i++) {
      const hounsfield = samples[i] * slope + intercept;
      let gray = 0;
      if (hounsfield <= lower) gray = 0;
      else if (hounsfield >= upper) gray = 255;
      else gray = Math.round(((hounsfield - lower) / (upper - lower)) * 255);
      if (invert) gray = 255 - gray;

      const offset = i * 4;
      out[offset] = gray;
      out[offset + 1] = gray;
      out[offset + 2] = gray;
      out[offset + 3] = 255;
    }

    return { rows, columns, pixels: out, wc, ww, photometric };
  }

  async function parseDicomFile(file) {
    ensureParser();
    const byteArray = new Uint8Array(await file.arrayBuffer());
    const dataSet = window.dicomParser.parseDicom(byteArray);
    const rendered = getWindowedPixelArray(dataSet, byteArray);

    const meta = {
      fileName: file.name,
      patientName: tagValue(dataSet, "x00100010"),
      studyDescription: tagValue(dataSet, "x00081030"),
      seriesDescription: tagValue(dataSet, "x0008103e"),
      bodyPartExamined: tagValue(dataSet, "x00180015"),
      modality: tagValue(dataSet, "x00080060"),
      seriesInstanceUID: tagValue(dataSet, "x0020000e"),
      instanceNumber: numericTag(dataSet, "x00200013", 0),
      rows: rendered.rows,
      columns: rendered.columns,
      windowCenter: rendered.wc,
      windowWidth: rendered.ww
    };

    return {
      meta,
      canvas: renderSliceCanvas(rendered.rows, rendered.columns, rendered.pixels)
    };
  }

  function renderSliceCanvas(rows, columns, pixels) {
    const canvas = document.createElement("canvas");
    canvas.width = columns;
    canvas.height = rows;
    const ctx = canvas.getContext("2d");
    const imageData = ctx.createImageData(columns, rows);
    imageData.data.set(pixels);
    ctx.putImageData(imageData, 0, 0);
    return canvas;
  }

  function disposeOverlay() {
    if (dynamicTexture) {
      dynamicTexture.dispose();
      dynamicTexture = null;
    }
    if (overlayMesh) {
      overlayMesh.dispose();
      overlayMesh = null;
    }
  }

  function applyPlacementForOrgan(organName) {
    mappedOrgan = organName || mappedOrgan;
    const preset = DEFAULT_PLACEMENT[mappedOrgan] || DEFAULT_PLACEMENT.Heart;
    placement = { ...preset };
  }

  function updatePreviewCanvas() {
    const preview = document.getElementById("dicomPreview");
    const metaPanel = document.getElementById("dicomMeta");
    if (!preview || !metaPanel) return;

    const ctx = preview.getContext("2d");
    if (!slices.length) {
      preview.width = 256;
      preview.height = 160;
      ctx.fillStyle = "#111822";
      ctx.fillRect(0, 0, preview.width, preview.height);
      ctx.fillStyle = "#8c98ad";
      ctx.font = "12px Segoe UI, sans-serif";
      ctx.fillText("No DICOM loaded", 12, 24);
      metaPanel.textContent = "Load a .dcm file to map imaging over the anatomy model.";
      return;
    }

    const slice = slices[activeSliceIndex];
    preview.width = slice.canvas.width;
    preview.height = slice.canvas.height;
    ctx.drawImage(slice.canvas, 0, 0);

    const meta = slice.meta;
    metaPanel.innerHTML = [
      `<strong>${meta.modality || "—"}</strong> · ${meta.bodyPartExamined || "unknown region"}`,
      meta.seriesDescription || meta.studyDescription || meta.fileName,
      `Slice ${activeSliceIndex + 1} / ${slices.length}`,
      mappedOrgan ? `Mapped organ: ${mappedOrgan}` : "Mapped organ: not inferred"
    ].map(line => `<div>${line}</div>`).join("");
  }

  function updateSliceControls() {
    const slider = document.getElementById("dicomSlice");
    const label = document.getElementById("dicomSliceLabel");
    if (!slider || !label) return;

    slider.min = "0";
    slider.max = String(Math.max(0, slices.length - 1));
    slider.value = String(activeSliceIndex);
    slider.disabled = slices.length <= 1;
    label.textContent = slices.length ? `Slice ${activeSliceIndex + 1} of ${slices.length}` : "No slices";
  }

  function syncOverlayMesh() {
    const scene = getScene();
    disposeOverlay();
    if (!scene || !babylon || !slices.length || !visible) {
      return;
    }

    const slice = slices[activeSliceIndex];
    dynamicTexture = new babylon.DynamicTexture(
      "dicomOverlayTexture",
      { width: slice.canvas.width, height: slice.canvas.height },
      scene,
      false
    );
    const textureContext = dynamicTexture.getContext();
    textureContext.drawImage(slice.canvas, 0, 0);
    dynamicTexture.update();

    const aspect = slice.canvas.width / slice.canvas.height;
    const planeHeight = placement.scale;
    const planeWidth = planeHeight * aspect;

    overlayMesh = babylon.MeshBuilder.CreatePlane(
      "dicomOverlayPlane",
      { width: planeWidth, height: planeHeight },
      scene
    );

    const material = new babylon.StandardMaterial("dicomOverlayMaterial", scene);
    material.diffuseTexture = dynamicTexture;
    material.emissiveTexture = dynamicTexture;
    material.disableLighting = true;
    material.backFaceCulling = false;
    material.alpha = opacity;
    material.transparencyMode = babylon.Material.MATERIAL_ALPHABLEND;
    overlayMesh.material = material;

    overlayMesh.position = new babylon.Vector3(
      placement.offsetX,
      placement.offsetY,
      placement.offsetZ
    );
    overlayMesh.rotation = new babylon.Vector3(0, placement.rotationY, 0);
    overlayMesh.isPickable = false;
    overlayMesh.renderingGroupId = 2;
  }

  function groupBySeries(parsedFiles) {
    const groups = new Map();
    for (const entry of parsedFiles) {
      const key = entry.meta.seriesInstanceUID || entry.meta.fileName || "default";
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key).push(entry);
    }
    let best = [];
    for (const group of groups.values()) {
      if (group.length > best.length) best = group;
    }
    return best.length ? best : parsedFiles;
  }

  async function loadFiles(fileList) {
    const files = [...fileList].filter(file => /\.dcm$/i.test(file.name) || file.type === "application/dicom");
    if (!files.length) {
      throw new Error("No .dcm files selected");
    }

    const parsed = groupBySeries(await Promise.all(files.map(parseDicomFile)));
    parsed.sort((a, b) => {
      if (a.meta.seriesInstanceUID === b.meta.seriesInstanceUID) {
        return a.meta.instanceNumber - b.meta.instanceNumber;
      }
      return a.meta.fileName.localeCompare(b.meta.fileName);
    });
    slices = parsed;
    activeSliceIndex = Math.floor(slices.length / 2);

    lastMeta = slices[activeSliceIndex].meta;
    const inferredOrgan = inferOrganFromTags(lastMeta);
    if (inferredOrgan) {
      applyPlacementForOrgan(inferredOrgan);
    }

    updatePreviewCanvas();
    updateSliceControls();
    syncOverlayMesh();

    return {
      sliceCount: slices.length,
      inferredOrgan,
      meta: lastMeta
    };
  }

  function setSlice(index) {
    if (!slices.length) return;
    activeSliceIndex = Math.max(0, Math.min(slices.length - 1, index));
    updatePreviewCanvas();
    syncOverlayMesh();
  }

  function setOpacity(value) {
    opacity = Math.max(0.05, Math.min(1, value));
    if (overlayMesh?.material) {
      overlayMesh.material.alpha = opacity;
    }
  }

  function setVisible(value) {
    visible = value;
    if (!visible) {
      disposeOverlay();
    } else {
      syncOverlayMesh();
    }
  }

  function setPlacement(partial) {
    placement = { ...placement, ...partial };
    syncOverlayMesh();
  }

  function clear() {
    slices = [];
    activeSliceIndex = 0;
    mappedOrgan = null;
    lastMeta = null;
    disposeOverlay();
    updatePreviewCanvas();
    updateSliceControls();
  }

  function onSceneReady(scene, organName) {
    if (organName) {
      applyPlacementForOrgan(organName);
    }
    syncOverlayMesh();
  }

  function normalizePlacement(partial) {
    if (!partial) return null;
    return {
      offsetX: partial.offsetX ?? partial.offset_x ?? placement.offsetX,
      offsetY: partial.offsetY ?? partial.offset_y ?? placement.offsetY,
      offsetZ: partial.offsetZ ?? partial.offset_z ?? placement.offsetZ,
      scale: partial.scale ?? placement.scale,
      rotationY: partial.rotationY ?? partial.rotation_y ?? placement.rotationY
    };
  }

  function applyPayloadOverlay(payload) {
    const spec = payload?.dicomOverlay;
    if (!spec) return null;
    if (spec.organ) applyPlacementForOrgan(spec.organ);
    if (typeof spec.opacity === "number") setOpacity(spec.opacity);
    if (typeof spec.visible === "boolean") setVisible(spec.visible);
    const normalizedPlacement = normalizePlacement(spec.placement);
    if (normalizedPlacement) setPlacement(normalizedPlacement);
    updatePreviewCanvas();
    return spec;
  }

  async function init(options = {}) {
    babylon = options.babylon || window.BABYLON;
    getScene = options.getScene || (() => null);
    await loadOrganMap();
  }

  function getState() {
    return {
      loaded: slices.length > 0,
      sliceCount: slices.length,
      activeSliceIndex,
      mappedOrgan,
      opacity,
      visible,
      placement,
      meta: lastMeta
    };
  }

  return {
    init,
    loadFiles,
    setSlice,
    setOpacity,
    setVisible,
    setPlacement,
    applyPlacementForOrgan,
    applyPayloadOverlay,
    onSceneReady,
    clear,
    getState,
    inferOrganFromTags
  };
})();

if (typeof window !== "undefined") {
  window.DicomOverlay = DicomOverlay;
}
