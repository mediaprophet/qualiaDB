//! Bounded finite-difference potential solver.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PotentialError {
    InvalidGrid,
    DimensionMismatch,
    NonFiniteInput,
    ConvergenceFailed { residual: f64 },
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoissonGrid {
    pub width: usize,
    pub height: usize,
    pub spacing: f64,
}

impl PoissonGrid {
    pub fn point_count(self) -> Option<usize> {
        self.width.checked_mul(self.height)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoissonReport {
    pub iterations: u32,
    pub residual_inf: f64,
    pub minimum: f64,
    pub maximum: f64,
}

pub fn solve_poisson_dirichlet(
    grid: PoissonGrid,
    source: &[f64],
    boundary_values: &[f64],
    solution: &mut [f64],
    tolerance: f64,
    max_iterations: u32,
) -> Result<PoissonReport, PotentialError> {
    let count = grid.point_count().ok_or(PotentialError::InvalidGrid)?;
    if grid.width < 3
        || grid.height < 3
        || !grid.spacing.is_finite()
        || grid.spacing <= 0.0
        || !tolerance.is_finite()
        || tolerance <= 0.0
        || max_iterations == 0
    {
        return Err(PotentialError::InvalidGrid);
    }
    if source.len() != count || boundary_values.len() != count || solution.len() != count {
        return Err(PotentialError::DimensionMismatch);
    }
    if source
        .iter()
        .chain(boundary_values)
        .any(|value| !value.is_finite())
    {
        return Err(PotentialError::NonFiniteInput);
    }

    for y in 0..grid.height {
        for x in 0..grid.width {
            let index = y * grid.width + x;
            if x == 0 || y == 0 || x + 1 == grid.width || y + 1 == grid.height {
                solution[index] = boundary_values[index];
            } else if !solution[index].is_finite() {
                solution[index] = 0.0;
            }
        }
    }

    let h2 = grid.spacing * grid.spacing;
    let mut residual = f64::INFINITY;
    for iteration in 1..=max_iterations {
        for y in 1..grid.height - 1 {
            for x in 1..grid.width - 1 {
                let index = y * grid.width + x;
                solution[index] = 0.25
                    * (solution[index - 1]
                        + solution[index + 1]
                        + solution[index - grid.width]
                        + solution[index + grid.width]
                        + h2 * source[index]);
            }
        }

        residual = 0.0;
        for y in 1..grid.height - 1 {
            for x in 1..grid.width - 1 {
                let index = y * grid.width + x;
                let discrete = (4.0 * solution[index]
                    - solution[index - 1]
                    - solution[index + 1]
                    - solution[index - grid.width]
                    - solution[index + grid.width])
                    / h2;
                residual = residual.max((discrete - source[index]).abs());
            }
        }
        if residual <= tolerance {
            let (minimum, maximum) = solution.iter().fold(
                (f64::INFINITY, f64::NEG_INFINITY),
                |(minimum, maximum), value| (minimum.min(*value), maximum.max(*value)),
            );
            return Ok(PoissonReport {
                iterations: iteration,
                residual_inf: residual,
                minimum,
                maximum,
            });
        }
    }
    Err(PotentialError::ConvergenceFailed { residual })
}

pub fn discrete_maximum_principle_holds(
    grid: PoissonGrid,
    source: &[f64],
    boundary_values: &[f64],
    solution: &[f64],
    tolerance: f64,
) -> Result<bool, PotentialError> {
    let count = grid.point_count().ok_or(PotentialError::InvalidGrid)?;
    if source.len() != count || boundary_values.len() != count || solution.len() != count {
        return Err(PotentialError::DimensionMismatch);
    }
    if source.iter().any(|value| value.abs() > tolerance) {
        return Ok(false);
    }
    let mut boundary_minimum = f64::INFINITY;
    let mut boundary_maximum = f64::NEG_INFINITY;
    for y in 0..grid.height {
        for x in 0..grid.width {
            if x == 0 || y == 0 || x + 1 == grid.width || y + 1 == grid.height {
                let value = boundary_values[y * grid.width + x];
                boundary_minimum = boundary_minimum.min(value);
                boundary_maximum = boundary_maximum.max(value);
            }
        }
    }
    Ok(solution.iter().all(|value| {
        *value >= boundary_minimum - tolerance && *value <= boundary_maximum + tolerance
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manufactured_poisson_solution_converges_with_residual() {
        const N: usize = 25;
        let grid = PoissonGrid {
            width: N,
            height: N,
            spacing: 1.0 / (N - 1) as f64,
        };
        let mut source = vec![0.0; N * N];
        let boundary = vec![0.0; N * N];
        let mut solution = vec![0.0; N * N];
        for y in 0..N {
            let yy = y as f64 * grid.spacing;
            for x in 0..N {
                let xx = x as f64 * grid.spacing;
                source[y * N + x] = 2.0 * (yy - yy * yy) + 2.0 * (xx - xx * xx);
            }
        }
        let report =
            solve_poisson_dirichlet(grid, &source, &boundary, &mut solution, 1e-8, 50_000).unwrap();
        assert!(report.residual_inf <= 1e-8);
        let mut maximum_error = 0.0_f64;
        for y in 0..N {
            let yy = y as f64 * grid.spacing;
            for x in 0..N {
                let xx = x as f64 * grid.spacing;
                let exact = xx * (1.0 - xx) * yy * (1.0 - yy);
                maximum_error = maximum_error.max((solution[y * N + x] - exact).abs());
            }
        }
        assert!(maximum_error < 5e-4);
    }

    #[test]
    fn harmonic_solution_obeys_discrete_maximum_principle() {
        const N: usize = 9;
        let grid = PoissonGrid {
            width: N,
            height: N,
            spacing: 1.0 / (N - 1) as f64,
        };
        let source = [0.0; N * N];
        let mut boundary = [0.0; N * N];
        for y in 0..N {
            boundary[y * N + N - 1] = 1.0;
        }
        let mut solution = [0.0; N * N];
        solve_poisson_dirichlet(grid, &source, &boundary, &mut solution, 1e-9, 20_000).unwrap();
        assert!(
            discrete_maximum_principle_holds(grid, &source, &boundary, &solution, 1e-9).unwrap()
        );
    }
}
