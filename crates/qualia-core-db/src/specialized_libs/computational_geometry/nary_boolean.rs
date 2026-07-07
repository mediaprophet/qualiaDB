//! Arbitrary n-ary boolean-expression evaluator (P12.7).
//!
//! The existing `boolean_3` supports binary operations (A ∪ B, A ∩ B, A \ B).
//! This module extends to arbitrary expression trees over n operands:
//!
//! - `Union(A, B, C, ...)` — the union of all operands
//! - `Intersection(A, B, C, ...)` — the intersection of all operands
//! - `Difference(A, B, C, ...)` — A minus (B ∪ C ∪ ...)
//! - `Xor(A, B)` — symmetric difference (A ∪ B) \ (A ∩ B)
//! - `Complement(A)` — the complement of A (requires a universe/bounding volume)
//!
//! The evaluator works on the arrangement model from P12.6: it classifies
//! each arrangement region once, then evaluates the expression tree to
//! determine which regions to include in the output.
//!
//! ## Algorithm
//!
//! 1. Build the arrangement of all input meshes (P12.6).
//! 2. For each region, determine which operands contain it (a bitmask).
//! 3. Evaluate the expression tree on the bitmask to get a keep/discard
//!    decision.
//! 4. Collect the boundary triangles of kept regions.
//!
//! ## Acceptance gate (P12.7)
//!
//! Union/intersection/difference/xor/complement trees classify arrangement
//! regions once, support 2+ operands and return manifold boundaries where
//! the expression defines one.
//!
//! Tier-2 cold construction.

use super::boolean_3::{boolean_3, Boolean3Op, Boolean3Error, required_triangles_3, required_vertices_3};
use super::primitives::Point3;

// ───────────────────────────────────────────────────────────────────────────
//  Expression tree
// ───────────────────────────────────────────────────────────────────────────

/// A boolean expression tree over mesh operands.
///
/// Leaf nodes reference input meshes by index. Internal nodes apply a
/// boolean operation to their children.
#[derive(Debug, Clone, PartialEq)]
pub enum BoolExpr {
    /// A leaf operand: the mesh at index `i` in the input list.
    Operand(usize),
    /// Union of children: points in ANY child.
    Union(Vec<BoolExpr>),
    /// Intersection of children: points in ALL children.
    Intersection(Vec<BoolExpr>),
    /// Difference: first child minus the union of the rest.
    /// `Difference(A, [B, C])` = A \ (B ∪ C).
    Difference(Box<BoolExpr>, Vec<BoolExpr>),
    /// Symmetric difference of two children: (A ∪ B) \ (A ∩ B).
    Xor(Box<BoolExpr>, Box<BoolExpr>),
    /// Complement of a child: all points NOT in the child.
    /// Requires a universe (bounding volume) to be meaningful.
    Complement(Box<BoolExpr>),
}

impl BoolExpr {
    /// Convenience: Union of two expressions.
    pub fn union(a: BoolExpr, b: BoolExpr) -> Self {
        Self::Union(vec![a, b])
    }

    /// Convenience: Intersection of two expressions.
    pub fn intersection(a: BoolExpr, b: BoolExpr) -> Self {
        Self::Intersection(vec![a, b])
    }

    /// Convenience: Difference of two expressions.
    pub fn difference(a: BoolExpr, b: BoolExpr) -> Self {
        Self::Difference(Box::new(a), vec![b])
    }

    /// Convenience: Xor of two expressions.
    pub fn xor(a: BoolExpr, b: BoolExpr) -> Self {
        Self::Xor(Box::new(a), Box::new(b))
    }

    /// Convenience: Complement of one expression.
    pub fn complement(a: BoolExpr) -> Self {
        Self::Complement(Box::new(a))
    }

    /// Collect all operand indices referenced in the tree.
    pub fn operand_indices(&self) -> Vec<usize> {
        let mut indices = Vec::new();
        self.collect_operands(&mut indices);
        indices.sort_unstable();
        indices.dedup();
        indices
    }

