//! Admitted task-program context. Scheduler execution never creates a Host or budget.
use super::*;

impl<'a, H: Host> Engine<'a, H> {
    pub(crate) fn charge_task_source(&mut self, bytes: u64, span: Span) -> Result<(), Diagnostic> {
        self.budget.charge(bytes, span)
    }

    /// A task program can request only grants already present on its admitted caller.
    pub(crate) fn eval_task_program(
        &mut self,
        program: &Program,
        env: &mut Env,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let grants = self.granted_refs();
        for request in &program.requires {
            if !grants.iter().any(|grant| *grant == request.id)
                && !(crate::catalog::is_known(&request.id)
                    && crate::catalog::granted_covers(&grants, &request.id))
            {
                return Err(Diagnostic::new(
                    DiagCode::E400,
                    span,
                    "task program requests a capability not granted by its caller",
                ));
            }
            // The pinned Engine grant list contains IDs, not adopted constraint
            // entailment. Never drop a parent's context/resource restrictions or
            // interpret authored child arguments as proof of attenuation.
            let constraint_error = || {
                Diagnostic::new(DiagCode::E400, span,
                "task capability constraints require owning host evaluation; this local profile admits only provably unconstrained declarations")
            };
            if !request.args.is_empty() {
                return Err(constraint_error());
            }
            let Some(parent) = self.program.as_ref() else {
                return Err(constraint_error());
            };
            let unconstrained_parent = parent.requires.iter().any(|grant| {
                grant.args.is_empty()
                    && (grant.id == request.id
                        || (crate::catalog::is_known(&request.id)
                            && crate::catalog::granted_covers(&[grant.id.as_str()], &request.id)))
            });
            if !unconstrained_parent {
                return Err(constraint_error());
            }
        }
        let previous_aliases = env.aliases.clone();
        if let Err(diag) = populate_import_aliases(env, &program.imports) {
            env.aliases = previous_aliases;
            return Err(diag);
        }
        let previous_grants = self.granted.clone();
        let previous_program = self.program.replace(std::sync::Arc::new(program.clone()));
        let result = self.eval_program(program, env);
        self.program = previous_program;
        self.granted = previous_grants;
        env.aliases = previous_aliases;
        result
    }
}
