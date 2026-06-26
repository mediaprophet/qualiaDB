# Statistical Learning (ISL) — Implementation Plan

**Goal (Timothy, 2026-06-27):** implement the *An Introduction to Statistical Learning*
(ISL, 2nd ed.) method surface in the rust-core — **completely**, **without duplicating**
existing math, **properly categorised** (CLAUDE.md §13), and **dispatch-ready** against
the compute bridge (`platform/compute_bridge`).

This is *statistical **learning*** (predictive modelling). It builds **on** the classical
statistics foundation already landed this session:
- `solvers/statistics/distributions` — Normal / t / χ² / F (pdf/cdf/quantile), erf, incomplete γ/β.
- `solvers/statistics/{descriptive,correlation,regression,hypothesis}` — moments, covariance,
  Pearson/Spearman, simple OLS, real p-value tests.
- `solvers/linear_algebra` — `gemm`, `qr` (incl. `qr_solve_least_squares`), `cholesky`,
  `eigen`, `svd`, `lu`, `vector`.
- `platform/compute_bridge` — `KernelClass` + `ComputePolicy::select` (per-class GPU/CPU).

## No-duplication rule
Every estimator **reuses** the above. It must NOT re-implement mean/variance/covariance
(use `descriptive`), a CDF/p-value (use `distributions`), a matrix solve / eigen / SVD
(use `linear_algebra`), or a device choice (use `ComputePolicy`). If a primitive is
missing, it is **added to the foundation library it belongs in**, then called — never
copied into an estimator.

## Category (new): `solvers/learning/`
Statistical-learning estimators live in one new solver category, each method-family a
modular sub-library (one concern per file, own `#[cfg(test)]`, wired via `mod.rs`):

```
solvers/learning/
  preprocessing/   — standardize/scale, design matrix, train/test split            [reuse: descriptive]
  metrics/         — MSE, R², accuracy, confusion matrix, ROC/AUC, log-loss         [reuse: statistics]
  regression/      — multiple OLS, ridge, lasso, PCR, PLS                           [reuse: linear_algebra, distributions]
  glm/             — logistic (IRLS), multinomial, Poisson                          [reuse: linear_algebra, distributions]
  classification/  — LDA, QDA, naive Bayes, KNN                                     [reuse: linear_algebra, distributions, descriptive]
  resampling/      — k-fold CV, LOOCV, bootstrap (generic over an Estimator trait)  [reuse: —]
  dimensionality/  — PCA, PCR, PLS basis                                            [reuse: linear_algebra (svd/eigen)]
  clustering/      — k-means, hierarchical (agglomerative)                          [reuse: descriptive, distances]
  trees/           — CART (regression+classification), bagging, random forest,
                     gradient boosting                                              [reuse: —]
  splines/         — basis (B-spline/natural), regression & smoothing splines, GAM  [reuse: linear_algebra]
  survival/        — Kaplan-Meier, Cox proportional hazards                         [reuse: distributions]
  multiple_testing/— Bonferroni, Holm, Benjamini-Hochberg (FDR), q-values           [reuse: distributions]
```

**Deep learning (ISL ch 10) — deliberately NOT re-implemented here.** The engine already
has a native neural-inference stack (gguf/wgpu LLM lane). Duplicating CNN/RNN training in
`solvers/learning` would violate the no-duplication rule. Flagged for Timothy; revisit only
if a classical-NN training primitive is explicitly wanted separate from the LLM stack.

## Dispatch-readiness (§13)
Each estimator classifies its hot kernel into a `KernelClass` and keeps an always-present
CPU path; where a GPU win is plausible it routes through `ComputePolicy::select`:
- matrix solves / GEMM (OLS, ridge, LDA, PCA) → `DenseLinear`
- pairwise distances (KNN, k-means, hierarchical) → `AllPairs`
- per-sample reductions (metrics, CV folds) → `Reduction`
- bootstrap / permutation → `Divergent`
Scalar fit-loops (IRLS, tree splits) are CPU — flagged where GPU does not help.

## ISL chapter → deliverable map + status
| ISL ch | Methods | Module | Status |
|---|---|---|---|
| 2 | MSE, bias-variance, error rate | metrics | ✅ done |
| 3 | multiple linear regression (+inference) | regression/linear | ✅ done |
| 3 | KNN regression | classification/knn | ☐ |
| 4 | logistic, Poisson (GLM, IRLS) | glm | ✅ done (multinomial pending) |
| 4 | LDA, QDA, naive Bayes | classification | ☐ |
| 4 | ROC/AUC, confusion matrix | metrics | ☐ |
| 5 | validation set, LOOCV, k-fold CV, bootstrap | resampling | ✅ done |
| 6 | best-subset / stepwise, ridge, lasso, PCR, PLS | regression + dimensionality | ☐ |
| 7 | poly, step, regression/smoothing splines, GAM | splines | ☐ |
| 8 | CART, bagging, random forest, boosting | trees | ☐ |
| 9 | SVM (margin, kernel, multiclass) | classification/svm | ☐ |
| 10 | deep learning | — (defer to LLM stack) | ⊘ |
| 11 | Kaplan-Meier, Cox PH | survival | ☐ |
| 12 | PCA, K-means, hierarchical clustering | dimensionality + clustering | ☐ |
| 13 | Bonferroni, Holm, BH-FDR | multiple_testing | ✅ done |

## Build order (highest leverage / most-reused first)
1. **metrics** + **preprocessing** (everything is evaluated/scaled through these)
2. **regression/linear** (multiple OLS — completes ch 3, underpins ridge/PCR/GLM)
3. **glm/logistic** (+Poisson) — ch 4 core
4. **resampling** (CV + bootstrap) — ch 5, generic harness reused by all
5. **regression/ridge + lasso** — ch 6
6. **dimensionality/pca** + **regression/pcr,pls** — ch 6/12
7. **clustering/kmeans + hierarchical** — ch 12
8. **multiple_testing** — ch 13 (small, high value)
9. **classification/lda,qda,naive_bayes,knn** — ch 4
10. **trees** (CART → RF → boosting) — ch 8
11. **splines/GAM** — ch 7
12. **classification/svm** — ch 9
13. **survival** (KM, Cox) — ch 11

Each step: a real, tested implementation validated against a known result; a commit; an
update to this table; the full crate stays green. Progress is logged here and in
`coordination/NOTICES.md`.

## Progress log
- 2026-06-27: plan written. Foundation (distributions/descriptive/correlation/regression/
  linear_algebra/compute_bridge) in place. Starting build order step 1.
