//! Regression — simple and multiple OLS, verification diagnostics, influence,
//! selection, design helpers, and discrete-Y wrappers.
//!
//! Multiple OLS with residuals/fitted and the Ch.4 verification battery live here
//! so Civics receipts can fail closed on hetero / autocorr / non-normal / VIF /
//! RESET without auto-deleting outliers (flag only).

mod design;
mod diagnostics;
mod discrete;
mod influence;
mod multiple;
mod outliers;
mod selection;
mod simple;
mod spurious;
mod verify;

pub use design::{design_dummies, seasonal_dummies, transform_series, DummyDesign, TransformKind};
pub use diagnostics::{
    arch_lm, box_pearce_ac, breusch_pagan, chow_test, durbin_watson, jarque_bera, ramsey_reset,
    residual_runs, residual_symmetry, vif_columns, ArchLmResult, BoxPearceResult,
    BreuschPaganResult, ChowResult, DurbinWatsonResult, JarqueBeraResult, RamseyResetResult,
    RunsResult, SymmetryResult, VifResult,
};
pub use discrete::{fit_lda_2class, fit_logit, LdaFitSummary, LogitFitSummary};
pub use influence::{influence_measures, InfluenceMeasures, InfluenceRow};
pub use multiple::{multiple_ols, MultipleOls};
pub use outliers::{
    mahalanobis_outliers, univariate_outlier_screen, MahalanobisOutliers, UnivariateOutlierScreen,
};
pub use selection::{stepwise_backward, StepwiseResult, StepwiseStep};
pub use simple::{simple_linear_regression, LinearRegression};
pub use spurious::{spurious_regression_guard, SpuriousGuard};
pub use verify::{verify_regression_model, VerificationFlag, VerificationReport, VerifyOptions};
