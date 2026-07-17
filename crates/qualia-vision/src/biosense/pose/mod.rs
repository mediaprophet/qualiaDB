//! Body / hand pose packing from MediaPipe-layout float buffers.
//! Runtime inference is AdapterMissing until ORT/TFLite session lands;
//! packing + asset resolve are PermissiveReady (Apache-2.0 MediaPipe).

pub mod pack_pose_landmarks;
pub mod pack_hand_landmarks;

pub use pack_pose_landmarks::{pack_pose_landmarks_xy, PoseLandmark, MAX_POSE_LANDMARKS};
pub use pack_hand_landmarks::{pack_hand_landmarks_xy, HandLandmark, MAX_HAND_LANDMARKS};
