use super::*;
use crate::solvers::SolversError;
fn model()->DrivenSpring {DrivenSpring{target:1.0,stiffness:25.0,damping:10.0,max_displacement:2.0,max_speed:10.0}}
#[test]
fn driven_response_matches_critical_damping_reference() {
    let mut state=SpringState{position:0.0,velocity:0.0};
    for _ in 0..20 {state=step_spring(state,model(),0.05).unwrap();}
    let expected=1.0-6.0*(-5.0_f64).exp();
    let velocity=25.0*(-5.0_f64).exp();
    assert!((state.position-expected).abs()<1e-7);
    assert!((state.velocity-velocity).abs()<1e-7);
}
#[test]
fn subintervals_preserve_response_and_zero_time_preserves_state() {
    let state=SpringState{position:0.0,velocity:0.0};
    assert_eq!(step_spring(state,model(),0.0).unwrap(),state);
    let one=step_spring(state,model(),0.05).unwrap();
    let a=step_spring(state,model(),0.025).unwrap();
    let two=step_spring(a,model(),0.025).unwrap();
    assert!((one.position-two.position).abs()<1e-12);
    assert!((one.velocity-two.velocity).abs()<1e-12);
}
#[test]
fn constrained_response_cannot_accumulate_outward_velocity() {
    let mut m=model();m.max_displacement=0.1;
    let state=step_spring(SpringState{position:4.0,velocity:9.0},m,0.01).unwrap();
    assert_eq!(state.position,1.1);assert_eq!(state.velocity,0.0);
}
#[test]
fn invalid_inputs_are_refused() {
    let state=SpringState{position:0.0,velocity:0.0};
    assert_eq!(step_spring(state,model(),0.051),Err(SolversError::InvalidParameters));
    let mut m=model();m.damping=-1.0;
    assert_eq!(step_spring(state,m,0.01),Err(SolversError::InvalidParameters));
    m=model();m.target=f64::NAN;
    assert_eq!(step_spring(state,m,0.01),Err(SolversError::InvalidParameters));
}
#[test]
fn thermal_response_matches_analytic_cooling_and_preserves_zero_time() {
    let initial=[140.0,112.0];let targets=ThermalTargets{temperatures:[85.0,75.0],rates:[0.028,0.04]};
    assert_eq!(step_thermal(initial,targets,0.0).unwrap(),initial);
    let mut state=initial;
    for _ in 0..200 {state=step_thermal(state,targets,0.05).unwrap();}
    for i in 0..2 {
        let expected=targets.temperatures[i]+(initial[i]-targets.temperatures[i])*(-targets.rates[i]*10.0).exp();
        assert!((state[i]-expected).abs()<1e-9);
    }
    let bad=ThermalTargets{rates:[-0.01,0.04],..targets};
    assert_eq!(step_thermal(initial,bad,0.01),Err(SolversError::InvalidParameters));
}
#[cfg(not(target_arch="wasm32"))]
#[test]
fn thermal_kernel_allocates_no_heap() {
    crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc("thermal_response",||{
        step_thermal([94.0,89.0],ThermalTargets{temperatures:[110.0,100.0],rates:[0.028,0.04]},0.05).unwrap();
    });
}
#[cfg(not(target_arch="wasm32"))]
#[test]
fn shared_solver_response_allocates_no_heap() {
    crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc("driven_spring",||{
        step_spring(SpringState{position:0.0,velocity:0.0},model(),0.05).unwrap();
    });
}
