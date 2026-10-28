//! Bounded, explicit tapered-branch implicit field over caller-provided samples.
//!
//! This is a scalar *field*, not an exact signed-distance function. For a
//! sample `p` and segment `a`→`b`, clamp the orthogonal projection `t` to
//! `[0,1]`, linearly interpolate the caller's endpoint radii, then compute
//! `distance(p,a+t*(b-a)) - (r0+t*(r1-r0))`. Outside the segment, projection
//! clamps to its endpoint and its endpoint radius. A varying-radius field of
//! this form is generally not an exact Euclidean signed distance and can have
//! derivative discontinuities at clamped segment ends.
//!
//! Segments are combined left-to-right in the caller-supplied order by the
//! existing QualiaDB polynomial smooth-union semantics using an additive
//! numerically stable Core owner, with the explicit blend parameter.
//! Pairwise smooth union is order-dependent. Every segment participates in a
//! global union, so spatially adjacent but graph-unrelated segments can merge;
//! the operation does not validate intended branch adjacency or topology.
//! The caller owns ordering, blend,
//! units, sampling density, source/attribute provenance and publication policy.
//! This API neither creates a mesh nor certifies a branch junction, manifold,
//! biological representation, UV mapping or asset.
//!
//! All inputs/outputs are caller-owned; no heap allocation. Preflight validates
//! all inputs and buffers before writes. Arithmetic is checked for finite
//! outputs as a defensive backstop: if that late check fails, output is scratch
//! and may be partially modified. Only `Ok` permits publication. The operator
//! has no internal cancellation; callers bound and cancel their enclosing job.

use super::primitives::Point3;
use super::sdf_ops::sdf_smooth_union_stable;

