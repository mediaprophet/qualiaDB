//! Bounded, Zero-Heap Reversible Command Stack (`command_stack.rs`)
//!
//! Implements Epic 9.3b of the QualiaDB Superior Game Engine architecture:
//! - Bounded undo/redo ring/stack ([`StudioCommandStack<CAP>`]) operating in **zero heap**.
//! - Supports [`StudioCommand::TransformNode`], [`StudioCommand::EditTensor10D`],
//!   [`StudioCommand::EditProcSynthesis`], and [`StudioCommand::EditOntogeny`].
//! - Supports command coalescing (`execute_coalesced`) so continuous 60 Hz gizmo drags or
//!   10D slider scrubs merge into a single atomic undo step.

use crate::render::gizmo_3d::Motor;
use crate::render::inspector_10d::{
    ProceduralSynthesisHeader, StudioNodeInspector10D, Tensor10D,
};

/// Reversible studio editing command over 10D scene nodes and `.10d` sections.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StudioCommand {
    /// PGA `Motor` + 3D scale manipulation (`MotorGizmo3D`).
    TransformNode {
        node_id: u32,
        old_motor: Motor,
        new_motor: Motor,
        old_scale: [f32; 3],
        new_scale: [f32; 3],
    },
    /// 10D manifold coordinate (`[q, v, w, x, y, z, t, α, μ, σ]`) edit.
    EditTensor10D {
        node_id: u32,
        old_tensor: Tensor10D,
        new_tensor: Tensor10D,
    },
    /// `.10d` `PROC1` (`ProceduralSynthesisHeader`) parameter edit.
    EditProcSynthesis {
        node_id: u32,
        old_header: ProceduralSynthesisHeader,
        new_header: ProceduralSynthesisHeader,
    },
    /// `.10d` `EONT1` ontogenetic stage and `[energy, damage, hydration, wear]` permill edit.
    EditOntogeny {
        node_id: u32,
        old_stage: u8,
        new_stage: u8,
        old_permill: [u16; 4],
        new_permill: [u16; 4],
    },
}

impl Default for StudioCommand {
    fn default() -> Self {
        Self::TransformNode {
            node_id: 0,
            old_motor: Motor::identity(),
            new_motor: Motor::identity(),
            old_scale: [1.0, 1.0, 1.0],
            new_scale: [1.0, 1.0, 1.0],
        }
    }
}

impl StudioCommand {
    /// Returns the `node_id` targeted by this command.
    #[inline]
    pub const fn node_id(&self) -> u32 {
        match *self {
            Self::TransformNode { node_id, .. }
            | Self::EditTensor10D { node_id, .. }
            | Self::EditProcSynthesis { node_id, .. }
            | Self::EditOntogeny { node_id, .. } => node_id,
        }
    }

    /// Returns the exact inverse command (swapping `old_*` and `new_*` states).
    #[inline]
    pub const fn inverted(self) -> Self {
        match self {
            Self::TransformNode {
                node_id,
                old_motor,
                new_motor,
                old_scale,
                new_scale,
            } => Self::TransformNode {
                node_id,
                old_motor: new_motor,
                new_motor: old_motor,
                old_scale: new_scale,
                new_scale: old_scale,
            },
            Self::EditTensor10D {
                node_id,
                old_tensor,
                new_tensor,
            } => Self::EditTensor10D {
                node_id,
                old_tensor: new_tensor,
                new_tensor: old_tensor,
            },
            Self::EditProcSynthesis {
                node_id,
                old_header,
                new_header,
            } => Self::EditProcSynthesis {
                node_id,
                old_header: new_header,
                new_header: old_header,
            },
            Self::EditOntogeny {
                node_id,
                old_stage,
                new_stage,
                old_permill,
                new_permill,
            } => Self::EditOntogeny {
                node_id,
                old_stage: new_stage,
                new_stage: old_stage,
                old_permill: new_permill,
                new_permill: old_permill,
            },
        }
    }

