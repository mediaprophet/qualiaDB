//! Human-facing Tool Chest copy and hover tooltips.
//!
//! Machine ids and live capability strings stay on `data-*` for agents.
//! Visible labels never say capability.invoke, Family.method, or ALL_BOUND.

use super::tool_proficiency::Proficiency;
use crate::tool_chest::core::tool::ToolKind;
use web_sys::Element;

pub struct Presentation {
    pub label: String,
    pub tooltip: String,
    pub min_proficiency: Proficiency,
}

pub fn presentation(id: &str, fallback_label: &str, fallback_tooltip: &str) -> Presentation {
    if let Some(spec) = super::spec_tools::lookup(id) {
        return Presentation {
            label: spec.label.into(),
            tooltip: spec.tooltip.into(),
            min_proficiency: spec.proficiency,
        };
    }
    if let Some(copy) = named(id) {
        return copy;
    }
    Presentation {
        label: if fallback_label.is_empty() {
            "This tool".into()
        } else {
            fallback_label.into()
        },
        tooltip: if fallback_tooltip.is_empty() {
            "A tool on this work surface.".into()
        } else {
            fallback_tooltip.into()
        },
        min_proficiency: Proficiency::Novice,
    }
}

pub fn kind_badge(kind: ToolKind) -> &'static str {
    match (super::tool_proficiency::current(), kind) {
        (Proficiency::Expert, ToolKind::PlaceContainer) => "place",
        (Proficiency::Expert, ToolKind::RunAction) => "run",
        (Proficiency::Expert, ToolKind::Query) => "look-up",
        (Proficiency::Expert, ToolKind::Navigate) => "go",
        (Proficiency::Expert, ToolKind::Toggle) => "toggle",
        (_, ToolKind::PlaceContainer) => "Add",
        (_, ToolKind::RunAction) => "Use",
        (_, ToolKind::Query) => "Look up",
        (_, ToolKind::Navigate) => "Go",
        (_, ToolKind::Toggle) => "Switch",
    }
}

pub fn decorate(
    button: &Element,
    id: &str,
    fallback_label: &str,
    fallback_tooltip: &str,
    capability: Option<&str>,
    gated_reason: Option<&str>,
) -> Presentation {
    let copy = presentation(id, fallback_label, fallback_tooltip);
    let mut tooltip = copy.tooltip.clone();
    if super::tool_proficiency::current() == Proficiency::Expert {
        if let Some(scope) = capability {
            if !scope.is_empty() {
                tooltip.push_str(" · ");
                tooltip.push_str(scope);
            }
        }
    }
    if let Some(reason) = gated_reason {
        tooltip.push_str(" — ");
        tooltip.push_str(reason);
    }
    let _ = button.set_attribute("data-tool-id", id);
    let _ = button.set_attribute("data-tooltip", &tooltip);
    let _ = button.set_attribute("title", &tooltip);
    let _ = button.set_attribute("aria-label", &copy.label);
    let _ = button.set_attribute("aria-description", &tooltip);
    let _ = button.set_attribute("data-min-proficiency", copy.min_proficiency.as_token());
    let _ = button.set_attribute("data-audience", "human agent");
    if let Some(scope) = capability {
        let _ = button.set_attribute("data-capability", scope);
    }
    if !super::tool_proficiency::current().shows(copy.min_proficiency) {
        let _ = button.set_attribute("hidden", "");
        let _ = button.set_attribute("data-proficiency-hidden", "1");
    } else {
        let _ = button.remove_attribute("hidden");
        let _ = button.remove_attribute("data-proficiency-hidden");
    }
    let tip = button
        .owner_document()
        .and_then(|document| document.create_element("span").ok());
    if let Some(tip) = tip {
        tip.set_class_name("tool-tip");
        tip.set_attribute("role", "tooltip").ok();
        tip.set_text_content(Some(&tooltip));
        let _ = button.append_child(&tip);
    }
    copy
}

