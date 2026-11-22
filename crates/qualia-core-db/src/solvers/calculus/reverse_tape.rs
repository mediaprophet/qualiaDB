//! Reverse-mode scalar automatic differentiation in caller-owned, Sentinel-bounded storage.

use core::marker::PhantomData;

use super::workspace::{checked_workspace_bytes, WorkspaceError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReverseTapeError {
    Workspace(WorkspaceError),
    InvalidVariable,
    InvalidDomain,
    NonFinite,
    OutputLength { expected: usize, actual: usize },
}

type Brand<'id> = PhantomData<fn(&'id ()) -> &'id ()>;

/// Tape-local handle that cannot escape the branded evaluation.
#[derive(Clone, Copy)]
pub struct Variable<'id> {
    index: usize,
    _brand: Brand<'id>,
}

#[derive(Clone, Copy)]
struct Edge {
    parent: usize,
    partial: f64,
}

/// One caller-owned Wengert record. Its fields are populated by the tape.
#[derive(Clone, Copy)]
pub struct ReverseNode {
    value: f64,
    adjoint: f64,
    edges: [Edge; 2],
    arity: u8,
}

impl ReverseNode {
    const NO_EDGE: Edge = Edge { parent: 0, partial: 0.0 };

    pub const EMPTY: Self = Self {
        value: 0.0,
        adjoint: 0.0,
        edges: [Self::NO_EDGE; 2],
        arity: 0,
    };
}

/// A fixed Wengert tape and its in-place reverse scratch.
pub struct ReverseTape<'storage, 'id> {
    nodes: &'storage mut [ReverseNode],
    used: usize,
    _brand: Brand<'id>,
}

/// Open a fresh tape for one pass. Reusing the same slice starts a new branded
/// evaluation and cannot revive variables from a previous pass.
pub fn with_reverse_tape<'storage, R>(
    storage: &'storage mut [ReverseNode],
    evaluate: impl for<'id> FnOnce(&mut ReverseTape<'storage, 'id>) -> R,
) -> Result<R, ReverseTapeError> {
    checked_workspace_bytes::<ReverseNode>(storage.len()).map_err(ReverseTapeError::Workspace)?;
    let mut tape = ReverseTape { nodes: storage, used: 0, _brand: PhantomData };
    Ok(evaluate(&mut tape))
}