    /// Attempts to coalesce `next` into `self` if both commands target the same `node_id`
    /// and the same property variant (e.g. continuous gizmo drag).
    #[inline]
    pub fn try_coalesce(&mut self, next: &StudioCommand) -> bool {
        match (self, next) {
            (
                Self::TransformNode {
                    node_id: id_a,
                    new_motor: ref mut nm_a,
                    new_scale: ref mut ns_a,
                    ..
                },
                Self::TransformNode {
                    node_id: id_b,
                    new_motor: nm_b,
                    new_scale: ns_b,
                    ..
                },
            ) if *id_a == *id_b => {
                *nm_a = *nm_b;
                *ns_a = *ns_b;
                true
            }
            (
                Self::EditTensor10D {
                    node_id: id_a,
                    new_tensor: ref mut nt_a,
                    ..
                },
                Self::EditTensor10D {
                    node_id: id_b,
                    new_tensor: nt_b,
                    ..
                },
            ) if *id_a == *id_b => {
                *nt_a = *nt_b;
                true
            }
            (
                Self::EditProcSynthesis {
                    node_id: id_a,
                    new_header: ref mut nh_a,
                    ..
                },
                Self::EditProcSynthesis {
                    node_id: id_b,
                    new_header: nh_b,
                    ..
                },
            ) if *id_a == *id_b => {
                *nh_a = *nh_b;
                true
            }
            (
                Self::EditOntogeny {
                    node_id: id_a,
                    new_stage: ref mut ns_a,
                    new_permill: ref mut np_a,
                    ..
                },
                Self::EditOntogeny {
                    node_id: id_b,
                    new_stage: ns_b,
                    new_permill: np_b,
                    ..
                },
            ) if *id_a == *id_b => {
                *ns_a = *ns_b;
                *np_a = *np_b;
                true
            }
            _ => false,
        }
    }

    /// Applies the forward (`new_*`) state of this command to a [`StudioNodeInspector10D`].
    #[inline]
    pub fn apply_to_inspector(&self, inspector: &mut StudioNodeInspector10D) {
        if inspector.node_id != self.node_id() {
            return;
        }
        match *self {
            Self::TransformNode {
                new_motor,
                new_scale,
                ..
            } => {
                inspector.local_motor = new_motor;
                inspector.local_scale = new_scale;
            }
            Self::EditTensor10D { new_tensor, .. } => {
                inspector.tensor_10d = new_tensor;
            }
            Self::EditProcSynthesis { new_header, .. } => {
                inspector.proc_header = new_header;
            }
            Self::EditOntogeny {
                new_stage,
                new_permill,
                ..
            } => {
                inspector.eont_header.stage = new_stage;
                inspector.eont_header.energy_permill = new_permill[0];
                inspector.eont_header.damage_permill = new_permill[1];
                inspector.eont_header.hydration_permill = new_permill[2];
                inspector.eont_header.wear_permill = new_permill[3];
            }
        }
    }
}

/// Bounded zero-heap reversible command stack (`CAP = 128` by default).
#[derive(Debug, Clone)]
pub struct StudioCommandStack<const CAP: usize = 128> {
    entries: [StudioCommand; CAP],
    /// Current undo cursor (`0..=len`).
    cursor: usize,
    /// Total valid entries (`>= cursor`; entries in `cursor..len` are available for `redo`).
    len: usize,
}

impl<const CAP: usize> Default for StudioCommandStack<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> StudioCommandStack<CAP> {
    /// Creates an empty command stack.
    pub fn new() -> Self {
        Self {
            entries: [StudioCommand::default(); CAP],
            cursor: 0,
            len: 0,
        }
    }

    /// Clears all undo/redo history.
    #[inline]
    pub fn clear(&mut self) {
        self.cursor = 0;
        self.len = 0;
    }

    /// Number of commands available to undo.
    #[inline]
    pub const fn undo_count(&self) -> usize {
        self.cursor
    }

    /// Number of commands available to redo.
    #[inline]
    pub const fn redo_count(&self) -> usize {
        self.len.saturating_sub(self.cursor)
    }