fn named(id: &str) -> Option<Presentation> {
    let (label, tooltip, min) = match id {
        "office:place_doc" => (
            "Writing page",
            "Put a blank writing page on the work surface.",
            Proficiency::Novice,
        ),
        "office:place_ontology" => (
            "Meaning map",
            "Put a page for browsing meanings and relationships.",
            Proficiency::Novice,
        ),
        "office:place_slide" => (
            "Slide",
            "Put a presentation slide on the work surface.",
            Proficiency::Novice,
        ),
        "office:typography_bold" => (
            "Bold",
            "Make the selected writing thicker.",
            Proficiency::Novice,
        ),
        "office:typography_italic" => {
            ("Italic", "Slope the selected writing.", Proficiency::Novice)
        }
        "office:typography_code" => (
            "Code look",
            "Show the selected writing in a fixed-width type.",
            Proficiency::Intermediate,
        ),
        "office:paragraph_heading" => (
            "Heading",
            "Turn the selected writing into a heading.",
            Proficiency::Novice,
        ),
        "office:paragraph_align_left" => (
            "Align left",
            "Line the selected writing up on the left.",
            Proficiency::Novice,
        ),
        "office:paragraph_align_center" => (
            "Align centre",
            "Centre the selected writing.",
            Proficiency::Novice,
        ),
        "graph:sparql_query" => (
            "Search records",
            "Look through your notes and records for a match.",
            Proficiency::Intermediate,
        ),
        "n3:evaluate" => (
            "Apply written rules",
            "Run the if-then rules written on this page.",
            Proficiency::Intermediate,
        ),
        "shacl:validate" => (
            "Check the template",
            "See whether this page matches the expected shape.",
            Proficiency::Intermediate,
        ),
        "epistemic:tag_objective" => (
            "Mark as shared fact",
            "Say this note is about something anyone could check.",
            Proficiency::Novice,
        ),
        "epistemic:tag_subjective" => (
            "Mark as a point of view",
            "Say this note is from one person's standpoint.",
            Proficiency::Novice,
        ),
        "epistemic:tag_intersubjective" => (
            "Mark as agreed together",
            "Say this note is something a group holds in common.",
            Proficiency::Novice,
        ),
        "epistemic:tag_normative" => (
            "Mark as a should",
            "Say this note is about what ought to happen.",
            Proficiency::Intermediate,
        ),
        "image:place_media" => (
            "Picture window",
            "Put a place for pictures and drawings.",
            Proficiency::Novice,
        ),
        "image:marker" => (
            "Pin a mark",
            "Leave a mark on the selected page or map.",
            Proficiency::Novice,
        ),
        "image:heatmap" => (
            "Colour by numbers",
            "Tint the page by the numbers written on it.",
            Proficiency::Intermediate,
        ),
        "image:brush_stroke" => (
            "Outline",
            "Draw a visible edge around the selected page.",
            Proficiency::Novice,
        ),
        "image:brush_clear" => (
            "Clear outline",
            "Remove the edge from the selected page.",
            Proficiency::Novice,
        ),
        "image:fill_warm" => (
            "Warm wash",
            "Tint the selected page with a warm colour.",
            Proficiency::Novice,
        ),
        "image:fill_cool" => (
            "Cool wash",
            "Tint the selected page with a cool colour.",
            Proficiency::Novice,
        ),
        "sheet:place_sheet" => (
            "Table",
            "Put a numbers table on the work surface.",
            Proficiency::Novice,
        ),
        "sheet:import" => (
            "Bring in a table",
            "Load a comma-separated file into this table, starting at the first cell.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_mean" => (
            "Average",
            "Find the average of the numbers on this table.",
            Proficiency::Novice,
        ),
        "sheet:stats_median" => (
            "Middle value",
            "Find the middle value of the numbers on this table.",
            Proficiency::Novice,
        ),
        "sheet:stats_variance" => (
            "Spread",
            "Measure how spread out the numbers on this table are.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_std_dev" => (
            "Typical spread",
            "Find the typical distance of numbers from their average.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_min" => (
            "Smallest",
            "Find the smallest number on this table.",
            Proficiency::Novice,
        ),
        "sheet:stats_max" => (
            "Largest",
            "Find the largest number on this table.",
            Proficiency::Novice,
        ),
        "sheet:stats_sum" => (
            "Total",
            "Add up all the numbers on this table.",
            Proficiency::Novice,
        ),
        "sheet:stats_skewness" => (
            "Skew",
            "See whether the numbers lean left or right of their average.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_kurtosis" => (
            "Tails",
            "Measure how heavy the extreme tails of these numbers are.",
            Proficiency::Expert,
        ),
        "sheet:stats_quantile" => (
            "Percentile",
            "Find a chosen percentile of the numbers (default 95th).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_iqr" => (
            "Middle spread",
            "Measure the spread between the 25th and 75th percentiles.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_mode" => (
            "Most common",
            "Find the number that appears most often on this table.",
            Proficiency::Novice,
        ),
        "sheet:stats_trimmed_mean" => (
            "Trimmed average",
            "Average after dropping a fraction from each end.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_mad" => (
            "Robust spread",
            "Measure typical distance from the middle value (MAD).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_pearson" => (
            "Correlation",
            "Correlate the first half of numbers with the second half.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_covariance" => (
            "Covariance",
            "Measure how the two halves of the numbers move together.",
            Proficiency::Expert,
        ),
        "sheet:stats_z_score_outliers" => (
            "Outliers",
            "Flag numbers that sit far from the average by z-score.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_argmax" => (
            "Peak index",
            "Find which position holds the largest number.",
            Proficiency::Novice,
        ),
        "sheet:stats_binomial_pmf" => (
            "Binomial chance",
            "Chance of exactly k successes in n trials (optional data-k/n/p).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_binomial_cdf" => (
            "Binomial up to k",
            "Chance of at most k successes in n trials (optional data-k/n/p).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_beta_pdf" => (
            "Beta density",
            "Beta distribution density at the first number (optional data-alpha/beta).",
            Proficiency::Expert,
        ),
        "sheet:stats_chi_squared_pdf" => (
            "Chi-squared density",
            "Chi-squared density at the first number (optional data-k).",
            Proficiency::Expert,
        ),
        "sheet:stats_chi_squared_cdf" => (
            "Chi-squared CDF",
            "Chi-squared cumulative probability at the first number (optional data-k).",
            Proficiency::Expert,
        ),
        "sheet:stats_chi_squared_quantile" => (
            "Chi-squared percentile",
            "Chi-squared critical value for a probability (optional data-p/data-k).",
            Proficiency::Expert,
        ),
        "sheet:stats_autocorrelation" => (
            "Lag correlation",
            "How strongly this series correlates with itself at a lag.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_bootstrap_means" => (
            "Bootstrap means",
            "Resample the numbers to sketch the sampling distribution of the mean.",
            Proficiency::Expert,
        ),
        "sheet:stats_normal_pdf" => (
            "Normal density",
            "Normal distribution density at the first number (optional data-mu/sigma).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_normal_cdf" => (
            "Normal CDF",
            "Normal cumulative probability at the first number (optional data-mu/sigma).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_normal_quantile" => (
            "Normal percentile",
            "Normal critical value for a probability (optional data-p/mu/sigma).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_standard_normal_cdf" => (
            "Standard normal CDF",
            "Φ(z) for a z-score from the sheet or data-z.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_poisson_pmf" => (
            "Poisson chance",
            "Chance of exactly k events (optional data-k/data-lambda).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_poisson_cdf" => (
            "Poisson up to k",
            "Chance of at most k events (optional data-k/data-lambda).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_exponential_pdf" => (
            "Exponential density",
            "Exponential density at the first number (optional data-rate).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_exponential_cdf" => (
            "Exponential CDF",
            "Exponential cumulative probability at the first number (optional data-rate).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_spearman" => (
            "Spearman",
            "Rank correlation between the first and second halves of sheet numbers.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_kendall" => (
            "Kendall",
            "Kendall tau between the first and second halves of sheet numbers.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_winsorized_mean" => (
            "Winsorized mean",
            "Mean after clamping extremes (optional data-proportion, default 0.1).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_erf" => (
            "Error function",
            "erf(x) from the first sheet number or data-x.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_erfc" => (
            "Complementary erf",
            "erfc(x) from the first sheet number or data-x.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_uniform_pdf" => (
            "Uniform density",
            "Uniform density at the first number (optional data-a/data-b).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_laplace_pdf" => (
            "Laplace density",
            "Laplace density at the first number (optional data-mu/data-b).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_standard_pdf" => (
            "Standard normal density",
            "φ(z) for a z-score from the sheet or data-z.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_uniform_cdf" => (
            "Uniform CDF",
            "Uniform cumulative probability at the first number (optional data-a/data-b).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_laplace_cdf" => (
            "Laplace CDF",
            "Laplace cumulative probability at the first number (optional data-mu/data-b).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_lognormal_pdf" => (
            "Lognormal density",
            "Lognormal density at the first number (optional data-mu/data-sigma).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_lognormal_cdf" => (
            "Lognormal CDF",
            "Lognormal cumulative probability at the first number (optional data-mu/data-sigma).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_standard_quantile" => (
            "Standard normal quantile",
            "Φ⁻¹(p) from data-p or the first sheet number (default 0.975).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_ln_gamma" => (
            "Log-gamma",
            "ln Γ(x) from the first sheet number or data-x.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_gamma_fn" => (
            "Gamma",
            "Γ(x) from the first sheet number or data-x.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_weibull_pdf" => (
            "Weibull density",
            "Weibull density at the first number (optional data-shape/data-scale).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_gamma_pdf" => (
            "Gamma density",
            "Gamma density at the first number (optional data-shape/data-scale).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_two_sided_p" => (
            "Two-sided p",
            "Two-sided normal p-value from the first number or data-z.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_chi_squared_upper_p" => (
            "Chi-squared upper p",
            "Chi-squared upper-tail p at the first number (optional data-k).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_students_t_pdf" => (
            "Student's t density",
            "Student's t density at the first number (optional data-nu).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_fisher_f_pdf" => (
            "Fisher F density",
            "Fisher F density at the first number (optional data-d1/data-d2).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_gammp" => (
            "Incomplete gamma P",
            "Regularized lower incomplete gamma P(a,x) from sheet numbers or data-a/data-x.",
            Proficiency::Expert,
        ),
        "sheet:stats_gammq" => (
            "Incomplete gamma Q",
            "Regularized upper incomplete gamma Q(a,x) from sheet numbers or data-a/data-x.",
            Proficiency::Expert,
        ),
        "sheet:stats_betai" => (
            "Incomplete beta",
            "Regularized incomplete beta I_x(a,b) from sheet numbers or data-a/data-b/data-x.",
            Proficiency::Expert,
        ),
        "sheet:stats_students_t_cdf" => (
            "Student's t CDF",
            "Student's t cumulative probability at the first number (optional data-nu).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_students_t_two_sided_p" => (
            "Student's t two-sided p",
            "Two-sided Student's t p-value at the first number (optional data-nu).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_students_t_upper_p" => (
            "Student's t upper p",
            "Student's t upper-tail p at the first number (optional data-nu).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_students_t_quantile" => (
            "Student's t quantile",
            "Student's t quantile from data-p or the first sheet number (optional data-nu).",
            Proficiency::Expert,
        ),
        "sheet:stats_fisher_f_cdf" => (
            "Fisher F CDF",
            "Fisher F cumulative probability at the first number (optional data-d1/data-d2).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_fisher_f_upper_p" => (
            "Fisher F upper p",
            "Fisher F upper-tail p at the first number (optional data-d1/data-d2).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_fisher_f_quantile" => (
            "Fisher F quantile",
            "Fisher F quantile from data-p or the first sheet number (optional data-d1/data-d2).",
            Proficiency::Expert,
        ),
        "sheet:stats_tukey_fences" => (
            "Tukey fences",
            "Tukey fence bounds on sheet numbers (optional data-k, default 1.5).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_empirical_cdf" => (
            "Empirical CDF",
            "Empirical CDF at data-x or the last sheet number over the remaining samples.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_entropy" => (
            "Entropy",
            "Shannon entropy (bits) of sheet numbers as a probability mass.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_kl_divergence" => (
            "KL divergence",
            "KL(p‖q) between the first and second halves of sheet numbers.",
            Proficiency::Expert,
        ),
        "sheet:stats_cross_entropy" => (
            "Cross entropy",
            "Cross-entropy H(p,q) between series halves on the selected sheet.",
            Proficiency::Expert,
        ),
        "sheet:stats_entropy_from_counts" => (
            "Entropy from counts",
            "Shannon entropy from non-negative floored sheet counts.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_moving_average" => (
            "Moving average",
            "Simple moving average of sheet numbers (optional data-window, default 3).",
            Proficiency::Novice,
        ),
        "sheet:stats_modified_z_score_outliers" => (
            "Modified z-score outliers",
            "Modified z-score outlier count (optional data-threshold, default 3.5).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_iqr_outliers" => (
            "IQR outliers",
            "IQR/Tukey outlier count on sheet numbers (optional data-k, default 1.5).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_exponential_smoothing" => (
            "Exponential smoothing",
            "Brown exponential smoothing of sheet numbers (optional data-alpha, default 0.3).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_adf_proxy" => (
            "ADF proxy",
            "Augmented Dickey–Fuller stationarity proxy on the selected sheet.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_histogram" => (
            "Histogram",
            "Equal-width histogram of sheet numbers (optional data-bins, default 10).",
            Proficiency::Novice,
        ),
        "sheet:stats_ks_1sample" => (
            "KS one-sample",
            "Kolmogorov–Smirnov test of sheet numbers against Uniform(0,1).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_grubbs_test" => (
            "Grubbs test",
            "Grubbs single-outlier test (optional data-alpha, default 0.05).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_one_sample_t" => (
            "One-sample t",
            "One-sample t-test of the sheet mean (optional data-mu, default 0).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_two_sample_t" => (
            "Two-sample t",
            "Two-sample t-test between series halves (optional data-equal-var).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_paired_t" => (
            "Paired t",
            "Paired t-test between the first and second halves of sheet numbers.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_linear_regression" => (
            "Linear regression",
            "Simple OLS of series halves as x and y on the selected sheet.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_chi_square_gof" => (
            "Chi-square GOF",
            "Chi-square goodness-of-fit using observed then expected series halves.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_chi_square_independence" => (
            "Chi-square independence",
            "Chi-square independence on a contingency table (optional data-cols).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_correlation_p_value" => (
            "Correlation p-value",
            "P-value for a correlation (Pearson of halves, or data-r and data-n).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_friedman" => (
            "Friedman test",
            "Friedman test across blocks (optional data-treatments, default 3).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_ljung_box" => (
            "Ljung–Box",
            "Ljung–Box Q statistic from sheet autocorrelations (optional data-h).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_mann_whitney_u" => (
            "Mann–Whitney U",
            "Mann–Whitney U test between the first and second halves of sheet numbers.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_mcnemar" => (
            "McNemar",
            "McNemar test from discordant counts (first two numbers or data-b/data-c).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_mutual_information" => (
            "Mutual information",
            "Mutual information of floored discrete labels from series halves.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_one_way_anova" => (
            "One-way ANOVA",
            "One-way ANOVA across equal groups (optional data-groups, default 2).",
            Proficiency::Intermediate,
        ),
        "sheet:stats_mahalanobis_sq" => (
            "Mahalanobis squared",
            "Squared Mahalanobis distance (optional data-dim; x, mean, inv_cov or I).",
            Proficiency::Expert,
        ),
        "sheet:stats_mvn_log_pdf" => (
            "MVN log density",
            "Multivariate normal log-density (optional data-dim; x, mean, cov or I).",
            Proficiency::Expert,
        ),
        "sheet:stats_mvn_pdf" => (
            "MVN density",
            "Multivariate normal density (optional data-dim; x, mean, cov or I).",
            Proficiency::Expert,
        ),
        "sheet:stats_mvn_sample" => (
            "MVN sample",
            "Draw one MVN sample (optional data-dim/data-seed; mean, cov or I).",
            Proficiency::Expert,
        ),
        "sheet:stats_mvn_mle" => (
            "MVN MLE",
            "Maximum-likelihood MVN mean and covariance (optional data-dim/data-n).",
            Proficiency::Expert,
        ),
        "sheet:stats_validate_probability" => (
            "Validate probability",
            "Check non-negative sheet masses that sum to one.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_simplex_project" => (
            "Simplex project",
            "Euclidean projection of sheet masses onto the probability simplex.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_fisher_distance" => (
            "Fisher distance",
            "Fisher–Rao geodesic distance between series halves.",
            Proficiency::Expert,
        ),
        "sheet:stats_neg_entropy" => (
            "Negative entropy",
            "Σ pᵢ ln(pᵢ) over sheet masses (KL Bregman generator).",
            Proficiency::Intermediate,
        ),
        "sheet:poly_eval" => (
            "Evaluate polynomial",
            "Evaluate little-endian sheet coeffs at x (data-x or last number).",
            Proficiency::Novice,
        ),
        "sheet:poly_add" => (
            "Add polynomials",
            "Add two polynomials from sheet halves (or data-a-len).",
            Proficiency::Novice,
        ),
        "sheet:poly_sub" => (
            "Subtract polynomials",
            "Subtract the second sheet polynomial from the first.",
            Proficiency::Novice,
        ),
        "sheet:poly_mul" => (
            "Multiply polynomials",
            "Multiply two polynomials from sheet halves (or data-a-len).",
            Proficiency::Intermediate,
        ),
        "sheet:poly_gcd" => (
            "Polynomial GCD",
            "Monic greatest common divisor of two sheet polynomials.",
            Proficiency::Expert,
        ),
        "sheet:poly_degree" => (
            "Polynomial degree",
            "Degree of the sheet polynomial (empty means zero).",
            Proficiency::Novice,
        ),
        "sheet:poly_leading" => (
            "Leading coefficient",
            "Highest-order coefficient of the sheet polynomial.",
            Proficiency::Novice,
        ),
        "sheet:poly_is_zero" => (
            "Is zero polynomial",
            "Check whether the trimmed coefficient list is empty.",
            Proficiency::Novice,
        ),
        "sheet:poly_scale" => (
            "Scale polynomial",
            "Multiply every coefficient by s (data-s or last number).",
            Proficiency::Novice,
        ),
        "sheet:poly_zero" => (
            "Zero polynomial",
            "Build the empty (zero) coefficient list.",
            Proficiency::Novice,
        ),
        "sheet:poly_constant" => (
            "Constant polynomial",
            "Build a constant polynomial from data-c or the first number.",
            Proficiency::Novice,
        ),
        "sheet:poly_derivative" => (
            "Polynomial derivative",
            "First derivative of little-endian sheet coefficients.",
            Proficiency::Intermediate,
        ),
        "sheet:poly_monic" => (
            "Monic polynomial",
            "Scale sheet coefficients so the leading coefficient is one.",
            Proficiency::Novice,
        ),
        "sheet:poly_div_rem" => (
            "Polynomial division",
            "Divide two sheet polynomials into quotient and remainder.",
            Proficiency::Intermediate,
        ),
        "sheet:poly_resultant" => (
            "Polynomial resultant",
            "Euclidean resultant of two sheet polynomials (common-root test).",
            Proficiency::Expert,
        ),
        "sheet:stats_simplex_project_idempotent" => (
            "Simplex idempotent",
            "Check that projecting sheet masses twice matches projecting once.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_fisher_inner_product" => (
            "Fisher inner product",
            "Fisher metric inner product ⟨u,v⟩_p from sheet thirds.",
            Proficiency::Expert,
        ),
        "sheet:stats_neg_entropy_grad" => (
            "Neg-entropy gradient",
            "Gradient ∇ψ(p)_i = ln(pᵢ)+1 over sheet masses.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_kl_bregman_form" => (
            "KL Bregman form",
            "KL divergence via the neg-entropy Bregman generator on halves.",
            Proficiency::Expert,
        ),
        "sheet:stats_bregman_pythagorean_test" => (
            "Bregman–Pythagorean",
            "Three KL parts for the Bregman–Pythagorean identity (thirds).",
            Proficiency::Expert,
        ),
        "sheet:stats_probability_hash" => (
            "Probability hash",
            "Stable FNV-1a hash of sheet mass bit patterns.",
            Proficiency::Novice,
        ),
        "spatial:place_map" => ("Map", "Put a map on the work surface.", Proficiency::Novice),
        "spatial:place_dual_studio" => (
            "Script and picture studio",
            "Put a studio that holds a script beside a picture.",
            Proficiency::Intermediate,
        ),
        "spatial:place_scene_view" => (
            "Scene",
            "Put a scene inspector on the work surface.",
            Proficiency::Intermediate,
        ),
        "spatial:place_3d" => (
            "3D view",
            "Put a three-dimensional view on the work surface.",
            Proficiency::Novice,
        ),
        "spatial:pin" => (
            "Drop a pin",
            "Drop a location pin on the selected map.",
            Proficiency::Novice,
        ),
        "spatial:track" => (
            "Follow someone",
            "Follow a path on the map. Needs consent and a live path.",
            Proficiency::Expert,
        ),
        "spatial:camera_reset" => (
            "Reset view",
            "Return the map or 3D view to a straight-on look.",
            Proficiency::Novice,
        ),
        "spatial:orbit_preview" => (
            "Spin preview",
            "Preview a gentle orbit around the scene.",
            Proficiency::Intermediate,
        ),
        "spatial:threed_add_object" => (
            "Add object",
            "Add a named 3D object on the selected surface.",
            Proficiency::Novice,
        ),
        "spatial:threed_set_transform" => (
            "Set transform",
            "Set object position from three numbers on the surface.",
            Proficiency::Novice,
        ),
        "spatial:threed_set_material" => (
            "Set material",
            "Assign a material id to a 3D object.",
            Proficiency::Novice,
        ),
        "spatial:threed_add_camera" => (
            "Add camera",
            "Add a camera with a numeric field of view.",
            Proficiency::Novice,
        ),
        "spatial:threed_add_light" => (
            "Add light",
            "Add a point, directional, spot, or ambient light.",
            Proficiency::Novice,
        ),
        "spatial:threed_add_rig" => (
            "Add rig",
            "Add a named control or animation rig.",
            Proficiency::Intermediate,
        ),
        "spatial:threed_add_animation" => (
            "Add animation",
            "Add an animation clip with a numeric duration.",
            Proficiency::Novice,
        ),
        "spatial:threed_set_mesh" => (
            "Set mesh",
            "Assign a mesh id to a 3D object.",
            Proficiency::Novice,
        ),
        "spatial:scene_lerp_camera" => (
            "Blend cameras",
            "Blend two orbit camera looks with a number between zero and one.",
            Proficiency::Novice,
        ),
        "spatial:scene_camera_frame_node" => (
            "Frame a point",
            "Aim the orbit camera so a point sits in view.",
            Proficiency::Novice,
        ),
        "spatial:scene_smooth_damp" => (
            "Ease a number",
            "Ease one number toward another with smooth damping.",
            Proficiency::Novice,
        ),
        "spatial:scene_smooth_damp_vec3" => (
            "Ease a point",
            "Ease a three-number point toward a target with smooth damping.",
            Proficiency::Novice,
        ),
        "spatial:scene_ik_look_at" => (
            "Aim a chain",
            "Aim a joint chain so the tip looks at a target point.",
            Proficiency::Intermediate,
        ),
        "spatial:scene_ik_ccd" => (
            "Bend a chain",
            "Bend a joint chain toward a target with CCD inverse kinematics.",
            Proficiency::Intermediate,
        ),
        "spatial:scene_set_render_budget" => (
            "Frame budget",
            "Set how many milliseconds a render frame may spend.",
            Proficiency::Novice,
        ),
        "spatial:scene_set_clear_colour" => (
            "Clear colour",
            "Set the viewport wipe colour from four numbers.",
            Proficiency::Novice,
        ),
        "spatial:scene_create" => (
            "Create scene",
            "Create a named scene graph (Host Scene.create).",
            Proficiency::Novice,
        ),
        "spatial:scene_add_node" => (
            "Add scene node",
            "Add a numbered node at x, y, z via Scene.add_node.",
            Proficiency::Novice,
        ),
        "spatial:scene_set_transform" => (
            "Set scene transform",
            "Set a node’s position, rotation, and scale via Scene.set_transform.",
            Proficiency::Novice,
        ),
        "spatial:scene_set_mesh" => (
            "Set scene mesh",
            "Assign a mesh IRI to a node via Scene.set_mesh.",
            Proficiency::Novice,
        ),
        "spatial:scene_add_camera" => (
            "Add scene camera",
            "Add a camera with position and field of view via Scene.add_camera.",
            Proficiency::Novice,
        ),
        "spatial:scene_render" => (
            "Render scene",
            "Request a render of a named scene via Scene.render.",
            Proficiency::Novice,
        ),
        "spatial:scene_set_viewport" => (
            "Set viewport",
            "Set viewport width, height, and format via Scene.set_viewport.",
            Proficiency::Novice,
        ),
        "spatial:scene_capture_frame" => (
            "Capture frame",
            "Request a frame capture via Scene.capture_frame.",
            Proficiency::Novice,
        ),
        "spatial:scene_add_light" => (
            "Add scene light",
            "Add a point, directional, spot, or ambient light via Scene.add_light.",
            Proficiency::Novice,
        ),
        "spatial:scene_link_semantic" => (
            "Link semantic",
            "Link a scene node to a semantic IRI via Scene.link_semantic.",
            Proficiency::Intermediate,
        ),
        "spatial:scene_duplicate_node" => (
            "Duplicate node",
            "Duplicate a scene node with a new id via Scene.duplicate_node.",
            Proficiency::Novice,
        ),
        "audio:place_audio_session" => (
            "Sound session",
            "Put a sound session on the work surface.",
            Proficiency::Novice,
        ),
        "audio:place_media" => (
            "Voice colour",
            "Put a live sound-colour surface.",
            Proficiency::Intermediate,
        ),
        "audio:mic_capture" => (
            "Listen",
            "Capture a short sound. Needs microphone permission.",
            Proficiency::Expert,
        ),
        "audio:neural_latents" => (
            "Sound model",
            "Inspect a loaded sound model. Needs a mounted model.",
            Proficiency::Expert,
        ),
        "audio:dsp_ep_temp" => (
            "Epistemic temperature",
            "Map q to τ = clamp(q², 0, 4) (Host Audio.epistemic_temperature_from_q).",
            Proficiency::Intermediate,
        ),
        "audio:dsp_ep_fm" => (
            "Epistemic FM index",
            "FM index from epistemic q and carrier μ.",
            Proficiency::Intermediate,
        ),
        "audio:dsp_sigma_freq" => (
            "Dominant frequency",
            "Map 64 σ preview bins to a fundamental in Hz.",
            Proficiency::Intermediate,
        ),
        "audio:dsp_parametric_sample" => (
            "Parametric sample",
            "Advance one sample from a parametric voice state.",
            Proficiency::Intermediate,
        ),
        "audio:dsp_bin_freq_linear" => (
            "Linear bin to Hz",
            "Convert an STFT bin index to frequency in Hz.",
            Proficiency::Novice,
        ),
        "audio:dsp_bin_freq_log" => (
            "Log bin to Hz",
            "Convert a CQT-style log bin to frequency in Hz.",
            Proficiency::Novice,
        ),
        "audio:dsp_midi_note" => (
            "MIDI to frequency",
            "Convert a MIDI note number to Hz (A4 = 440).",
            Proficiency::Novice,
        ),
        "audio:dsp_quantize" => (
            "Quantize beat",
            "Snap a beat position onto a grid division.",
            Proficiency::Novice,
        ),
        "audio:dsp_transpose" => (
            "Transpose note",
            "Transpose a MIDI note by semitones (clamped 0–127).",
            Proficiency::Novice,
        ),
        "audio:fx_oscillator" => (
            "Oscillator",
            "Render a sine/square/saw/triangle buffer via Audio.oscillator.",
            Proficiency::Novice,
        ),
        "audio:fx_envelope" => (
            "Envelope",
            "Render an ADSR envelope buffer via Audio.envelope.",
            Proficiency::Novice,
        ),
        "audio:fx_filter" => (
            "Biquad filter",
            "Apply a lowpass/highpass/bandpass/notch filter via Audio.filter.",
            Proficiency::Novice,
        ),
        "audio:fx_lfo" => (
            "LFO",
            "Render a low-frequency oscillator buffer via Audio.lfo.",
            Proficiency::Novice,
        ),
        "audio:fx_delay" => (
            "Delay",
            "Apply a delay with feedback and mix via Audio.delay.",
            Proficiency::Novice,
        ),
        "audio:fx_reverb" => (
            "Reverb",
            "Apply a room reverb via Audio.reverb.",
            Proficiency::Novice,
        ),
        "audio:fx_compressor" => (
            "Compressor",
            "Apply dynamic-range compression via Audio.compressor.",
            Proficiency::Novice,
        ),
        "audio:fx_eq" => (
            "Three-band EQ",
            "Apply low/mid/high gains via Audio.eq.",
            Proficiency::Novice,
        ),
        "audio:fx_transport" => (
            "Transport",
            "Play, stop, pause, record, or query status via Audio.transport.",
            Proficiency::Novice,
        ),
        "audio:fx_waveform_meter" => (
            "Waveform meter",
            "Measure peak/RMS and a display envelope via Audio.waveform_meter.",
            Proficiency::Novice,
        ),
        "audio:fx_phase_meter" => (
            "Phase meter",
            "Measure stereo phase correlation via Audio.phase_meter.",
            Proficiency::Novice,
        ),
        "audio:fx_loudness_meter" => (
            "Loudness meter",
            "Measure LUFS loudness via Audio.loudness_meter.",
            Proficiency::Novice,
        ),
        "audio:fx_spectrum" => (
            "Spectrum",
            "Read a rasterised spectrum via Audio.spectrum.",
            Proficiency::Intermediate,
        ),
        "comm:place_social" => (
            "People graph",
            "Put a page of people and connections.",
            Proficiency::Novice,
        ),
        "comm:place_webrtc" => (
            "Live call",
            "Put a live audio or video window.",
            Proficiency::Intermediate,
        ),
        "comm:place_webview" => (
            "Web page",
            "Put a window onto a web page.",
            Proficiency::Novice,
        ),
        "comm:pulse_presence" => (
            "I'm here",
            "Let others see that you are present.",
            Proficiency::Intermediate,
        ),
        "erp:place_kanban" => (
            "Task board",
            "Put a shared task board on the work surface.",
            Proficiency::Novice,
        ),
        "erp:place_gantt" => (
            "Timeline plan",
            "Put a timeline of work on the work surface.",
            Proficiency::Intermediate,
        ),
        "erp:place_voting" => (
            "Group vote",
            "Put a page where a group can vote.",
            Proficiency::Intermediate,
        ),
        "mail:place_mail" => (
            "Letters",
            "Put an inbox for addressed letters.",
            Proficiency::Novice,
        ),
        "mail:composer" => (
            "Write a letter",
            "Open a letter to send.",
            Proficiency::Novice,
        ),
        "mail:publisher" => (
            "Publish a page",
            "Publish a finished page. Needs a destination and permission.",
            Proficiency::Expert,
        ),
        "scientific:place_health" => (
            "Clinic bench",
            "Put a clinical workbench. Health review still governs live calculators.",
            Proficiency::Intermediate,
        ),
        "scientific:place_3d" => (
            "Molecule view",
            "Put a three-dimensional science view.",
            Proficiency::Intermediate,
        ),
        "scientific:thermodynamics" => (
            "Heat model",
            "Run a heat model. Needs a prepared target.",
            Proficiency::Expert,
        ),
        "scientific:la_matmul" => (
            "Multiply matrices",
            "Multiply two matrices from numbers on this surface.",
            Proficiency::Intermediate,
        ),
        "scientific:la_matvec" => (
            "Matrix times vector",
            "Multiply a matrix by a vector on this surface.",
            Proficiency::Intermediate,
        ),
        "scientific:la_transpose" => (
            "Flip matrix",
            "Transpose a matrix from numbers on this surface.",
            Proficiency::Novice,
        ),
        "scientific:la_determinant" => (
            "Matrix determinant",
            "Compute the determinant of a square matrix on this surface.",
            Proficiency::Intermediate,
        ),
        "scientific:la_solve" => (
            "Solve equations",
            "Solve A x = b from a square matrix and right-hand side.",
            Proficiency::Intermediate,
        ),
        "scientific:la_scale" => (
            "Scale numbers",
            "Multiply surface numbers by a scale factor (data-s).",
            Proficiency::Novice,
        ),
        "scientific:la_add_into" => (
            "Add number lists",
            "Add two equal-length number lists from this surface.",
            Proficiency::Novice,
        ),
        "scientific:la_axpy" => (
            "Scaled vector add",
            "Add a scaled vector to another (y += α·x).",
            Proficiency::Intermediate,
        ),
        "scientific:la_hadamard_into" => (
            "Pairwise product",
            "Multiply matching pairs from two equal-length lists.",
            Proficiency::Novice,
        ),
        "scientific:la_qr_factor" => (
            "QR factor",
            "Factor a tall or square matrix with Householder QR.",
            Proficiency::Expert,
        ),
        "scientific:la_cholesky_factor" => (
            "Cholesky factor",
            "Factor a positive-definite square matrix into L Lᵀ.",
            Proficiency::Expert,
        ),
        "scientific:la_cholesky_solve" => (
            "Cholesky solve",
            "Solve A x = b after Cholesky-factoring SPD A.",
            Proficiency::Expert,
        ),
        "scientific:la_lu_decompose" => (
            "LU factor",
            "Factor a square matrix with partial pivoting.",
            Proficiency::Expert,
        ),
        "scientific:la_lu_solve" => (
            "LU solve",
            "Solve A x = b using LU decomposition.",
            Proficiency::Expert,
        ),
        "scientific:la_qr_form_q" => (
            "Build Q",
            "Form the thin Q matrix from a QR factor.",
            Proficiency::Expert,
        ),
        "scientific:la_qr_solve_ls" => (
            "Least squares",
            "Solve a least-squares problem with QR.",
            Proficiency::Expert,
        ),
        "scientific:la_svd" => (
            "Singular values",
            "Compute the singular value decomposition of a matrix.",
            Proficiency::Expert,
        ),
        "scientific:la_eigenvalues" => (
            "Eigenvalues",
            "Compute eigenvalues of a square matrix.",
            Proficiency::Expert,
        ),
        "scientific:la_add_assign" => (
            "Add in place",
            "Add matching pairs into the first list.",
            Proficiency::Novice,
        ),
        "scientific:la_hadamard_assign" => (
            "Multiply in place",
            "Multiply matching pairs into the first list.",
            Proficiency::Novice,
        ),
        "scientific:la_cholesky_det" => (
            "Cholesky determinant",
            "Determinant of an SPD matrix via its Cholesky factor.",
            Proficiency::Expert,
        ),
        "scientific:la_charpoly" => (
            "Characteristic poly",
            "Coefficients of the characteristic polynomial.",
            Proficiency::Expert,
        ),
        "scientific:la_eigenvalues_general" => (
            "General eigenvalues",
            "Eigenvalues of a general square matrix.",
            Proficiency::Expert,
        ),
        "scientific:la_eigen_symmetric" => (
            "Symmetric eigen",
            "Eigenvalues and vectors of a symmetric matrix.",
            Proficiency::Expert,
        ),
        "scientific:la_polynomial_roots" => (
            "Polynomial roots",
            "Find complex roots from descending coefficients.",
            Proficiency::Intermediate,
        ),
        "scientific:la_solve_linear_system" => (
            "Solve linear system",
            "Solve A x = b with Gaussian elimination.",
            Proficiency::Intermediate,
        ),
        "scientific:la_symmetric_eigen_3x3" => (
            "3×3 symmetric eigen",
            "Closed-form eigenvalues of a symmetric 3×3 matrix.",
            Proficiency::Intermediate,
        ),
        "scientific:chem_boys" => (
            "Boys function",
            "Evaluate Boys F_n(t) from data-n/data-t or two surface numbers.",
            Proficiency::Intermediate,
        ),
        "scientific:chem_overlap_s" => (
            "s-GTO overlap",
            "s-type overlap integral (a|b) from surface GTO numbers or demos.",
            Proficiency::Intermediate,
        ),
        "scientific:chem_kinetic_s" => (
            "s-GTO kinetic",
            "s-type kinetic integral from surface GTO numbers or demos.",
            Proficiency::Intermediate,
        ),
        "scientific:chem_nuclear_s" => (
            "s-GTO nuclear",
            "s-type nuclear attraction with center and charge Z.",
            Proficiency::Expert,
        ),
        "scientific:chem_dipole_s" => (
            "s-GTO dipole",
            "s-type dipole moment vector from two GTOs.",
            Proficiency::Intermediate,
        ),
        "scientific:chem_evaluate_eri" => (
            "Two-electron ERI",
            "Evaluate (ab|cd) electron-repulsion for four s-GTOs.",
            Proficiency::Expert,
        ),
        "scientific:chem_total_angular_momentum" => (
            "GTO angular momentum",
            "Sum lx+ly+lz for a primitive GTO (data-lx/ly/lz).",
            Proficiency::Novice,
        ),
        "scientific:chem_letter" => (
            "Spectroscopic letter",
            "Map angular momentum l to s/p/d/f/g/h.",
            Proficiency::Novice,
        ),
        "scientific:chem_n_cartesian" => (
            "Cartesian shells",
            "Count Cartesian GTO components for angular momentum l.",
            Proficiency::Novice,
        ),
        "scientific:chem_n_spherical" => (
            "Spherical shells",
            "Count spherical GTO components (2l+1) for angular momentum l.",
            Proficiency::Novice,
        ),
        "scientific:chem_from_letter" => (
            "Letter to l",
            "Map spectroscopic letter s/p/d/f/g/h to angular momentum.",
            Proficiency::Novice,
        ),
        "scientific:chem_gaussian_elim" => (
            "Gaussian elimination",
            "Solve A x = b for 2×2/3×3 chemistry SCF stack matrices.",
            Proficiency::Intermediate,
        ),
        "scientific:chem_jacobi" => (
            "Jacobi diagonalization",
            "Eigenpairs of a real symmetric 2×2/3×3 matrix (SCF path).",
            Proficiency::Intermediate,
        ),
        "scientific:chem_transpose" => (
            "Matrix transpose",
            "Transpose a 2×2/3×3 matrix on the chemistry Host path.",
            Proficiency::Novice,
        ),
        "scientific:chem_orthogonalize" => (
            "Löwdin orthogonalize",
            "Build Löwdin X = S^{-1/2} for a 2×2/3×3 overlap matrix.",
            Proficiency::Intermediate,
        ),
        "scientific:chem_element_symbol" => (
            "Element symbol",
            "Map atomic number Z to element symbol (data-z or surface).",
            Proficiency::Novice,
        ),
        "scientific:chem_atomic_number" => (
            "Atomic number",
            "Map element symbol to atomic number Z (data-symbol).",
            Proficiency::Novice,
        ),
        "scientific:chem_atomic_weight" => (
            "Atomic weight",
            "Standard atomic weight for an element symbol.",
            Proficiency::Novice,
        ),
        "scientific:chem_lda_exchange" => (
            "LDA exchange",
            "LDA exchange energy and potential from density ρ.",
            Proficiency::Intermediate,
        ),
        "scientific:chem_lda_vwn" => (
            "LDA VWN correlation",
            "VWN LDA correlation energy and potential from density ρ.",
            Proficiency::Intermediate,
        ),
        "scientific:chem_sto3g_h2" => (
            "STO-3G H₂ summary",
            "STO-3G H₂ minimal-basis summary (R and energy constants).",
            Proficiency::Novice,
        ),
        "scientific:sf_airy_ai" => (
            "Airy Ai",
            "Evaluate Ai(x) from data-x or one surface number.",
            Proficiency::Intermediate,
        ),
        "scientific:sf_airy_bi" => (
            "Airy Bi",
            "Evaluate Bi(x) from data-x or one surface number.",
            Proficiency::Intermediate,
        ),
        "scientific:sf_zeta" => (
            "Riemann zeta",
            "Evaluate ζ(s) for s>1 from data-s or one surface number.",
            Proficiency::Intermediate,
        ),
        "scientific:sf_legendre" => (
            "Legendre P_n",
            "Evaluate Legendre P_n(x) from data-n/data-x or two numbers.",
            Proficiency::Intermediate,
        ),
        "scientific:sf_chebyshev_t" => (
            "Chebyshev T_n",
            "Evaluate Chebyshev T_n(x) from data-n/data-x or two numbers.",
            Proficiency::Intermediate,
        ),
        "scientific:sf_chebyshev_u" => (
            "Chebyshev U_n",
            "Evaluate Chebyshev U_n(x) from data-n/data-x or two numbers.",
            Proficiency::Intermediate,
        ),
        "scientific:sf_hermite" => (
            "Hermite H_n",
            "Evaluate physicists' Hermite H_n(x) from data-n/data-x.",
            Proficiency::Intermediate,
        ),
        "scientific:sf_laguerre" => (
            "Laguerre L_n",
            "Evaluate Laguerre L_n(x) from data-n/data-x or two numbers.",
            Proficiency::Intermediate,
        ),
        "scientific:sf_bessel_j" => (
            "Bessel J_n",
            "Evaluate Bessel J_n(x) from data-n/data-x or two numbers.",
            Proficiency::Intermediate,
        ),
        "scientific:sf_bessel_i" => (
            "Bessel I_n",
            "Evaluate modified Bessel I_n(x) from data-n/data-x.",
            Proficiency::Intermediate,
        ),
        "scientific:sf_bessel_y" => (
            "Bessel Y_n",
            "Evaluate Bessel Y_n(x) for x>0 from data-n/data-x.",
            Proficiency::Intermediate,
        ),
        "scientific:sf_bessel_k" => (
            "Bessel K_n",
            "Evaluate modified Bessel K_n(x) for x>0 from data-n/data-x.",
            Proficiency::Intermediate,
        ),
        "scientific:xform_dft" => (
            "DFT (real)",
            "Forward DFT of a real signal from surface numbers (Host IntegralTransforms.dft).",
            Proficiency::Intermediate,
        ),
        "scientific:xform_dft_complex" => (
            "DFT (complex)",
            "Forward DFT of [re,im] sample pairs (Host IntegralTransforms.dft_complex).",
            Proficiency::Intermediate,
        ),
        "scientific:xform_idft" => (
            "IDFT",
            "Inverse DFT from [re,im] spectrum pairs (Host IntegralTransforms.idft).",
            Proficiency::Intermediate,
        ),
        "scientific:xform_z_transform_finite" => (
            "Z-transform (finite)",
            "Finite-sequence Z-transform at complex z (data-z-re/data-z-im).",
            Proficiency::Intermediate,
        ),
        "scientific:xform_unit_step_z" => (
            "Unit-step Z",
            "Closed-form Z{u[n]} at complex z with |z|>1.",
            Proficiency::Intermediate,
        ),
        "scientific:xform_geometric_z" => (
            "Geometric Z",
            "Closed-form Z{a^n} at complex z (data-a / data-z-re/im).",
            Proficiency::Intermediate,
        ),
        "scientific:xform_laplace_numeric" => (
            "Laplace (numeric)",
            "Numerical Laplace transform of expr at s (data-expr/data-s).",
            Proficiency::Intermediate,
        ),
        "scientific:xform_laplace_symbolic" => (
            "Laplace (symbolic)",
            "Table Laplace transform of a simple expr (data-expr).",
            Proficiency::Intermediate,
        ),
        "scientific:calc_hermite_dense" => (
            "Hermite dense output",
            "Cubic Hermite state at θ∈[0,1] from y0,f0,y1,f1,h,theta.",
            Proficiency::Intermediate,
        ),
        "scientific:calc_bdf1" => (
            "BDF1 step",
            "One implicit Euler step (power-law RHS; default decay −y).",
            Proficiency::Intermediate,
        ),
        "scientific:calc_bdf2" => (
            "BDF2 step",
            "One BDF2 stiff step from two prior states.",
            Proficiency::Intermediate,
        ),
        "scientific:calc_verlet" => (
            "Verlet step",
            "One Störmer–Verlet step for a harmonic oscillator.",
            Proficiency::Intermediate,
        ),
        "scientific:calc_ruth3" => (
            "Ruth3 step",
            "One Ruth 3rd-order symplectic step (harmonic defaults).",
            Proficiency::Intermediate,
        ),
        "scientific:calc_yoshida4" => (
            "Yoshida4 step",
            "One Yoshida 4th-order symplectic step (harmonic defaults).",
            Proficiency::Intermediate,
        ),
        "scientific:calc_integrate_bdf" => (
            "Integrate BDF",
            "BDF2 integration over steps (power-law RHS; default −y).",
            Proficiency::Intermediate,
        ),
        "scientific:calc_integrate_sens" => (
            "Integrate + sensitivity",
            "RK4 state and ∂y/∂y₀ for a power-law ODE.",
            Proficiency::Intermediate,
        ),
        "scientific:calc_invariant_drift" => (
            "Invariant drift",
            "Absolute and relative drift of a conserved quantity.",
            Proficiency::Novice,
        ),
        "scientific:calc_perm_parity" => (
            "Permutation parity",
            "Even (+1) or odd (−1) parity of a permutation list.",
            Proficiency::Novice,
        ),
        "scientific:calc_pack_f32" => (
            "Pack f32 pair",
            "Pack step and compensation floats into one u64.",
            Proficiency::Novice,
        ),
        "scientific:calc_unpack_f32" => (
            "Unpack f32 pair",
            "Unpack a u64 back into step and compensation floats.",
            Proficiency::Novice,
        ),
        "scientific:calc_poisson_bracket" => (
            "Poisson bracket",
            "Canonical Poisson bracket from ∂f/∂q,∂f/∂p,∂g/∂q,∂g/∂p pairs.",
            Proficiency::Intermediate,
        ),
        "scientific:calc_stormer_verlet" => (
            "Störmer–Verlet step",
            "One Störmer–Verlet harmonic step (q,p,h; optional k,mass).",
            Proficiency::Intermediate,
        ),
        "scientific:calc_gauss_kronrod" => (
            "Gauss–Kronrod 15",
            "Adaptive G7-K15 quadrature of data-expr over [a,b].",
            Proficiency::Intermediate,
        ),
        "scientific:calc_jvp" => (
            "JVP (2×2)",
            "Jacobian-vector product for a fixed 2×2 linear map.",
            Proficiency::Intermediate,
        ),
        "scientific:calc_vjp" => (
            "VJP (2×2)",
            "Vector-Jacobian product for a fixed 2×2 linear map.",
            Proficiency::Intermediate,
        ),
        "scientific:calc_adaptive_simpson" => (
            "Adaptive Simpson",
            "Adaptive Simpson quadrature of data-expr over [a,b].",
            Proficiency::Intermediate,
        ),
        "scientific:calc_adaptive_deriv" => (
            "Adaptive derivative",
            "Adaptive central difference of data-expr at x.",
            Proficiency::Intermediate,
        ),
        "scientific:calc_newton_solve" => (
            "Newton solve",
            "Multidimensional Newton–Raphson from data-exprs/vars/guess.",
            Proficiency::Intermediate,
        ),
        "scientific:calc_num_jacobian" => (
            "Numerical Jacobian",
            "Finite-difference Jacobian of data-exprs at a point.",
            Proficiency::Intermediate,
        ),
        "scientific:calc_num_hessian" => (
            "Numerical Hessian",
            "Finite-difference Hessian of data-expr at a point.",
            Proficiency::Intermediate,
        ),
        "scientific:cg_distance_2d" => (
            "Distance 2D",
            "Euclidean distance between two 2D points from surface numbers.",
            Proficiency::Novice,
        ),
        "scientific:cg_distance_3d" => (
            "Distance 3D",
            "Euclidean distance between two 3D points from surface numbers.",
            Proficiency::Novice,
        ),
        "scientific:cg_point_segment_2d" => (
            "Point–segment 2D",
            "Distance from a 2D point to a line segment.",
            Proficiency::Novice,
        ),
        "scientific:cg_orientation_2" => (
            "Orientation 2D",
            "Orientation of three 2D points (counter-clockwise, clockwise, or collinear).",
            Proficiency::Novice,
        ),
        "scientific:cg_orient_3d" => (
            "Orient 3D",
            "Signed orientation of four 3D points forming a tetrahedron.",
            Proficiency::Intermediate,
        ),
        "scientific:cg_morton_encode_2d" => (
            "Morton encode 2D",
            "Morton (Z-order) code from integer x,y (data-x/data-y).",
            Proficiency::Novice,
        ),
        "scientific:cg_morton_decode_2d" => (
            "Morton decode 2D",
            "Decode a 2D Morton code back to integer x,y.",
            Proficiency::Novice,
        ),
        "scientific:cg_morton_encode_3d" => (
            "Morton encode 3D",
            "Morton code from integer x,y,z.",
            Proficiency::Novice,
        ),
        "scientific:cg_hilbert_encode_2d" => (
            "Hilbert encode 2D",
            "Hilbert curve code from integer x,y.",
            Proficiency::Intermediate,
        ),
        "scientific:cg_circumcenter" => (
            "Circumcenter",
            "Circumcenter of a 2D triangle from three points.",
            Proficiency::Novice,
        ),
        "scientific:cg_point_segment_3d" => (
            "Point–segment 3D",
            "Distance from a 3D point to a line segment (px..bz).",
            Proficiency::Novice,
        ),
        "scientific:cg_point_triangle_3d" => (
            "Point–triangle 3D",
            "Squared distance from a 3D point to a triangle.",
            Proficiency::Intermediate,
        ),
        "scientific:cg_convex_hull_2" => (
            "Convex hull 2D",
            "2D convex hull from flat x,y point pairs on the surface.",
            Proficiency::Novice,
        ),
        "scientific:cg_triangulate" => (
            "Triangulate polygon",
            "Ear-clip triangulation of a 2D polygon from vertex pairs.",
            Proficiency::Intermediate,
        ),
        "scientific:cg_surface_area" => (
            "Surface area",
            "Triangle-mesh surface area from 3D vertices (and optional data-triangles).",
            Proficiency::Intermediate,
        ),
        "scientific:cg_signed_volume" => (
            "Signed volume",
            "Signed volume of a closed triangle mesh.",
            Proficiency::Intermediate,
        ),
        "scientific:cg_segment_intersect_2" => (
            "Segment ∩ 2D",
            "Intersection of two 2D line segments (eight numbers).",
            Proficiency::Novice,
        ),
        "scientific:cg_bezier_eval" => (
            "Bézier eval",
            "Evaluate a 3D Bézier curve at t (control xyz triples + data-t).",
            Proficiency::Intermediate,
        ),
        "scientific:cg_nearest_site" => (
            "Nearest site",
            "Nearest Voronoi site by brute-force distance (sites + query).",
            Proficiency::Novice,
        ),
        "scientific:ga_dot" => (
            "Dot product",
            "Dot product of two 3-vectors from surface numbers or data-a*/data-b*.",
            Proficiency::Novice,
        ),
        "scientific:ga_cross" => (
            "Cross product",
            "Cross product of two 3-vectors.",
            Proficiency::Novice,
        ),
        "scientific:ga_normalize" => (
            "Normalize vector",
            "Unit-length 3-vector from data-v* or three surface numbers.",
            Proficiency::Novice,
        ),
        "scientific:ga_angle" => (
            "Angle between vectors",
            "Angle in radians between two 3-vectors.",
            Proficiency::Novice,
        ),
        "scientific:ga_geometric_product" => (
            "Geometric product",
            "Cl(3,0) geometric product of two vectors as 8-coeff multivectors.",
            Proficiency::Intermediate,
        ),
        "scientific:ga_outer_product" => (
            "Outer product",
            "Cl(3,0) outer (wedge) product of two vectors as multivectors.",
            Proficiency::Intermediate,
        ),
        "scientific:ga_rotor" => (
            "Rotor from angle/axis",
            "Build a Cl(3,0) rotor from angle (rad) and unit axis.",
            Proficiency::Intermediate,
        ),
        "scientific:ga_apply_rotor" => (
            "Apply rotor",
            "Rotate a 3-vector by a 4-component rotor.",
            Proficiency::Intermediate,
        ),
        "scientific:ga_translator" => (
            "Translator from displacement",
            "Build a translator from a 3-vector displacement.",
            Proficiency::Intermediate,
        ),
        "scientific:ga_apply_translator" => (
            "Apply translator",
            "Translate a 3-vector by a 4-component translator.",
            Proficiency::Intermediate,
        ),
        "scientific:ga_is_simd" => (
            "GA SIMD available?",
            "Report whether AVX2 GA kernels are available on the Host.",
            Proficiency::Novice,
        ),
        "scientific:eng_natural_freq" => (
            "SDOF natural frequency",
            "Undamped SDOF ωₙ = √(k/m) from stiffness and mass.",
            Proficiency::Novice,
        ),
        "scientific:eng_harmonic_sdof" => (
            "Harmonic SDOF FRF",
            "Forced SDOF amplitude and phase at excitation frequencies.",
            Proficiency::Intermediate,
        ),
        "scientific:eng_euler" => (
            "Euler buckling",
            "Euler column critical loads for modes 1..=N.",
            Proficiency::Intermediate,
        ),
        "scientific:eng_reliability" => (
            "Reliability index β",
            "β = −Φ⁻¹(p_f) from a failure probability.",
            Proficiency::Intermediate,
        ),
        "scientific:eng_kinematics" => (
            "Kinematics",
            "Constant-acceleration positions and velocities over times.",
            Proficiency::Novice,
        ),
        "scientific:eng_cauchy" => (
            "Cauchy stress",
            "von Mises / principals from a 3×3 row-major stress tensor.",
            Proficiency::Intermediate,
        ),
        "scientific:eng_drag" => (
            "Drag force",
            "Aerodynamic drag F = ½ρv²C_dA.",
            Proficiency::Novice,
        ),
        "scientific:eng_reynolds" => (
            "Reynolds number",
            "Re = ρvL/μ for laminar/turbulent regime.",
            Proficiency::Novice,
        ),
        "scientific:eng_fatigue" => (
            "Fatigue cycles",
            "Basquin cycles-to-failure from stress amplitude.",
            Proficiency::Intermediate,
        ),
        "scientific:eng_miner" => (
            "Miner damage",
            "Palmgren–Miner cumulative damage from load-block pairs.",
            Proficiency::Intermediate,
        ),
        "scientific:phys_doppler" => (
            "Doppler shift",
            "Shift a source frequency by relative velocity (relativistic).",
            Proficiency::Intermediate,
        ),
        "scientific:phys_emf_attenuation" => (
            "EMF attenuation",
            "Estimate received power with inverse-square loss and absorption.",
            Proficiency::Intermediate,
        ),
        "scientific:phys_harmonic" => (
            "Harmonic oscillator",
            "Integrate a spring–mass oscillator from mass, k, and initial state.",
            Proficiency::Intermediate,
        ),
        "scientific:phys_pendulum" => (
            "Pendulum",
            "Integrate a nonlinear pendulum from length, g, and initial angle.",
            Proficiency::Intermediate,
        ),
        "scientific:phys_logistic" => (
            "Logistic growth",
            "Grow a population toward a carrying capacity over time.",
            Proficiency::Novice,
        ),
        "scientific:phys_cfd_step" => (
            "CFD residual",
            "Measure the Burgers residual of a 1D velocity field.",
            Proficiency::Intermediate,
        ),
        "scientific:phys_heat_1d" => (
            "Heat diffusion",
            "Diffuse a 1D temperature field forward in time.",
            Proficiency::Intermediate,
        ),
        "scientific:phys_wave_1d" => (
            "Wave equation",
            "Evolve a 1D wave from displacement (and optional velocity).",
            Proficiency::Intermediate,
        ),
        "scientific:phys_advection_1d" => (
            "Advection–diffusion",
            "Advance a 1D scalar field with advection and diffusion.",
            Proficiency::Intermediate,
        ),
        "scientific:phys_quantum_1d" => (
            "Quantum states",
            "Solve a classical 1D Schrödinger eigenproblem on a potential.",
            Proficiency::Expert,
        ),
        "scientific:phys_n_body" => (
            "N-body gravity",
            "Integrate 2D Newtonian N-body gravitation from masses and state.",
            Proficiency::Intermediate,
        ),
        "scientific:phys_molecular_dynamics" => (
            "Molecular dynamics",
            "Run 2D Lennard-Jones particles with velocity-Verlet.",
            Proficiency::Intermediate,
        ),
        "scientific:phys_emf_interference" => (
            "EMF interference",
            "Superpose EMF sources at a 3D observation point.",
            Proficiency::Intermediate,
        ),
        "scientific:phys_emf_field_grid" => (
            "EMF field grid",
            "Build a 4D EMF physics grid (x×y×z×t) with manifold tags.",
            Proficiency::Expert,
        ),
        "scientific:phys_emf_sample_depth" => (
            "EMF at depth",
            "Sample an EMF field along a camera ray at chosen depths.",
            Proficiency::Intermediate,
        ),
        "scientific:phys_field_sample" => (
            "Field sample",
            "Sample an ambient field value at a 3D position.",
            Proficiency::Novice,
        ),
        "scientific:phys_material_query" => (
            "Material query",
            "Look up faceted mechanical, optical, and chemical traits.",
            Proficiency::Novice,
        ),
        "scientific:phys_evaluate_interaction" => (
            "Evaluate interaction",
            "Apply ambient field laws to a material continuant.",
            Proficiency::Intermediate,
        ),
        "scientific:cosmic_geodetic_distance" => (
            "Geodetic distance",
            "Great-circle distance between two latitude/longitude points.",
            Proficiency::Novice,
        ),
        "scientific:cosmic_surface_gravity" => (
            "Surface gravity",
            "Surface gravity for named Solar System bodies.",
            Proficiency::Novice,
        ),
        "scientific:cosmic_flrw_distance" => (
            "FLRW distance",
            "Comoving distance from redshift in a flat FLRW sketch.",
            Proficiency::Intermediate,
        ),
        "scientific:cosmic_flrw_redshift" => (
            "FLRW redshift",
            "Redshift from scale factor at emission.",
            Proficiency::Intermediate,
        ),
        "scientific:cosmic_flrw_hubble" => (
            "Hubble velocity",
            "Hubble-flow velocity for a distance.",
            Proficiency::Novice,
        ),
        "scientific:cosmic_warp_velocity" => (
            "Warp velocity",
            "Warp-factor velocity on TOS/TNG scales.",
            Proficiency::Intermediate,
        ),
        "scientific:cosmic_warp_factor_c" => (
            "Warp factor c",
            "Dimensionless warp velocity in units of c.",
            Proficiency::Intermediate,
        ),
        "scientific:cosmic_typical_length" => (
            "Typical length",
            "Typical length scale for a hierarchy level.",
            Proficiency::Novice,
        ),
        "scientific:cosmic_observe_redshift" => (
            "Observe redshift",
            "FLRW observation record from measured redshift.",
            Proficiency::Intermediate,
        ),
        "scientific:cosmic_compton" => (
            "Compton wavelength",
            "Compton wavelength for electron or proton.",
            Proficiency::Novice,
        ),
        "scientific:cosmic_de_broglie" => (
            "de Broglie wavelength",
            "de Broglie wavelength from particle and velocity.",
            Proficiency::Novice,
        ),
        "scientific:cosmic_atm_pressure" => (
            "Atmosphere pressure",
            "Atmospheric pressure at altitude for earth/mars/venus.",
            Proficiency::Novice,
        ),
        "scientific:cosmic_geodetic_to_ecef" => (
            "Geodetic to ECEF",
            "WGS84 geodetic to Earth-centered Earth-fixed metres.",
            Proficiency::Novice,
        ),
        "scientific:cosmic_ecef_to_geodetic" => (
            "ECEF to geodetic",
            "ECEF metres back to WGS84 latitude/longitude/altitude.",
            Proficiency::Novice,
        ),
        "scientific:cosmic_ecef_to_enu" => (
            "ECEF to ENU",
            "ECEF to local East-North-Up about a reference point.",
            Proficiency::Intermediate,
        ),
        "scientific:cosmic_enu_to_ecef" => (
            "ENU to ECEF",
            "Local East-North-Up to ECEF about a reference point.",
            Proficiency::Intermediate,
        ),
        "scientific:cosmic_body_profile" => (
            "Body profile",
            "Radius, mass, and class for a named celestial body.",
            Proficiency::Novice,
        ),
        "scientific:cosmic_stardate" => (
            "Stardate to Gregorian",
            "Convert a stardate to approximate Gregorian year.",
            Proficiency::Novice,
        ),
        "scientific:cosmic_cochrane" => (
            "Cochrane units",
            "Warp-field Cochrane units (dimensionless v/c).",
            Proficiency::Intermediate,
        ),
        "scientific:cosmic_atm_temperature" => (
            "Atmosphere temperature",
            "Atmospheric temperature at altitude for earth/mars/venus.",
            Proficiency::Novice,
        ),
        "scientific:cosmic_magnetosphere" => (
            "Magnetosphere field",
            "Dipole magnetosphere field strength at distance.",
            Proficiency::Intermediate,
        ),
        "scientific:cosmic_scale_factor" => (
            "Scale factor",
            "Hierarchy scale factor between two levels.",
            Proficiency::Novice,
        ),
        "scientific:cosmic_usri_parse" => (
            "Parse USRI",
            "Parse a Universal Spacetime & Reality Identifier.",
            Proficiency::Intermediate,
        ),
        "rights:authors_group" => ("Authors", "Open the authors group.", Proficiency::Novice),
        "rights:fiduciary_sign" => (
            "Sign in trust",
            "Sign as a trustee. Needs identity, consent, and an unlocked key.",
            Proficiency::Expert,
        ),
        "rights:did_sign" => (
            "Sign as yourself",
            "Sign with your identity. Needs an unlocked key.",
            Proficiency::Expert,
        ),
        "rights:deontic_obligate" => (
            "Mark as a duty",
            "Say this page records a duty.",
            Proficiency::Intermediate,
        ),
        "health:place_health_overview" => (
            "Health overview",
            "Put your health overview. You stay in control of it.",
            Proficiency::Novice,
        ),
        "health:place_health_documents" => (
            "Health papers",
            "Put a place for health papers you choose to keep.",
            Proficiency::Novice,
        ),
        "health:place_disclosure_log" => (
            "Share log",
            "Put a log of what you have shared, and with whom.",
            Proficiency::Intermediate,
        ),
        "health:place_conditions" => (
            "Conditions",
            "Put a page of conditions that belong to you.",
            Proficiency::Novice,
        ),
        "health:place_health" => (
            "Health vault",
            "Put a locked place for health records.",
            Proficiency::Novice,
        ),
        "health:place_health_calculators" => (
            "Clinical calculators",
            "Put the Framingham, CHA₂DS₂-VASc, and SCORE2 forms. Fields start empty.",
            Proficiency::Intermediate,
        ),
        "health:place_chemical_explorer" => (
            "Compound evidence",
            "Put the food/compound explorer. Research evidence only; import ChEBI compounds.tsv locally — no remote fetch.",
            Proficiency::Intermediate,
        ),
        "health:anatomy_10d" => (
            "Body map",
            "Put a detailed body map.",
            Proficiency::Intermediate,
        ),
        "health:pathology" => (
            "Lab result",
            "Read a lab result. Needs consent and the right numbers.",
            Proficiency::Expert,
        ),
        "health:framingham" => (
            "Heart-risk estimate",
            "Opens the Framingham form. ClinicalRisk.framingham runs only after age, sex, lipids, blood pressure, and the yes/no questions are entered. The result is not a diagnosis.",
            Proficiency::Expert,
        ),
        "health:cha2ds2" => (
            "Stroke-risk estimate",
            "Opens the CHA₂DS₂-VASc form. ClinicalRisk.cha2ds2_vasc applies only when atrial fibrillation is present. The result is not a diagnosis.",
            Proficiency::Expert,
        ),
        "health:score2" => (
            "European heart-risk estimate",
            "Opens the SCORE2 form. ClinicalRisk.score2 needs a named European risk region. The result is not a diagnosis.",
            Proficiency::Expert,
        ),
        "code:place_vibe" => (
            "Script cell",
            "Put a cell for a small script.",
            Proficiency::Intermediate,
        ),
        "code:vibe_diagnose" => (
            "Check the script",
            "Find mistakes in the selected script without running it.",
            Proficiency::Intermediate,
        ),
        "code:quin_statement" => (
            "Link three names",
            "Store a who–relates-to–what note on this page.",
            Proficiency::Expert,
        ),
        "ai:triad" => (
            "Three-part view",
            "Put a view that holds script, picture, and sound together.",
            Proficiency::Intermediate,
        ),
        "ai:extractor" => (
            "Pick out names",
            "Find names and short phrases in the selected writing.",
            Proficiency::Novice,
        ),
        "ai:sentinel" => (
            "Safety look",
            "Check this surface for obvious safety problems.",
            Proficiency::Intermediate,
        ),
        "ai:grounding" => (
            "Check the sources",
            "See whether this writing is tied to records you already have.",
            Proficiency::Intermediate,
        ),
        "ai:detect_ungrounded" => (
            "Find floating claims",
            "Flag writing that has no clear record behind it.",
            Proficiency::Intermediate,
        ),
        "ai:verify_turn" => (
            "Check this answer",
            "Run a post-turn check on the selected generation.",
            Proficiency::Intermediate,
        ),
        "ai:inf_relu" => (
            "ReLU",
            "Clamp negatives to zero on surface numbers (Host Inference.relu).",
            Proficiency::Intermediate,
        ),
        "ai:inf_sigmoid" => (
            "Sigmoid",
            "Map surface numbers through the logistic sigmoid.",
            Proficiency::Intermediate,
        ),
        "ai:inf_gelu" => (
            "GELU",
            "Apply GELU (tanh approximation) to surface numbers.",
            Proficiency::Intermediate,
        ),
        "ai:inf_softmax" => (
            "Softmax",
            "Turn surface numbers into a probability distribution.",
            Proficiency::Intermediate,
        ),
        "ai:inf_rms_norm" => (
            "RMS norm",
            "RMS-normalize surface numbers with weight (data-weight / data-eps).",
            Proficiency::Intermediate,
        ),
        "ai:inf_embed" => (
            "Embed text",
            "Embed selected text into a fixed vector (Host Inference.embed).",
            Proficiency::Intermediate,
        ),
        "ai:inf_run_classifier" => (
            "Run classifier",
            "Classify a query from surface features (default knn; data-method/n/p/k).",
            Proficiency::Intermediate,
        ),
        "ai:inf_vector_search" => (
            "Vector search",
            "Search corpus lines for nearest neighbours (data-query / data-k).",
            Proficiency::Intermediate,
        ),
        "ai:inf_load_model" => (
            "Load model",
            "Mount a resident GGUF from path (Host Inference.load_model; native).",
            Proficiency::Expert,
        ),
        "ai:inf_unload_model" => (
            "Unload model",
            "Drop the resident model mmap via Inference.unload_model.",
            Proficiency::Intermediate,
        ),
        "ai:inf_run_transformer" => (
            "Run transformer",
            "Forward-pass token ids through a resident model (needs load_model).",
            Proficiency::Expert,
        ),
        "ai:inf_run_reranker" => (
            "Rerank candidates",
            "Rank candidate lines by relevance to a query (data-query).",
            Proficiency::Intermediate,
        ),
        "ai:inf_constrained_decode" => (
            "Constrained decode",
            "Mask logits to an allowed vocab via Inference.constrained_decode.",
            Proficiency::Expert,
        ),
        "ai:orch_session_create" => (
            "Session create",
            "Create an orchestration session from surface tokens.",
            Proficiency::Intermediate,
        ),
        "ai:orch_session_plan" => (
            "Session plan",
            "Plan the selected orchestration session's task.",
            Proficiency::Intermediate,
        ),
        "ai:orch_session_execute" => (
            "Session execute",
            "Execute the planned orchestration session DAG.",
            Proficiency::Intermediate,
        ),
        "ai:orch_session_status" => (
            "Session status",
            "Query orchestration session status and summary.",
            Proficiency::Novice,
        ),
        "ai:orch_roster_register" => (
            "Roster register",
            "Register an agent on the session roster.",
            Proficiency::Intermediate,
        ),
        "ai:orch_roster_list" => (
            "Roster list",
            "List agents registered on the session roster.",
            Proficiency::Novice,
        ),
        "ai:orch_roster_capabilities" => (
            "Roster capabilities",
            "List all capabilities on the session roster.",
            Proficiency::Novice,
        ),
        "ai:orch_assign_agents" => (
            "Assign agents",
            "Assign roster agents to planned orchestration steps.",
            Proficiency::Intermediate,
        ),
        "epistemic:evaluate" => (
            "Scan what is known",
            "Read knows and believes claims from the live graph.",
            Proficiency::Intermediate,
        ),
        "epistemic:paraconsistent_route" => (
            "Route contradictions",
            "Isolate conflicting claims without stopping the rest of the page.",
            Proficiency::Intermediate,
        ),
        "code:ltl_evaluate" => (
            "Check over time",
            "See whether a property holds across a sequence of steps.",
            Proficiency::Intermediate,
        ),
        "code:symbolic_eval" => (
            "Work out the formula",
            "Evaluate the formula on the selected surface.",
            Proficiency::Intermediate,
        ),
        "code:symbolic_differentiate" => (
            "Differentiate",
            "Take the derivative of the formula on this surface.",
            Proficiency::Intermediate,
        ),
        "code:symbolic_simplify" => (
            "Simplify",
            "Fold constants and clear identities in the formula.",
            Proficiency::Novice,
        ),
        "code:symbolic_expand" => (
            "Expand",
            "Distribute products and expand small powers in the formula.",
            Proficiency::Intermediate,
        ),
        "code:symbolic_factor" => (
            "Factor quadratic",
            "Factor a real quadratic from coefficients on this surface.",
            Proficiency::Intermediate,
        ),
        "code:symbolic_integrate" => (
            "Integrate",
            "Find an antiderivative of the formula on this surface.",
            Proficiency::Intermediate,
        ),
        "code:symbolic_simplify_trig" => (
            "Simplify trig",
            "Rewrite trig identities in the formula on this surface.",
            Proficiency::Intermediate,
        ),
        "code:symbolic_partial" => (
            "Partial derivative",
            "Take a partial derivative of the formula on this surface.",
            Proficiency::Expert,
        ),
        "code:symbolic_limit" => (
            "Limit",
            "Evaluate the limit of the formula as the variable approaches a point.",
            Proficiency::Expert,
        ),
        "code:symbolic_add" => (
            "Add",
            "Build an addition node from two expressions on this surface.",
            Proficiency::Novice,
        ),
        "code:symbolic_sub" => (
            "Subtract",
            "Build a subtraction node from two expressions on this surface.",
            Proficiency::Novice,
        ),
        "code:symbolic_mul" => (
            "Multiply",
            "Build a multiplication node from two expressions on this surface.",
            Proficiency::Novice,
        ),
        "code:symbolic_div" => (
            "Divide",
            "Build a division node from two expressions on this surface.",
            Proficiency::Novice,
        ),
        "code:symbolic_pow" => (
            "Power",
            "Raise the formula to an integer power on this surface.",
            Proficiency::Intermediate,
        ),
        "code:symbolic_neg" => (
            "Negate",
            "Apply unary minus to the formula on this surface.",
            Proficiency::Novice,
        ),
        "code:symbolic_sqrt" => (
            "Square root",
            "Build a square-root node from the formula on this surface.",
            Proficiency::Novice,
        ),
        "code:symbolic_exp" => (
            "Exponential",
            "Build an exponential node from the formula on this surface.",
            Proficiency::Intermediate,
        ),
        "code:symbolic_ln" => (
            "Natural log",
            "Build a natural-log node from the formula on this surface.",
            Proficiency::Intermediate,
        ),
        "code:symbolic_sin" => (
            "Sine",
            "Build a sine node from the formula on this surface.",
            Proficiency::Novice,
        ),
        "code:symbolic_cos" => (
            "Cosine",
            "Build a cosine node from the formula on this surface.",
            Proficiency::Novice,
        ),
        "code:symbolic_tan" => (
            "Tangent",
            "Build a tangent node from the formula on this surface.",
            Proficiency::Intermediate,
        ),
        "code:symbolic_parse" => (
            "Parse formula",
            "Parse the formula on this surface into canonical form.",
            Proficiency::Novice,
        ),
        "code:symbolic_c" => (
            "Constant",
            "Build a constant leaf from data-value on this surface.",
            Proficiency::Novice,
        ),
        "code:symbolic_var" => (
            "Variable",
            "Build a variable leaf from data-name or data-var on this surface.",
            Proficiency::Novice,
        ),
        "code:symbolic_hessian" => (
            "Hessian",
            "Build the symbolic Hessian of the formula over the surface variables.",
            Proficiency::Expert,
        ),
        "code:symbolic_integrate_definite" => (
            "Definite integral",
            "Integrate the formula between bounds on this surface.",
            Proficiency::Intermediate,
        ),
        "code:symbolic_limit_at_infinity" => (
            "Limit at infinity",
            "Probe the limit of the formula as the variable goes to infinity.",
            Proficiency::Expert,
        ),
        "code:symbolic_real_roots" => (
            "Real roots",
            "Find real roots of the polynomial coefficients on this surface.",
            Proficiency::Intermediate,
        ),
        "code:symbolic_roots" => (
            "Complex roots",
            "Find complex roots of the polynomial coefficients on this surface.",
            Proficiency::Expert,
        ),
        "code:symbolic_taylor_coefficients" => (
            "Taylor coefficients",
            "Expand the formula as Taylor coefficients about a center on this surface.",
            Proficiency::Expert,
        ),
        "code:symbolic_taylor_eval" => (
            "Taylor evaluate",
            "Evaluate a truncated Taylor series from coefficients on this surface.",
            Proficiency::Intermediate,
        ),
        "code:symbolic_jacobian" => (
            "Jacobian",
            "Build the symbolic Jacobian of the expression list on this surface.",
            Proficiency::Expert,
        ),
        "code:symbolic_gradient_at" => (
            "Gradient at point",
            "Evaluate the numeric gradient of the formula at a point on this surface.",
            Proficiency::Expert,
        ),
        "econ:capm" => (
            "Expected return",
            "Estimate return from risk-free rate, beta, and market premium on this surface.",
            Proficiency::Intermediate,
        ),
        "econ:gini" => (
            "Income inequality",
            "Measure inequality of the income numbers on this table or page.",
            Proficiency::Intermediate,
        ),
        "econ:mixed_nash" => (
            "Game balance",
            "Find a mixed-strategy balance from two 2-by-2 payoff tables.",
            Proficiency::Expert,
        ),
        "econ:black_scholes" => (
            "Option price",
            "Price a call or put from spot, strike, time, rate, and volatility.",
            Proficiency::Expert,
        ),
        "econ:solow" => (
            "Steady growth",
            "Compute steady-state capital and output from savings and depreciation.",
            Proficiency::Expert,
        ),
        "econ:cournot" => (
            "Quantity race",
            "Find Cournot quantities and price from demand and two firm costs.",
            Proficiency::Expert,
        ),
        "econ:bertrand" => (
            "Price race",
            "Find Bertrand equilibrium price from two marginal costs.",
            Proficiency::Expert,
        ),
        "econ:historical_var" => (
            "Tail loss",
            "Estimate historical value-at-risk from return numbers on this surface.",
            Proficiency::Expert,
        ),
        "econ:atkinson" => (
            "Welfare gap",
            "Measure Atkinson inequality from positive incomes and aversion epsilon.",
            Proficiency::Expert,
        ),
        "econ:gordon_growth" => (
            "Dividend price",
            "Price a stock from next dividend, required return, and perpetual growth.",
            Proficiency::Intermediate,
        ),
        "econ:binomial_option" => (
            "Tree option",
            "Price a European option with a binomial tree from spot, strike, and volatility.",
            Proficiency::Expert,
        ),
        "econ:forward_rate" => (
            "Forward rate",
            "Compute an annualized forward between two tenors from a zero curve.",
            Proficiency::Expert,
        ),
        "econ:gbm_simulate" => (
            "Price path",
            "Simulate a geometric Brownian motion path from starting price and volatility.",
            Proficiency::Expert,
        ),
        "econ:headcount_poverty" => (
            "Poverty share",
            "Count incomes below a poverty line and report the headcount rate.",
            Proficiency::Intermediate,
        ),
        "econ:hyperbolic_discount" => (
            "Present bias",
            "Compute a β-δ hyperbolic discount factor for a chosen horizon.",
            Proficiency::Expert,
        ),
        "econ:fiscal_multiplier" => (
            "Spending impact",
            "Estimate GDP impact from fiscal spending, MPC, and leakage.",
            Proficiency::Intermediate,
        ),
        "econ:drawdown" => (
            "Peak loss",
            "Measure drawdowns along a wealth index on this surface.",
            Proficiency::Intermediate,
        ),
        "econ:covariance_matrix" => (
            "Return cov",
            "Estimate a sample covariance matrix from period-by-asset returns.",
            Proficiency::Expert,
        ),
        "econ:capm_beta" => (
            "Asset beta",
            "Estimate CAPM beta from paired asset and market returns on this surface.",
            Proficiency::Intermediate,
        ),
        "econ:autocorrelation" => (
            "Series lag",
            "Measure lag autocorrelation of the numbers on this surface.",
            Proficiency::Intermediate,
        ),
        "econ:cross_correlation" => (
            "Series link",
            "Measure cross-correlation between two equal-length series on this surface.",
            Proficiency::Expert,
        ),
        "econ:bertrand_with_demand" => (
            "Price and qty",
            "Find Bertrand price and quantity from linear demand and two costs.",
            Proficiency::Expert,
        ),
        "econ:check_budget_balance" => (
            "Payment surplus",
            "Check whether listed mechanism payments sum to a non-negative surplus.",
            Proficiency::Intermediate,
        ),
        "econ:ccapm_equity_premium" => (
            "Equity premium",
            "Estimate a consumption-CAPM equity premium from risk aversion and volatilities.",
            Proficiency::Expert,
        ),
        "econ:mean_return" => (
            "Average return",
            "Compute the arithmetic mean of return numbers on this surface.",
            Proficiency::Novice,
        ),
        "econ:poverty_gap" => (
            "Gap ratio",
            "Measure average shortfall below a poverty line as a share of the line.",
            Proficiency::Intermediate,
        ),
        "econ:sample_variance" => (
            "Return variance",
            "Estimate sample variance of return numbers on this surface.",
            Proficiency::Novice,
        ),
        "econ:utilitarian_welfare" => (
            "Total utility",
            "Sum utilities from the numbers on this sheet or page.",
            Proficiency::Novice,
        ),
        "econ:rawlsian_welfare" => (
            "Worst-off utility",
            "Report the minimum utility among the numbers on this surface.",
            Proficiency::Intermediate,
        ),
        "econ:nash_welfare" => (
            "Nash product",
            "Multiply strictly positive utilities into a Nash welfare score.",
            Proficiency::Intermediate,
        ),
        "econ:stackelberg" => (
            "Leader quantities",
            "Solve Stackelberg leader and follower quantities from demand and costs.",
            Proficiency::Expert,
        ),
        "econ:put_call_parity" => (
            "Parity residual",
            "Check put-call parity from call, put, spot, strike, and rates.",
            Proficiency::Expert,
        ),
        "econ:parametric_var" => (
            "Gaussian VaR",
            "Estimate parametric value-at-risk from mean, volatility, and confidence.",
            Proficiency::Expert,
        ),
        "econ:laffer_curve" => (
            "Tax revenue",
            "Estimate Laffer-curve revenue from tax rate, base, and elasticity.",
            Proficiency::Intermediate,
        ),
        "econ:historical_cvar" => (
            "Tail average",
            "Estimate historical expected shortfall from return numbers on this surface.",
            Proficiency::Expert,
        ),
        "econ:endowment_effect" => (
            "WTA markup",
            "Compute willingness-to-accept from WTP and loss-aversion lambda.",
            Proficiency::Intermediate,
        ),
        "econ:prospect_value" => (
            "Prospect value",
            "Score an outcome with Kahneman–Tversky prospect-theory parameters.",
            Proficiency::Intermediate,
        ),
        "econ:probability_weight" => (
            "Prelec weight",
            "Transform a probability with the Prelec weighting function.",
            Proficiency::Intermediate,
        ),
        "econ:ccapm_sdf" => (
            "SDF factor",
            "Compute the consumption-CAPM stochastic discount factor.",
            Proficiency::Expert,
        ),
        "econ:gravity_flow" => (
            "Gravity flow",
            "Estimate spatial gravity flow between two masses and a distance.",
            Proficiency::Intermediate,
        ),
        "econ:transfer_payment" => (
            "Means-tested aid",
            "Compute a means-tested transfer from base, income, and phaseout.",
            Proficiency::Intermediate,
        ),
        "econ:efficiency_units" => (
            "Effective labor",
            "Multiply raw labor hours by human capital into efficiency units.",
            Proficiency::Novice,
        ),
        "econ:social_cost_of_carbon" => (
            "Carbon cost",
            "Multiply emissions by damage per ton for social cost of carbon.",
            Proficiency::Intermediate,
        ),
        "econ:pollution_damage" => (
            "Pollution damage",
            "Estimate quadratic pollution damage from emissions and a coefficient.",
            Proficiency::Intermediate,
        ),
        "econ:marginal_damage" => (
            "Marginal damage",
            "Estimate linear marginal pollution damage from emissions.",
            Proficiency::Intermediate,
        ),
        "econ:ramsey_steady_state" => (
            "Ramsey capital",
            "Solve Ramsey steady-state capital from α, β, and depreciation.",
            Proficiency::Expert,
        ),
        "econ:simple_returns" => (
            "Simple returns",
            "Convert a positive price series into period simple returns.",
            Proficiency::Novice,
        ),
        "econ:log_returns" => (
            "Log returns",
            "Convert a positive price series into log returns.",
            Proficiency::Novice,
        ),
        "econ:rolling_mean" => (
            "Rolling mean",
            "Compute a rolling mean over a chosen window on this surface.",
            Proficiency::Intermediate,
        ),
        "econ:rolling_variance" => (
            "Rolling variance",
            "Compute rolling population variance over a chosen window.",
            Proficiency::Intermediate,
        ),
        "econ:labor_supply" => (
            "Labor hours",
            "Solve Cobb–Douglas labor supply and consumption from wage params.",
            Proficiency::Intermediate,
        ),
        "econ:optimal_abatement" => (
            "Abate optimally",
            "Solve optimal abatement from baseline emissions and MAC/MD coeffs.",
            Proficiency::Expert,
        ),
        "econ:optimal_pollution" => (
            "Pollution optimum",
            "Solve optimal emissions from baseline and abatement/damage coeffs.",
            Proficiency::Expert,
        ),
        "econ:olg_steady_state" => (
            "OLG capital",
            "Solve overlapping-generations steady-state capital and output.",
            Proficiency::Expert,
        ),
        "econ:ramsey_euler_residual" => (
            "Euler residual",
            "Evaluate the Ramsey Euler residual on a capital path.",
            Proficiency::Expert,
        ),
        "econ:present_biased_utility" => (
            "Present bias",
            "Discount a utility path with β-δ present-biased preferences.",
            Proficiency::Intermediate,
        ),
        "econ:reference_dependent_utility" => (
            "Reference utility",
            "Score an outcome relative to a reference with prospect parameters.",
            Proficiency::Intermediate,
        ),
        "econ:npv" => (
            "Net present value",
            "Discount paired benefit and cost streams to a present value.",
            Proficiency::Intermediate,
        ),
        "econ:multi_period_ddm" => (
            "Multi-period DDM",
            "Price a dividend path with a Gordon growth terminal value.",
            Proficiency::Intermediate,
        ),
        "econ:portfolio_max_drawdown" => (
            "Max drawdown",
            "Measure peak-to-trough drawdown from portfolio returns on this surface.",
            Proficiency::Intermediate,
        ),
        "econ:interpolate_zero_rate" => (
            "Zero interpolate",
            "Interpolate a zero rate at a chosen maturity on the surface curve.",
            Proficiency::Intermediate,
        ),
        "econ:discount_factor" => (
            "Discount factor",
            "Convert an interpolated zero curve into a discount factor.",
            Proficiency::Intermediate,
        ),
        "econ:par_yield" => (
            "Par yield",
            "Compute the par coupon implied by a zero curve at a maturity.",
            Proficiency::Expert,
        ),
        "econ:progressive_tax" => (
            "Progressive tax",
            "Compute total tax and effective rate from brackets and income.",
            Proficiency::Intermediate,
        ),
        "econ:abatement_net_benefit" => (
            "Abatement benefit",
            "Net social benefit of cutting emissions below a baseline path.",
            Proficiency::Expert,
        ),
        "econ:household_production_ces" => (
            "Household CES",
            "CES household production from time, market goods, alpha, and rho.",
            Proficiency::Expert,
        ),
        "econ:malfeasance_delta" => (
            "Malfeasance delta",
            "Measure the gap between capital allocated and utility delivered.",
            Proficiency::Intermediate,
        ),
        "econ:portfolio_variance" => (
            "Portfolio variance",
            "Compute portfolio variance from weights and a covariance matrix.",
            Proficiency::Intermediate,
        ),
        "econ:portfolio_returns" => (
            "Portfolio returns",
            "Build a weighted portfolio return series from asset returns.",
            Proficiency::Intermediate,
        ),
        "econ:distributional_npv" => (
            "Distributional NPV",
            "Discount benefits and costs with distributional period weights.",
            Proficiency::Expert,
        ),
        "econ:stress_scenario" => (
            "Stress scenario",
            "Scale selected returns by a uniform shock factor.",
            Proficiency::Intermediate,
        ),
        "econ:repeated_game_payoff" => (
            "Repeated-game payoff",
            "Sum discounted stage payoffs over a finite repeated game.",
            Proficiency::Intermediate,
        ),
        "econ:total_transport_cost" => (
            "Transport cost",
            "Sum flow times distance over an origin–destination matrix.",
            Proficiency::Intermediate,
        ),
        "econ:transition_probability" => (
            "Transition prob",
            "Look up a Markov transition probability P[from,to].",
            Proficiency::Intermediate,
        ),
        "econ:expected_holding_time" => (
            "Holding time",
            "Compute expected sojourn time 1/(1−P_ii) for a Markov state.",
            Proficiency::Intermediate,
        ),
        "econ:check_ir" => (
            "Individual rationality",
            "Check that each payment stays within the agent's valuation.",
            Proficiency::Novice,
        ),
        "econ:vcg_payment" => (
            "VCG payment",
            "Compute the Vickrey second-price payment from valuations.",
            Proficiency::Intermediate,
        ),
        "econ:validate_transition_matrix" => (
            "Validate transition",
            "Check that a Markov transition matrix is row-stochastic.",
            Proficiency::Intermediate,
        ),
        "econ:stationary_distribution" => (
            "Stationary distribution",
            "Compute the stationary distribution of a Markov chain.",
            Proficiency::Expert,
        ),
        "econ:mean_first_passage" => (
            "Mean first passage",
            "Compute mean first-passage times to a chosen Markov state.",
            Proficiency::Expert,
        ),
        "econ:degree_centrality" => (
            "Degree centrality",
            "Compute out-degree centrality from an adjacency matrix.",
            Proficiency::Intermediate,
        ),
        "econ:eigenvector_centrality" => (
            "Eigenvector centrality",
            "Compute eigenvector centrality from an adjacency matrix.",
            Proficiency::Expert,
        ),
        "econ:new_keynesian_solve" => (
            "New Keynesian",
            "Solve one-period New Keynesian output gap, inflation, and rate.",
            Proficiency::Expert,
        ),
        "econ:nearest_facility" => (
            "Nearest facility",
            "Assign each demand point to its nearest facility.",
            Proficiency::Intermediate,
        ),
        "econ:pure_nash_equilibria" => (
            "Pure Nash",
            "Find pure-strategy Nash equilibria of a two-player game.",
            Proficiency::Intermediate,
        ),
        "econ:morans_i" => (
            "Moran's I",
            "Measure spatial autocorrelation with Moran's I.",
            Proficiency::Expert,
        ),
        "econ:strategy_proofness" => (
            "Strategy-proofness",
            "Check whether a 2×2 mechanism is strategy-proof for agent 0.",
            Proficiency::Expert,
        ),
        "econ:lorenz_curve" => (
            "Lorenz curve",
            "Build Lorenz population and income shares from income numbers.",
            Proficiency::Intermediate,
        ),
        "econ:ols" => (
            "OLS regression",
            "Fit ordinary least squares on paired x and y numbers.",
            Proficiency::Intermediate,
        ),
        "econ:wls" => (
            "WLS regression",
            "Fit weighted least squares from x, y, and weight columns.",
            Proficiency::Intermediate,
        ),
        "econ:lucas_asset_price" => (
            "Lucas asset price",
            "Price a Lucas tree from dividend and consumption paths.",
            Proficiency::Expert,
        ),
        "econ:bellman_update" => (
            "Bellman update",
            "Apply one Bellman operator update for a selected MDP state.",
            Proficiency::Expert,
        ),
        "econ:block_bootstrap" => (
            "Block bootstrap",
            "Resample block-bootstrap means from return numbers.",
            Proficiency::Intermediate,
        ),
        "econ:simulate_chain" => (
            "Simulate Markov",
            "Simulate a path on a row-stochastic transition matrix.",
            Proficiency::Intermediate,
        ),
        "econ:value_iteration" => (
            "Value iteration",
            "Solve a small MDP with value-function iteration.",
            Proficiency::Expert,
        ),
        "econ:iv_2sls" => (
            "IV / 2SLS",
            "Estimate two-stage least squares with instruments.",
            Proficiency::Expert,
        ),
        "econ:logistic_mle" => (
            "Logistic MLE",
            "Fit binary logistic regression via Newton–Raphson.",
            Proficiency::Expert,
        ),
        "econ:interbank_clearing" => (
            "Interbank clearing",
            "Clear Eisenberg–Noe payments from interbank exposures and capital.",
            Proficiency::Expert,
        ),
        "econ:leontief_inverse" => (
            "Leontief inverse",
            "Invert a technical-coefficient matrix via the Neumann series.",
            Proficiency::Expert,
        ),
        "econ:output_multipliers" => (
            "Output multipliers",
            "Sum Leontief-inverse columns into sector output multipliers.",
            Proficiency::Intermediate,
        ),
        "econ:agent_based_aggregate_wealth" => (
            "Agent wealth status",
            "Probe Host status for agent-based aggregate wealth.",
            Proficiency::Novice,
        ),
        "econ:validate_scalar_constraint" => (
            "Scalar constraint",
            "Check that a scalar econ value lies inside a min/max band.",
            Proficiency::Novice,
        ),
        "econ:aggregate_paper_fills" => (
            "Paper fills status",
            "Probe Host status for paper-trading fill aggregation.",
            Proficiency::Novice,
        ),
        "ai:ml_mse" => (
            "Mean square error",
            "Measure squared error between paired true and predicted numbers on this surface.",
            Proficiency::Intermediate,
        ),
        "ai:ml_rmse" => (
            "Root mean square",
            "Measure root mean squared error between paired predictions on this surface.",
            Proficiency::Intermediate,
        ),
        "ai:ml_mae" => (
            "Mean abs error",
            "Measure mean absolute error between paired predictions on this surface.",
            Proficiency::Intermediate,
        ),
        "ai:ml_r2" => (
            "R-squared",
            "Score how well predictions explain the true values on this surface.",
            Proficiency::Intermediate,
        ),
        "ai:ml_accuracy" => (
            "Class accuracy",
            "Measure classification accuracy from paired label integers.",
            Proficiency::Intermediate,
        ),
        "ai:ml_ols" => (
            "Linear fit",
            "Fit ordinary least squares from feature and response pairs.",
            Proficiency::Intermediate,
        ),
        "ai:ml_train_test_split" => (
            "Split rows",
            "Make train and test index splits for a chosen sample size.",
            Proficiency::Novice,
        ),
        "ai:ml_kmeans" => (
            "Cluster points",
            "Run k-means on row-major points from this surface.",
            Proficiency::Intermediate,
        ),
        "ai:ml_log_loss" => (
            "Cross-entropy",
            "Score probability predictions against boolean labels.",
            Proficiency::Expert,
        ),
        "ai:ml_bonferroni" => (
            "Correct p-values",
            "Apply Bonferroni correction to p-values on this surface.",
            Proficiency::Expert,
        ),
        "ai:ml_confusion_binary" => (
            "Confusion grid",
            "Build a binary confusion matrix from paired true and predicted labels.",
            Proficiency::Intermediate,
        ),
        "ai:ml_holm" => (
            "Holm correct",
            "Apply Holm–Bonferroni correction to p-values on this surface.",
            Proficiency::Expert,
        ),
        "ai:ml_benjamini_hochberg" => (
            "BH FDR",
            "Apply Benjamini–Hochberg FDR correction to surface p-values.",
            Proficiency::Expert,
        ),
        "ai:ml_ab_test" => (
            "A/B proportions",
            "Run a two-proportion A/B test from conversion counts.",
            Proficiency::Intermediate,
        ),
        "ai:ml_bootstrap_estimate" => (
            "Bootstrap SE",
            "Estimate bootstrap standard error and bias for the sample mean.",
            Proficiency::Expert,
        ),
        "ai:ml_bootstrap_ci" => (
            "Bootstrap CI",
            "Estimate a bootstrap percentile confidence interval for the mean.",
            Proficiency::Expert,
        ),
        "ai:ml_permutation_test" => (
            "Permute groups",
            "Run a two-sample permutation test on paired group halves.",
            Proficiency::Expert,
        ),
        "ai:ml_n_rejected" => (
            "Count rejects",
            "Count how many adjusted p-values reject at the chosen alpha.",
            Proficiency::Intermediate,
        ),
        "ai:ml_power_two_sample" => (
            "Two-sample power",
            "Estimate power of a two-sample test for a given effect size.",
            Proficiency::Expert,
        ),
        "ai:ml_roc_auc" => (
            "ROC AUC",
            "Score ranking quality from paired scores and boolean labels.",
            Proficiency::Expert,
        ),
        "ai:ml_k_fold" => (
            "k-fold splits",
            "Build k-fold train and test index splits for a sample size.",
            Proficiency::Intermediate,
        ),
        "ai:ml_bootstrap_indices" => (
            "Bootstrap draws",
            "Draw a bootstrap resample of row indices of length n.",
            Proficiency::Intermediate,
        ),
        "ai:ml_pca" => (
            "Principal axes",
            "Fit principal components on row-major points from this surface.",
            Proficiency::Expert,
        ),
        "ai:ml_required_sample_size" => (
            "Required n",
            "Estimate required sample size for a two-sample test at target power.",
            Proficiency::Expert,
        ),
        "ai:ml_polynomial_regression" => (
            "Polynomial fit",
            "Fit a degree-d polynomial from x and y halves on this surface.",
            Proficiency::Intermediate,
        ),
        "ai:ml_required_n_two_prop" => (
            "Required n (rates)",
            "Estimate required sample size for a two-proportion test.",
            Proficiency::Expert,
        ),
        "ai:ml_loocv" => (
            "Leave-one-out",
            "Build leave-one-out cross-validation folds for n samples.",
            Proficiency::Intermediate,
        ),
        "ai:ml_transe_score" => (
            "TransE score",
            "Score a knowledge-graph triple from h / r / t embedding thirds.",
            Proficiency::Expert,
        ),
        "ai:ml_distmult_score" => (
            "DistMult score",
            "Score a knowledge-graph triple with DistMult embeddings.",
            Proficiency::Expert,
        ),
        "ai:ml_complex_score" => (
            "ComplEx score",
            "Score a knowledge-graph triple with ComplEx embeddings.",
            Proficiency::Expert,
        ),
        "ai:ml_rotate_score" => (
            "RotatE score",
            "Score a knowledge-graph triple with RotatE embeddings.",
            Proficiency::Expert,
        ),
        "ai:ml_ridge_fit" => (
            "Ridge fit",
            "Fit ridge regression from x and y halves on this surface.",
            Proficiency::Intermediate,
        ),
        "ai:ml_lasso_fit" => (
            "Lasso fit",
            "Fit lasso regression from x and y halves on this surface.",
            Proficiency::Intermediate,
        ),
        "ai:ml_pls_fit" => (
            "PLS fit",
            "Fit PLS1 regression from x and y halves on this surface.",
            Proficiency::Expert,
        ),
        "ai:ml_standard_scaler" => (
            "Standardize",
            "Z-score standardize row-major points from this surface.",
            Proficiency::Intermediate,
        ),
        "ai:ml_kmeans_fit" => (
            "k-means fit",
            "Fit k-means++ clusters on row-major points from this surface.",
            Proficiency::Intermediate,
        ),
        "ai:ml_gmm_fit" => (
            "GMM fit",
            "Fit a diagonal Gaussian mixture on row-major points.",
            Proficiency::Expert,
        ),
        "ai:ml_logistic_fit" => (
            "Logistic fit",
            "Fit logistic regression from x and label halves on this surface.",
            Proficiency::Intermediate,
        ),
        "ai:ml_poisson_fit" => (
            "Poisson fit",
            "Fit Poisson regression from x and count halves on this surface.",
            Proficiency::Expert,
        ),
        "ai:ml_naive_bayes_fit" => (
            "Naive Bayes",
            "Fit Gaussian naive Bayes from x and class-label halves.",
            Proficiency::Intermediate,
        ),
        "ai:ml_knn_fit" => (
            "k-NN fit",
            "Fit a k-nearest-neighbours classifier from x and label halves.",
            Proficiency::Intermediate,
        ),
        "ai:ml_lda_fit" => (
            "LDA fit",
            "Fit linear discriminant analysis from x and label halves.",
            Proficiency::Expert,
        ),
        "ai:ml_pcr_fit" => (
            "PCR fit",
            "Fit principal component regression from x and y halves.",
            Proficiency::Expert,
        ),
        "ai:ml_qda_fit" => (
            "QDA fit",
            "Fit quadratic discriminant analysis from x and label halves.",
            Proficiency::Expert,
        ),
        "ai:ml_multinomial_logistic_fit" => (
            "Multinomial logistic",
            "Fit multinomial logistic regression from x and label halves.",
            Proficiency::Expert,
        ),
        "ai:ml_hierarchical_fit" => (
            "Hierarchical fit",
            "Fit agglomerative hierarchical clustering on row-major points.",
            Proficiency::Expert,
        ),
        "ai:ml_hierarchical_labels" => (
            "Hierarchical labels",
            "Cut a hierarchical dendrogram into k cluster labels.",
            Proficiency::Expert,
        ),
        "ai:ml_bayesian_linear_fit" => (
            "Bayesian linear",
            "Fit Bayesian linear regression from x and y halves.",
            Proficiency::Expert,
        ),
        "ai:ml_decision_tree_regressor" => (
            "Tree regressor",
            "Fit a regression decision tree from x and y halves.",
            Proficiency::Intermediate,
        ),
        "ai:ml_decision_tree_classifier" => (
            "Tree classifier",
            "Fit a classification decision tree from x and label halves.",
            Proficiency::Intermediate,
        ),
        "ai:ml_gp_fit" => (
            "GP fit",
            "Fit a Gaussian process regressor from x and y halves.",
            Proficiency::Expert,
        ),
        "ai:ml_svm_fit" => (
            "SVM fit",
            "Fit a soft-margin SVM from x and boolean-label halves.",
            Proficiency::Expert,
        ),
        "ai:ml_kaplan_meier_fit" => (
            "Kaplan–Meier",
            "Fit a Kaplan–Meier survival curve from time and event halves.",
            Proficiency::Expert,
        ),
        "ai:ml_cox_fit" => (
            "Cox PH",
            "Fit Cox proportional hazards from covariate, time, and event thirds.",
            Proficiency::Expert,
        ),
        "ai:ml_hmm_baum_welch" => (
            "HMM Baum–Welch",
            "Learn HMM parameters by Baum–Welch from discrete observation symbols.",
            Proficiency::Expert,
        ),
        "ai:ml_variational_gaussian_fit" => (
            "Variational Gaussian",
            "Mean-field variational inference for a univariate Gaussian sample.",
            Proficiency::Expert,
        ),
        "ai:ml_mcmc_metropolis" => (
            "MCMC Metropolis",
            "Random-walk Metropolis–Hastings sampling on a standard normal target.",
            Proficiency::Expert,
        ),
        "ai:ml_svm_multiclass_fit" => (
            "Multiclass SVM",
            "Fit a one-vs-rest multiclass SVM from x and label halves.",
            Proficiency::Expert,
        ),
        "ai:ml_som_train" => (
            "SOM train",
            "Train a self-organizing map on row-major points from this surface.",
            Proficiency::Expert,
        ),
        "ai:ml_random_forest_regressor" => (
            "RF regressor",
            "Fit a random forest regressor from x and y halves.",
            Proficiency::Expert,
        ),
        "ai:ml_random_forest_classifier" => (
            "RF classifier",
            "Fit a random forest classifier from x and label halves.",
            Proficiency::Expert,
        ),
        "ai:ml_gradient_boosting_regressor" => (
            "GBM regressor",
            "Fit gradient boosting regression from x and y halves.",
            Proficiency::Expert,
        ),
        "ai:ml_bart_fit" => (
            "BART fit",
            "Fit Bayesian additive regression trees from x and y halves.",
            Proficiency::Expert,
        ),
        "ai:ml_kg_mean_rank" => (
            "KG mean rank",
            "Rank knowledge-graph test triples by mean rank under embeddings.",
            Proficiency::Expert,
        ),
        "ai:ml_kg_mrr" => (
            "KG MRR",
            "Mean reciprocal rank of knowledge-graph test triples.",
            Proficiency::Expert,
        ),
        "ai:ml_kg_hits_at_k" => (
            "KG Hits@k",
            "Hits@k of knowledge-graph test triples under an embedding table.",
            Proficiency::Expert,
        ),
        "ai:ml_kalman_new" => (
            "Kalman step",
            "Run one Kalman filter predict and update cycle.",
            Proficiency::Expert,
        ),
        "ai:ml_factor_graph_marginals" => (
            "Factor-graph BP",
            "Belief-propagation marginals on a compact factor graph.",
            Proficiency::Expert,
        ),
        "ai:ml_al_row_score" => (
            "AL row score",
            "Score one probability row for active-learning uncertainty.",
            Proficiency::Intermediate,
        ),
        "ai:ml_al_score" => (
            "AL score pool",
            "Score a pool of probability rows for active-learning uncertainty.",
            Proficiency::Intermediate,
        ),
        "ai:ml_al_cosine" => (
            "AL cosine",
            "Cosine similarity of two feature vectors from surface halves.",
            Proficiency::Intermediate,
        ),
        "ai:ml_al_rank_informative" => (
            "AL rank informative",
            "Rank a probability pool by active-learning uncertainty.",
            Proficiency::Intermediate,
        ),
        "ai:ml_al_most_informative" => (
            "AL most informative",
            "Pick the most informative sample index from a probability pool.",
            Proficiency::Intermediate,
        ),
        "ai:ml_al_representativeness" => (
            "AL representativeness",
            "Score how representative each feature point is of the pool.",
            Proficiency::Intermediate,
        ),
        "ai:ml_al_information_density" => (
            "AL information density",
            "Combine uncertainty with representativeness for density scores.",
            Proficiency::Expert,
        ),
        "ai:ml_al_rank_by_density" => (
            "AL rank by density",
            "Rank a pool by information-density weighted uncertainty.",
            Proficiency::Expert,
        ),
        "ai:ml_al_vote_entropy" => (
            "AL vote entropy",
            "Measure committee vote entropy over discrete class votes.",
            Proficiency::Intermediate,
        ),
        "ai:ml_al_consensus" => (
            "AL consensus",
            "Average committee probability distributions into a consensus.",
            Proficiency::Intermediate,
        ),
        "ai:ml_al_consensus_entropy" => (
            "AL consensus entropy",
            "Entropy of the query-by-committee consensus distribution.",
            Proficiency::Intermediate,
        ),
        "ai:ml_al_kl_disagreement" => (
            "AL KL disagreement",
            "Mean KL divergence of committee members from consensus.",
            Proficiency::Expert,
        ),
        "ai:ml_al_rank_by_disagreement" => (
            "AL rank by disagreement",
            "Rank samples by committee KL disagreement.",
            Proficiency::Expert,
        ),
        "image:histogram" => (
            "Tones",
            "Show how light and dark this picture is.",
            Proficiency::Novice,
        ),
        "image:equalize_hist" => (
            "Equalize tones",
            "Spread greyscale tones more evenly on this picture.",
            Proficiency::Intermediate,
        ),
        "image:rgb_to_gray" => (
            "Greyscale from colour",
            "Convert RGB pixels on this surface to greyscale.",
            Proficiency::Novice,
        ),
        "image:dhash" => (
            "Difference hash",
            "Compute a perceptual difference hash from greyscale pixels.",
            Proficiency::Intermediate,
        ),
        "image:hamming_distance" => (
            "Hash distance",
            "Count differing bits between two perceptual hashes on this surface.",
            Proficiency::Intermediate,
        ),
        "image:cosine_similarity" => (
            "Embedding similarity",
            "Cosine similarity of two embedding vectors on this surface.",
            Proficiency::Expert,
        ),
        "image:edit_new" => (
            "New image",
            "Create an image document via Image.new.",
            Proficiency::Novice,
        ),
        "image:edit_add_layer" => (
            "Add layer",
            "Add a named layer via Image.add_layer.",
            Proficiency::Novice,
        ),
        "image:edit_remove_layer" => (
            "Remove layer",
            "Remove a layer by index via Image.remove_layer.",
            Proficiency::Novice,
        ),
        "image:edit_set_pixel" => (
            "Set pixel",
            "Set a pixel RGBA via Image.set_pixel.",
            Proficiency::Novice,
        ),
        "image:edit_fill" => (
            "Fill image",
            "Fill the document with RGB via Image.fill.",
            Proficiency::Novice,
        ),
        "image:edit_brush" => (
            "Brush stroke",
            "Apply a brush stroke via Image.brush.",
            Proficiency::Novice,
        ),
        "image:edit_apply_filter" => (
            "Apply filter",
            "Apply a named filter via Image.apply_filter.",
            Proficiency::Novice,
        ),
        "image:edit_set_opacity" => (
            "Set opacity",
            "Set layer opacity via Image.set_opacity.",
            Proficiency::Novice,
        ),
        "image:edit_set_blend_mode" => (
            "Blend mode",
            "Set blend mode via Image.set_blend_mode.",
            Proficiency::Novice,
        ),
        "image:edit_set_visible" => (
            "Set visible",
            "Show or hide a layer via Image.set_visible.",
            Proficiency::Novice,
        ),
        "image:edit_set_mask" => (
            "Set mask",
            "Set a rectangular mask via Image.set_mask.",
            Proficiency::Novice,
        ),
        "image:edit_clear_mask" => (
            "Clear mask",
            "Clear the current mask via Image.clear_mask.",
            Proficiency::Novice,
        ),
        "image:edit_composite" => (
            "Composite",
            "Composite layers via Image.composite.",
            Proficiency::Novice,
        ),
        "image:edit_add_selection" => (
            "Add selection",
            "Add a named selection via Image.add_selection.",
            Proficiency::Novice,
        ),
        "image:edit_clear_selections" => (
            "Clear selections",
            "Clear all selections via Image.clear_selections.",
            Proficiency::Novice,
        ),
        "dmx:live_new_universe" => (
            "New universe",
            "Create a 512-channel DMX universe via Dmx.new_universe.",
            Proficiency::Novice,
        ),
        "dmx:live_set_channel" => (
            "Set channel",
            "Set a DMX channel via Dmx.set_channel.",
            Proficiency::Novice,
        ),
        "dmx:live_add_fixture" => (
            "Add fixture",
            "Add a lighting fixture via Dmx.add_fixture.",
            Proficiency::Novice,
        ),
        "dmx:live_fixture_set_colour" => (
            "Fixture colour",
            "Set fixture RGB via Dmx.fixture_set_colour.",
            Proficiency::Novice,
        ),
        "dmx:live_fixture_set_intensity" => (
            "Fixture intensity",
            "Set fixture intensity via Dmx.fixture_set_intensity.",
            Proficiency::Novice,
        ),
        "dmx:live_fixture_set_pan_tilt" => (
            "Fixture pan/tilt",
            "Set fixture pan and tilt via Dmx.fixture_set_pan_tilt.",
            Proficiency::Intermediate,
        ),
        "dmx:live_new_cue" => (
            "New cue",
            "Create a lighting cue via Dmx.new_cue.",
            Proficiency::Novice,
        ),
        "dmx:live_cue_set_channel" => (
            "Cue channel",
            "Set a cue channel via Dmx.cue_set_channel.",
            Proficiency::Novice,
        ),
        "dmx:live_cue_set_fade" => (
            "Cue fade",
            "Set cue fade times via Dmx.cue_set_fade.",
            Proficiency::Novice,
        ),
        "dmx:live_new_cue_stack" => (
            "New cue stack",
            "Create a cue stack via Dmx.new_cue_stack.",
            Proficiency::Novice,
        ),
        "dmx:live_cue_stack_add" => (
            "Stack add cue",
            "Add a cue to a stack via Dmx.cue_stack_add.",
            Proficiency::Novice,
        ),
        "dmx:live_cue_stack_go" => (
            "Stack go",
            "Advance the cue stack via Dmx.cue_stack_go.",
            Proficiency::Novice,
        ),
        "dmx:live_cue_stack_go_back" => (
            "Stack go back",
            "Step the cue stack back via Dmx.cue_stack_go_back.",
            Proficiency::Novice,
        ),
        "dmx:live_cue_stack_reset" => (
            "Stack reset",
            "Reset the cue stack via Dmx.cue_stack_reset.",
            Proficiency::Novice,
        ),
        "video:live_new_project" => (
            "New project",
            "Create a video project via Video.new_project.",
            Proficiency::Novice,
        ),
        "video:live_add_track" => (
            "Add track",
            "Add a named track via Video.add_track.",
            Proficiency::Novice,
        ),
        "video:live_add_clip" => (
            "Add clip",
            "Add a source clip via Video.add_clip.",
            Proficiency::Novice,
        ),
        "video:live_trim_clip" => (
            "Trim clip",
            "Trim clip in/out via Video.trim_clip.",
            Proficiency::Novice,
        ),
        "video:live_set_speed" => (
            "Set speed",
            "Set playback speed via Video.set_speed.",
            Proficiency::Novice,
        ),
        "video:live_colour_grade" => (
            "Colour grade",
            "Grade brightness, contrast, and saturation via Video.colour_grade.",
            Proficiency::Intermediate,
        ),
        "video:live_add_transition" => (
            "Add transition",
            "Add a transition via Video.add_transition.",
            Proficiency::Novice,
        ),
        "video:live_set_render_format" => (
            "Render format",
            "Set the render format via Video.set_render_format.",
            Proficiency::Novice,
        ),
        "video:live_set_render_bitrate" => (
            "Render bitrate",
            "Set the render bitrate via Video.set_render_bitrate.",
            Proficiency::Novice,
        ),
        "video:live_remove_clip" => (
            "Remove clip",
            "Remove a clip via Video.remove_clip.",
            Proficiency::Novice,
        ),
        "hid:live_poll" => (
            "Poll HID",
            "Poll the next HID event via HID.poll.",
            Proficiency::Novice,
        ),
        "hid:live_wait" => (
            "Wait HID",
            "Wait for a HID event via HID.wait.",
            Proficiency::Novice,
        ),
        "hid:live_clear" => (
            "Clear HID",
            "Clear queued HID events via HID.clear.",
            Proficiency::Novice,
        ),
        "hid:live_pointer_capture" => (
            "Pointer capture",
            "Capture pointer focus via HID.pointer_capture.",
            Proficiency::Novice,
        ),
        "hid:live_pointer_release" => (
            "Pointer release",
            "Release pointer capture via HID.pointer_release.",
            Proficiency::Novice,
        ),
        "hid:live_set_cursor" => (
            "Set cursor",
            "Set cursor style via HID.set_cursor.",
            Proficiency::Novice,
        ),
        "hid:live_gamepad_poll" => (
            "Gamepad poll",
            "Poll gamepad state via HID.gamepad_poll.",
            Proficiency::Novice,
        ),
        "hid:live_gamepad_vibrate" => (
            "Gamepad rumble",
            "Dispatch gamepad rumble via HID.gamepad_vibrate.",
            Proficiency::Novice,
        ),
        "hid:live_midi_send" => (
            "MIDI send",
            "Send a MIDI packet via HID.midi_send.",
            Proficiency::Intermediate,
        ),
        "hid:live_midi_poll" => (
            "MIDI poll",
            "Poll incoming MIDI via HID.midi_poll.",
            Proficiency::Novice,
        ),
        "hid:live_haptic_pulse" => (
            "Haptic pulse",
            "Trigger a haptic pulse via HID.haptic_pulse.",
            Proficiency::Novice,
        ),
        "hid:live_haptic_pattern" => (
            "Haptic pattern",
            "Play a haptic pattern via HID.haptic_pattern.",
            Proficiency::Novice,
        ),
        "hid:live_spatial_head_pose" => (
            "Head pose",
            "Read spatial head pose via HID.spatial_head_pose.",
            Proficiency::Intermediate,
        ),
        "hid:live_spatial_hand_skeleton" => (
            "Hand skeleton",
            "Read hand skeleton via HID.spatial_hand_skeleton.",
            Proficiency::Intermediate,
        ),
        "hid:live_spatial_gaze_ray" => (
            "Gaze ray",
            "Read gaze ray via HID.spatial_gaze_ray.",
            Proficiency::Intermediate,
        ),
        "hid:live_biosignal_poll" => (
            "Biosignal poll",
            "Poll privacy-filtered biosignal via HID.biosignal_poll.",
            Proficiency::Expert,
        ),
        "scientific:vc_gradient" => (
            "Gradient",
            "Symbolic gradient via VectorCalculus.gradient.",
            Proficiency::Intermediate,
        ),
        "scientific:vc_divergence" => (
            "Divergence",
            "Vector-field divergence via VectorCalculus.divergence.",
            Proficiency::Intermediate,
        ),
        "scientific:vc_curl" => (
            "Curl",
            "3-component curl via VectorCalculus.curl.",
            Proficiency::Intermediate,
        ),
        "scientific:vc_laplacian" => (
            "Laplacian",
            "Scalar Laplacian via VectorCalculus.laplacian.",
            Proficiency::Intermediate,
        ),
        "scientific:vc_line_integral_scalar" => (
            "Scalar line integral",
            "Scalar line integral via VectorCalculus.line_integral_scalar.",
            Proficiency::Expert,
        ),
        "scientific:vc_line_integral_work" => (
            "Work line integral",
            "Work line integral via VectorCalculus.line_integral_work.",
            Proficiency::Expert,
        ),
        "scientific:vc_surface_flux" => (
            "Surface flux",
            "Surface flux via VectorCalculus.surface_flux.",
            Proficiency::Expert,
        ),
        "scientific:interp_linear" => (
            "Linear interpolate",
            "Piecewise linear sample via Interpolation.linear_interp.",
            Proficiency::Novice,
        ),
        "scientific:interp_lagrange" => (
            "Lagrange evaluate",
            "Lagrange polynomial via Interpolation.lagrange_eval.",
            Proficiency::Intermediate,
        ),
        "scientific:interp_newton_coef" => (
            "Newton coefficients",
            "Divided differences via Interpolation.newton_coefficients.",
            Proficiency::Intermediate,
        ),
        "scientific:interp_newton_eval" => (
            "Newton evaluate",
            "Newton form via Interpolation.newton_eval.",
            Proficiency::Intermediate,
        ),
        "scientific:interp_poly_fit" => (
            "Polynomial fit",
            "Least-squares polynomial via Interpolation.poly_fit.",
            Proficiency::Intermediate,
        ),
        "scientific:interp_poly_eval" => (
            "Polynomial evaluate",
            "Polynomial evaluation via Interpolation.poly_eval.",
            Proficiency::Novice,
        ),
        "scientific:spectral_emf_to_spd" => (
            "EMF to SPD",
            "Emission SPD via Spectral.emf_to_spd.",
            Proficiency::Intermediate,
        ),
        "scientific:spectral_spd_to_xyz" => (
            "SPD to XYZ",
            "CIE XYZ via Spectral.spd_to_xyz.",
            Proficiency::Intermediate,
        ),
        "scientific:spectral_emf_to_rgb" => (
            "EMF to RGB",
            "Display RGB via Spectral.emf_to_rgb.",
            Proficiency::Intermediate,
        ),
        "scientific:spectral_blend" => (
            "Blend spectra",
            "Blend two EMFs via Spectral.blend.",
            Proficiency::Intermediate,
        ),
        "scientific:spectral_gamut_map" => (
            "Gamut map",
            "Map XYZ via Spectral.gamut_map.",
            Proficiency::Expert,
        ),
        "spatial:world_new" => (
            "New world",
            "Create a world via World.new.",
            Proficiency::Novice,
        ),
        "spatial:world_add_object" => (
            "Add object",
            "Add a world object via World.add_object.",
            Proficiency::Novice,
        ),
        "spatial:world_add_portal" => (
            "Add portal",
            "Add a portal via World.add_portal.",
            Proficiency::Intermediate,
        ),
        "spatial:world_add_avatar" => (
            "Add avatar",
            "Add an avatar via World.add_avatar.",
            Proficiency::Intermediate,
        ),
        "spatial:world_set_gravity" => (
            "Set gravity",
            "Set world gravity via World.set_gravity.",
            Proficiency::Novice,
        ),
        "spatial:world_object_apply_force" => (
            "Apply force",
            "Apply object force via World.object_apply_force.",
            Proficiency::Intermediate,
        ),
        "spatial:world_object_step_physics" => (
            "Step physics",
            "Step world physics via World.object_step_physics.",
            Proficiency::Intermediate,
        ),
        "office:asset_create" => (
            "Create asset",
            "Create an aspect-graph record via Asset.create.",
            Proficiency::Novice,
        ),
        "office:asset_add_temporal" => (
            "Add temporal aspect",
            "Add a temporal aspect via Asset.add_temporal.",
            Proficiency::Intermediate,
        ),
        "office:asset_add_topic" => (
            "Add topic",
            "Associate a topic via Asset.add_topic.",
            Proficiency::Novice,
        ),
        "office:asset_set_spatial" => (
            "Set spatial anchor",
            "Set a spatial anchor via Asset.set_spatial.",
            Proficiency::Intermediate,
        ),
        "office:asset_compile" => (
            "Compile asset",
            "Compile an asset to quins via Asset.compile.",
            Proficiency::Intermediate,
        ),
        "office:asset_temporal_span" => (
            "Temporal span",
            "Measure aspect span via Asset.temporal_span.",
            Proficiency::Novice,
        ),
        "office:asset_query_aspects" => (
            "Query aspects",
            "Query temporal aspects via Asset.query_aspects.",
            Proficiency::Novice,
        ),
        "office:asset_persist" => (
            "Persist asset",
            "Persist an asset via Asset.persist.",
            Proficiency::Intermediate,
        ),
        "office:asset_resolve" => (
            "Resolve asset",
            "Resolve an asset by id via Asset.resolve.",
            Proficiency::Novice,
        ),
        "office:asset_resolve_by_spatial" => (
            "Resolve by spatial",
            "Resolve assets by anchor via Asset.resolve_by_spatial.",
            Proficiency::Intermediate,
        ),
        "office:asset_resolve_by_topic" => (
            "Resolve by topic",
            "Resolve assets by topic via Asset.resolve_by_topic.",
            Proficiency::Novice,
        ),
        "office:asset_resolve_by_temporal" => (
            "Resolve by temporal",
            "Resolve assets by aspect kind via Asset.resolve_by_temporal.",
            Proficiency::Intermediate,
        ),
        "office:asset_list" => (
            "List assets",
            "List persisted asset ids via Asset.list.",
            Proficiency::Novice,
        ),
        "office:asset_count" => (
            "Count assets",
            "Count persisted assets via Asset.count.",
            Proficiency::Novice,
        ),
        "office:asset_persist_create" => (
            "Persist create",
            "Create and persist an asset via Asset.persist_create.",
            Proficiency::Intermediate,
        ),
        "office:asset_persist_add_temporal" => (
            "Persist add temporal",
            "Add a temporal aspect via Asset.persist_add_temporal.",
            Proficiency::Intermediate,
        ),
        "office:asset_persist_add_topic" => (
            "Persist add topic",
            "Add a topic via Asset.persist_add_topic.",
            Proficiency::Novice,
        ),
        "office:asset_persist_set_spatial" => (
            "Persist set spatial",
            "Set a spatial anchor via Asset.persist_set_spatial.",
            Proficiency::Intermediate,
        ),
        "office:asset_persist_compile" => (
            "Persist compile",
            "Compile a persisted asset via Asset.persist_compile.",
            Proficiency::Intermediate,
        ),
        "office:asset_persist_temporal_span" => (
            "Persist temporal span",
            "Measure persisted aspect span via Asset.persist_temporal_span.",
            Proficiency::Novice,
        ),
        "office:asset_persist_query_aspects" => (
            "Persist query aspects",
            "Query persisted aspects via Asset.persist_query_aspects.",
            Proficiency::Novice,
        ),
        "comm:pulse_live_publish" => (
            "Publish pulse",
            "Publish a generic pulse via Pulse.publish.",
            Proficiency::Novice,
        ),
        "comm:pulse_live_graph_mutation" => (
            "Publish graph mutation",
            "Publish a graph-mutation pulse via Pulse.publish_graph_mutation.",
            Proficiency::Intermediate,
        ),
        "comm:pulse_live_notification" => (
            "Publish notification",
            "Publish a notification pulse via Pulse.publish_notification.",
            Proficiency::Novice,
        ),
        "comm:pulse_live_telemetry" => (
            "Publish telemetry",
            "Publish a telemetry pulse via Pulse.publish_telemetry.",
            Proficiency::Intermediate,
        ),
        "comm:pulse_live_agent_message" => (
            "Publish agent message",
            "Publish an agent-message pulse via Pulse.publish_agent_message.",
            Proficiency::Intermediate,
        ),
        "comm:pulse_live_sync" => (
            "Publish sync",
            "Publish a sync pulse via Pulse.publish_sync.",
            Proficiency::Novice,
        ),
        "comm:pulse_live_open_channel" => (
            "Open channel",
            "Open a pulse channel via Pulse.open_channel.",
            Proficiency::Intermediate,
        ),
        "comm:pulse_live_close_channel" => (
            "Close channel",
            "Close a pulse channel via Pulse.close_channel.",
            Proficiency::Novice,
        ),
        "comm:pulse_live_set_transport" => (
            "Set transport",
            "Set pulse transport via Pulse.set_transport.",
            Proficiency::Intermediate,
        ),
        "spatial:portal_set_target" => (
            "Portal target",
            "Set a portal target via Portal.set_target.",
            Proficiency::Novice,
        ),
        "spatial:portal_activate" => (
            "Activate portal",
            "Activate a portal via Portal.activate.",
            Proficiency::Novice,
        ),
        "spatial:portal_deactivate" => (
            "Deactivate portal",
            "Deactivate a portal via Portal.deactivate.",
            Proficiency::Novice,
        ),
        "spatial:avatar_move" => (
            "Move avatar",
            "Move an avatar via Avatar.move.",
            Proficiency::Novice,
        ),
        "spatial:avatar_set_appearance" => (
            "Avatar appearance",
            "Set avatar appearance via Avatar.set_appearance.",
            Proficiency::Novice,
        ),
        "research:live_new" => (
            "New enquiry",
            "Start a research enquiry via Research.new.",
            Proficiency::Novice,
        ),
        "research:live_set_purpose" => (
            "Set purpose",
            "Set enquiry purpose via Research.set_purpose.",
            Proficiency::Novice,
        ),
        "research:live_define_scope" => (
            "Define scope",
            "Define enquiry scope lines via Research.define_scope.",
            Proficiency::Novice,
        ),
        "research:live_add_constraint" => (
            "Add constraint",
            "Add a research constraint via Research.add_constraint.",
            Proficiency::Novice,
        ),
        "research:live_add_question" => (
            "Add question",
            "Add a research question via Research.add_question.",
            Proficiency::Novice,
        ),
        "research:live_link_questions" => (
            "Link questions",
            "Link two questions via Research.link_questions.",
            Proficiency::Novice,
        ),
        "research:live_add_corpus_item" => (
            "Add corpus item",
            "Add a corpus item via Research.add_corpus_item.",
            Proficiency::Novice,
        ),
        "research:live_import_literature" => (
            "Import literature",
            "Import literature via Research.import_literature.",
            Proficiency::Novice,
        ),
        "research:live_import_dataset" => (
            "Import dataset",
            "Import a dataset via Research.import_dataset.",
            Proficiency::Novice,
        ),
        "research:live_set_corpus_confidence" => (
            "Corpus confidence",
            "Set corpus confidence via Research.set_corpus_confidence.",
            Proficiency::Intermediate,
        ),
        "research:live_extract_from_corpus" => (
            "Extract from corpus",
            "Extract facts for a keyword via Research.extract_from_corpus.",
            Proficiency::Intermediate,
        ),
        "research:live_infer_dark_link" => (
            "Infer dark link",
            "Infer a dark link via Research.infer_dark_link.",
            Proficiency::Intermediate,
        ),
        "research:live_detect_provenance_gaps" => (
            "Provenance gaps",
            "Detect provenance gaps via Research.detect_provenance_gaps.",
            Proficiency::Intermediate,
        ),
        "research:live_detect_concealment" => (
            "Detect concealment",
            "Detect concealment patterns via Research.detect_concealment.",
            Proficiency::Intermediate,
        ),
        "research:live_confirm_dark_link" => (
            "Confirm dark link",
            "Confirm a dark link via Research.confirm_dark_link.",
            Proficiency::Intermediate,
        ),
        "research:live_refute_dark_link" => (
            "Refute dark link",
            "Refute a dark link via Research.refute_dark_link.",
            Proficiency::Intermediate,
        ),
        "research:live_make_inference" => (
            "Make inference",
            "Record a premise→conclusion via Research.make_inference.",
            Proficiency::Intermediate,
        ),
        "research:live_chain_inference" => (
            "Chain inference",
            "Chain an inference via Research.chain_inference.",
            Proficiency::Intermediate,
        ),
        "research:live_set_inference_confidence" => (
            "Inference confidence",
            "Set inference confidence via Research.set_inference_confidence.",
            Proficiency::Intermediate,
        ),
        "research:live_validate_inference" => (
            "Validate inference",
            "Validate an inference via Research.validate_inference.",
            Proficiency::Intermediate,
        ),
        "scientific:ode_lin1" => (
            "Linear first-order ODE",
            "Solve y' + a·y = b via SymbolicODE.solve_linear_first_order.",
            Proficiency::Intermediate,
        ),
        "scientific:ode_lin2" => (
            "Linear second-order ODE",
            "Solve a·y'' + b·y' + c·y = 0 via SymbolicODE.solve_linear_second_order.",
            Proficiency::Expert,
        ),
        "scientific:ode_classify_pde" => (
            "Classify PDE",
            "Classify a second-order PDE via SymbolicODE.classify_second_order_pde.",
            Proficiency::Expert,
        ),
        "scientific:ode_separable" => (
            "Separable ODE",
            "Solve a separable ODE via SymbolicODE.solve_separable.",
            Proficiency::Expert,
        ),
        "scientific:ode_pde1" => (
            "Linear first-order PDE",
            "Solve a·uₓ + b·u_y = 0 via SymbolicODE.solve_first_order_linear_pde.",
            Proficiency::Expert,
        ),
        "ai:agent_trace" => (
            "Agent trace",
            "Inspect instrument trace via Agent.trace.",
            Proficiency::Intermediate,
        ),
        "ai:agent_verify" => (
            "Agent verify",
            "Verify agent priority via Agent.verify.",
            Proficiency::Intermediate,
        ),
        "ai:agent_plan" => (
            "Agent plan",
            "Plan a task via Agent.plan.",
            Proficiency::Intermediate,
        ),
        "ai:agent_execute" => (
            "Agent execute",
            "Prepare planned execution via Agent.execute.",
            Proficiency::Expert,
        ),
        "ai:agent_evaluate" => (
            "Agent evaluate",
            "Score outputs against expected via Agent.evaluate.",
            Proficiency::Expert,
        ),
        "ai:co_author" => (
            "Write together",
            "Ask the local model to help write. Needs a selected page and a loaded model.",
            Proficiency::Expert,
        ),
        "sdn:place_webrtc" => (
            "Share swarm",
            "Put a page for sharing files with peers.",
            Proficiency::Intermediate,
        ),
        "sdn:place_finance" => (
            "Unit costs",
            "Put a page for shared costs and contributions.",
            Proficiency::Intermediate,
        ),
        "sdn:energy_governor" => (
            "Power budget",
            "Watch battery or solar use. Needs live power readings.",
            Proficiency::Expert,
        ),
        "code:constr_regular_polygon" => (
            "Regular polygon",
            "Check whether a regular n-gon is constructible with compass and straightedge.",
            Proficiency::Intermediate,
        ),
        "code:constr_fermat_prime" => (
            "Fermat prime",
            "Check whether this side count is a Fermat prime.",
            Proficiency::Intermediate,
        ),
        "code:constr_min_poly_degree" => (
            "Wantzel degree",
            "Decide constructibility from the minimal-polynomial degree on this surface.",
            Proficiency::Intermediate,
        ),
        "code:constr_power_of_two" => (
            "Power of two",
            "Check whether this number is a power of two.",
            Proficiency::Novice,
        ),
        "code:constr_central_angle" => (
            "Central angle",
            "Check whether the central angle 2π/n is constructible.",
            Proficiency::Intermediate,
        ),
        "code:constr_doubling_cube" => (
            "Doubling the cube",
            "Ask the classical doubling-the-cube impossibility.",
            Proficiency::Intermediate,
        ),
        "code:constr_trisect_angle" => (
            "Trisect angle",
            "Ask the classical angle-trisection impossibility.",
            Proficiency::Intermediate,
        ),
        "code:constr_square_circle" => (
            "Square the circle",
            "Ask the classical squaring-the-circle impossibility.",
            Proficiency::Intermediate,
        ),
        "code:constr_number" => (
            "Constructible number",
            "Decide whether the formula on this surface denotes a constructible number.",
            Proficiency::Expert,
        ),
        "code:nt_gcd" => (
            "GCD",
            "Greatest common divisor of the two integers on this surface.",
            Proficiency::Novice,
        ),
        "code:nt_lcm" => (
            "LCM",
            "Least common multiple of the two integers on this surface.",
            Proficiency::Novice,
        ),
        "code:nt_is_prime" => (
            "Is prime",
            "Check whether this integer is prime.",
            Proficiency::Novice,
        ),
        "code:nt_factorial" => (
            "Factorial",
            "Compute n! for the integer on this surface.",
            Proficiency::Novice,
        ),
        "code:nt_binomial" => (
            "Binomial",
            "Binomial coefficient C(n, k) from this surface.",
            Proficiency::Intermediate,
        ),
        "code:nt_euler_totient" => (
            "Euler totient",
            "Euler's totient φ(n) for the integer on this surface.",
            Proficiency::Intermediate,
        ),
        "code:nt_mod_pow" => (
            "Mod pow",
            "Raise base^exp modulo this surface's modulus.",
            Proficiency::Intermediate,
        ),
        "code:nt_next_prime" => (
            "Next prime",
            "Find the next prime after this integer.",
            Proficiency::Novice,
        ),
        "code:nt_mod_inverse" => (
            "Mod inverse",
            "Modular multiplicative inverse when it exists.",
            Proficiency::Intermediate,
        ),
        "code:nt_divisor_count" => (
            "Divisor count",
            "Count the positive divisors of this integer.",
            Proficiency::Novice,
        ),
        "code:nt_prime_factors" => (
            "Prime factors",
            "Factor this integer into primes with exponents.",
            Proficiency::Intermediate,
        ),
        "code:nt_divisors" => (
            "Divisors",
            "List every positive divisor of this integer.",
            Proficiency::Novice,
        ),
        "code:nt_mobius" => (
            "Möbius",
            "Evaluate the Möbius function μ(n) on this integer.",
            Proficiency::Intermediate,
        ),
        "code:nt_divisor_sum" => (
            "Divisor sum",
            "Sum the positive divisors σ(n) of this integer.",
            Proficiency::Novice,
        ),
        "code:nt_partitions" => (
            "Partitions",
            "Count integer partitions p(n) for this integer.",
            Proficiency::Intermediate,
        ),
        "code:nt_catalan" => (
            "Catalan",
            "Compute the Catalan number C_n for this integer.",
            Proficiency::Intermediate,
        ),
        "code:nt_stirling_second" => (
            "Stirling 2nd",
            "Stirling number of the second kind S(n, k) from this surface.",
            Proficiency::Expert,
        ),
        "code:nt_stirling_first" => (
            "Stirling 1st",
            "Unsigned Stirling number of the first kind c(n, k).",
            Proficiency::Expert,
        ),
        "code:nt_extended_gcd" => (
            "Extended GCD",
            "Bézout coefficients for the two signed integers on this surface.",
            Proficiency::Intermediate,
        ),
        "code:nt_crt" => (
            "CRT",
            "Chinese Remainder for two congruences with coprime moduli.",
            Proficiency::Expert,
        ),
        "code:fuzzy_triangular" => (
            "Triangular",
            "Triangular membership degree for the numbers on this surface.",
            Proficiency::Novice,
        ),
        "code:fuzzy_trapezoidal" => (
            "Trapezoidal",
            "Trapezoidal membership degree for the numbers on this surface.",
            Proficiency::Novice,
        ),
        "code:fuzzy_approximately" => (
            "Approximately",
            "How close this value is to the target within the given tolerance.",
            Proficiency::Novice,
        ),
        "code:fuzzy_ramp_up" => (
            "Ramp up",
            "Rising ramp membership between the two endpoints on this surface.",
            Proficiency::Novice,
        ),
        "code:fuzzy_ramp_down" => (
            "Ramp down",
            "Falling ramp membership between the two endpoints on this surface.",
            Proficiency::Novice,
        ),
        "code:fuzzy_much_greater_than" => (
            "Much greater",
            "How much greater this value is than the reference, given the spread.",
            Proficiency::Intermediate,
        ),
        "code:fuzzy_much_less_than" => (
            "Much less",
            "How much less this value is than the reference, given the spread.",
            Proficiency::Intermediate,
        ),
        "code:fuzzy_threshold" => (
            "Threshold",
            "Keep only degrees at or above the α-cut on this surface.",
            Proficiency::Intermediate,
        ),
        "code:fuzzy_top_k" => (
            "Top k",
            "Keep the k highest membership degrees on this surface.",
            Proficiency::Intermediate,
        ),
        "code:fuzzy_negate" => (
            "Negate",
            "Complement each degree under the chosen fuzzy norm.",
            Proficiency::Intermediate,
        ),
        "code:fuzzy_and" => (
            "And",
            "Combine two degrees with a t-norm (Gödel, product, or Łukasiewicz).",
            Proficiency::Intermediate,
        ),
        "code:fuzzy_or" => (
            "Or",
            "Combine two degrees with a t-conorm (Gödel, product, or Łukasiewicz).",
            Proficiency::Intermediate,
        ),
        _ => return None,
    };
    Some(Presentation {
        label: label.into(),
        tooltip: tooltip.into(),
        min_proficiency: min,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_copy_avoids_coder_verbs() {
        for id in [
            "graph:sparql_query",
            "n3:evaluate",
            "shacl:validate",
            "code:quin_statement",
            "code:vibe_diagnose",
        ] {
            let copy = named(id).expect(id);
            let blob = format!("{} {}", copy.label, copy.tooltip).to_lowercase();
            assert!(!blob.contains("sparql"), "{id}");
            assert!(!blob.contains("quin.statement"), "{id}");
            assert!(!blob.contains("capability"), "{id}");
            assert!(!blob.contains("n3logic"), "{id}");
        }
    }

    #[test]
    fn quin_statement_is_workshop_only() {
        let copy = named("code:quin_statement").unwrap();
        assert_eq!(copy.min_proficiency, Proficiency::Expert);
    }
}