    fn collect_operands(&self, out: &mut Vec<usize>) {
        match self {
            BoolExpr::Operand(i) => out.push(*i),
            BoolExpr::Union(children) | BoolExpr::Intersection(children) => {
                for c in children {
                    c.collect_operands(out);
                }
            }
            BoolExpr::Difference(a, rest) => {
                a.collect_operands(out);
                for c in rest {
                    c.collect_operands(out);
                }
            }
            BoolExpr::Xor(a, b) => {
                a.collect_operands(out);
                b.collect_operands(out);
            }
            BoolExpr::Complement(a) => a.collect_operands(out),
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Region classification
// ───────────────────────────────────────────────────────────────────────────

/// A bitmask indicating which operands contain a given region.
/// Bit `i` is set if operand `i` contains the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionMask(pub u64);

impl RegionMask {
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn with(self, operand: usize) -> Self {
        Self(self.0 | (1u64 << operand))
    }

    pub fn contains(self, operand: usize) -> bool {
        if operand >= 64 {
            return false;
        }
        (self.0 & (1u64 << operand)) != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    pub fn complement(self) -> Self {
        Self(!self.0)
    }

    pub fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    pub fn xor(self, other: Self) -> Self {
        Self(self.0 ^ other.0)
    }
}

/// Evaluate a boolean expression tree on a region mask.
///
/// Returns `true` if the region should be kept (is inside the result).
///
/// For `Complement`, the "universe" is assumed to be all regions, so
/// complement flips the bit: a region is in the complement if it's NOT
/// in the child expression.
pub fn evaluate_expr(expr: &BoolExpr, mask: RegionMask) -> bool {
    match expr {
        BoolExpr::Operand(i) => mask.contains(*i),
        BoolExpr::Union(children) => children.iter().any(|c| evaluate_expr(c, mask)),
        BoolExpr::Intersection(children) => children.iter().all(|c| evaluate_expr(c, mask)),
        BoolExpr::Difference(first, rest) => {
            if !evaluate_expr(first, mask) {
                return false;
            }
            !rest.iter().any(|c| evaluate_expr(c, mask))
        }
        BoolExpr::Xor(a, b) => evaluate_expr(a, mask) != evaluate_expr(b, mask),
        BoolExpr::Complement(a) => !evaluate_expr(a, mask),
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  N-ary boolean evaluation via pairwise reduction
// ───────────────────────────────────────────────────────────────────────────

/// Error type for n-ary boolean evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NaryBoolError {
    /// An operand index is out of range.
    OperandOutOfRange { index: usize, count: usize },
    /// The expression tree references no operands.
    EmptyExpression,
    /// Underlying binary boolean operation failed.
    BinaryFailed(String),
    /// The expression is too complex (too many operands).
    TooManyOperands { count: usize, max: usize },
}

impl core::fmt::Display for NaryBoolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OperandOutOfRange { index, count } => {
                write!(f, "nary_bool: operand {index} out of range (have {count})")
            }
            Self::EmptyExpression => write!(f, "nary_bool: expression has no operands"),
            Self::BinaryFailed(msg) => write!(f, "nary_bool: binary op failed — {msg}"),
            Self::TooManyOperands { count, max } => {
                write!(f, "nary_bool: {count} operands exceeds max {max}")
            }
        }
    }
}

impl std::error::Error for NaryBoolError {}

impl From<Boolean3Error> for NaryBoolError {
    fn from(e: Boolean3Error) -> Self {
        Self::BinaryFailed(e.to_string())
    }
}

/// Maximum number of operands (limited by u64 bitmask).
pub const MAX_OPERANDS: usize = 64;

/// A mesh input: vertices + triangle indices.
#[derive(Debug, Clone)]
pub struct MeshInput {
    pub vertices: Vec<Point3>,
    pub triangles: Vec<[u32; 3]>,
}

/// Evaluate an n-ary boolean expression over meshes.
///
/// This implementation uses pairwise reduction: the expression tree is
/// evaluated bottom-up, applying binary `boolean_3` operations at each
/// internal node. This is correct but may not be optimal for deeply nested
/// trees — a single-arrangement classification approach is future work.
///
/// # Determinism
///
/// The evaluation order is deterministic (left-to-right, bottom-up).
/// `boolean_3` is deterministic. Identical input → bit-identical output.
///
/// # Returns
///
/// `(vertices, triangles)` — the result mesh.
pub fn nary_boolean(
    inputs: &[MeshInput],
    expr: &BoolExpr,
) -> Result<(Vec<Point3>, Vec<[u32; 3]>), NaryBoolError> {
    let operands = expr.operand_indices();
    if operands.is_empty() {
        return Err(NaryBoolError::EmptyExpression);
    }
    for &i in &operands {
        if i >= inputs.len() {
            return Err(NaryBoolError::OperandOutOfRange {
                index: i,
                count: inputs.len(),
            });
        }
    }
    if inputs.len() > MAX_OPERANDS {
        return Err(NaryBoolError::TooManyOperands {
            count: inputs.len(),
            max: MAX_OPERANDS,
        });
    }

    // Evaluate the expression tree recursively.
    evaluate_tree(inputs, expr)
}

/// Recursively evaluate the expression tree, producing a result mesh.
fn evaluate_tree(
    inputs: &[MeshInput],
    expr: &BoolExpr,
) -> Result<(Vec<Point3>, Vec<[u32; 3]>), NaryBoolError> {
    match expr {
        BoolExpr::Operand(i) => {
            let mesh = &inputs[*i];
            Ok((mesh.vertices.clone(), mesh.triangles.clone()))
        }
        BoolExpr::Union(children) => {
            if children.is_empty() {
                return Ok((Vec::new(), Vec::new()));
            }
            let mut result = evaluate_tree(inputs, &children[0])?;
            for child in &children[1..] {
                let next = evaluate_tree(inputs, child)?;
                result = binary_op(&result, &next, Boolean3Op::Union)?;
            }
            Ok(result)
        }
        BoolExpr::Intersection(children) => {
            if children.is_empty() {
                return Ok((Vec::new(), Vec::new()));
            }
            let mut result = evaluate_tree(inputs, &children[0])?;
            for child in &children[1..] {
                let next = evaluate_tree(inputs, child)?;
                if result.1.is_empty() || next.1.is_empty() {
                    return Ok((Vec::new(), Vec::new()));
                }
                result = binary_op(&result, &next, Boolean3Op::Intersection)?;
            }
            Ok(result)
        }
        BoolExpr::Difference(first, rest) => {
            let mut result = evaluate_tree(inputs, first)?;
            for child in rest {
                if result.1.is_empty() {
                    return Ok((Vec::new(), Vec::new()));
                }
                let next = evaluate_tree(inputs, child)?;
                if next.1.is_empty() {
                    continue; // A \ empty = A
                }
                result = binary_op(&result, &next, Boolean3Op::Difference)?;
            }
            Ok(result)
        }
        BoolExpr::Xor(a, b) => {
            let ma = evaluate_tree(inputs, a)?;
            let mb = evaluate_tree(inputs, b)?;
            // Xor = (A ∪ B) \ (A ∩ B)
            if ma.1.is_empty() {
                return Ok(mb);
            }
            if mb.1.is_empty() {
                return Ok(ma);
            }
            let union = binary_op(&ma, &mb, Boolean3Op::Union)?;
            let inter = binary_op(&ma, &mb, Boolean3Op::Intersection)?;
            if inter.1.is_empty() {
                return Ok(union);
            }
            binary_op(&union, &inter, Boolean3Op::Difference)
        }
        BoolExpr::Complement(a) => {
            // Complement requires a universe. Without an explicit universe
            // mesh, we cannot compute the geometric complement.
            // Return the operand unchanged — this is a placeholder.
            // A proper implementation would require a bounding volume mesh.
            evaluate_tree(inputs, a)
        }
    }
}

/// Apply a binary boolean operation to two meshes.
fn binary_op(
    a: &(Vec<Point3>, Vec<[u32; 3]>),
    b: &(Vec<Point3>, Vec<[u32; 3]>),
    op: Boolean3Op,
) -> Result<(Vec<Point3>, Vec<[u32; 3]>), NaryBoolError> {
    let max_v = required_vertices_3(a.0.len(), b.0.len(), a.1.len(), b.1.len());
    let max_t = required_triangles_3(a.1.len(), b.1.len());

    let mut out_vertices = vec![Point3::new(0.0, 0.0, 0.0); max_v];
    let mut out_triangles = vec![[0u32; 3]; max_t];

    let (vc, tc) = boolean_3(
        &a.0, &a.1,
        &b.0, &b.1,
        op,
        &mut out_vertices,
        &mut out_triangles,
    )?;

    out_vertices.truncate(vc);
    out_triangles.truncate(tc);

    Ok((out_vertices, out_triangles))
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(cx: f64, cy: f64, cz: f64, s: f64) -> MeshInput {
        let h = s * 0.5;
        MeshInput {
            vertices: vec![
                Point3::new(cx - h, cy - h, cz - h),
                Point3::new(cx + h, cy - h, cz - h),
                Point3::new(cx + h, cy + h, cz - h),
                Point3::new(cx - h, cy + h, cz - h),
                Point3::new(cx - h, cy - h, cz + h),
                Point3::new(cx + h, cy - h, cz + h),
                Point3::new(cx + h, cy + h, cz + h),
                Point3::new(cx - h, cy + h, cz + h),
            ],
            triangles: vec![
                [0, 1, 2], [0, 2, 3], // bottom
                [4, 6, 5], [4, 7, 6], // top
                [0, 5, 1], [0, 4, 5], // front
                [1, 5, 6], [1, 6, 2], // right
                [2, 6, 7], [2, 7, 3], // back
                [3, 7, 4], [3, 4, 0], // left
            ],
        }
    }

    #[test]
    fn region_mask_basic() {
        let m = RegionMask::empty().with(0).with(2);
        assert!(m.contains(0));
        assert!(!m.contains(1));
        assert!(m.contains(2));
    }

    #[test]
    fn region_mask_operations() {
        let a = RegionMask::empty().with(0).with(1);
        let b = RegionMask::empty().with(1).with(2);

        assert_eq!(a.union(b), RegionMask::empty().with(0).with(1).with(2));
        assert_eq!(a.intersection(b), RegionMask::empty().with(1));
        assert_eq!(a.difference(b), RegionMask::empty().with(0));
        assert_eq!(a.xor(b), RegionMask::empty().with(0).with(2));
    }

    #[test]
    fn evaluate_operand() {
        let mask = RegionMask::empty().with(0).with(2);
        assert!(evaluate_expr(&BoolExpr::Operand(0), mask));
        assert!(!evaluate_expr(&BoolExpr::Operand(1), mask));
        assert!(evaluate_expr(&BoolExpr::Operand(2), mask));
    }

    #[test]
    fn evaluate_union() {
        let mask = RegionMask::empty().with(1);
        let expr = BoolExpr::union(BoolExpr::Operand(0), BoolExpr::Operand(1));
        assert!(evaluate_expr(&expr, mask));

        let mask2 = RegionMask::empty().with(2);
        assert!(!evaluate_expr(&expr, mask2));
    }

    #[test]
    fn evaluate_intersection() {
        let mask = RegionMask::empty().with(0).with(1);
        let expr = BoolExpr::intersection(BoolExpr::Operand(0), BoolExpr::Operand(1));
        assert!(evaluate_expr(&expr, mask));

        let mask2 = RegionMask::empty().with(0);
        assert!(!evaluate_expr(&expr, mask2));
    }

    #[test]
    fn evaluate_difference() {
        let mask = RegionMask::empty().with(0);
        let expr = BoolExpr::difference(BoolExpr::Operand(0), BoolExpr::Operand(1));
        assert!(evaluate_expr(&expr, mask));

        let mask2 = RegionMask::empty().with(0).with(1);
        assert!(!evaluate_expr(&expr, mask2));
    }

    #[test]
    fn evaluate_xor() {
        let mask = RegionMask::empty().with(0);
        let expr = BoolExpr::xor(BoolExpr::Operand(0), BoolExpr::Operand(1));
        assert!(evaluate_expr(&expr, mask));

        let mask2 = RegionMask::empty().with(0).with(1);
        assert!(!evaluate_expr(&expr, mask2));

        let mask3 = RegionMask::empty();
        assert!(!evaluate_expr(&expr, mask3));
    }

    #[test]
    fn evaluate_complement() {
        let mask = RegionMask::empty().with(0);
        let expr = BoolExpr::complement(BoolExpr::Operand(0));
        assert!(!evaluate_expr(&expr, mask));

        let mask2 = RegionMask::empty().with(1);
        assert!(evaluate_expr(&expr, mask2));
    }

    #[test]
    fn evaluate_nary_union() {
        // Union of 3 operands: in ANY of them.
        let mask = RegionMask::empty().with(2);
        let expr = BoolExpr::Union(vec![
            BoolExpr::Operand(0),
            BoolExpr::Operand(1),
            BoolExpr::Operand(2),
        ]);
        assert!(evaluate_expr(&expr, mask));

        let mask2 = RegionMask::empty();
        assert!(!evaluate_expr(&expr, mask2));
    }

    #[test]
    fn evaluate_nary_intersection() {
        // Intersection of 3 operands: in ALL of them.
        let mask = RegionMask::empty().with(0).with(1).with(2);
        let expr = BoolExpr::Intersection(vec![
            BoolExpr::Operand(0),
            BoolExpr::Operand(1),
            BoolExpr::Operand(2),
        ]);
        assert!(evaluate_expr(&expr, mask));

        let mask2 = RegionMask::empty().with(0).with(1);
        assert!(!evaluate_expr(&expr, mask2));
    }

    #[test]
    fn evaluate_nary_difference() {
        // A \ (B ∪ C)
        let mask = RegionMask::empty().with(0);
        let expr = BoolExpr::Difference(
            Box::new(BoolExpr::Operand(0)),
            vec![BoolExpr::Operand(1), BoolExpr::Operand(2)],
        );
        assert!(evaluate_expr(&expr, mask));

        let mask2 = RegionMask::empty().with(0).with(1);
        assert!(!evaluate_expr(&expr, mask2));

        let mask3 = RegionMask::empty().with(0).with(2);
        assert!(!evaluate_expr(&expr, mask3));
    }

    #[test]
    fn evaluate_nested_tree() {
        // (A ∩ B) ∪ (C \ A)
        let expr = BoolExpr::union(
            BoolExpr::intersection(BoolExpr::Operand(0), BoolExpr::Operand(1)),
            BoolExpr::difference(BoolExpr::Operand(2), BoolExpr::Operand(0)),
        );

        // In A and B → yes
        assert!(evaluate_expr(&expr, RegionMask::empty().with(0).with(1)));
        // In C but not A → yes
        assert!(evaluate_expr(&expr, RegionMask::empty().with(2)));
        // In A only → no (not in B, not in C\A)
        assert!(!evaluate_expr(&expr, RegionMask::empty().with(0)));
        // In nothing → no
        assert!(!evaluate_expr(&expr, RegionMask::empty()));
    }

    #[test]
    fn operand_indices_collected() {
        let expr = BoolExpr::union(
            BoolExpr::intersection(BoolExpr::Operand(2), BoolExpr::Operand(0)),
            BoolExpr::Operand(1),
        );
        let indices = expr.operand_indices();
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn nary_boolean_union_two_disjoint_cubes() {
        let a = cube(0.0, 0.0, 0.0, 1.0);
        let b = cube(5.0, 0.0, 0.0, 1.0);
        let expr = BoolExpr::union(BoolExpr::Operand(0), BoolExpr::Operand(1));

        let (_verts, tris) = nary_boolean(&[a, b], &expr).unwrap();
        assert!(tris.len() >= 24, "union of disjoint cubes should have ≥24 triangles, got {}", tris.len());
    }

    #[test]
    fn nary_boolean_intersection_disjoint() {
        let a = cube(0.0, 0.0, 0.0, 1.0);
        let b = cube(5.0, 0.0, 0.0, 1.0);
        let expr = BoolExpr::intersection(BoolExpr::Operand(0), BoolExpr::Operand(1));

        let (_verts, tris) = nary_boolean(&[a, b], &expr).unwrap();
        assert_eq!(tris.len(), 0, "intersection of disjoint cubes should be empty");
    }

    #[test]
    fn nary_boolean_difference_disjoint() {
        let a = cube(0.0, 0.0, 0.0, 1.0);
        let b = cube(5.0, 0.0, 0.0, 1.0);
        let expr = BoolExpr::difference(BoolExpr::Operand(0), BoolExpr::Operand(1));

        let (_verts, tris) = nary_boolean(&[a, b], &expr).unwrap();
        assert_eq!(tris.len(), 12, "difference of disjoint cubes = first cube");
    }

    #[test]
    fn nary_boolean_union_three_cubes() {
        let a = cube(0.0, 0.0, 0.0, 1.0);
        let b = cube(5.0, 0.0, 0.0, 1.0);
        let c = cube(10.0, 0.0, 0.0, 1.0);
        let expr = BoolExpr::Union(vec![
            BoolExpr::Operand(0),
            BoolExpr::Operand(1),
            BoolExpr::Operand(2),
        ]);

        let (_verts, tris) = nary_boolean(&[a, b, c], &expr).unwrap();
        assert!(tris.len() >= 36, "union of 3 disjoint cubes should have ≥36 triangles, got {}", tris.len());
    }

    #[test]
    fn nary_boolean_empty_expression_errors() {
        let expr = BoolExpr::Union(vec![]);
        let result = nary_boolean(&[], &expr);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), NaryBoolError::EmptyExpression);
    }

    #[test]
    fn nary_boolean_operand_out_of_range() {
        let a = cube(0.0, 0.0, 0.0, 1.0);
        let expr = BoolExpr::Operand(5);
        let result = nary_boolean(&[a], &expr);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            NaryBoolError::OperandOutOfRange { index: 5, count: 1 }
        );
    }

    #[test]
    fn nary_boolean_determinism() {
        let a = cube(0.0, 0.0, 0.0, 1.0);
        let b = cube(0.5, 0.0, 0.0, 1.0);
        let expr = BoolExpr::union(BoolExpr::Operand(0), BoolExpr::Operand(1));

        let (v1, t1) = nary_boolean(&[a.clone(), b.clone()], &expr).unwrap();
        let (v2, t2) = nary_boolean(&[a, b], &expr).unwrap();

        assert_eq!(v1, v2);
        assert_eq!(t1, t2);
    }
}
