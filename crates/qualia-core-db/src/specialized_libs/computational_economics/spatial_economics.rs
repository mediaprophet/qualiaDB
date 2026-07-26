//! Spatial economics: gravity model, transport costs, location-allocation,
//! spatial autocorrelation (Moran's I), and Hotelling resource extraction.
//!
//! Allocation class: **HotZeroHeap**. No `Vec`/`String`/`Box` in any kernel.
//!
//! Assumptions:
//! - Gravity model: flow is proportional to masses and inversely proportional
//!   to distance raised to a friction power `gamma > 0`.
//! - Moran's I assumes symmetric weights matrix.
//! - Hotelling: non-renewable resource, known reserves, competitive market,
//!   price grows at the discount rate.

/// Maximum locations in a bounded spatial problem.
pub const MAX_LOCATIONS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialError {
    InvalidInput,
    BufferTooSmall,
    NonFinite,
    InsufficientData,
}

fn require_finite(x: f64) -> Result<(), SpatialError> {
    if x.is_finite() {
        Ok(())
    } else {
        Err(SpatialError::NonFinite)
    }
}

/// Gravity flow between two locations: `mass_i^alpha * mass_j^beta / distance^gamma`.
pub fn gravity_flow(
    mass_i: f64,
    mass_j: f64,
    distance: f64,
    alpha: f64,
    beta: f64,
    gamma: f64,
) -> Result<f64, SpatialError> {
    require_finite(mass_i)?;
    require_finite(mass_j)?;
    require_finite(distance)?;
    require_finite(alpha)?;
    require_finite(beta)?;
    require_finite(gamma)?;
    if mass_i < 0.0 || mass_j < 0.0 || distance <= 0.0 {
        return Err(SpatialError::InvalidInput);
    }
    Ok(mass_i.powf(alpha) * mass_j.powf(beta) / distance.powf(gamma))
}

/// Build an `n x n` gravity flow matrix from masses and distances.
///
/// `masses` is length `n`, `distances` is `n x n` row-major. Writes into `out`.
pub fn gravity_flow_matrix_into(
    masses: &[f64],
    distances: &[f64],
    n: usize,
    alpha: f64,
    beta: f64,
    gamma: f64,
    out: &mut [f64],
) -> Result<usize, SpatialError> {
    if n == 0
        || n > MAX_LOCATIONS
        || masses.len() < n
        || distances.len() < n * n
        || out.len() < n * n
    {
        return Err(SpatialError::InvalidInput);
    }
    for i in 0..n {
        for j in 0..n {
            if i == j {
                out[i * n + j] = 0.0;
            } else {
                out[i * n + j] = gravity_flow(
                    masses[i],
                    masses[j],
                    distances[i * n + j],
                    alpha,
                    beta,
                    gamma,
                )?;
            }
        }
    }
    Ok(n * n)
}

/// Euclidean distance matrix from 2D coordinates.
///
/// `coordinates` is `n x 2` row-major. Writes `n x n` distances into `out`.
pub fn transport_cost_matrix_into(
    coordinates: &[f64],
    n: usize,
    out: &mut [f64],
) -> Result<usize, SpatialError> {
    if n == 0 || n > MAX_LOCATIONS || coordinates.len() < n * 2 || out.len() < n * n {
        return Err(SpatialError::InvalidInput);
    }
    for v in coordinates.iter().take(n * 2) {
        require_finite(*v)?;
    }
    for i in 0..n {
        for j in 0..n {
            let dx = coordinates[i * 2] - coordinates[j * 2];
            let dy = coordinates[i * 2 + 1] - coordinates[j * 2 + 1];
            out[i * n + j] = (dx * dx + dy * dy).sqrt();
        }
    }
    Ok(n * n)
}

/// Total transport cost: `sum_{i,j} flows[i][j] * distances[i][j]`.
pub fn total_transport_cost(
    flows: &[f64],
    distances: &[f64],
    n: usize,
) -> Result<f64, SpatialError> {
    if n == 0 || n > MAX_LOCATIONS || flows.len() < n * n || distances.len() < n * n {
        return Err(SpatialError::InvalidInput);
    }
    let mut total = 0.0;
    for i in 0..n {
        for j in 0..n {
            total += flows[i * n + j] * distances[i * n + j];
        }
    }
    Ok(total)
}

