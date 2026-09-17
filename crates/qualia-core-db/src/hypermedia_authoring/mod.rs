//! Hypermedia asset authoring — image, video, 3D, interactive, portals, DMX.
//!
//! N9: Build-new high-level authoring operations across 7 domains.
//! These are data-structure + operation layers that route to host-side
//! rendering/compositing implementations via VibeScript invoke IDs.

pub mod dmx;
pub mod image;
pub mod interactive;
pub mod portals;
pub mod video;
pub mod world3d;

pub use dmx::{Cue, CueStack, DmxFixture, DmxUniverse};
pub use image::{BrushStroke, ImageDocument, ImageFilter, ImageLayer, Selection};
pub use interactive::{HbbTVApp, InteractiveStream, SecondScreen};
pub use portals::{Avatar, Portal, World, WorldObject};
pub use video::{VideoClip, VideoProject, VideoTrack, VideoTransition};
pub use world3d::{AnimationClip, Camera3D, Light3D, Material3D, Object3D, Rig};
