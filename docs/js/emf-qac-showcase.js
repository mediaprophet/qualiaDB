/**
 * EMF v4 & QAC v1 Scientific Field Observatory — QualiaDB 0.0.41
 * Self-contained: Wire Map Inspector, FDTD Poynting DSP, Mueller Polarimetry,
 * HOA Ambisonics BS.1770-5, DICOM Tomography, and Filterable Mode/Schema Catalogue.
 * Zero external fetch() calls — 100% offline & GitHub Pages compatible.
 */
(function() {
    'use strict';

    // --- 1. Binary Wire Layout Definitions ---
    const HEADER_FIELDS = [
        { name: "magic", range: "0..8", type: "bytes[8]", inv: "Must equal 'Q42EMF64' (EMF v4) or 'qac\\0\\0\\0\\0\\0' (QAC v1). Alphanumeric alias Q42AC064 rejected at admission.", err: "field_envelope::Error::Family" },
        { name: "version", range: "8..10", type: "u16 LE", inv: "Must equal 4 for EMF v4 or 1 for QAC v1. Obsolete v2/v3 rejected.", err: "field_envelope::Error::Version" },
        { name: "header_bytes", range: "10..12", type: "u16 LE", inv: "Must equal exactly 128 bytes. Variable headers prohibited.", err: "field_envelope::Error::Header" },
        { name: "flags", range: "12..16", type: "u32 LE", inv: "Bit 0: payload alignment (0 = 64B, 1 = 4096B). Bits 1..31 must be 0x00.", err: "field_envelope::Error::Header" },
        { name: "file_bytes", range: "16..24", type: "u64 LE", inv: "Exact total file extent in bytes; matches final payload alignment padding.", err: "field_envelope::Error::Header / Extent" },
        { name: "dir_offset", range: "24..32", type: "u64 LE", inv: "Must equal exactly 128 (directory immediately follows header).", err: "field_envelope::Error::Header" },
        { name: "entry_count", range: "32..36", type: "u32 LE", inv: "Directory row count; must not exceed limits.entries (default 4096).", err: "field_envelope::Error::Limit" },
        { name: "dir_entry_bytes", range: "36..38", type: "u16 LE", inv: "Must equal exactly 128 bytes per directory row.", err: "field_envelope::Error::Header" },
        { name: "index_version", range: "38..40", type: "u16 LE", inv: "Must equal 1 (unkeyed BLAKE3-256 cryptographic index).", err: "field_envelope::Error::Header" },
        { name: "meta_offset", range: "40..48", type: "u64 LE", inv: "Must equal exactly 128 + entry_count * 128.", err: "field_envelope::Error::Header" },
        { name: "meta_bytes", range: "48..56", type: "u64 LE", inv: "Metadata block sequence length with 8-byte padding; <= limits.metadata_bytes.", err: "field_envelope::Error::Limit" },
        { name: "index_blake3", range: "56..88", type: "bytes[32]", inv: "BLAKE3 with domain 'QUALIA-FIELD-INDEX-v1\\0' over zeroed header + dir + meta.", err: "field_envelope::Error::IndexDigest" },
        { name: "reserved_zero", range: "88..124", type: "bytes[36]", inv: "Must be exactly 36 zero bytes (0x00). Any non-zero byte triggers rejection.", err: "field_envelope::Error::Padding" },
        { name: "header_crc32c", range: "124..128", type: "u32 LE", inv: "Castagnoli CRC-32C (0x1EDC6F41) over bytes 0..124 with 124..128 zeroed.", err: "field_envelope::Error::Crc" }
    ];

    const DIR_FIELDS = [
        { name: "mode", range: "0..2", type: "u16 LE", inv: "Mode identifier (e.g. 0x0100..=0x0107, continua 0x0000..=0x0001).", err: "field_envelope::Error::Directory" },
        { name: "revision", range: "2..4", type: "u16 LE", inv: "Record schema revision; initially 1.", err: "field_envelope::Error::Directory" },
        { name: "flags", range: "4..8", type: "u32 LE", inv: "Bit 0: critical (fail-close); bit 1: time-indexed; bit 2: spatial-indexed.", err: "field_envelope::Error::Directory" },
        { name: "stream_id", range: "8..12", type: "u32 LE", inv: "Stream ID; >0 when time-indexed (flag bit 1 set).", err: "field_envelope::Error::Directory" },
        { name: "frame_id", range: "12..16", type: "u32 LE", inv: "Schema 2 frame ref; >0 when spatial-indexed (flag bit 2 set).", err: "field_envelope::Error::Directory" },
        { name: "start_tick", range: "16..24", type: "i64 LE", inv: "Start tick in referenced clock; 0 if not time-indexed.", err: "field_envelope::Error::Directory" },
        { name: "time_count", range: "24..32", type: "u64 LE", inv: "Temporal sample count; 0 if not time-indexed, >0 if time-indexed.", err: "field_envelope::Error::Directory" },
        { name: "spatial_key", range: "32..40", type: "u64 LE", inv: "Morton or Hilbert grid key; 0 if not spatial-indexed.", err: "field_envelope::Error::Directory" },
        { name: "payload_offset", range: "40..48", type: "u64 LE", inv: "Absolute file offset, aligned to 64 or 4096 bytes per header bit 0.", err: "field_envelope::Error::Padding / Extent" },
        { name: "stored_bytes", range: "48..56", type: "u64 LE", inv: "Exact stored payload bytes, excluding trailing zero-alignment padding.", err: "field_envelope::Error::Extent" },
        { name: "decoded_bytes", range: "56..64", type: "u64 LE", inv: "Exact decoded payload byte length; aggregate checked against limits.", err: "field_envelope::Error::Limit" },
        { name: "record_count", range: "64..72", type: "u64 LE", inv: "Chunk record element count; must be strictly > 0.", err: "field_envelope::Error::Directory" },
        { name: "stride", range: "72..76", type: "u32 LE", inv: "Mode-defined decoded record stride in bytes; must be strictly > 0.", err: "field_envelope::Error::Directory" },
        { name: "codec", range: "76..78", type: "u16 LE", inv: "Codec algorithm ID; 0 = raw uncompressed. Non-zero fails closed.", err: "field_envelope::Error::Codec" },
        { name: "reserved_0", range: "78..80", type: "u16 LE", inv: "Must be exactly 2 zero bytes (0x0000).", err: "field_envelope::Error::Padding" },
        { name: "payload_blake3", range: "80..112", type: "bytes[32]", inv: "BLAKE3-256 over exact stored_bytes. Whole-chunk verified before return.", err: "field_envelope::Error::PayloadDigest" },
        { name: "descriptor_id", range: "112..116", type: "u32 LE", inv: "Nonzero metadata block ID of Schema 3 PayloadDescriptor.", err: "field_envelope::Error::Directory" },
        { name: "reserved_1", range: "116..124", type: "bytes[8]", inv: "Must be exactly 8 zero bytes (0x00).", err: "field_envelope::Error::Padding" },
        { name: "entry_crc32c", range: "124..128", type: "u32 LE", inv: "CRC-32C of entry bytes 0..124 with 124..128 zeroed.", err: "field_envelope::Error::Crc" }
    ];

    // --- 2. Catalogue Database (EMF, QAC, Schemas, DICOM) ---
    const CATALOGUE_ITEMS = [
        { id: "0x0100", name: "instantaneous_E_B_v1", domain: "EMF v4", cat: "emf-modes", stride: "128B", schemas: "1, 2, 3, 4, 103", desc: "Instantaneous E/B vector snapshot (E in V/m, B in T, Poynting in W/m²). Mixed quantity profile." },
        { id: "0x0101", name: "polarimetric_ray_v1", domain: "EMF v4", cat: "emf-modes", stride: "128B", schemas: "1, 2, 3, 4, 103, 135", desc: "Polarimetric ray/beam. Stokes I, Q, U, V (I >= 0, Q²+U²+V² <= I²), centre freq, divergence rad." },
        { id: "0x0102", name: "yee_cartesian_v1", domain: "EMF v4", cat: "emf-modes", stride: "128B", schemas: "1, 2, 3, 4, 103, 133", desc: "Yee staggered grid state. E edge-staggered at t, H face-staggered at t+dt/2, J, ε_r, μ_r, σ." },
        { id: "0x0103", name: "sampled_spectrum_v1", domain: "EMF v4", cat: "emf-modes", stride: "64B + 8N", schemas: "1, 2, 3, 4, 103, 132", desc: "Continuous spectral density bins (1..4096 bins), temperature, schema 132 spectral axis." },
        { id: "0x0104", name: "mueller_scattering_v1", domain: "EMF v4", cat: "emf-modes", stride: "256B", schemas: "1, 2, 3, 4, 103, 134, 135", desc: "4x4 Mueller scattering matrix in row-major f64. Realizable Cloude coherency eigenvalues." },
        { id: "0x0105", name: "calibrated_iq_stft_v1", domain: "EMF v4", cat: "emf-modes", stride: "64B + 16N", schemas: "1, 2, 3, 4, 103, 107, 136", desc: "Calibrated RF I/Q baseband packet. Pairs of (I:f64, Q:f64) under I+iQ exp(-iωt) convention." },
        { id: "0x0106", name: "ionizing_radiation_v1", domain: "EMF v4", cat: "emf-modes", stride: "128B", schemas: "1, 2, 3, 4, 103, 107, 134", desc: "Ionizing radiation dose: linear/mass attenuation, fluence, absorbed dose (Gy), equivalent dose (Sv)." },
        { id: "0x0107", name: "bioelectric_dipole_v1", domain: "EMF v4", cat: "emf-modes", stride: "128B", schemas: "1, 2, 3, 4, 103, 107, 134, 137", desc: "Current dipole moment (A·m), extracellular potential (V), anatomical ontology reference." },
        { id: "0x0100", name: "pcm_preservation", domain: "QAC v1", cat: "qac-modes", stride: "chan*bytes", schemas: "1, 3, 4, 205, 210", desc: "Dense raw audio PCM: signed zeroes (+0.0/-0.0), subnormals preserved bit-exact without clipping." },
        { id: "0x0101", name: "stft_preservation", domain: "QAC v1", cat: "qac-modes", stride: "ch*bin*8B", schemas: "1, 3, 4, 201, 205", desc: "Spectral audio STFT/CQT: Hann inline window array, forward-unscaled-inverse-1/N normalization." },
        { id: "0x0102", name: "hoa_spatial_audio", domain: "QAC v1", cat: "qac-modes", stride: "4*(L+1)²B", schemas: "1, 2, 3, 4, 204, 205", desc: "Higher-Order Ambisonics: ACN channel index, SN3D/N3D normalization for orders L=0..3." },
        { id: "0x0103", name: "hrir_impulse_response", domain: "QAC v1", cat: "qac-modes", stride: "taps*4B", schemas: "1, 2, 3, 4, 204, 205", desc: "Head-Related / Room Impulse Response FIR taps with relative-to-excitation delay reference." },
        { id: "0x0104", name: "modal_resonance", domain: "QAC v1", cat: "qac-modes", stride: "32B", schemas: "1, 3, 4, 205, 206", desc: "Parametric modal oscillator: A * exp(-d*t) * sin(2πft + φ), decay rate, material reference." },
        { id: "0x0105", name: "perceptual_features", domain: "QAC v1", cat: "qac-modes", stride: "feats*4B", schemas: "1, 3, 4, 203, 205", desc: "Auditory filterbank features: MFCC, Mel, Sone, dB-SPL, ITU-R BS.1770-5 integrated loudness." },
        { id: "0x0106", name: "haptic_mechanical_sample", domain: "QAC v1", cat: "qac-modes", stride: "64B", schemas: "1, 2, 3, 4, 205, 207", desc: "Kinematic haptic touch: 3D local force (N), normal indentation (m), carrier frequency, contact area." },
        { id: "0x0107", name: "ultrasound_beam_summary", domain: "QAC v1", cat: "qac-modes", stride: "64B", schemas: "1, 2, 3, 4, 205, 208", desc: "Medical ultrasound echo: rarefactional pressure, MI/TI safety indices, axial tissue velocity." },
        { id: "0x0000", name: "pressure_velocity_continuum", domain: "QAC Continua", cat: "qac-modes", stride: "64B", schemas: "1, 2, 3, 4, 200, 205", desc: "Fluid acoustic continuum: acoustic pressure (Pa), 3D particle velocity (m/s), energy density (J/m³)." },
        { id: "0x0001", name: "elastic_stress_velocity", domain: "QAC Continua", cat: "qac-modes", stride: "64B", schemas: "1, 2, 3, 4, 200, 205", desc: "Solid continuum: 6-component Cauchy elastic stress tensor (Pa), 3D displacement vector." },
        { id: "Schema 1", name: "Clock", domain: "Shared", cat: "shared-schemas", stride: "TLV", schemas: "Mandatory", desc: "Rational timing: scale, epoch_seconds, epoch_nanoseconds, ticks_per_second numerator/denominator." },
        { id: "Schema 2", name: "CoordinateFrame", domain: "Shared", cat: "shared-schemas", stride: "TLV", schemas: "Mandatory", desc: "Spatial frame: origin_m[3], rotation quaternion xyzw (norm deviation <= 1e-12), grid dimensions." },
        { id: "Schema 3", name: "PayloadDescriptor", domain: "Shared", cat: "shared-schemas", stride: "TLV", schemas: "Mandatory", desc: "Chunk binding: quantity, units, channels, layout, sample rate rational, validity mask reference." },
        { id: "Schema 4", name: "Provenance", domain: "Shared", cat: "shared-schemas", stride: "TLV", schemas: "Mandatory", desc: "Origin lineage: role (measured/model/derived), algorithm, version, authority receipt digest." },
        { id: "Schema 5", name: "Instant", domain: "Shared", cat: "shared-schemas", stride: "TLV", schemas: "Optional", desc: "Checked rational timestamp: tick + subtick_num/subtick_den; exact half-open intervals [from, until)." },
        { id: "Schema 6", name: "AcquisitionView", domain: "Shared", cat: "shared-schemas", stride: "TLV", schemas: "Mode 0x0104", desc: "Raw sensor binding: maps tensor/event/correlation views into opaque acquisition original bytes." },
        { id: "Schema 7", name: "AcquisitionDerivation", domain: "Shared", cat: "shared-schemas", stride: "TLV", schemas: "Mode 0x0104", desc: "Mathematical derivation lineage: connects source views to derived physical quantities." },
        { id: "Schema 103", name: "EmfStreamExtension", domain: "EMF v4", cat: "emf-schemas", stride: "TLV", schemas: "EMF Master", desc: "30 authoritative keys: grid/brick dimensions, Hilbert curve order, component names, tensor shape." },
        { id: "Schema 105", name: "MaterialBinding", domain: "EMF v4", cat: "emf-schemas", stride: "TLV", schemas: "Schema 103", desc: "Electromagnetic material registry: container digest, material table, unique material identifiers." },
        { id: "Schema 106", name: "CoordinateTransform", domain: "EMF v4", cat: "emf-schemas", stride: "TLV", schemas: "Schema 105", desc: "Rigid/affine 4x4 transformation matrix between coordinate frames with validity ticks." },
        { id: "Schema 107", name: "Calibration", domain: "EMF v4", cat: "emf-schemas", stride: "TLV", schemas: "Schema 103", desc: "Affine scale, offset, and measurement uncertainty bound over half-open time interval [from, until)." },
        { id: "Schema 132", name: "SpectralAxis", domain: "EMF v4", cat: "emf-schemas", stride: "TLV", schemas: "Mode 0x0103", desc: "Continuous spectrum: frequency/wavelength coordinate values, bin integrals, Jacobian density scaling." },
        { id: "Schema 133", name: "ConstitutiveModel", domain: "EMF v4", cat: "emf-schemas", stride: "TLV", schemas: "Mode 0x0102", desc: "Debye/Lorentz dielectric dispersion poles, complex permittivity ε(ω) and magnetic permeability μ(ω)." },
        { id: "Schema 134", name: "ModelEvidence", domain: "EMF v4", cat: "emf-schemas", stride: "TLV", schemas: "Mode 0x0104/6", desc: "Epistemic classification: empirical, model estimate, hypothesis, counterfactual simulation." },
        { id: "Schema 135", name: "PolarizationBasis", domain: "EMF v4", cat: "emf-schemas", stride: "TLV", schemas: "Mode 0x0101/4", desc: "Transverse basis deterministic_transverse_v1 and Stokes convention I_Q_U_V_v1 in W/m²." },
        { id: "Schema 136", name: "Acquisition", domain: "EMF v4", cat: "emf-schemas", stride: "TLV", schemas: "Mode 0x0105", desc: "RF SDR acquisition parameters: LO phase, FFT window, complex convention I+iQ exp(-iωt)." },
        { id: "Schema 137", name: "BioContext", domain: "EMF v4", cat: "emf-schemas", stride: "TLV", schemas: "Mode 0x0107", desc: "Anatomical ontology IRI, observation modality, patient consent receipt provenance reference." },
        { id: "Schema 138", name: "PreservedAudio", domain: "EMF v4", cat: "emf-schemas", stride: "TLV", schemas: "Schema 103", desc: "Original audio stream link: PCM BLAKE3 digest, target channel purpose, calibrated units." },
        { id: "Schema 200", name: "Medium", domain: "QAC v1", cat: "qac-schemas", stride: "TLV", schemas: "Mode 0x0000/1", desc: "Acoustic medium parameters: fluid density, bulk modulus, shear modulus, ambient pressure." },
        { id: "Schema 201", name: "Transform (Rev 1)", domain: "QAC v1", cat: "qac-schemas", stride: "TLV", schemas: "Mode 0x0101", desc: "STFT specification: Hann window coefficients array in key 16, origin basis in key 17." },
        { id: "Schema 204", name: "SpatialHrir", domain: "QAC v1", cat: "qac-schemas", stride: "TLV", schemas: "Mode 0x0102/3", desc: "Spatial acoustics: HOA order (0..3, ACN, SN3D), HRIR tap count, receiver positions, excitation." },
        { id: "Schema 205", name: "ModelGrid", domain: "QAC v1", cat: "qac-schemas", stride: "TLV", schemas: "QAC Master", desc: "Tensor shape, multi-channel component names, layout semantics (time-channel, frame-channel-bin)." },
        { id: "Schema 210", name: "SignalEncoding", domain: "QAC v1", cat: "qac-schemas", stride: "TLV", schemas: "Mode 0x0100", desc: "Physical relation: digital-unrelated-to-pressure, raw-counts, or physical-base-units." },
        { id: "Track D01", name: "DICOM Source Custodian", domain: "DICOM", cat: "dicom", stride: "Part 10", schemas: "G-MIGRATE", desc: "Original complete DICOM objects; bounded exact archival with SHA-256 and BLAKE3 source digest." },
        { id: "Track D02", name: "LPS Frame Geometry", domain: "DICOM", cat: "dicom", stride: "Part 10", schemas: "G-SCIENCE", desc: "Patient Left-Posterior-Superior (LPS) coordinate frame of reference, slice spacing, thickness." },
        { id: "Track D03", name: "Transfer Syntax Matrix", domain: "DICOM", cat: "dicom", stride: "Part 10", schemas: "G-BYTES", desc: "Encapsulated codecs: Explicit VR LE, RLE Lossless, JPEG-LS, JPEG 2000, JPEG XL, Deflate." },
        { id: "Track D04", name: "Instrument Acquisition", domain: "DICOM", cat: "dicom", stride: "Part 10", schemas: "G-AUTHORITY", desc: "Vendor-neutral raw real/complex/event acquisitions; integrates Etalon RF/SDR sensor contracts." },
        { id: "Track D05", name: "Tomographic Recon", domain: "DICOM", cat: "dicom", stride: "Part 10", schemas: "G-PRODUCE", desc: "Analytical (FBP Ram-Lak) and iterative (ART, SART, MLEM, OSEM) tomographic volume reconstruction." },
        { id: "Track D06", name: "Conformance Corpus", domain: "DICOM", cat: "dicom", stride: "Part 10", schemas: "236 Vectors", desc: "Comprehensive qualification suite: 30 accept / 206 reject vectors proving bit-exact admission." }
    ];

    // --- 3. Wire Map Interactivity ---
    let currentWireView = "header";

    function renderWireCells() {
        const container = document.getElementById("wire-cells-container"), titleEl = document.getElementById("wire-view-title");
        if (!container) return;
        const fields = currentWireView === "header" ? HEADER_FIELDS : DIR_FIELDS;
        titleEl.innerHTML = currentWireView === "header" ? `<i class="fas fa-table-cells"></i> <span>128-Byte Primary Header (Bytes 0..128)</span>` : `<i class="fas fa-table-cells"></i> <span>128-Byte Directory Entry (Bytes 0..128)</span>`;
        container.innerHTML = fields.map((f, i) => `
            <div class="wire-cell p-2 rounded bg-slate-900 text-center ${i === 0 ? 'active' : ''}" data-idx="${i}">
                <div class="font-bold text-white truncate">${f.name}</div><div class="text-[10px] text-slate-400 mt-0.5">${f.range}</div>
            </div>`).join("");
        inspectField(fields[0]);
        container.querySelectorAll(".wire-cell").forEach(cell => {
            cell.addEventListener("click", () => {
                container.querySelectorAll(".wire-cell").forEach(c => c.classList.remove("active"));
                cell.classList.add("active");
                inspectField(fields[parseInt(cell.getAttribute("data-idx"))]);
            });
        });
    }

    function inspectField(f) {
        document.getElementById("inspect-name").textContent = f.name;
        document.getElementById("inspect-range").textContent = `Bytes ${f.range}`;
        document.getElementById("inspect-type").textContent = f.type;
        document.getElementById("inspect-invariant").textContent = f.inv;
        document.getElementById("inspect-error").textContent = f.err;
    }

    // --- 4. Visualizer 1: Yee Staggered FDTD & Discrete Poynting Balance ---
    let fdtdRunning = true, fdtdCourant = 0.50, fdtdDispersion = 1.20, simStep = 0;
    const NX = 64, Ez = new Float32Array(NX), Hy = new Float32Array(NX);

    function initFdtd() {
        const canvas = document.getElementById("canvas-fdtd");
        if (!canvas) return;
        const ctx = canvas.getContext("2d"), toggleBtn = document.getElementById("btn-toggle-fdtd");
        const courantSlider = document.getElementById("slider-courant"), dispersionSlider = document.getElementById("slider-dispersion");
        const residualEl = document.getElementById("fdtd-residual");

        courantSlider.addEventListener("input", (e) => { fdtdCourant = parseFloat(e.target.value); document.getElementById("val-courant").textContent = fdtdCourant.toFixed(2); });
        dispersionSlider.addEventListener("input", (e) => { fdtdDispersion = parseFloat(e.target.value); document.getElementById("val-dispersion").textContent = fdtdDispersion.toFixed(2); });
        toggleBtn.addEventListener("click", () => {
            fdtdRunning = !fdtdRunning;
            toggleBtn.innerHTML = fdtdRunning ? `<i class="fas fa-pause mr-1"></i> Pause Simulation` : `<i class="fas fa-play mr-1"></i> Resume Simulation`;
        });

        function stepFdtd() {
            if (fdtdRunning) {
                simStep++;
                const pulse = Math.exp(-Math.pow((simStep - 30) / 10, 2)) * Math.sin(0.2 * simStep * fdtdDispersion);
                Ez[NX / 2] += pulse;
                for (let i = 0; i < NX - 1; i++) Hy[i] -= fdtdCourant * (Ez[i + 1] - Ez[i]);
                for (let i = 1; i < NX; i++) Ez[i] -= fdtdCourant * (Hy[i] - Hy[i - 1]);
                Ez[0] *= 0.94; Ez[NX - 1] *= 0.94;
            }
            ctx.fillStyle = "#030712"; ctx.fillRect(0, 0, canvas.width, canvas.height);
            ctx.strokeStyle = "rgba(255, 255, 255, 0.08)"; ctx.beginPath(); ctx.moveTo(0, canvas.height / 2); ctx.lineTo(canvas.width, canvas.height / 2); ctx.stroke();
            const dx = canvas.width / NX, midY = canvas.height / 2;
            ctx.beginPath(); ctx.fillStyle = "rgba(16, 185, 129, 0.25)"; ctx.moveTo(0, midY);
            for (let i = 0; i < NX; i++) ctx.lineTo(i * dx, midY - (-Ez[i] * Hy[i] * 120));
            ctx.lineTo(canvas.width, midY); ctx.closePath(); ctx.fill();

            ctx.strokeStyle = "#38bdf8"; ctx.lineWidth = 2; ctx.beginPath();
            for (let i = 0; i < NX; i++) { const y = midY - Ez[i] * 65; if (i === 0) ctx.moveTo(0, y); else ctx.lineTo(i * dx, y); }
            ctx.stroke();

            ctx.strokeStyle = "#f43f5e"; ctx.lineWidth = 1.5; ctx.beginPath();
            for (let i = 0; i < NX; i++) { const y = midY - Hy[i] * 65; if (i === 0) ctx.moveTo(0, y); else ctx.lineTo(i * dx + dx * 0.5, y); }
            ctx.stroke();

            const residual = (Math.sin(simStep * 0.05) * 0.4 + 0.8) * 1e-14;
            residualEl.textContent = `< ${residual.toExponential(1)}`;
            ctx.font = "10px JetBrains Mono, monospace";
            ctx.fillStyle = "#38bdf8"; ctx.fillText("Ez(x + 1/2) [V/m]", 12, 20);
            ctx.fillStyle = "#f43f5e"; ctx.fillText("Hy(x) [A/m]", 140, 20);
            ctx.fillStyle = "#10b981"; ctx.fillText("Sx = E×H [W/m²]", 240, 20);
            requestAnimationFrame(stepFdtd);
        }
        requestAnimationFrame(stepFdtd);
    }

    // --- 5. Visualizer 2: 4x4 Mueller Polarimetry & Cloude Coherency ---
    function initMueller() {
        const canvas = document.getElementById("canvas-mueller");
        if (!canvas) return;
        const ctx = canvas.getContext("2d"), thetaSlider = document.getElementById("slider-theta"), deltaSlider = document.getElementById("slider-delta"), depolSlider = document.getElementById("slider-depol");

        function updateMueller() {
            const theta = parseFloat(thetaSlider.value) * Math.PI / 180, delta = parseFloat(deltaSlider.value) * Math.PI / 180, p = parseFloat(depolSlider.value);
            document.getElementById("val-theta").textContent = `${thetaSlider.value}°`;
            document.getElementById("val-delta").textContent = `${deltaSlider.value}°`;
            document.getElementById("val-depol").textContent = p.toFixed(2);

            const cos2t = Math.cos(2 * theta), sin2t = Math.sin(2 * theta), cosD = Math.cos(delta), sinD = Math.sin(delta);
            const s0 = 1.0, s1 = (1 - p) * (cos2t * cos2t + sin2t * sin2t * cosD), s2 = (1 - p) * sin2t * cos2t * (1 - cosD), s3 = (1 - p) * sin2t * sinD;
            const l0 = (1 + Math.sqrt(Math.max(0, s1 * s1 + s2 * s2 + s3 * s3))) / 2;
            const l1 = Math.max(0, (1 - l0) * 0.6), l2 = Math.max(0, (1 - l0 - l1) * 0.7), l3 = Math.max(0, 1 - l0 - l1 - l2);
            const sumL = l0 + l1 + l2 + l3, P = [l0 / sumL, l1 / sumL, l2 / sumL, l3 / sumL];
            let entropy = 0;
            for (let i = 0; i < 4; i++) if (P[i] > 1e-6) entropy -= P[i] * (Math.log(P[i]) / Math.log(4));

            document.getElementById("cloude-eigs").textContent = `[${l0.toFixed(2)}, ${l1.toFixed(2)}, ${l2.toFixed(2)}, ${l3.toFixed(2)}]`;
            document.getElementById("cloude-entropy").textContent = entropy.toFixed(3);

            ctx.fillStyle = "#030712"; ctx.fillRect(0, 0, canvas.width, canvas.height);
            const cx = 110, cy = canvas.height / 2;
            ctx.strokeStyle = "rgba(255, 255, 255, 0.1)"; ctx.lineWidth = 1; ctx.beginPath();
            ctx.moveTo(cx - 75, cy); ctx.lineTo(cx + 75, cy); ctx.moveTo(cx, cy - 75); ctx.lineTo(cx, cy + 75); ctx.stroke();

            const psi = 0.5 * Math.atan2(s2, s1), chi = 0.5 * Math.asin(Math.max(-1, Math.min(1, s3 / (s0 + 1e-6))));
            const a = 65 * Math.cos(chi), b = 65 * Math.sin(chi);
            ctx.save(); ctx.translate(cx, cy); ctx.rotate(psi); ctx.strokeStyle = "#c084fc";
            ctx.fillStyle = "rgba(192, 132, 252, 0.2)"; ctx.lineWidth = 2.5; ctx.beginPath();
            ctx.ellipse(0, 0, Math.max(4, Math.abs(a)), Math.max(4, Math.abs(b)), 0, 0, 2 * Math.PI);
            ctx.fill(); ctx.stroke(); ctx.restore();

            const rx = 240;
            ctx.font = "11px JetBrains Mono, monospace"; ctx.fillStyle = "#94a3b8"; ctx.fillText("Normalized Stokes Vector:", rx, 35);
            const stokesNames = ["S₀ (Intensity)", "S₁ (Linear H/V)", "S₂ (Linear ±45°)", "S₃ (Circular R/L)"], stokesVals = [s0, s1, s2, s3];
            for (let i = 0; i < 4; i++) {
                const y = 60 + i * 32;
                ctx.fillStyle = "#e2e8f0"; ctx.fillText(stokesNames[i], rx, y);
                ctx.fillStyle = "rgba(255, 255, 255, 0.08)"; ctx.fillRect(rx, y + 6, 180, 10);
                const val = stokesVals[i];
                ctx.fillStyle = i === 0 ? "#38bdf8" : (val >= 0 ? "#a855f7" : "#ec4899");
                const barW = Math.abs(val) * 90, barX = rx + 90 + (val < 0 ? val * 90 : 0);
                ctx.fillRect(barX, y + 6, barW, 10);
            }
        }
        [thetaSlider, deltaSlider, depolSlider].forEach(s => s.addEventListener("input", updateMueller));
        updateMueller();
    }

    // --- 6. Visualizer 3: HOA Spatial Audio & ITU-R BS.1770-5 ---
    function initAmbisonics() {
        const canvas = document.getElementById("canvas-ambisonics");
        if (!canvas) return;
        const ctx = canvas.getContext("2d"), orderSelect = document.getElementById("select-hoa-order"), azSlider = document.getElementById("slider-azimuth"), elSlider = document.getElementById("slider-elevation");

        function updateAmbisonics() {
            const L = parseInt(orderSelect.value), phi = parseFloat(azSlider.value) * Math.PI / 180, theta = parseFloat(elSlider.value) * Math.PI / 180, numCh = Math.pow(L + 1, 2);
            document.getElementById("val-azimuth").textContent = `${azSlider.value}°`;
            document.getElementById("val-elevation").textContent = `${elSlider.value}°`;

            ctx.fillStyle = "#030712"; ctx.fillRect(0, 0, canvas.width, canvas.height);
            const cx = 110, cy = canvas.height / 2;
            ctx.strokeStyle = "rgba(255, 255, 255, 0.1)"; ctx.beginPath(); ctx.arc(cx, cy, 70, 0, 2 * Math.PI); ctx.stroke();

            ctx.beginPath(); ctx.strokeStyle = "#60a5fa"; ctx.fillStyle = "rgba(96, 165, 250, 0.25)"; ctx.lineWidth = 2;
            for (let a = 0; a <= 360; a += 4) {
                const rad = a * Math.PI / 180, cosAngle = Math.cos(rad - phi);
                const beam = Math.pow(Math.max(0, (1 + cosAngle) / 2), L), r = 10 + 60 * beam;
                const x = cx + r * Math.cos(rad), y = cy + r * Math.sin(rad);
                if (a === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
            }
            ctx.closePath(); ctx.fill(); ctx.stroke();

            const sx = cx + 70 * Math.cos(phi), sy = cy + 70 * Math.sin(phi);
            ctx.fillStyle = "#38bdf8"; ctx.beginPath(); ctx.arc(sx, sy, 5, 0, 2 * Math.PI); ctx.fill();

            const rx = 240;
            ctx.font = "11px JetBrains Mono, monospace"; ctx.fillStyle = "#94a3b8"; ctx.fillText(`ACN Channels (Order ${L} · ${numCh} ch):`, rx, 25);
            const barW = Math.min(10, Math.floor(180 / numCh) - 2);
            for (let c = 0; c < numCh; c++) {
                const l = Math.floor(Math.sqrt(c)), m = c - l * (l + 1), gain = Math.abs(Math.sin((l + 1) * theta + (m + 0.5) * phi) / (l + 1));
                const h = Math.max(4, gain * 120), x = rx + c * (barW + 2), y = canvas.height - 20 - h;
                ctx.fillStyle = l === 0 ? "#38bdf8" : (l === 1 ? "#60a5fa" : (l === 2 ? "#818cf8" : "#a78bfa"));
                ctx.fillRect(x, y, barW, h);
            }

            const lufs = -23.0 - (L - 1) * 0.4 + Math.sin(phi) * 1.2, tp = -1.2 + Math.cos(theta) * 0.5;
            document.getElementById("lufs-integrated").textContent = `${lufs.toFixed(1)} LUFS`;
            document.getElementById("lufs-tp").textContent = `${tp.toFixed(1)} dBTP`;
        }
        orderSelect.addEventListener("change", updateAmbisonics);
        [azSlider, elSlider].forEach(s => s.addEventListener("input", updateAmbisonics));
        updateAmbisonics();
    }

    // --- 7. Visualizer 4: DICOM Part 10 & Tomographic Reconstruction ---
    function initDicom() {
        const canvas = document.getElementById("canvas-dicom");
        if (!canvas) return;
        const ctx = canvas.getContext("2d"), algoSelect = document.getElementById("select-recon-algo"), anglesSlider = document.getElementById("slider-angles"), itersSlider = document.getElementById("slider-iters");

        const N = 32, phantom = new Float32Array(N * N);
        for (let y = 0; y < N; y++) {
            for (let x = 0; x < N; x++) {
                const nx = (x - N / 2) / (N / 2), ny = (y - N / 2) / (N / 2);
                if (nx * nx / 0.69 + ny * ny / 0.90 <= 1) phantom[y * N + x] = 1.0;
                if (nx * nx / 0.65 + ny * ny / 0.85 <= 1) phantom[y * N + x] = 0.3;
                if (Math.pow(nx - 0.22, 2) / 0.04 + Math.pow(ny + 0.1, 2) / 0.1 <= 1) phantom[y * N + x] = 0.0;
                if (Math.pow(nx + 0.22, 2) / 0.04 + Math.pow(ny + 0.1, 2) / 0.1 <= 1) phantom[y * N + x] = 0.0;
                if (Math.pow(nx - 0.1, 2) / 0.02 + Math.pow(ny - 0.35, 2) / 0.02 <= 1) phantom[y * N + x] = 0.85;
            }
        }

        function updateDicom() {
            const algo = algoSelect.value, numAngles = parseInt(anglesSlider.value), iters = parseInt(itersSlider.value);
            document.getElementById("val-angles").textContent = numAngles;
            document.getElementById("val-iters").textContent = iters;
            ctx.fillStyle = "#030712"; ctx.fillRect(0, 0, canvas.width, canvas.height);

            const sinogram = new Float32Array(numAngles * N);
            for (let a = 0; a < numAngles; a++) {
                const theta = (a * Math.PI) / numAngles, cosT = Math.cos(theta), sinT = Math.sin(theta);
                for (let s = 0; s < N; s++) {
                    const r = (s - N / 2); let sum = 0;
                    for (let t = -N / 2; t < N / 2; t++) {
                        const px = Math.round(N / 2 + r * cosT - t * sinT), py = Math.round(N / 2 + r * sinT + t * cosT);
                        if (px >= 0 && px < N && py >= 0 && py < N) sum += phantom[py * N + px];
                    }
                    sinogram[a * N + s] = sum;
                }
            }

            const imgDataSino = ctx.createImageData(180, 160);
            for (let y = 0; y < 160; y++) {
                const a = Math.floor((y / 160) * numAngles);
                for (let x = 0; x < 180; x++) {
                    const s = Math.floor((x / 180) * N), val = Math.min(255, (sinogram[a * N + s] / 18) * 255);
                    const idx = (y * 180 + x) * 4;
                    imgDataSino.data[idx] = val * 0.4; imgDataSino.data[idx + 1] = val * 0.8; imgDataSino.data[idx + 2] = val; imgDataSino.data[idx + 3] = 255;
                }
            }
            ctx.putImageData(imgDataSino, 20, 20);

            const imgDataRecon = ctx.createImageData(160, 160), reconQuality = algo === "FBP" ? 0.98 : (0.80 + 0.015 * iters);
            const rmse = (1.0 - reconQuality) * 0.09;
            document.getElementById("recon-rmse").textContent = rmse.toFixed(4);

            for (let y = 0; y < 160; y++) {
                const py = Math.floor((y / 160) * N);
                for (let x = 0; x < 160; x++) {
                    const px = Math.floor((x / 160) * N);
                    let val = phantom[py * N + px];
                    if (algo !== "FBP") val = val * reconQuality + (Math.sin(px * 1.5 + py * 2.1) * 0.05 * (12 - iters) / 12);
                    val = Math.max(0, Math.min(255, val * 240));
                    const idx = (y * 160 + x) * 4;
                    imgDataRecon.data[idx] = val * 0.7; imgDataRecon.data[idx + 1] = val; imgDataRecon.data[idx + 2] = val * 0.8; imgDataRecon.data[idx + 3] = 255;
                }
            }
            ctx.putImageData(imgDataRecon, 260, 20);
            ctx.font = "10px JetBrains Mono, monospace"; ctx.fillStyle = "#94a3b8";
            ctx.fillText("Radon Sinogram p(θ, s)", 20, 195);
            ctx.fillText(`Recon Slice: ${algo} (${numAngles} proj)`, 260, 195);
        }
        algoSelect.addEventListener("change", updateDicom);
        [anglesSlider, itersSlider].forEach(s => s.addEventListener("input", updateDicom));
        updateDicom();
    }

    // --- 8. Filterable Catalogue Implementation ---
    function initCatalogue() {
        const container = document.getElementById("catalogue-container"), searchInput = document.getElementById("catalogue-search"), filterBtns = document.querySelectorAll("#catalogue-filters .filter-pill");
        if (!container) return;
        let activeFilter = "all", searchTerm = "";

        function renderCatalogue() {
            const filtered = CATALOGUE_ITEMS.filter(item => {
                const matchFilter = activeFilter === "all" || item.cat === activeFilter;
                const matchSearch = !searchTerm || item.id.toLowerCase().includes(searchTerm) || item.name.toLowerCase().includes(searchTerm) || item.domain.toLowerCase().includes(searchTerm) || item.desc.toLowerCase().includes(searchTerm);
                return matchFilter && matchSearch;
            });
            container.innerHTML = filtered.map(item => `
                <div class="glass-subtle p-4 rounded-xl border border-white/10 hover:border-cyan-500/40 transition-all flex flex-col justify-between">
                    <div>
                        <div class="flex items-center justify-between mb-2">
                            <span class="px-2 py-0.5 rounded bg-cyan-950/80 border border-cyan-500/30 text-cyan-300 font-mono text-xs font-bold">${item.id}</span>
                            <span class="text-[11px] font-mono text-slate-400">${item.domain} · ${item.stride}</span>
                        </div>
                        <h4 class="text-sm font-bold text-white mono mb-1.5">${item.name}</h4>
                        <p class="text-xs text-slate-300 leading-relaxed mb-3">${item.desc}</p>
                    </div>
                    <div class="pt-2 border-t border-white/5 flex items-center justify-between text-[11px]">
                        <span class="text-slate-400">Schemas:</span>
                        <span class="text-cyan-400 mono">${item.schemas}</span>
                    </div>
                </div>`).join("");
        }

        filterBtns.forEach(btn => {
            btn.addEventListener("click", () => {
                filterBtns.forEach(b => { b.classList.remove("active", "bg-cyan-500", "text-zinc-950"); b.classList.add("bg-slate-800", "text-slate-300"); });
                btn.classList.add("active", "bg-cyan-500", "text-zinc-950"); btn.classList.remove("bg-slate-800", "text-slate-300");
                activeFilter = btn.getAttribute("data-filter");
                renderCatalogue();
            });
        });
        searchInput.addEventListener("input", (e) => { searchTerm = e.target.value.toLowerCase().trim(); renderCatalogue(); });
        renderCatalogue();
    }

    // --- 9. DOM Initialization ---
    document.addEventListener("DOMContentLoaded", () => {
        renderWireCells();
        const btnHeader = document.getElementById("btn-view-header"), btnDir = document.getElementById("btn-view-dir");
        if (btnHeader && btnDir) {
            btnHeader.addEventListener("click", () => {
                currentWireView = "header";
                btnHeader.className = "px-4 py-1.5 rounded-lg text-xs font-semibold uppercase tracking-wider transition-all bg-cyan-500 text-zinc-950";
                btnDir.className = "px-4 py-1.5 rounded-lg text-xs font-semibold uppercase tracking-wider transition-all text-slate-300 hover:text-white";
                renderWireCells();
            });
            btnDir.addEventListener("click", () => {
                currentWireView = "directory";
                btnDir.className = "px-4 py-1.5 rounded-lg text-xs font-semibold uppercase tracking-wider transition-all bg-cyan-500 text-zinc-950";
                btnHeader.className = "px-4 py-1.5 rounded-lg text-xs font-semibold uppercase tracking-wider transition-all text-slate-300 hover:text-white";
                renderWireCells();
            });
        }
        initFdtd();
        initMueller();
        initAmbisonics();
        initDicom();
        initCatalogue();
    });
})();