/// Assign each demand point to its nearest facility.
///
/// `demands` is `n_demands x 2`, `facilities` is `n_facilities x 2`.
/// Writes facility index into `out[..n_demands]`.
pub fn nearest_facility_into(
    demands: &[f64],
    facilities: &[f64],
    n_demands: usize,
    n_facilities: usize,
    out: &mut [usize],
) -> Result<usize, SpatialError> {
    if n_demands == 0
        || n_facilities == 0
        || n_demands > MAX_LOCATIONS
        || n_facilities > MAX_LOCATIONS
    {
        return Err(SpatialError::InvalidInput);
    }
    if demands.len() < n_demands * 2 || facilities.len() < n_facilities * 2 || out.len() < n_demands
    {
        return Err(SpatialError::BufferTooSmall);
    }
    for d in 0..n_demands {
        let dx = demands[d * 2];
        let dy = demands[d * 2 + 1];
        let mut best = 0usize;
        let mut best_dist = f64::INFINITY;
        for f in 0..n_facilities {
            let fx = facilities[f * 2];
            let fy = facilities[f * 2 + 1];
            let dist = (dx - fx).powi(2) + (dy - fy).powi(2);
            if dist < best_dist {
                best_dist = dist;
                best = f;
            }
        }
        out[d] = best;
    }
    Ok(n_demands)
}

/// Moran's I spatial autocorrelation index.
///
/// `I = (n / S0) * (sum_{i,j} w_ij * (x_i - x_bar) * (x_j - x_bar)) / (sum_i (x_i - x_bar)^2)`
///
/// where `S0 = sum of all weights`. Returns I in approximately [-1, 1].
pub fn morans_i(values: &[f64], weights: &[f64], n: usize) -> Result<f64, SpatialError> {
    if n < 2 || n > MAX_LOCATIONS || values.len() < n || weights.len() < n * n {
        return Err(SpatialError::InsufficientData);
    }
    for v in values.iter().take(n) {
        require_finite(*v)?;
    }
    let mean: f64 = values.iter().take(n).sum::<f64>() / n as f64;
    let mut s0 = 0.0;
    let mut numerator = 0.0;
    for i in 0..n {
        for j in 0..n {
            s0 += weights[i * n + j];
            numerator += weights[i * n + j] * (values[i] - mean) * (values[j] - mean);
        }
    }
    let mut denominator = 0.0;
    for i in 0..n {
        denominator += (values[i] - mean).powi(2);
    }
    if s0 == 0.0 || denominator == 0.0 {
        return Err(SpatialError::InvalidInput);
    }
    Ok((n as f64 / s0) * (numerator / denominator))
}

