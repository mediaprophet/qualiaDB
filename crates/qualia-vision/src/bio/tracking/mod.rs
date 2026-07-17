//! Particle tracking: Crocker–Grier linking + centroid extraction.

pub mod crocker_grier_link;
pub mod particle_features;

pub use crocker_grier_link::{
    crocker_grier_link, link_particles, CrockerGrierLinker, CrockerGrierParams, Detection2,
    LinkedParticle, TrackLink, MAX_FRAME_DETS, MAX_PARTICLE_TRACKS, NO_TRACK_ID,
};
pub use particle_features::{
    centroid_from_bbox, centroids_from_binary, centroids_from_labels, ParticleCentroid,
    MAX_PARTICLES_PER_FRAME,
};
