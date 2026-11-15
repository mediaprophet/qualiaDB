# Full-spectrum field and measurement contract

Development-tree status, 13 November 2026. Full-spectrum scope is an invariant of the EMF/QAC/.10d architecture. It is not a list that users must extend for each sensor, frequency band, apparatus, or nonvisual use case.

## Representation boundaries

Electromagnetic observations retain their actual physical quantities and coordinates across radio, microwave, infrared, visible, ultraviolet, X-ray and gamma measurements. Static and aperiodic fields retain absent carrier information explicitly; they do not fabricate zero-frequency wavelengths. Quantum-device observations may require frequency, time, phase, event counts, correlations or other registered observables. A frequency axis alone is not a complete model of those observations. Acoustic and mechanical quantities remain physically distinct in QAC, with explicit coupling to EMF and spatial records.

Source data, calibration, clock, uncertainty, coordinate frame and provenance must survive reconstruction and inference. Images, colour, sonification and Tensor10D projections are derived representations. They cannot replace original acquisition data or establish missing calibration. Device availability limits a provider, never the container's spectral scope. Unsupported metadata remains preserved as opaque data until a supported typed adapter admits it.

## Implemented spectral primitive

The QualiaDB module q42::field_spectrum admits candidate schema132 revision1 through admit_axis(Block, expected_samples, Limits, ResponseValidator). It has no visible-band or apparatus restriction. Coordinates are positive finite ascending f64 frequency_hz or vacuum_wavelength_m values. Point samples require N coordinates; bin integrals and averages require N+1 edges. Static/aperiodic models use other descriptors, without forcing this positive-carrier axis onto them.

Exact quantity/unit pairs:

| Quantity | Frequency axis | Vacuum wavelength axis |
|---|---|---|
| spectral_radiance | W sr^-1 m^-2 Hz^-1 | W sr^-1 m^-2 m^-1 |
| spectral_irradiance | W m^-2 Hz^-1 | W m^-2 m^-1 |
| bin_integrated_radiance | W sr^-1 m^-2 | W sr^-1 m^-2 |
| bin_integrated_irradiance | W m^-2 | W m^-2 |
| dimensionless_reflectance | 1 | 1 |

These five quantities are the implemented radiometric profile, not an exhaustive EMF observable registry. Electric/magnetic vectors, complex acquisition samples, correlations and device-specific observables require their own quantity/profile validators. The format must not recast them as radiance to fit this profile.

Checked helpers provide c/f, photon-energy conversion and point-density Jacobians. Wavelength/frequency conversion explicitly reverses ordering; coordinate and sample permutations are separate, including N+1 bin edges. Bin averages preserve their integral and divide by the new bin width. Per-metre/per-nanometre density conversion is explicit. Reflectance and bin integrals reject density Jacobians. Unrepresentable results fail rather than clip or silently underflow to zero.

Legacy display sigma labels are retained as nonphysical projection conventions. Physical coordinates remain authoritative. Response references require a supported typed validator; the default rejects them instead of assuming a flat sensor response. Readers are borrowed and caller-budgeted. Semantic admission does not imply payload verification, provenance authentication or algorithm validity.

## Integration status

The current shared envelope reader/writer, common metadata validation, typed EMF field/ray/spectrum admission, spatial field writer, typed QAC PCM/STFT admission, PCM writer and native QAC publication are implemented in the development tree. Former internal development versions impose no compatibility requirement; coordinated producer/consumer cutover is in progress. Source-owned tests cover spectrum extremes, unit consistency, ordering and conserved integrals. Complete typed EMF/QAC modes, acquisition adapters, inference/reconstruction providers, patient/anatomy registration and platform qualification remain tracked implementation work. Existing Etalon sensing and QualiaDB sensing records are reuse points, not replacements for preserved observations.

DICOM integration now includes exact retained-source acquisition inspection, bounded nested datasets, enhanced per-frame geometry/identities and source-bound quantitative mappings. Native integer views and the selected monochrome8/16-bit RLE Lossless decoder preserve stored samples. These explicit supported profiles do not imply all DICOM transfer syntaxes or complete IOD conformance. Floating-point/padding profiles and further codec/provider integrations remain in implementation.