    #[inline]
    pub const fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    #[inline]
    pub const fn can_redo(&self) -> bool {
        self.cursor < self.len
    }

    /// Pushes a new command onto the stack (truncating any redo tail). If the stack is full,
    /// shifts the oldest entry out so capacity remains strictly bounded in zero heap.
    pub fn execute(&mut self, cmd: StudioCommand, inspector: &mut StudioNodeInspector10D) {
        cmd.apply_to_inspector(inspector);
        if CAP == 0 {
            return;
        }
        if self.cursor >= CAP {
            // Shift left by 1 to drop the oldest entry
            for i in 1..CAP {
                self.entries[i - 1] = self.entries[i];
            }
            self.cursor = CAP - 1;
        }
        self.entries[self.cursor] = cmd;
        self.cursor += 1;
        self.len = self.cursor;
    }

    /// Executes `cmd`, coalescing it with the top undo entry if `coalesce == true` and both
    /// target the same node & variant (e.g., during a continuous gizmo drag).
    pub fn execute_coalesced(
        &mut self,
        cmd: StudioCommand,
        coalesce: bool,
        inspector: &mut StudioNodeInspector10D,
    ) {
        cmd.apply_to_inspector(inspector);
        if coalesce && self.cursor > 0 && self.cursor == self.len {
            if self.entries[self.cursor - 1].try_coalesce(&cmd) {
                return;
            }
        }
        self.execute(cmd, inspector);
    }

    /// Undoes the most recent command, applying its inverse to `inspector` and returning the
    /// inverted command.
    pub fn undo(&mut self, inspector: &mut StudioNodeInspector10D) -> Option<StudioCommand> {
        if self.cursor == 0 {
            return None;
        }
        self.cursor -= 1;
        let inv = self.entries[self.cursor].inverted();
        inv.apply_to_inspector(inspector);
        Some(inv)
    }

    /// Redoes the next command in the redo tail, applying it to `inspector`.
    pub fn redo(&mut self, inspector: &mut StudioNodeInspector10D) -> Option<StudioCommand> {
        if self.cursor >= self.len {
            return None;
        }
        let cmd = self.entries[self.cursor];
        self.cursor += 1;
        cmd.apply_to_inspector(inspector);
        Some(cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_stack_executes_coalesces_undoes_and_redoes() {
        let mut stack: StudioCommandStack<16> = StudioCommandStack::new();
        let mut inspector = StudioNodeInspector10D::new(7);

        let m0 = Motor::identity();
        let m1 = Motor::from_translation([1.0, 0.0, 0.0]);
        let m2 = Motor::from_translation([3.5, 0.0, 0.0]);

        // Initial drag step
        stack.execute_coalesced(
            StudioCommand::TransformNode {
                node_id: 7,
                old_motor: m0,
                new_motor: m1,
                old_scale: [1.0; 3],
                new_scale: [1.0; 3],
            },
            true,
            &mut inspector,
        );
        // Second drag step coalesced into the first
        stack.execute_coalesced(
            StudioCommand::TransformNode {
                node_id: 7,
                old_motor: m1,
                new_motor: m2,
                old_scale: [1.0; 3],
                new_scale: [2.0, 2.0, 2.0],
            },
            true,
            &mut inspector,
        );

        assert_eq!(stack.undo_count(), 1, "Continuous drag should coalesce into 1 undo step");
        assert_eq!(inspector.local_motor, m2);
        assert_eq!(inspector.local_scale, [2.0, 2.0, 2.0]);

        // Undo should restore m0 and [1.0; 3] in a single step
        stack.undo(&mut inspector).expect("Expected undo");
        assert_eq!(inspector.local_motor, m0);
        assert_eq!(inspector.local_scale, [1.0, 1.0, 1.0]);
        assert!(stack.can_redo());

        // Redo should reapply m2 and [2.0; 3]
        stack.redo(&mut inspector).expect("Expected redo");
        assert_eq!(inspector.local_motor, m2);
        assert_eq!(inspector.local_scale, [2.0, 2.0, 2.0]);
    }
}