/// Hotelling resource extraction path.
///
/// Under Hotelling's rule, the resource price grows at the discount rate, so
/// extraction declines geometrically: `q_t = q_0 / (1 + r)^t` where `q_0` is
/// chosen so that `sum_t q_t = initial_stock`. This gives
/// `q_0 = initial_stock * r / (1 - (1+r)^{-n})` for finite horizon.
///
/// Writes `n_periods` extraction quantities into `out`.
pub fn hotelling_extraction_into(
    initial_stock: f64,
    discount_rate: f64,
    n_periods: usize,
    out: &mut [f64],
) -> Result<usize, SpatialError> {
    if initial_stock <= 0.0 || discount_rate <= 0.0 || n_periods == 0 || out.len() < n_periods {
        return Err(SpatialError::InvalidInput);
    }
    require_finite(initial_stock)?;
    require_finite(discount_rate)?;
    // q_0 such that sum_{t=0}^{n-1} q_0/(1+r)^t = initial_stock
    // q_0 * (1 - (1+r)^{-n}) / (1 - (1+r)^{-1}) = initial_stock
    // q_0 * (1 - (1+r)^{-n}) / (r/(1+r)) = initial_stock
    // q_0 = initial_stock * r / ((1+r) * (1 - (1+r)^{-n}))
    let r = discount_rate;
    let n = n_periods as f64;
    let denom = (1.0 + r) * (1.0 - (1.0 + r).powf(-n));
    if denom <= 0.0 {
        return Err(SpatialError::InvalidInput);
    }
    let q0 = initial_stock * r / denom;
    for t in 0..n_periods {
        out[t] = q0 / (1.0 + r).powi(t as i32);
    }
    Ok(n_periods)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn gravity_flow_basic() {
        // mass=100, distance=10, alpha=beta=gamma=1 → 100*100/10 = 1000
        let f = gravity_flow(100.0, 100.0, 10.0, 1.0, 1.0, 1.0).unwrap();
        assert!(approx(f, 1000.0, 1e-6));
    }

    #[test]
    fn gravity_matrix_symmetric() {
        let masses = [10.0, 20.0, 30.0];
        // Distances: symmetric matrix.
        let distances = [0.0, 1.0, 2.0, 1.0, 0.0, 1.5, 2.0, 1.5, 0.0];
        let mut out = [0.0f64; 9];
        gravity_flow_matrix_into(&masses, &distances, 3, 1.0, 1.0, 1.0, &mut out).unwrap();
        // Check symmetry: flow[i][j] == flow[j][i] when masses and distances are symmetric.
        assert!(approx(out[1], out[3], 1e-9));
        assert!(approx(out[2], out[6], 1e-9));
        assert!(approx(out[5], out[7], 1e-9));
        // Diagonal is zero.
        assert_eq!(out[0], 0.0);
        assert_eq!(out[4], 0.0);
        assert_eq!(out[8], 0.0);
    }

    #[test]
    fn transport_cost_matrix_line() {
        // 3 points on a line: (0,0), (3,0), (7,0)
        let coords = [0.0, 0.0, 3.0, 0.0, 7.0, 0.0];
        let mut dist = [0.0f64; 9];
        transport_cost_matrix_into(&coords, 3, &mut dist).unwrap();
        assert!(approx(dist[0], 0.0, 1e-9));
        assert!(approx(dist[1], 3.0, 1e-9));
        assert!(approx(dist[2], 7.0, 1e-9));
        assert!(approx(dist[5], 4.0, 1e-9)); // |7-3|
    }

    #[test]
    fn total_transport_cost_hand_computed() {
        let flows = [0.0, 10.0, 5.0, 0.0];
        let distances = [0.0, 2.0, 3.0, 0.0];
        let cost = total_transport_cost(&flows, &distances, 2).unwrap();
        // 10*2 + 5*3 = 35
        assert!(approx(cost, 35.0, 1e-9));
    }

    #[test]
    fn nearest_facility_assignment() {
        // Demands: (0,0), (10,0), (5,5)
        // Facilities: (1,1), (8,1)
        let demands = [0.0, 0.0, 10.0, 0.0, 5.0, 5.0];
        let facilities = [1.0, 1.0, 8.0, 1.0];
        let mut assign = [0usize; 3];
        nearest_facility_into(&demands, &facilities, 3, 2, &mut assign).unwrap();
        // (0,0) → nearest is (1,1) → facility 0
        // (10,0) → nearest is (8,1) → facility 1
        // (5,5) → (1,1) dist^2 = 16+16=32; (8,1) dist^2 = 9+16=25 → facility 1
        assert_eq!(assign, [0, 1, 1]);
    }

    #[test]
    fn morans_i_clustered_positive() {
        // Clustered values with high weights on neighbors → positive I.
        let values = [1.0, 1.0, 10.0, 10.0];
        // Weights: neighbors (adjacent in sequence).
        let weights = [
            0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let i = morans_i(&values, &weights, 4).unwrap();
        assert!(
            i > 0.0,
            "Moran's I should be positive for clustered data, got {}",
            i
        );
    }

    #[test]
    fn morans_i_dispersed_negative() {
        // Dispersed: high values next to low values.
        let values = [10.0, 1.0, 10.0, 1.0];
        let weights = [
            0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let i = morans_i(&values, &weights, 4).unwrap();
        assert!(
            i < 0.0,
            "Moran's I should be negative for dispersed data, got {}",
            i
        );
    }

    #[test]
    fn hotelling_extraction_declines() {
        let mut path = [0.0f64; 10];
        hotelling_extraction_into(100.0, 0.1, 10, &mut path).unwrap();
        // Extraction should decline over time.
        for t in 1..10 {
            assert!(
                path[t] < path[t - 1],
                "path[{}] = {} should be < path[{}] = {}",
                t,
                path[t],
                t - 1,
                path[t - 1]
            );
        }
        // Total extraction should approximately equal initial stock.
        let total: f64 = path.iter().sum();
        assert!(approx(total, 100.0, 1.0));
    }

    #[test]
    fn empty_rejected() {
        assert_eq!(
            gravity_flow_matrix_into(&[], &[], 0, 1.0, 1.0, 1.0, &mut []).unwrap_err(),
            SpatialError::InvalidInput
        );
    }

    #[test]
    fn buffer_too_small() {
        let mut out = [0.0f64; 2];
        let err =
            transport_cost_matrix_into(&[0.0, 0.0, 1.0, 1.0, 2.0, 2.0], 3, &mut out).unwrap_err();
        assert_eq!(err, SpatialError::InvalidInput);
    }
}