impl<'id> ReverseTape<'_, 'id> {
    fn check(&self, variable: Variable<'id>) -> Result<usize, ReverseTapeError> {
        (variable.index < self.used)
            .then_some(variable.index)
            .ok_or(ReverseTapeError::InvalidVariable)
    }

    fn record(
        &mut self,
        value: f64,
        edges: [Edge; 2],
        arity: u8,
    ) -> Result<Variable<'id>, ReverseTapeError> {
        if !value.is_finite() || edges[..usize::from(arity)].iter().any(|edge| !edge.partial.is_finite()) {
            return Err(ReverseTapeError::NonFinite);
        }
        let index = self.used;
        let available = self.nodes.len();
        let node = self.nodes.get_mut(index).ok_or(ReverseTapeError::Workspace(
            WorkspaceError::BufferTooSmall { required: index + 1, available },
        ))?;
        *node = ReverseNode { value, adjoint: 0.0, edges, arity };
        self.used += 1;
        Ok(Variable { index, _brand: PhantomData })
    }

    /// Record an independent input. Pass the returned variable to `gradient_into`
    /// to request its derivative.
    pub fn input(&mut self, value: f64) -> Result<Variable<'id>, ReverseTapeError> {
        self.record(value, [ReverseNode::NO_EDGE; 2], 0)
    }

    pub fn value(&self, variable: Variable<'id>) -> Result<f64, ReverseTapeError> {
        Ok(self.nodes[self.check(variable)?].value)
    }

    pub fn used_nodes(&self) -> usize {
        self.used
    }

    /// Record a unary primitive with its derivative evaluated at the input.
    pub fn unary(
        &mut self,
        input: Variable<'id>,
        value: f64,
        partial: f64,
    ) -> Result<Variable<'id>, ReverseTapeError> {
        let parent = self.check(input)?;
        self.record(value, [Edge { parent, partial }, ReverseNode::NO_EDGE], 1)
    }

    /// Record a binary primitive with the two local partial derivatives.
    pub fn binary(
        &mut self,
        a: Variable<'id>,
        b: Variable<'id>,
        value: f64,
        partials: [f64; 2],
    ) -> Result<Variable<'id>, ReverseTapeError> {
        let first = self.check(a)?;
        let second = self.check(b)?;
        self.record(value, [
            Edge { parent: first, partial: partials[0] },
            Edge { parent: second, partial: partials[1] },
        ], 2)
    }

    pub fn add(&mut self, a: Variable<'id>, b: Variable<'id>) -> Result<Variable<'id>, ReverseTapeError> {
        self.binary(a, b, self.value(a)? + self.value(b)?, [1.0, 1.0])
    }

    pub fn sub(&mut self, a: Variable<'id>, b: Variable<'id>) -> Result<Variable<'id>, ReverseTapeError> {
        self.binary(a, b, self.value(a)? - self.value(b)?, [1.0, -1.0])
    }

    pub fn mul(&mut self, a: Variable<'id>, b: Variable<'id>) -> Result<Variable<'id>, ReverseTapeError> {
        let left = self.value(a)?;
        let right = self.value(b)?;
        self.binary(a, b, left * right, [right, left])
    }

    pub fn div(&mut self, a: Variable<'id>, b: Variable<'id>) -> Result<Variable<'id>, ReverseTapeError> {
        let denominator = self.value(b)?;
        if denominator == 0.0 {
            return Err(ReverseTapeError::InvalidDomain);
        }
        let value = self.value(a)? / denominator;
        self.binary(a, b, value, [denominator.recip(), -value / denominator])
    }

    pub fn sin(&mut self, input: Variable<'id>) -> Result<Variable<'id>, ReverseTapeError> {
        let (sin, cos) = self.value(input)?.sin_cos();
        self.unary(input, sin, cos)
    }

    pub fn cos(&mut self, input: Variable<'id>) -> Result<Variable<'id>, ReverseTapeError> {
        let (sin, cos) = self.value(input)?.sin_cos();
        self.unary(input, cos, -sin)
    }

    pub fn exp(&mut self, input: Variable<'id>) -> Result<Variable<'id>, ReverseTapeError> {
        let value = self.value(input)?.exp();
        self.unary(input, value, value)
    }

    pub fn ln(&mut self, input: Variable<'id>) -> Result<Variable<'id>, ReverseTapeError> {
        let value = self.value(input)?;
        if value <= 0.0 {
            return Err(ReverseTapeError::InvalidDomain);
        }
        self.unary(input, value.ln(), value.recip())
    }

    pub fn sqrt(&mut self, input: Variable<'id>) -> Result<Variable<'id>, ReverseTapeError> {
        let value = self.value(input)?;
        if value <= 0.0 {
            return Err(ReverseTapeError::InvalidDomain);
        }
        let root = value.sqrt();
        self.unary(input, root, 0.5 / root)
    }

    pub fn gradient_into(
        &mut self,
        output: Variable<'id>,
        inputs: &[Variable<'id>],
        gradient: &mut [f64],
    ) -> Result<(), ReverseTapeError> {
        self.pullback_into(&[(output, 1.0)], inputs, gradient)
    }

    /// Calculate a vector–Jacobian product without allocating. Invalid shape,
    /// domain or arithmetic leaves the caller's gradient buffer unchanged.
    pub fn pullback_into(
        &mut self,
        seeds: &[(Variable<'id>, f64)],
        inputs: &[Variable<'id>],
        gradient: &mut [f64],
    ) -> Result<(), ReverseTapeError> {
        if gradient.len() != inputs.len() {
            return Err(ReverseTapeError::OutputLength { expected: inputs.len(), actual: gradient.len() });
        }
        for &input in inputs {
            self.check(input)?;
        }
        for &(output, seed) in seeds {
            self.check(output)?;
            if !seed.is_finite() {
                return Err(ReverseTapeError::NonFinite);
            }
        }
        for node in &mut self.nodes[..self.used] {
            node.adjoint = 0.0;
        }
        for &(output, seed) in seeds {
            self.accumulate(output.index, seed)?;
        }
        for index in (0..self.used).rev() {
            let node = self.nodes[index];
            if node.adjoint == 0.0 {
                continue;
            }
            for edge in &node.edges[..usize::from(node.arity)] {
                self.accumulate(edge.parent, node.adjoint * edge.partial)?;
            }
        }
        for (&input, result) in inputs.iter().zip(gradient) {
            *result = self.nodes[input.index].adjoint;
        }
        Ok(())
    }

    fn accumulate(&mut self, index: usize, update: f64) -> Result<(), ReverseTapeError> {
        let adjoint = self.nodes[index].adjoint + update;
        if !adjoint.is_finite() {
            return Err(ReverseTapeError::NonFinite);
        }
        self.nodes[index].adjoint = adjoint;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_gradient_matches_128_parameter_central_differences() {
        const N: usize = 128;
        let mut arena = [ReverseNode::EMPTY; N * 5 + 1];
        with_reverse_tape(&mut arena, |tape| {
            let zero = tape.input(0.0).unwrap();
            let mut parameters = [zero; N];
            let point: [f64; N] = core::array::from_fn(|i| 0.1 + i as f64 * 0.001);
            let mut objective = zero;
            for (i, parameter) in parameters.iter_mut().enumerate() {
                *parameter = tape.input(point[i]).unwrap();
                let square = tape.mul(*parameter, *parameter).unwrap();
                let term = tape.exp(square).unwrap();
                objective = tape.add(objective, term).unwrap();
            }
            let mut gradient = [0.0; N];
            tape.gradient_into(objective, &parameters, &mut gradient).unwrap();
            for (x, actual) in point.into_iter().zip(gradient) {
                let h = 1e-5;
                let plus = (x + h).powi(2).exp();
                let minus = (x - h).powi(2).exp();
                let expected = (plus - minus) / (2.0 * h);
                assert!(((expected - actual) / expected).abs() < 1e-7);
            }
            assert_eq!(tape.used_nodes(), 4 * N + 1);
        }).unwrap();
    }

    #[test]
    fn repeatable_pullback_and_atomic_failure_on_adjoint_overflow() {
        let mut arena = [ReverseNode::EMPTY; 4];
        with_reverse_tape(&mut arena, |tape| {
            let x = tape.input(3.0).unwrap();
            let square = tape.mul(x, x).unwrap();
            let large = tape.unary(square, 0.0, f64::MAX).unwrap();
            let mut gradient = [17.0];
            assert_eq!(
                tape.gradient_into(large, &[x], &mut gradient),
                Err(ReverseTapeError::NonFinite)
            );
            assert_eq!(gradient, [17.0]);
            tape.pullback_into(&[(square, 2.0), (x, -1.0)], &[x], &mut gradient).unwrap();
            assert_eq!(gradient, [11.0]);
            tape.gradient_into(square, &[x], &mut gradient).unwrap();
            assert_eq!(gradient, [6.0]);
        }).unwrap();
    }
}