/// Deliberately conservative arithmetic envelope, not a species assumption.
/// Coordinates, endpoint radii and blend are expressed in the same caller unit.
pub const TAPERED_FIELD_MAGNITUDE_LIMIT: f64 = 1.0e100;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TaperedFieldSegment {
    pub a: Point3,
    pub b: Point3,
    pub radius_a: f64,
    pub radius_b: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaperedFieldError {
    EmptySamples,
    EmptySegments,
    SizeOverflow,
    WorkLimitExceeded,
    OutputTooSmall,
    InvalidInput,
    DegenerateSegment,
    NumericPrecision,
    NonFiniteResult,
}

fn finite_bounded_scalar(v: f64, limit: f64) -> bool { v.is_finite() && v.abs() <= limit }
fn finite_bounded_point(p: Point3, limit: f64) -> bool {
    finite_bounded_scalar(p.x,limit) && finite_bounded_scalar(p.y,limit) && finite_bounded_scalar(p.z,limit)
}
fn difference(a: Point3,b: Point3) -> Point3 { Point3::new(a.x-b.x,a.y-b.y,a.z-b.z) }
fn length(v: Point3) -> f64 { v.x.hypot(v.y).hypot(v.z) }

/// Compute bounded work before evaluation. `max_evaluations` counts one
/// sample–segment field evaluation; it is not a runtime or memory measurement.
pub fn tapered_field_work(sample_count:usize, segment_count:usize, max_samples:usize, max_segments:usize, max_evaluations:usize)
    -> Result<usize,TaperedFieldError> {
    if sample_count==0 { return Err(TaperedFieldError::EmptySamples); }
    if segment_count==0 { return Err(TaperedFieldError::EmptySegments); }
    let work=sample_count.checked_mul(segment_count).ok_or(TaperedFieldError::SizeOverflow)?;
    if sample_count>max_samples || segment_count>max_segments || work>max_evaluations { return Err(TaperedFieldError::WorkLimitExceeded); }
    Ok(work)
}

/// Evaluates samples into `out`; only its first `samples.len()` entries change.
/// `min_segment_length` is a caller-selected positive threshold in coordinate
/// units and rejects degenerate/nearly degenerate axes before any output write.
/// `projection_tolerance` is a caller-selected positive absolute conditioning
/// threshold in coordinate units. A conservative scale screen and consistency
/// residual can refuse during evaluation; this screen is not a formal proof of
/// an error bound. `blend` is finite and nonnegative; zero means pointwise minimum.
pub fn tapered_branch_field_into(
    samples:&[Point3], segments:&[TaperedFieldSegment], blend:f64,
    min_segment_length:f64, projection_tolerance:f64, max_samples:usize, max_segments:usize, max_evaluations:usize,
    out:&mut [f64],
) -> Result<usize,TaperedFieldError> {
    let work=tapered_field_work(samples.len(),segments.len(),max_samples,max_segments,max_evaluations)?;
    if out.len()<samples.len() { return Err(TaperedFieldError::OutputTooSmall); }
    let limit=TAPERED_FIELD_MAGNITUDE_LIMIT;
    if !finite_bounded_scalar(blend,limit) || blend<0.0 || !finite_bounded_scalar(min_segment_length,limit) || min_segment_length<=0.0 || !finite_bounded_scalar(projection_tolerance,limit) || projection_tolerance<=0.0 {
        return Err(TaperedFieldError::InvalidInput);
    }
    if samples.iter().any(|p|!finite_bounded_point(*p,limit)) { return Err(TaperedFieldError::InvalidInput); }
    for segment in segments {
        if !finite_bounded_point(segment.a,limit) || !finite_bounded_point(segment.b,limit) ||
           !finite_bounded_scalar(segment.radius_a,limit) || !finite_bounded_scalar(segment.radius_b,limit) ||
           segment.radius_a<=0.0 || segment.radius_b<=0.0 { return Err(TaperedFieldError::InvalidInput); }
        let axis=length(difference(segment.b,segment.a));
        if !axis.is_finite() || axis<f64::MIN_POSITIVE || axis<min_segment_length { return Err(TaperedFieldError::DegenerateSegment); }
    }
    for (index,&p) in samples.iter().enumerate() {
        let mut value=0.0;
        for (which,segment) in segments.iter().enumerate() {
            let axis=difference(segment.b,segment.a); let axis_length=length(axis);
            let unit=Point3::new(axis.x/axis_length,axis.y/axis_length,axis.z/axis_length);
            let da=difference(p,segment.a);let db=difference(p,segment.b);
            let from_a=da.x*unit.x+da.y*unit.y+da.z*unit.z;
            let from_b=db.x*unit.x+db.y*unit.y+db.z*unit.z;
            if !from_a.is_finite() || !from_b.is_finite() {return Err(TaperedFieldError::NumericPrecision);}
            // Screen cancellation using world-coordinate scale and the identity
            // dot(p-a,u)-dot(p-b,u)=|b-a|. This conservative screen is not a
            // certified error bound; the caller chooses its absolute threshold.
            let scale=p.x.abs()+p.y.abs()+p.z.abs()+segment.a.x.abs()+segment.a.y.abs()+segment.a.z.abs()+segment.b.x.abs()+segment.b.y.abs()+segment.b.z.abs()+axis_length;
            let rounding_screen=scale*(64.0*f64::EPSILON);
            let residual=((from_a-from_b)-axis_length).abs();
            if !rounding_screen.is_finite() || !residual.is_finite() || rounding_screen>projection_tolerance || residual>projection_tolerance {return Err(TaperedFieldError::NumericPrecision);}
            let (center,radius)=if from_a<=0.0 && from_b<0.0 {(segment.a,segment.radius_a)}
            else if from_a>0.0 && from_b>=0.0 {(segment.b,segment.radius_b)}
            else if from_a>0.0 && from_b<0.0 {
                // Project from the nearer endpoint. A single projection from
                // `a` can round to the far endpoint and lose local detail.
                let u=from_a;let v= -from_b;
                let scale=u.max(v);let denom=(u/scale)+(v/scale);
                let (center,radius)=if u<=v {
                    if u>axis_length {return Err(TaperedFieldError::NumericPrecision);}
                    let fraction=(u/scale)/denom;
                    if !fraction.is_finite() || fraction<=0.0 {return Err(TaperedFieldError::NumericPrecision);}
                    (Point3::new(segment.a.x+u*unit.x,segment.a.y+u*unit.y,segment.a.z+u*unit.z),
                     (1.0-fraction)*segment.radius_a+fraction*segment.radius_b)
                } else {
                    if v>axis_length {return Err(TaperedFieldError::NumericPrecision);}
                    let fraction=(v/scale)/denom;
                    if !fraction.is_finite() || fraction<=0.0 {return Err(TaperedFieldError::NumericPrecision);}
                    (Point3::new(segment.b.x-v*unit.x,segment.b.y-v*unit.y,segment.b.z-v*unit.z),
                     fraction*segment.radius_a+(1.0-fraction)*segment.radius_b)
                };
                if center==segment.a || center==segment.b || !radius.is_finite() || radius<=0.0 {return Err(TaperedFieldError::NumericPrecision);}
                (center,radius)
            } else {return Err(TaperedFieldError::NumericPrecision);};
            let field=length(difference(p,center))-radius;
            value=if which==0 {field} else {sdf_smooth_union_stable(value,field,blend)};
            if !value.is_finite() { return Err(TaperedFieldError::NonFiniteResult); }
        }
        out[index]=value;
    }
    Ok(work)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::sdf_ops::sdf_capsule;
    fn seg(r0:f64,r1:f64)->TaperedFieldSegment { TaperedFieldSegment {a:Point3::new(0.0,0.0,0.0),b:Point3::new(0.0,0.0,2.0),radius_a:r0,radius_b:r1} }
    #[test]
    fn explicit_endpoint_radii_produce_declared_linear_profile() {
        let points=[Point3::new(0.35,0.0,0.0),Point3::new(0.35,0.0,1.0),Point3::new(0.35,0.0,2.0)];
        let mut out=[99.0;4];let n=tapered_branch_field_into(&points,&[seg(0.4,0.2)],0.0,1e-6,1e-9,3,1,3,&mut out).unwrap();
        assert_eq!(n,3);for (actual,expected) in out[..3].iter().zip([-0.05,0.05,0.15]) {assert!((actual-expected).abs()<1e-12);}
        assert_eq!(out[3],99.0);
    }
    #[test]
    fn constant_radius_matches_existing_capsule_and_blend_is_ordered() {
        let segment=seg(0.5,0.5);let points=[Point3::new(1.0,0.0,1.0),Point3::new(0.0,0.0,2.2),Point3::new(0.0,0.0,-0.2)];let mut out=[0.0;3];
        tapered_branch_field_into(&points,&[segment],0.0,1e-6,1e-9,3,1,3,&mut out).unwrap();
        for (p,value) in points.iter().zip(out) {assert!((value-sdf_capsule(*p,segment.a,segment.b,0.5)).abs()<1e-12);}
        let p=[Point3::new(1.0,0.0,1.0)];let mut forward=[0.0];let mut reverse=[0.0];
        let v=[seg(0.8,0.8),seg(0.9,0.9),seg(1.0,1.0)];let reversed=[v[2],v[1],v[0]];
        tapered_branch_field_into(&p,&v,0.5,1e-6,1e-9,1,3,3,&mut forward).unwrap();
        tapered_branch_field_into(&p,&reversed,0.5,1e-6,1e-9,1,3,3,&mut reverse).unwrap();
        assert!((forward[0]-reverse[0]).abs()>1e-6);
    }
    #[test]
    fn extreme_endpoints_and_distant_union_preserve_representable_values() {
        let mut out=[77.0];
        let long=TaperedFieldSegment {a:Point3::new(1e100,0.0,0.0),b:Point3::new(1.0,0.0,0.0),radius_a:1.0,radius_b:1.0};
        tapered_branch_field_into(&[long.b],&[long],0.0,1e-6,1e90,1,1,1,&mut out).unwrap(); assert_eq!(out,[-1.0]);
        let diagonal=TaperedFieldSegment {a:Point3::new(1e100,1e100,0.0),b:Point3::new(0.0,0.0,0.0),radius_a:0.8,radius_b:0.8};
        tapered_branch_field_into(&[Point3::new(1.0,0.0,0.0)],&[diagonal],0.0,1e-6,1e90,1,1,1,&mut out).unwrap(); assert!((out[0]-(0.5_f64.sqrt()-0.8)).abs()<1e-12);
        let reversed=TaperedFieldSegment {a:diagonal.b,b:diagonal.a,radius_a:0.8,radius_b:0.8};
        tapered_branch_field_into(&[Point3::new(1.0,0.0,0.0)],&[reversed],0.0,1e-6,1e90,1,1,1,&mut out).unwrap(); assert!((out[0]-(0.5_f64.sqrt()-0.8)).abs()<1e-12);
        let tiny=seg(1.0,1e-100);
        tapered_branch_field_into(&[tiny.b],&[tiny],0.0,1e-6,1e90,1,1,1,&mut out).unwrap(); assert_eq!(out,[-1e-100]);
        let local=seg(1.0,1.0);let distant=TaperedFieldSegment {a:Point3::new(1e100,0.0,0.0),b:Point3::new(1e100,0.0,2.0),radius_a:1.0,radius_b:1.0};
        let sample=[Point3::new(0.0,0.0,1.0)];
        for pair in [[local,distant],[distant,local]] {tapered_branch_field_into(&sample,&pair,1.0,1e-6,1e90,1,2,2,&mut out).unwrap();assert_eq!(out,[-1.0]);}
        let near_limit=TaperedFieldSegment {a:Point3::new(5e99,0.0,0.0),b:Point3::new(5e99,0.0,2.0),radius_a:1.0,radius_b:1.0};
        for pair in [[local,near_limit],[near_limit,local]] {tapered_branch_field_into(&sample,&pair,5.000000000000001e99,1e-6,1e90,1,2,2,&mut out).unwrap();assert!(out[0].is_finite() && out[0]< -1.0,"inside blend must not invert the sign: {}",out[0]);}
    }
    #[test]
    fn ill_conditioned_projection_refuses_with_caller_tolerance() {
        let segment=TaperedFieldSegment {a:Point3::new(0.0,0.0,0.0),b:Point3::new(4.0,4.0,0.0),radius_a:0.8,radius_b:0.8};
        let sample=[Point3::new(1e16,-1e16+2.0,0.0)];let mut out=[77.0];
        assert_eq!(tapered_branch_field_into(&sample,&[segment],0.0,1e-6,1e-8,1,1,1,&mut out),Err(TaperedFieldError::NumericPrecision));assert_eq!(out,[77.0]);
        assert_eq!(tapered_branch_field_into(&sample,&[segment],0.0,1e-6,0.0,1,1,1,&mut out),Err(TaperedFieldError::InvalidInput));assert_eq!(out,[77.0]);
    }
    #[test]
    fn subnormal_axes_refuse_before_writes() {
        let tiny=TaperedFieldSegment {a:Point3::new(0.0,0.0,0.0),b:Point3::new(f64::from_bits(1),0.0,0.0),radius_a:1.0,radius_b:1.0};
        let mut out=[77.0];
        assert_eq!(tapered_branch_field_into(&[tiny.a],&[tiny],0.0,f64::from_bits(1),1e-9,1,1,1,&mut out),Err(TaperedFieldError::DegenerateSegment));assert_eq!(out,[77.0]);
    }
    #[test]
    fn nonfinite_input_and_empty_inventories_refuse_without_writes() {
        let points=[Point3::new(0.0,0.0,0.0),Point3::new(f64::NAN,0.0,0.0)];let valid=seg(0.4,0.2);let mut out=[77.0;2];
        assert_eq!(tapered_branch_field_into(&points,&[valid],0.0,1e-6,1e-9,2,1,2,&mut out),Err(TaperedFieldError::InvalidInput));assert_eq!(out,[77.0;2]);
        assert_eq!(tapered_branch_field_into(&points[..0],&[valid],0.0,1e-6,1e-9,2,1,2,&mut out),Err(TaperedFieldError::EmptySamples));assert_eq!(out,[77.0;2]);
        assert_eq!(tapered_branch_field_into(&points[..1],&[],0.0,1e-6,1e-9,2,1,2,&mut out),Err(TaperedFieldError::EmptySegments));assert_eq!(out,[77.0;2]);
        for (blend,min_len,tolerance) in [(f64::INFINITY,1e-6,1e-9),(0.0,f64::NAN,1e-9),(0.0,1e-6,f64::NAN)] {
            assert_eq!(tapered_branch_field_into(&points[..1],&[valid],blend,min_len,tolerance,1,1,1,&mut out),Err(TaperedFieldError::InvalidInput));assert_eq!(out,[77.0;2]);
        }
    }
    #[test]
    fn bad_inputs_work_and_short_output_fail_before_any_write() {
        let points=[Point3::new(0.0,0.0,0.0),Point3::new(0.0,1.0,0.0)];let good=seg(0.4,0.2);
        let mut out=[77.0;2];let mut bad=good;bad.b=bad.a;
        assert_eq!(tapered_branch_field_into(&points,&[good,bad],0.1,1e-6,1e-9,2,2,4,&mut out),Err(TaperedFieldError::DegenerateSegment));assert_eq!(out,[77.0;2]);
        bad=good;bad.radius_b=f64::NAN;
        assert_eq!(tapered_branch_field_into(&points,&[good,bad],0.1,1e-6,1e-9,2,2,4,&mut out),Err(TaperedFieldError::InvalidInput));assert_eq!(out,[77.0;2]);
        assert_eq!(tapered_branch_field_into(&points,&[good],-1.0,1e-6,1e-9,2,1,2,&mut out),Err(TaperedFieldError::InvalidInput));assert_eq!(out,[77.0;2]);
        assert_eq!(tapered_branch_field_into(&points,&[good],0.0,1e-6,1e-9,2,1,1,&mut out),Err(TaperedFieldError::WorkLimitExceeded));assert_eq!(out,[77.0;2]);
        assert_eq!(tapered_branch_field_into(&points,&[good],0.0,1e-6,1e-9,2,1,2,&mut out[..1]),Err(TaperedFieldError::OutputTooSmall));assert_eq!(out,[77.0;2]);
        assert_eq!(tapered_field_work(usize::MAX,2,usize::MAX,usize::MAX,usize::MAX),Err(TaperedFieldError::SizeOverflow));
    }
}
