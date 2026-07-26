//! Biology / microscopy / medical vision vertical (D7).
//!
//! - [`morphology`] — OD index, top-hat, watershed / Voronoi-Otsu labels, nucleus features
//! - [`histopathology`] — stain OD, Reinhard, Macenko, background correct, SNMF-lite
//! - [`radiomics`] — first-order, GLCM, shape lite
//! - [`tracking`] — Crocker–Grier particle link + centroids from blobs
//! - [`medical`] — HU window, MIP, isotropic resample, spectral unmix NNLS
//! - [`dicom_lite`] — partial LE-explicit tags, PHI redact, SUV formula
//!
//! Clinical-adjacent outputs are **non-diagnosis** proposals; PHI/DICOM paths fail closed
//! under sensitivity / Wellfair policy (see bio-medical CV catalogue).

pub mod dicom_lite;
pub mod histopathology;
pub mod medical;
pub mod morphology;
pub mod radiomics;
pub mod tracking;

pub use dicom_lite::{
    anonymize_tag_map, parse_dicom_tags_basic, suv_bw, suv_from_activity, AnonymizeReport,
    DicomLiteError, DicomTagMap, ParsedDicomTags, PHI_TAG_KEYS,
};
pub use histopathology::{
    apply_background_correct, background_intensity_sample, lab_to_rgb, macenko_deconvolution,
    optical_density_rgb, reinhard_normalize, rgb_to_lab, snmf_unmix_lite, HistoError, LabStats,
    MacenkoResult, RgbBg, StainBasis, DEFAULT_HE_TARGET_MEAN, DEFAULT_HE_TARGET_STD,
};
pub use medical::{
    apply_hu_window_f32, apply_hu_window_i16, hu_window_i16, isotropic_resample_2d_nn,
    isotropic_resample_3d_nn, isotropic_resample_nn_2d, mip_project_axis, mip_project_z,
    spectral_unmix_nnls, spectral_unmix_roi_mean, MedicalError, MipAxis, UnmixResult,
};
pub use morphology::{
    extended_minima, intensity_to_od_u8, morphological_tophat, nucleus_features,
    otsu_threshold_from_hist, positive_od_threshold, voronoi_otsu_label, watershed_markers,
    NucleusFeature, OdPositiveIndex, TopHatKind, MAX_NUCLEUS_LABELS, MAX_OD_LABELS,
};
pub use radiomics::{
    first_order_stats, glcm_features, glcm_features_d1, shape_2d_features, shape_2d_from_mask,
    shape_3d_from_voxels, FirstOrderStats, GlcmFeatures, RadiomicsError, Shape2d, Shape2dFeatures,
    Shape3dFeatures, GLCM_LEVELS_16, GLCM_LEVELS_32,
};
pub use tracking::{
    centroid_from_bbox, centroids_from_binary, centroids_from_labels, crocker_grier_link,
    link_particles, CrockerGrierLinker, CrockerGrierParams, Detection2, LinkedParticle,
    ParticleCentroid, TrackLink, MAX_FRAME_DETS, MAX_PARTICLES_PER_FRAME, MAX_PARTICLE_TRACKS,
    NO_TRACK_ID,
};
