//! Temporary standalone verification crate for P2.6 combinatorial_map.
//! Includes the real topology.rs and the new combinatorial_map.rs under the
//! exact module path they will live at, so `super::topology` and
//! `crate::specialized_libs::computational_geometry::topology` resolve
//! identically to the real crate. Deleted after verification.

pub mod specialized_libs {
    pub mod computational_geometry {
        pub mod topology {
            include!("C:/Projects/qualia-27062026/crates/qualia-core-db/src/specialized_libs/computational_geometry/topology.rs");
        }
        pub mod combinatorial_map {
            include!("C:/Projects/qualia-27062026/crates/qualia-core-db/src/specialized_libs/computational_geometry/combinatorial_map.rs");
        }
    }
}
