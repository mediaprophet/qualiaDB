# POET Health & Person-Controlled Records Specification

**Document ID:** `POET-SPEC-006`  
**Status:** Canonical Domain Specification  
**Scope:** Person-controlled health timelines, clinical risk calculators, biometric trend visualizers, and cryptographic medical disclosure consent in POET.

---

## 1. Overview & Person-Controlled Health Paradigm

Unlike traditional hospital-centric Electronic Health Records (EHRs) where institutional silos control patient data, POET provides a **Person-Controlled Health Habitat**. The natural person (or their legally appointed representative/guardian) controls the cryptographic keys to their medical records, grants time-bounded, granular disclosure permissions, and accesses verified analytical calculators locally.

```
+-----------------------------------------------------------------------------------+
|                        HEALTH & PERSON-RECORDS TOPOLOGY                           |
+-----------------------------------------------------------------------------------+
|  [Person-Controlled Health Timeline] <===> [Biometric & Vitals Trend Charts]      |
|  - Chronological episode diary             - Systolic / Diastolic Blood Pressure  |
|  - Medications, symptoms & encounters      - Heart rate, blood glucose, biomarkers|
|  - Verified clinical document extracts     - Interactive visual trend graphs      |
|                                                                                   |
|  [Clinical Analytical Calculators]   <===> [Granular Disclosure & Consent Gates]  |
|  - Native Framingham, CHA2DS2-VASc, SCORE2 - Clinician / Guardian DID grants     |
|  - Medical HU windowing for slice arrays   - Time-bounded, revocable scopes       |
|  - Local zero-heap algorithmic execution   - Cryptographic disclosure receipts    |
+-----------------------------------------------------------------------------------+
```

---

## 2. Person-Controlled Health Timeline

- **Episode & Encounter Diary:** Chronological log of health observations, clinical visits, specialist consults, and personal symptom notes.
- **Medication & Treatment Tracker:** Active prescriptions, dosages, schedules, and allergy warnings.
- **Document Extract Ingestion:** Ingestion of text extracts from lab reports and clinical summaries with sensitivity tags (`Restricted`, `Classified`, `Secret`).

---

## 3. Biometric Trends & Interactive Visualizers

- **Visual Vitals Charts:** Dynamic line and area charts plotting physiological metrics over time:
  - Blood Pressure (Systolic/Diastolic mmHg) with standard clinical range zones (Normal, Elevated, Stage 1, Stage 2).
  - Resting Heart Rate & HRV.
  - Blood Glucose (mg/dL or mmol/L) with fasting / postprandial markers.
  - Custom lab biomarker trends (e.g., HbA1c, Lipid panel, eGFR).
- **Unit Safety:** Strict enforcement of standardized clinical units; zero automatic conversions with ambiguous unit definitions.

---

## 4. Clinical Analytical Engines & Medical Imaging

- **Native Algorithmic Solvers:** Real zero-heap execution of validated clinical risk algorithms:
  - `ClinicalRisk.framingham`: 10-year cardiovascular risk percentage.
  - `ClinicalRisk.cha2ds2_vasc`: Stroke risk stratification in atrial fibrillation.
  - `ClinicalRisk.score2`: European cardiovascular risk estimation.
- **Medical Imaging HU Windowing:** Interactive Hounsfield Unit (HU) windowing slider (Window Center / Window Width) applying CT density presets (Bone, Soft Tissue, Lung, Brain) to 2D slice matrices.

---

## 5. Granular Privacy, Disclosure & Consent Gates

- **Disclosure Grants:** The patient issues cryptographic consent grants authorizing a specific clinician DID or research institution DID to access designated record categories (e.g., `Cardiology-Vitals-Only`).
- **Time Bounds & Expiration:** All grants require an explicit expiration timestamp (e.g., 24 hours, 30 days) after which access fails closed.
- **Revocation & Audit:** Patients can revoke grants instantly with one click, generating an immutable revocation receipt in the audit ledger.

---

## 6. Health Requirements

| Requirement ID | Title | Description | Target Component |
|---|---|---|---|
| `POET-HLT-001` | **Person-Controlled Health Timeline** | Chronological visual diary of clinical encounters, symptom logs, and medication history. | `health_views`, `specialist_persist.rs` |
| `POET-HLT-002` | **Interactive Vitals Trend Charts** | Visual time-series charts for blood pressure, heart rate, glucose, and lab biomarkers. | `health_views`, `topbar.rs` |
| `POET-HLT-003` | **Native Clinical Risk Calculators** | Interactive risk scoring (Framingham, CHA2DS2-VASc, SCORE2) driven by entered patient vitals. | `clinical`, `health_views` |
| `POET-HLT-004` | **Medical CT HU Windowing** | Interactive Hounsfield Unit windowing slider applying radiologic tissue presets to slice data. | `medical`, `health_views` |
| `POET-HLT-005` | **Granular Consent Grants** | Visual workflow for granting time-bounded, scoped record access to specific clinician DIDs. | `health_views`, `governance_views` |
| `POET-HLT-006` | **Instant Consent Revocation** | One-click consent revocation generating an auditable, immutable cryptographic receipt. | `health_views`, `crdt.rs` |
| `POET-HLT-007` | **Sensitivity Labeling** | Strict classification tagging (`Public`, `Restricted`, `Classified`, `Secret`) on health records. | `poet_record_api.rs`, `health_views` |
| `POET-HLT-008` | **No Demo Badging Policy** | Prohibit clinical risk badges or diagnostic markers derived from mock data from being marked `Live`. | `health_views`, `surface_honesty.rs` |
