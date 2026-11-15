use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loss {
    /// Explicit claim of no observational loss, not a reversibility proof.
    NoneDeclared,
    Quantization,
    Resampling,
    PhaseDiscarded,
    ChannelReduction,
    EventAggregation,
    Clipping,
    Windowing,
    BandSelection,
    Registered(ArtifactLink),
}
/// Source/parameter/output identities and declared losses for an existing derivative.
/// This declaration neither executes nor qualifies an algorithm.
#[derive(Clone, Copy, Debug)]
pub struct Derivation<'a> {
    output: Original<'a>,
    inputs: &'a [Original<'a>],
    algorithm: ArtifactLink,
    parameters: ArtifactLink,
    losses: &'a [Loss],
}
impl<'a> Derivation<'a> {
    pub fn admit(
        output: Original<'a>,
        inputs: &'a [Original<'a>],
        algorithm: ArtifactLink,
        parameters: ArtifactLink,
        losses: &'a [Loss],
        max_links: usize,
    ) -> Result<Self, Error> {
        if inputs.is_empty() || losses.is_empty() {
            return Err(Error::LossDeclaration);
        }
        if inputs
            .len()
            .checked_add(losses.len())
            .ok_or(Error::Overflow)?
            > max_links
        {
            return Err(Error::Budget);
        }
        algorithm.validate()?;
        parameters.validate()?;
        if losses.len() > 1 && losses.contains(&Loss::NoneDeclared) {
            return Err(Error::LossDeclaration);
        }
        for loss in losses {
            if let Loss::Registered(r) = loss {
                r.validate()?;
            }
        }
        Ok(Self {
            output,
            inputs,
            algorithm,
            parameters,
            losses,
        })
    }
    pub fn output(self) -> Original<'a> {
        self.output
    }
    pub fn inputs(self) -> &'a [Original<'a>] {
        self.inputs
    }
    pub fn algorithm(self) -> ArtifactLink {
        self.algorithm
    }
    pub fn parameters(self) -> ArtifactLink {
        self.parameters
    }
    pub fn losses(self) -> &'a [Loss] {
        self.losses
    }
}
