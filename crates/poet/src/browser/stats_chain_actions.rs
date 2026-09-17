//! Dual-path Tool Chest actions for curated `Statistics.*` ALL_BOUND ids.
//!
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.
//! Keeps sheet stats growth out of `chain_actions.rs`.

use serde_json::json;
use web_sys::{Document, Element};

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn selected_source(document: &Document) -> Option<String> {
    let container = selected_container(document)?;
    let text = container
        .query_selector(".vibe-editor, .vibe-editor-textarea, .doc-editor, .sheet-grid")
        .ok()
        .flatten()
        .and_then(|editor| editor.text_content())
        .or_else(|| container.text_content())?;
    let bounded: String = text.chars().take(16_384).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn parse_numbers(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(4096)
        .collect()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
}

fn local_mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

fn local_sum(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum())
}

fn local_central_moment(values: &[f64], k: i32) -> Option<f64> {
    let m = local_mean(values)?;
    Some(values.iter().map(|v| (v - m).powi(k)).sum::<f64>() / values.len() as f64)
}

fn local_skewness(values: &[f64]) -> Option<f64> {
    let m2 = local_central_moment(values, 2)?;
    let m3 = local_central_moment(values, 3)?;
    if m2 <= 0.0 {
        return Some(0.0);
    }
    Some(m3 / m2.powf(1.5))
}

fn local_kurtosis(values: &[f64]) -> Option<f64> {
    let m2 = local_central_moment(values, 2)?;
    let m4 = local_central_moment(values, 4)?;
    if m2 <= 0.0 {
        return Some(0.0);
    }
    Some(m4 / (m2 * m2) - 3.0)
}

fn local_quantile(values: &[f64], q: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let n = sorted.len();
    if n == 1 {
        return Some(sorted[0]);
    }
    let q = q.clamp(0.0, 1.0);
    let pos = q * (n - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    let frac = pos - lo as f64;
    Some(sorted[lo] + (sorted[hi] - sorted[lo]) * frac)
}

fn local_iqr(values: &[f64]) -> Option<f64> {
    let q3 = local_quantile(values, 0.75)?;
    let q1 = local_quantile(values, 0.25)?;
    Some(q3 - q1)
}

fn local_mode(values: &[f64]) -> Option<(f64, usize)> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let mut best_val = sorted[0];
    let mut best_count = 1usize;
    let mut cur_val = sorted[0];
    let mut cur_count = 1usize;
    for &v in sorted.iter().skip(1) {
        if v == cur_val {
            cur_count += 1;
        } else {
            cur_val = v;
            cur_count = 1;
        }
        if cur_count > best_count {
            best_count = cur_count;
            best_val = cur_val;
        }
    }
    Some((best_val, best_count))
}

fn local_trimmed_mean(values: &[f64], proportion: f64) -> Option<f64> {
    let n = values.len();
    if n == 0 || !(0.0..0.5).contains(&proportion) {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let cut = (n as f64 * proportion).floor() as usize;
    if 2 * cut >= n {
        return local_quantile(values, 0.5);
    }
    local_mean(&sorted[cut..n - cut])
}

fn local_mad(values: &[f64], scaled: bool) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let med = local_quantile(values, 0.5)?;
    let deviations: Vec<f64> = values.iter().map(|x| (x - med).abs()).collect();
    let mad = local_quantile(&deviations, 0.5)?;
    Some(if scaled {
        1.482_602_218_505_602 * mad
    } else {
        mad
    })
}

/// Split a flat sheet series into two equal halves for bivariate Host ids.
fn split_pair_series(values: &[f64]) -> Option<(Vec<f64>, Vec<f64>)> {
    let half = values.len() / 2;
    if half < 2 {
        return None;
    }
    Some((values[..half].to_vec(), values[half..half * 2].to_vec()))
}

/// Split into three equal thirds for ternary Host ids (p/u/v or p/q★/q).
fn split_triple_series(values: &[f64]) -> Option<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    let third = values.len() / 3;
    if third < 2 {
        return None;
    }
    Some((
        values[..third].to_vec(),
        values[third..2 * third].to_vec(),
        values[2 * third..3 * third].to_vec(),
    ))
}

fn local_pearson(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len();
    if n != y.len() || n < 2 {
        return None;
    }
    let mx = local_mean(x)?;
    let my = local_mean(y)?;
    let mut num = 0.0;
    let mut dx2 = 0.0;
    let mut dy2 = 0.0;
    for i in 0..n {
        let dx = x[i] - mx;
        let dy = y[i] - my;
        num += dx * dy;
        dx2 += dx * dx;
        dy2 += dy * dy;
    }
    if dx2 <= 0.0 || dy2 <= 0.0 {
        return Some(0.0);
    }
    Some(num / (dx2.sqrt() * dy2.sqrt()))
}

fn local_covariance(x: &[f64], y: &[f64], sample: bool) -> Option<f64> {
    let n = x.len();
    if n != y.len() || n == 0 {
        return None;
    }
    let mx = local_mean(x)?;
    let my = local_mean(y)?;
    let acc: f64 = (0..n).map(|i| (x[i] - mx) * (y[i] - my)).sum();
    let denom = if sample { (n - 1) as f64 } else { n as f64 };
    Some(acc / denom)
}

fn local_z_score_outlier_count(values: &[f64], threshold: f64) -> Option<usize> {
    if values.len() < 2 || !(threshold > 0.0) || !threshold.is_finite() {
        return None;
    }
    let n = values.len();
    let mu = values.iter().copied().sum::<f64>() / n as f64;
    let mut ss = 0.0;
    for &x in values {
        let d = x - mu;
        ss += d * d;
    }
    let sd = (ss / (n - 1) as f64).sqrt();
    if !(sd > 0.0) || !sd.is_finite() {
        return None;
    }
    let mut count = 0usize;
    for &x in values {
        if ((x - mu) / sd).abs() > threshold {
            count += 1;
        }
    }
    Some(count)
}

fn local_argmax(values: &[f64]) -> Option<(usize, f64)> {
    if values.is_empty() {
        return None;
    }
    let mut best = 0usize;
    for i in 1..values.len() {
        if values[i] > values[best] {
            best = i;
        }
    }
    Some((best, values[best]))
}

fn local_ln_binom(n: u32, k: u32) -> Option<f64> {
    if k > n {
        return None;
    }
    let mut s = 0.0;
    for i in 0..k {
        s += ((n - i) as f64).ln() - ((i + 1) as f64).ln();
    }
    Some(s)
}

fn local_binomial_pmf(k: u32, n: u32, p: f64) -> Option<f64> {
    if !(0.0..=1.0).contains(&p) || k > n || !p.is_finite() {
        return None;
    }
    if n == 0 {
        return Some(if k == 0 { 1.0 } else { 0.0 });
    }
    if p == 0.0 {
        return Some(if k == 0 { 1.0 } else { 0.0 });
    }
    if p == 1.0 {
        return Some(if k == n { 1.0 } else { 0.0 });
    }
    let ln_c = local_ln_binom(n, k)?;
    let ln_p = (k as f64) * p.ln() + ((n - k) as f64) * (1.0 - p).ln();
    let v = (ln_c + ln_p).exp();
    v.is_finite().then_some(v)
}

fn local_binomial_cdf(k: u32, n: u32, p: f64) -> Option<f64> {
    if !(0.0..=1.0).contains(&p) || !p.is_finite() {
        return None;
    }
    let kk = k.min(n);
    let mut s = 0.0;
    for i in 0..=kk {
        s += local_binomial_pmf(i, n, p)?;
        if !s.is_finite() {
            return None;
        }
    }
    Some(s)
}

/// Stirling / reflection ln Γ sketch for offline Beta / χ² PDFs (not Host special-fn parity).
fn local_ln_gamma_stirling(x: f64) -> Option<f64> {
    if !(x > 0.0) || !x.is_finite() {
        return None;
    }
    // Exact for small positive integers — Stirling is poor near Γ(1)/Γ(2).
    let nearest = x.round();
    if (x - nearest).abs() < 1e-12 && nearest >= 1.0 && nearest <= 32.0 {
        let n = nearest as u32;
        let mut s = 0.0;
        for i in 2..n {
            s += (i as f64).ln();
        }
        return Some(s);
    }
    if x < 0.5 {
        // Γ(x)Γ(1−x) = π / sin(πx) → ln Γ(x) = ln π − ln sin(πx) − ln Γ(1−x)
        let reflection = local_ln_gamma_stirling(1.0 - x)?;
        let sin_term = (std::f64::consts::PI * x).sin();
        if !(sin_term > 0.0) {
            return None;
        }
        return Some(std::f64::consts::PI.ln() - sin_term.ln() - reflection);
    }
    // ln Γ(x) ≈ (x−0.5) ln x − x + 0.5 ln(2π) + 1/(12x)
    let v = (x - 0.5) * x.ln() - x
        + 0.5 * (2.0 * std::f64::consts::PI).ln()
        + 1.0 / (12.0 * x);
    v.is_finite().then_some(v)
}

fn local_beta_pdf(x: f64, alpha: f64, beta: f64) -> Option<f64> {
    if !(x > 0.0 && x < 1.0) || !(alpha > 0.0) || !(beta > 0.0) {
        return Some(0.0);
    }
    let log_b =
        local_ln_gamma_stirling(alpha)? + local_ln_gamma_stirling(beta)?
            - local_ln_gamma_stirling(alpha + beta)?;
    let v = ((alpha - 1.0) * x.ln() + (beta - 1.0) * (1.0 - x).ln() - log_b).exp();
    v.is_finite().then_some(v)
}

fn local_chi_squared_pdf(x: f64, k: f64) -> Option<f64> {
    if !(k > 0.0) || !k.is_finite() || !x.is_finite() {
        return None;
    }
    if x < 0.0 {
        return Some(0.0);
    }
    if x == 0.0 {
        return Some(if k < 2.0 {
            f64::INFINITY
        } else if (k - 2.0).abs() < 1e-12 {
            0.5
        } else {
            0.0
        });
    }
    let kh = k / 2.0;
    let v = ((kh - 1.0) * x.ln()
        - x / 2.0
        - kh * std::f64::consts::LN_2
        - local_ln_gamma_stirling(kh)?)
    .exp();
    v.is_finite().then_some(v)
}

fn local_erf_approx(x: f64) -> f64 {
    // Abramowitz & Stegun 7.1.26
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let ax = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * ax);
    let poly = t
        * (0.254829592
            + t * (-0.284496736
                + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
    sign * (1.0 - poly * (-ax * ax).exp())
}

fn local_standard_normal_cdf(z: f64) -> f64 {
    0.5 * (1.0 + local_erf_approx(z / std::f64::consts::SQRT_2))
}

/// Wilson–Hilferty χ² CDF sketch (honest offline approx; Host uses incomplete gamma).
fn local_chi_squared_cdf(x: f64, k: f64) -> Option<f64> {
    if !(k > 0.0) || !k.is_finite() || !x.is_finite() {
        return None;
    }
    if x <= 0.0 {
        return Some(0.0);
    }
    if (k - 2.0).abs() < 1e-12 {
        return Some(1.0 - (-x / 2.0).exp());
    }
    let h = 2.0 / (9.0 * k);
    let cube = (x / k).powf(1.0 / 3.0);
    let z = (cube - (1.0 - h)) / h.sqrt();
    let p = local_standard_normal_cdf(z).clamp(0.0, 1.0);
    Some(p)
}

/// Beasley–Springer / Moro-ish Φ⁻¹ on (0,1) for offline Normal / χ² sketches.
fn local_inv_standard_normal(p: f64) -> Option<f64> {
    if !(p > 0.0 && p < 1.0) {
        return None;
    }
    let a = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_657e2,
        1.383_577_518_672_690e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    let b = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    let c = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    let d = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    let plow = 0.02425;
    let phigh = 1.0 - plow;
    let z = if p < plow {
        let q = (-2.0 * p.ln()).sqrt();
        (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    } else if p <= phigh {
        let q = p - 0.5;
        let r = q * q;
        (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q
            / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    };
    z.is_finite().then_some(z)
}

fn local_chi_squared_quantile(p: f64, k: f64) -> Option<f64> {
    if !(p > 0.0 && p < 1.0) || !(k > 0.0) || !k.is_finite() {
        return None;
    }
    if (k - 2.0).abs() < 1e-12 {
        return Some(-2.0 * (1.0 - p).ln());
    }
    // Invert Wilson–Hilferty: z = Φ⁻¹(p), then x = k (z√h + 1 − h)³
    let z = local_inv_standard_normal(p)?;
    let h = 2.0 / (9.0 * k);
    let x = k * (z * h.sqrt() + 1.0 - h).powi(3);
    (x.is_finite() && x >= 0.0).then_some(x.max(0.0))
}

fn local_normal_pdf(x: f64, mu: f64, sigma: f64) -> Option<f64> {
    if !(sigma > 0.0) || !x.is_finite() || !mu.is_finite() {
        return None;
    }
    let z = (x - mu) / sigma;
    let v = (-0.5 * z * z).exp() / (sigma * (2.0 * std::f64::consts::PI).sqrt());
    v.is_finite().then_some(v)
}

fn local_normal_cdf(x: f64, mu: f64, sigma: f64) -> Option<f64> {
    if !(sigma > 0.0) || !x.is_finite() || !mu.is_finite() {
        return None;
    }
    Some(local_standard_normal_cdf((x - mu) / sigma))
}

fn local_normal_quantile(p: f64, mu: f64, sigma: f64) -> Option<f64> {
    if !(sigma > 0.0) || !mu.is_finite() {
        return None;
    }
    let z = local_inv_standard_normal(p)?;
    let q = mu + sigma * z;
    q.is_finite().then_some(q)
}

fn local_poisson_pmf(k: u32, lambda: f64) -> Option<f64> {
    if !(lambda >= 0.0) || !lambda.is_finite() {
        return None;
    }
    if lambda == 0.0 {
        return Some(if k == 0 { 1.0 } else { 0.0 });
    }
    let mut ln_fact = 0.0;
    for i in 2..=k {
        ln_fact += (i as f64).ln();
    }
    let v = (-lambda + (k as f64) * lambda.ln() - ln_fact).exp();
    v.is_finite().then_some(v)
}

fn local_poisson_cdf(k: u32, lambda: f64) -> Option<f64> {
    if !(lambda >= 0.0) || !lambda.is_finite() {
        return None;
    }
    let mut s = 0.0;
    for i in 0..=k {
        s += local_poisson_pmf(i, lambda)?;
        if !s.is_finite() {
            return None;
        }
    }
    Some(s.min(1.0))
}

fn local_exponential_pdf(x: f64, rate: f64) -> Option<f64> {
    if !(rate > 0.0) || !x.is_finite() {
        return None;
    }
    if x < 0.0 {
        return Some(0.0);
    }
    let v = rate * (-rate * x).exp();
    v.is_finite().then_some(v)
}

fn local_exponential_cdf(x: f64, rate: f64) -> Option<f64> {
    if !(rate > 0.0) || !x.is_finite() {
        return None;
    }
    if x < 0.0 {
        return Some(0.0);
    }
    let v = 1.0 - (-rate * x).exp();
    v.is_finite().then_some(v)
}

/// Average ranks (1-based) with midranks for ties — offline Spearman sketch.
fn local_average_ranks(values: &[f64]) -> Option<Vec<f64>> {
    let n = values.len();
    if n == 0 {
        return None;
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_unstable_by(|&i, &j| {
        values[i]
            .partial_cmp(&values[j])
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    let mut ranks = vec![0.0; n];
    let mut i = 0;
    while i < n {
        let mut j = i + 1;
        while j < n && values[order[j]] == values[order[i]] {
            j += 1;
        }
        // Midrank of 1-based positions [i+1 .. j]
        let avg = ((i + 1) + j) as f64 / 2.0;
        for k in i..j {
            ranks[order[k]] = avg;
        }
        i = j;
    }
    Some(ranks)
}

fn local_spearman(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() != y.len() || x.len() < 2 {
        return None;
    }
    let rx = local_average_ranks(x)?;
    let ry = local_average_ranks(y)?;
    local_pearson(&rx, &ry)
}

fn local_kendall(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len();
    if n != y.len() || n < 2 {
        return None;
    }
    let mut concordant = 0i64;
    let mut discordant = 0i64;
    for i in 0..n {
        for j in (i + 1)..n {
            let dx = x[i] - x[j];
            let dy = y[i] - y[j];
            let prod = dx * dy;
            if prod > 0.0 {
                concordant += 1;
            } else if prod < 0.0 {
                discordant += 1;
            }
        }
    }
    let pairs = (n * (n - 1) / 2) as f64;
    if pairs == 0.0 {
        return None;
    }
    Some((concordant - discordant) as f64 / pairs)
}

fn local_winsorized_mean(values: &[f64], proportion: f64) -> Option<f64> {
    let n = values.len();
    if n == 0 || !(0.0..0.5).contains(&proportion) {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let cut = (n as f64 * proportion).floor() as usize;
    if 2 * cut >= n {
        return local_quantile(values, 0.5);
    }
    let lo = sorted[cut];
    let hi = sorted[n - 1 - cut];
    let mut acc = 0.0;
    for &v in values {
        acc += v.clamp(lo, hi);
    }
    Some(acc / n as f64)
}

fn local_erfc_approx(x: f64) -> f64 {
    1.0 - local_erf_approx(x)
}

fn local_uniform_pdf(x: f64, a: f64, b: f64) -> Option<f64> {
    if !(b > a) || !x.is_finite() || !a.is_finite() || !b.is_finite() {
        return None;
    }
    if x < a || x > b {
        return Some(0.0);
    }
    Some(1.0 / (b - a))
}

fn local_laplace_pdf(x: f64, mu: f64, b: f64) -> Option<f64> {
    if !(b > 0.0) || !x.is_finite() || !mu.is_finite() {
        return None;
    }
    let v = (-(x - mu).abs() / b).exp() / (2.0 * b);
    v.is_finite().then_some(v)
}

fn local_standard_pdf(z: f64) -> Option<f64> {
    local_normal_pdf(z, 0.0, 1.0)
}

fn local_uniform_cdf(x: f64, a: f64, b: f64) -> Option<f64> {
    if !(b > a) || !x.is_finite() || !a.is_finite() || !b.is_finite() {
        return None;
    }
    if x < a {
        return Some(0.0);
    }
    if x > b {
        return Some(1.0);
    }
    Some((x - a) / (b - a))
}

fn local_laplace_cdf(x: f64, mu: f64, b: f64) -> Option<f64> {
    if !(b > 0.0) || !x.is_finite() || !mu.is_finite() {
        return None;
    }
    let z = (x - mu) / b;
    let v = if z < 0.0 {
        0.5 * z.exp()
    } else {
        1.0 - 0.5 * (-z).exp()
    };
    v.is_finite().then_some(v)
}

fn local_lognormal_pdf(x: f64, mu: f64, sigma: f64) -> Option<f64> {
    if !(x > 0.0) || !(sigma > 0.0) || !mu.is_finite() {
        return None;
    }
    let z = (x.ln() - mu) / sigma;
    let v = (-0.5 * z * z).exp() / (x * sigma * (2.0 * std::f64::consts::PI).sqrt());
    v.is_finite().then_some(v)
}

fn local_lognormal_cdf(x: f64, mu: f64, sigma: f64) -> Option<f64> {
    if !(sigma > 0.0) || !mu.is_finite() || !x.is_finite() {
        return None;
    }
    if x <= 0.0 {
        return Some(0.0);
    }
    local_normal_cdf(x.ln(), mu, sigma)
}

fn local_standard_quantile(p: f64) -> Option<f64> {
    local_inv_standard_normal(p)
}

fn local_gamma_fn(x: f64) -> Option<f64> {
    let ln = local_ln_gamma_stirling(x)?;
    let v = ln.exp();
    v.is_finite().then_some(v)
}

fn local_weibull_pdf(x: f64, shape: f64, scale: f64) -> Option<f64> {
    if !(shape > 0.0) || !(scale > 0.0) || !x.is_finite() {
        return None;
    }
    if x < 0.0 {
        return Some(0.0);
    }
    let z = x / scale;
    let v = (shape / scale) * z.powf(shape - 1.0) * (-z.powf(shape)).exp();
    v.is_finite().then_some(v)
}

fn local_gamma_pdf(x: f64, shape: f64, scale: f64) -> Option<f64> {
    if !(shape > 0.0) || !(scale > 0.0) || !x.is_finite() {
        return None;
    }
    if x <= 0.0 {
        return Some(0.0);
    }
    let ln = (shape - 1.0) * x.ln() - x / scale - shape * scale.ln()
        - local_ln_gamma_stirling(shape)?;
    let v = ln.exp();
    v.is_finite().then_some(v)
}

fn local_two_sided_p(z: f64) -> Option<f64> {
    if !z.is_finite() {
        return None;
    }
    let p = 2.0 * (1.0 - local_standard_normal_cdf(z.abs()));
    p.is_finite().then_some(p.clamp(0.0, 1.0))
}

fn local_chi_squared_upper_p(x: f64, k: f64) -> Option<f64> {
    let cdf = local_chi_squared_cdf(x, k)?;
    Some((1.0 - cdf).clamp(0.0, 1.0))
}

fn local_students_t_pdf(t: f64, nu: f64) -> Option<f64> {
    if !(nu > 0.0) || !t.is_finite() {
        return None;
    }
    let c = (local_ln_gamma_stirling((nu + 1.0) / 2.0)? - local_ln_gamma_stirling(nu / 2.0)?)
        .exp()
        / (nu * std::f64::consts::PI).sqrt();
    let v = c * (1.0 + t * t / nu).powf(-(nu + 1.0) / 2.0);
    v.is_finite().then_some(v)
}

fn local_fisher_f_pdf(x: f64, d1: f64, d2: f64) -> Option<f64> {
    if !(d1 > 0.0) || !(d2 > 0.0) || !x.is_finite() {
        return None;
    }
    if x <= 0.0 {
        return Some(0.0);
    }
    let ln_b = local_ln_gamma_stirling(d1 / 2.0)? + local_ln_gamma_stirling(d2 / 2.0)?
        - local_ln_gamma_stirling((d1 + d2) / 2.0)?;
    let ln_num = (d1 / 2.0) * (d1 / d2).ln() + (d1 / 2.0 - 1.0) * x.ln()
        - ((d1 + d2) / 2.0) * (1.0 + d1 * x / d2).ln();
    let v = (ln_num - ln_b).exp();
    v.is_finite().then_some(v)
}

/// Offline regularized lower incomplete gamma P(a,x) series sketch (Host uses series/CF).
fn local_gammp(a: f64, x: f64) -> Option<f64> {
    if !(a > 0.0) || !x.is_finite() || !a.is_finite() {
        return None;
    }
    if x <= 0.0 {
        return Some(0.0);
    }
    // Exact for a=1: P(1,x)=1−e^{−x}
    if (a - 1.0).abs() < 1e-12 {
        let v = 1.0 - (-x).exp();
        return v.is_finite().then_some(v.clamp(0.0, 1.0));
    }
    let mut ap = a;
    let mut sum = 1.0 / a;
    let mut del = sum;
    for _ in 0..128 {
        ap += 1.0;
        del *= x / ap;
        sum += del;
        if !sum.is_finite() {
            return None;
        }
        if del.abs() < sum.abs() * 1e-12 {
            break;
        }
    }
    let v = sum * (-x + a * x.ln() - local_ln_gamma_stirling(a)?).exp();
    v.is_finite().then_some(v.clamp(0.0, 1.0))
}

fn local_gammq(a: f64, x: f64) -> Option<f64> {
    let p = local_gammp(a, x)?;
    Some((1.0 - p).clamp(0.0, 1.0))
}

/// Offline regularized incomplete beta I_x(a,b) continued-fraction sketch.
fn local_betai(a: f64, b: f64, x: f64) -> Option<f64> {
    if !(a > 0.0) || !(b > 0.0) || !x.is_finite() {
        return None;
    }
    if x <= 0.0 {
        return Some(0.0);
    }
    if x >= 1.0 {
        return Some(1.0);
    }
    let bt = (local_ln_gamma_stirling(a + b)?
        - local_ln_gamma_stirling(a)?
        - local_ln_gamma_stirling(b)?
        + a * x.ln()
        + b * (1.0 - x).ln())
    .exp();
    if !bt.is_finite() {
        return None;
    }
    let cf = |aa: f64, bb: f64, xx: f64| -> Option<f64> {
        const FPMIN: f64 = 1e-30;
        let qab = aa + bb;
        let qap = aa + 1.0;
        let qam = aa - 1.0;
        let mut c = 1.0;
        let mut d = 1.0 - qab * xx / qap;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        d = 1.0 / d;
        let mut h = d;
        for m in 1..=100 {
            let m = m as f64;
            let m2 = 2.0 * m;
            let aa_even = m * (bb - m) * xx / ((qam + m2) * (aa + m2));
            d = 1.0 + aa_even * d;
            if d.abs() < FPMIN {
                d = FPMIN;
            }
            c = 1.0 + aa_even / c;
            if c.abs() < FPMIN {
                c = FPMIN;
            }
            d = 1.0 / d;
            h *= d * c;
            let aa_odd = -((aa + m) * (qab + m) * xx) / ((aa + m2) * (qap + m2));
            d = 1.0 + aa_odd * d;
            if d.abs() < FPMIN {
                d = FPMIN;
            }
            c = 1.0 + aa_odd / c;
            if c.abs() < FPMIN {
                c = FPMIN;
            }
            d = 1.0 / d;
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() < 1e-12 {
                break;
            }
        }
        h.is_finite().then_some(h)
    };
    let v = if x < (a + 1.0) / (a + b + 2.0) {
        bt * cf(a, b, x)? / a
    } else {
        1.0 - bt * cf(b, a, 1.0 - x)? / b
    };
    v.is_finite().then_some(v.clamp(0.0, 1.0))
}

/// Offline Student's t CDF via local betai sketch (Host uses exact betai).
fn local_students_t_cdf(t: f64, nu: f64) -> Option<f64> {
    if !(nu > 0.0) || !t.is_finite() {
        return None;
    }
    let x = nu / (nu + t * t);
    let ib = 0.5 * local_betai(nu / 2.0, 0.5, x)?;
    let v = if t >= 0.0 { 1.0 - ib } else { ib };
    v.is_finite().then_some(v.clamp(0.0, 1.0))
}

fn local_students_t_two_sided_p(t: f64, nu: f64) -> Option<f64> {
    if !(nu > 0.0) || !t.is_finite() {
        return None;
    }
    let x = nu / (nu + t * t);
    let v = local_betai(nu / 2.0, 0.5, x)?;
    v.is_finite().then_some(v.clamp(0.0, 1.0))
}

fn local_students_t_upper_p(t: f64, nu: f64) -> Option<f64> {
    let cdf = local_students_t_cdf(t, nu)?;
    Some((1.0 - cdf).clamp(0.0, 1.0))
}

/// Bisection inverse of [`local_students_t_cdf`] (honest offline sketch).
fn local_students_t_quantile(p: f64, nu: f64) -> Option<f64> {
    if !(p > 0.0 && p < 1.0) || !(nu > 0.0) {
        return None;
    }
    if (p - 0.5).abs() < 1e-15 {
        return Some(0.0);
    }
    let z = local_inv_standard_normal(p)?;
    let mut lo = z - 20.0;
    let mut hi = z + 20.0;
    for _ in 0..48 {
        let flo = local_students_t_cdf(lo, nu)? - p;
        let fhi = local_students_t_cdf(hi, nu)? - p;
        if flo * fhi <= 0.0 {
            break;
        }
        lo -= 20.0;
        hi += 20.0;
    }
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if local_students_t_cdf(mid, nu)? < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let q = 0.5 * (lo + hi);
    q.is_finite().then_some(q)
}

fn local_fisher_f_cdf(x: f64, d1: f64, d2: f64) -> Option<f64> {
    if !(d1 > 0.0) || !(d2 > 0.0) || !x.is_finite() {
        return None;
    }
    if x <= 0.0 {
        return Some(0.0);
    }
    let z = d1 * x / (d1 * x + d2);
    local_betai(d1 / 2.0, d2 / 2.0, z)
}

fn local_fisher_f_upper_p(x: f64, d1: f64, d2: f64) -> Option<f64> {
    let cdf = local_fisher_f_cdf(x, d1, d2)?;
    Some((1.0 - cdf).clamp(0.0, 1.0))
}

/// Bisection inverse of [`local_fisher_f_cdf`] on [0, ∞).
fn local_fisher_f_quantile(p: f64, d1: f64, d2: f64) -> Option<f64> {
    if !(p > 0.0 && p < 1.0) || !(d1 > 0.0) || !(d2 > 0.0) {
        return None;
    }
    let mut lo = 0.0;
    let mut hi = 1.0;
    while local_fisher_f_cdf(hi, d1, d2)? < p {
        hi *= 2.0;
        if hi > 1e12 {
            return None;
        }
    }
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if local_fisher_f_cdf(mid, d1, d2)? < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let q = 0.5 * (lo + hi);
    (q.is_finite() && q >= 0.0).then_some(q)
}

fn local_tukey_fences(values: &[f64], k: f64) -> Option<(f64, f64)> {
    if values.len() < 4 || !(k >= 0.0) || !k.is_finite() {
        return None;
    }
    let q1 = local_quantile(values, 0.25)?;
    let q3 = local_quantile(values, 0.75)?;
    let iqr = q3 - q1;
    Some((q1 - k * iqr, q3 + k * iqr))
}

/// Shannon entropy in bits (Host normalizes unnormalized masses).
fn local_entropy(p: &[f64]) -> Option<f64> {
    if p.is_empty() {
        return None;
    }
    let total: f64 = p.iter().sum();
    if !(total > 0.0) {
        return None;
    }
    let mut h = 0.0;
    for &pi in p {
        let q = pi / total;
        if q > 0.0 {
            h -= q * q.log2();
        }
    }
    Some(h)
}

fn local_entropy_from_counts(counts: &[u64]) -> Option<f64> {
    if counts.is_empty() {
        return None;
    }
    let p: Vec<f64> = counts.iter().map(|&c| c as f64).collect();
    local_entropy(&p)
}

fn local_kl_divergence(p: &[f64], q: &[f64]) -> Option<f64> {
    if p.is_empty() || p.len() != q.len() {
        return None;
    }
    let (sp, sq): (f64, f64) = (p.iter().sum(), q.iter().sum());
    if !(sp > 0.0) || !(sq > 0.0) {
        return None;
    }
    let mut d = 0.0;
    for (&pi, &qi) in p.iter().zip(q) {
        let pn = pi / sp;
        let qn = qi / sq;
        if pn > 0.0 {
            if qn <= 0.0 {
                return None;
            }
            d += pn * (pn / qn).log2();
        }
    }
    Some(d)
}

fn local_cross_entropy(p: &[f64], q: &[f64]) -> Option<f64> {
    Some(local_entropy(p)? + local_kl_divergence(p, q)?)
}

fn local_empirical_cdf(samples: &[f64], x: f64) -> Option<f64> {
    if samples.is_empty() || !x.is_finite() {
        return None;
    }
    let count = samples.iter().filter(|&&s| s <= x).count();
    Some(count as f64 / samples.len() as f64)
}

/// Last moving-average point (offline sketch); Host returns the full series.
fn local_moving_average_last(values: &[f64], window: usize) -> Option<(f64, usize)> {
    let n = values.len();
    if window == 0 || window > n {
        return None;
    }
    let count = n - window + 1;
    let mut acc: f64 = values[..window].iter().sum();
    let w = window as f64;
    let mut last = acc / w;
    for j in 1..count {
        acc += values[j + window - 1] - values[j - 1];
        last = acc / w;
    }
    Some((last, count))
}

fn local_modified_z_outlier_count(values: &[f64], threshold: f64) -> Option<usize> {
    if values.len() < 2 || !(threshold > 0.0) || !threshold.is_finite() {
        return None;
    }
    let med = local_quantile(values, 0.5)?;
    let mad = local_mad(values, true)?;
    if !(mad > 0.0) || !mad.is_finite() {
        return None;
    }
    let mut count = 0usize;
    for &x in values {
        if ((x - med) / mad).abs() > threshold {
            count += 1;
        }
    }
    Some(count)
}

fn local_iqr_outlier_count(values: &[f64], k: f64) -> Option<usize> {
    let (lo, hi) = local_tukey_fences(values, k)?;
    Some(values.iter().filter(|&&x| x < lo || x > hi).count())
}

fn local_sample_variance(values: &[f64]) -> Option<f64> {
    super::chain_actions::local_variance(values, true)
}

fn local_sample_std_dev(values: &[f64]) -> Option<f64> {
    super::chain_actions::local_std_dev(values, true)
}

/// Last smoothed value from Brown's exponential smoothing.
fn local_exponential_smoothing_last(values: &[f64], alpha: f64) -> Option<(f64, usize)> {
    if values.is_empty() || !(alpha > 0.0 && alpha <= 1.0) {
        return None;
    }
    let mut s = values[0];
    for &x in &values[1..] {
        s = alpha * x + (1.0 - alpha) * s;
    }
    Some((s, values.len()))
}

fn local_adf_proxy(series: &[f64]) -> Option<f64> {
    if series.len() < 3 {
        return None;
    }
    let mut sum_diff = 0.0;
    let mut sum_lag = 0.0;
    for i in 1..series.len() {
        let diff = series[i] - series[i - 1];
        sum_diff += diff * series[i - 1];
        sum_lag += series[i - 1] * series[i - 1];
    }
    if sum_lag.abs() < 1e-12 {
        return Some(0.0);
    }
    Some(sum_diff / sum_lag)
}

fn local_histogram_summary(values: &[f64], bins: usize) -> Option<(usize, f64, f64, f64)> {
    if values.is_empty() || bins == 0 || bins > 256 {
        return None;
    }
    let min_v = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max_v = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !min_v.is_finite() || !max_v.is_finite() {
        return None;
    }
    let bin_width = (max_v - min_v) / bins as f64;
    let mut counts = vec![0u32; bins];
    for &v in values {
        let idx = if !bin_width.is_finite() || bin_width <= 0.0 {
            0
        } else {
            let i = ((v - min_v) / bin_width) as usize;
            i.min(bins - 1)
        };
        counts[idx] += 1;
    }
    let occupied = counts.iter().filter(|&&c| c > 0).count();
    Some((occupied, min_v, max_v, bin_width))
}

/// KS D against Uniform(0,1) (Host clamps each sample into [0,1]).
fn local_ks_1sample(data: &[f64]) -> Option<(f64, f64)> {
    if data.is_empty() {
        return None;
    }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let n = sorted.len() as f64;
    let mut d = 0.0f64;
    for (i, &x) in sorted.iter().enumerate() {
        let cdf = x.clamp(0.0, 1.0);
        let i_f = i as f64;
        d = d.max(((i_f + 1.0) / n - cdf).abs());
        d = d.max((cdf - i_f / n).abs());
    }
    let p = (-2.0 * n * d * d).exp().min(1.0);
    Some((d, p))
}

fn local_grubbs_test(values: &[f64], alpha: f64) -> Option<(usize, f64, f64, bool)> {
    let n = values.len();
    if n < 3 || !(alpha > 0.0 && alpha < 1.0) {
        return None;
    }
    let mu = local_mean(values)?;
    let sd = local_sample_std_dev(values)?;
    if sd <= 0.0 {
        return None;
    }
    let (index, statistic) = values
        .iter()
        .enumerate()
        .map(|(i, &x)| (i, (x - mu).abs() / sd))
        .fold((0usize, f64::NEG_INFINITY), |best, cur| {
            if cur.1 > best.1 {
                cur
            } else {
                best
            }
        });
    let nf = n as f64;
    let t = local_students_t_quantile(1.0 - alpha / (2.0 * nf), nf - 2.0)?;
    let t2 = t * t;
    let critical = ((nf - 1.0) / nf.sqrt()) * (t2 / (nf - 2.0 + t2)).sqrt();
    Some((index, statistic, critical, statistic > critical))
}

fn local_one_sample_t(values: &[f64], mu: f64) -> Option<(f64, f64, u32)> {
    let n = values.len();
    if n < 2 {
        return None;
    }
    let m = local_mean(values)?;
    let var = local_sample_variance(values)?;
    let df = (n - 1) as f64;
    let std_error = (var / n as f64).sqrt();
    if std_error == 0.0 {
        let t = if m == mu {
            0.0
        } else {
            f64::INFINITY.copysign(m - mu)
        };
        let p = if m == mu { 1.0 } else { 0.0 };
        return Some((t, p, (n - 1) as u32));
    }
    let t = (m - mu) / std_error;
    let p = local_students_t_two_sided_p(t, df)?;
    Some((t, p, (n - 1) as u32))
}

fn local_paired_t(a: &[f64], b: &[f64]) -> Option<(f64, f64, u32)> {
    if a.len() != b.len() || a.len() < 2 {
        return None;
    }
    let diffs: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x - y).collect();
    local_one_sample_t(&diffs, 0.0)
}

fn local_two_sample_t(a: &[f64], b: &[f64], equal_var: bool) -> Option<(f64, f64, f64, f64)> {
    let (na, nb) = (a.len(), b.len());
    if na < 2 || nb < 2 {
        return None;
    }
    let (ma, mb) = (local_mean(a)?, local_mean(b)?);
    let (va, vb) = (local_sample_variance(a)?, local_sample_variance(b)?);
    let (na_f, nb_f) = (na as f64, nb as f64);
    let diff = ma - mb;
    let (se, df) = if equal_var {
        let sp2 = ((na_f - 1.0) * va + (nb_f - 1.0) * vb) / (na_f + nb_f - 2.0);
        let se = (sp2 * (1.0 / na_f + 1.0 / nb_f)).sqrt();
        (se, na_f + nb_f - 2.0)
    } else {
        let se2 = va / na_f + vb / nb_f;
        let se = se2.sqrt();
        let num = se2 * se2;
        let den = (va / na_f).powi(2) / (na_f - 1.0) + (vb / nb_f).powi(2) / (nb_f - 1.0);
        let df = if den > 0.0 {
            num / den
        } else {
            na_f + nb_f - 2.0
        };
        (se, df)
    };
    if se == 0.0 {
        let t = if diff == 0.0 {
            0.0
        } else {
            f64::INFINITY.copysign(diff)
        };
        let p = if diff == 0.0 { 1.0 } else { 0.0 };
        return Some((t, p, df, diff));
    }
    let t = diff / se;
    let p = local_students_t_two_sided_p(t, df)?;
    Some((t, p, df, diff))
}

fn local_linear_regression(x: &[f64], y: &[f64]) -> Option<(f64, f64, f64, usize)> {
    let n = x.len();
    if n != y.len() || n < 3 {
        return None;
    }
    let mx = local_mean(x)?;
    let my = local_mean(y)?;
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    let mut sst = 0.0;
    for i in 0..n {
        let dx = x[i] - mx;
        let dy = y[i] - my;
        sxx += dx * dx;
        sxy += dx * dy;
        sst += dy * dy;
    }
    if sxx <= 0.0 {
        return None;
    }
    let slope = sxy / sxx;
    let intercept = my - slope * mx;
    let mut ssr = 0.0;
    for i in 0..n {
        let yhat = intercept + slope * x[i];
        ssr += (y[i] - yhat).powi(2);
    }
    let r2 = if sst > 0.0 { 1.0 - ssr / sst } else { 1.0 };
    Some((slope, intercept, r2, n))
}

fn local_autocorrelation(values: &[f64], lag: usize) -> Option<f64> {
    let n = values.len();
    if n == 0 || lag >= n {
        return None;
    }
    let m = local_mean(values)?;
    let mut denom = 0.0;
    for &v in values {
        let d = v - m;
        denom += d * d;
    }
    if denom == 0.0 {
        return None;
    }
    let mut num = 0.0;
    for t in lag..n {
        num += (values[t] - m) * (values[t - lag] - m);
    }
    Some(num / denom)
}

/// Split flat values into `k` contiguous equal-length groups (drops remainder).
fn split_k_groups(values: &[f64], k: usize) -> Option<Vec<Vec<f64>>> {
    if k < 2 {
        return None;
    }
    let n_per = values.len() / k;
    if n_per == 0 {
        return None;
    }
    let mut groups = Vec::with_capacity(k);
    for i in 0..k {
        groups.push(values[i * n_per..(i + 1) * n_per].to_vec());
    }
    Some(groups)
}

/// Reshape flat values into `n` blocks of `treatments` columns (drops remainder).
fn split_blocks(values: &[f64], treatments: usize) -> Option<Vec<Vec<f64>>> {
    if treatments < 2 {
        return None;
    }
    let n = values.len() / treatments;
    if n < 2 {
        return None;
    }
    let mut blocks = Vec::with_capacity(n);
    for i in 0..n {
        blocks.push(values[i * treatments..(i + 1) * treatments].to_vec());
    }
    Some(blocks)
}

/// Reshape flat counts into an `R×C` table (drops remainder).
fn reshape_table(values: &[f64], cols: usize) -> Option<Vec<Vec<f64>>> {
    if cols < 2 {
        return None;
    }
    let rows = values.len() / cols;
    if rows < 2 {
        return None;
    }
    let mut table = Vec::with_capacity(rows);
    for i in 0..rows {
        table.push(values[i * cols..(i + 1) * cols].to_vec());
    }
    Some(table)
}

fn local_chi_square_gof(observed: &[f64], expected: &[f64]) -> Option<(f64, f64, f64)> {
    if observed.len() != expected.len() || observed.len() < 2 {
        return None;
    }
    if expected.iter().any(|&e| e <= 0.0) {
        return None;
    }
    let stat: f64 = observed
        .iter()
        .zip(expected.iter())
        .map(|(&o, &e)| (o - e).powi(2) / e)
        .sum();
    let dof = (observed.len() - 1) as f64;
    let p = local_chi_squared_upper_p(stat, dof)?;
    Some((stat, p, dof))
}

fn local_chi_square_independence(table: &[Vec<f64>]) -> Option<(f64, f64, f64)> {
    let rows = table.len();
    if rows < 2 {
        return None;
    }
    let cols = table[0].len();
    if cols < 2 || table.iter().any(|r| r.len() != cols) {
        return None;
    }
    let row_sums: Vec<f64> = table.iter().map(|r| r.iter().sum()).collect();
    let mut col_sums = vec![0.0; cols];
    for r in table {
        for (j, &v) in r.iter().enumerate() {
            col_sums[j] += v;
        }
    }
    let total: f64 = row_sums.iter().sum();
    if total <= 0.0 {
        return None;
    }
    let mut stat = 0.0;
    for (i, r) in table.iter().enumerate() {
        for (j, &o) in r.iter().enumerate() {
            let e = row_sums[i] * col_sums[j] / total;
            if e > 0.0 {
                stat += (o - e).powi(2) / e;
            }
        }
    }
    let dof = ((rows - 1) * (cols - 1)) as f64;
    let p = local_chi_squared_upper_p(stat, dof)?;
    Some((stat, p, dof))
}

fn local_correlation_p_value(r: f64, n: usize) -> Option<f64> {
    if n < 3 || !r.is_finite() {
        return None;
    }
    let df = (n - 2) as f64;
    let denom = 1.0 - r * r;
    if denom <= 0.0 {
        return Some(0.0);
    }
    let t = r * (df / denom).sqrt();
    local_students_t_two_sided_p(t, df)
}

fn local_ljung_box(acf: &[f64], n: usize, h: usize) -> Option<f64> {
    if h == 0 || acf.len() < h || n <= h {
        return None;
    }
    let mut q = 0.0;
    for k in 1..=h {
        q += acf[k - 1].powi(2) / (n - k) as f64;
    }
    Some(q * n as f64)
}

fn local_mann_whitney_u(x: &[f64], y: &[f64]) -> Option<(f64, f64, usize, usize)> {
    let n1 = x.len();
    let n2 = y.len();
    if n1 == 0 || n2 == 0 {
        return None;
    }
    let mut all: Vec<(f64, u8)> = Vec::with_capacity(n1 + n2);
    for &v in x {
        all.push((v, 0));
    }
    for &v in y {
        all.push((v, 1));
    }
    all.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));
    let mut rank_sum1 = 0.0;
    let mut i = 0;
    while i < all.len() {
        let mut j = i;
        while j < all.len() && (all[j].0 - all[i].0).abs() < 1e-12 {
            j += 1;
        }
        let rank = (i + j) as f64 / 2.0 + 0.5;
        for k in i..j {
            if all[k].1 == 0 {
                rank_sum1 += rank;
            }
        }
        i = j;
    }
    let u1 = rank_sum1 - (n1 as f64 * (n1 as f64 + 1.0) / 2.0);
    let u2 = (n1 * n2) as f64 - u1;
    let u = u1.min(u2);
    let mu = (n1 * n2) as f64 / 2.0;
    let sigma = ((n1 * n2) as f64 * (n1 + n2 + 1) as f64 / 12.0).sqrt();
    let z = (u - mu) / sigma.max(1e-9);
    let p = (2.0 * (1.0 - 0.5 * (1.0 + (z / (2.0f64.sqrt())).tanh()))).clamp(0.0, 1.0);
    Some((u, p, n1, n2))
}

fn local_mcnemar(b: u64, c: u64) -> Option<(f64, f64, f64)> {
    if b + c == 0 {
        return None;
    }
    let (nb, nc) = (b as f64, c as f64);
    let diff = (nb - nc).abs();
    let stat = if diff >= 1.0 {
        (diff - 1.0).powi(2) / (nb + nc)
    } else {
        0.0
    };
    let p = local_chi_squared_upper_p(stat, 1.0)?;
    Some((stat, p, 1.0))
}

fn local_mutual_information_discrete(x: &[usize], y: &[usize]) -> Option<f64> {
    let n = x.len();
    if n == 0 || n != y.len() {
        return None;
    }
    let nx = x.iter().copied().max().unwrap_or(0) + 1;
    let ny = y.iter().copied().max().unwrap_or(0) + 1;
    let mut joint = vec![0.0f64; nx * ny];
    let mut px = vec![0.0f64; nx];
    let mut py = vec![0.0f64; ny];
    for (&xi, &yi) in x.iter().zip(y) {
        joint[xi * ny + yi] += 1.0;
        px[xi] += 1.0;
        py[yi] += 1.0;
    }
    let nf = n as f64;
    let mut mi = 0.0;
    for xi in 0..nx {
        for yi in 0..ny {
            let pxy = joint[xi * ny + yi] / nf;
            if pxy > 0.0 {
                let pxi = px[xi] / nf;
                let pyi = py[yi] / nf;
                mi += pxy * (pxy / (pxi * pyi)).log2();
            }
        }
    }
    Some(mi.max(0.0))
}

fn local_one_way_anova(groups: &[Vec<f64>]) -> Option<(f64, f64, f64, f64)> {
    let k = groups.len();
    if k < 2 || groups.iter().any(|g| g.is_empty()) {
        return None;
    }
    let n_total: usize = groups.iter().map(|g| g.len()).sum();
    if n_total <= k {
        return None;
    }
    let grand = groups.iter().flat_map(|g| g.iter()).sum::<f64>() / n_total as f64;
    let mut ss_between = 0.0;
    let mut ss_within = 0.0;
    for g in groups {
        let gm = local_mean(g)?;
        ss_between += g.len() as f64 * (gm - grand).powi(2);
        for &x in g {
            ss_within += (x - gm).powi(2);
        }
    }
    let df_between = (k - 1) as f64;
    let df_within = (n_total - k) as f64;
    let ms_between = ss_between / df_between;
    let ms_within = ss_within / df_within;
    let f = if ms_within > 0.0 {
        ms_between / ms_within
    } else if ms_between > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };
    let p = if f.is_finite() {
        local_fisher_f_upper_p(f, df_between, df_within)?
    } else {
        0.0
    };
    Some((f, p, df_between, df_within))
}

fn local_friedman(blocks: &[Vec<f64>]) -> Option<(f64, f64, f64)> {
    let n = blocks.len();
    if n < 2 {
        return None;
    }
    let k = blocks[0].len();
    if k < 2 || blocks.iter().any(|b| b.len() != k) {
        return None;
    }
    let mut rank_sum = vec![0.0; k];
    for block in blocks {
        let mut idx: Vec<usize> = (0..k).collect();
        idx.sort_by(|&a, &b| {
            block[a]
                .partial_cmp(&block[b])
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        let mut ranks = vec![0.0; k];
        let mut i = 0;
        while i < k {
            let mut j = i;
            while j < k && (block[idx[j]] - block[idx[i]]).abs() < 1e-12 {
                j += 1;
            }
            let avg = (i + j + 1) as f64 / 2.0;
            for t in i..j {
                ranks[idx[t]] = avg;
            }
            i = j;
        }
        for j in 0..k {
            rank_sum[j] += ranks[j];
        }
    }
    let kf = k as f64;
    let nf = n as f64;
    let grand = (kf + 1.0) / 2.0;
    let ss: f64 = rank_sum
        .iter()
        .map(|&s| {
            let mean_r = s / nf;
            (mean_r - grand).powi(2)
        })
        .sum();
    let chi = 12.0 * nf / (kf * (kf + 1.0)) * ss;
    let df = kf - 1.0;
    let p = local_chi_squared_upper_p(chi, df)?;
    Some((chi, p, df))
}

fn local_mahalanobis_sq(x: &[f64], mean: &[f64], inv_cov: &[f64]) -> Option<f64> {
    let d = x.len();
    if d == 0 || mean.len() != d || inv_cov.len() != d * d {
        return None;
    }
    let diff: Vec<f64> = x.iter().zip(mean).map(|(&xi, &mi)| xi - mi).collect();
    let mut acc = 0.0;
    for i in 0..d {
        let mut row = 0.0;
        for j in 0..d {
            row += inv_cov[i * d + j] * diff[j];
        }
        acc += diff[i] * row;
    }
    Some(acc)
}

fn parse_mahalanobis_args(values: &[f64], d: usize) -> Option<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    if d == 0 || values.len() < 2 * d {
        return None;
    }
    let x = values[..d].to_vec();
    let mean = values[d..2 * d].to_vec();
    let need = d * d;
    let inv_cov = if values.len() >= 2 * d + need {
        values[2 * d..2 * d + need].to_vec()
    } else {
        let mut m = vec![0.0; need];
        for i in 0..d {
            m[i * d + i] = 1.0;
        }
        m
    };
    Some((x, mean, inv_cov))
}

const LOCAL_LN_2PI: f64 = 1.837_877_066_409_345_6;
const LOCAL_PROB_NORM_TOL: f64 = 1e-6;

fn identity_cov(p: usize) -> Vec<f64> {
    let mut m = vec![0.0; p * p];
    for i in 0..p {
        m[i * p + i] = 1.0;
    }
    m
}

fn local_cholesky_factor(p: usize, a: &[f64], l: &mut [f64]) -> bool {
    if a.len() != p * p || l.len() != p * p || p == 0 {
        return false;
    }
    for x in l.iter_mut() {
        *x = 0.0;
    }
    for j in 0..p {
        let mut diag = a[j * p + j];
        for k in 0..j {
            diag -= l[j * p + k] * l[j * p + k];
        }
        if !(diag > 0.0) {
            return false;
        }
        let ljj = diag.sqrt();
        l[j * p + j] = ljj;
        for i in (j + 1)..p {
            let mut s = a[i * p + j];
            for k in 0..j {
                s -= l[i * p + k] * l[j * p + k];
            }
            l[i * p + j] = s / ljj;
        }
    }
    true
}

fn local_cholesky_solve(p: usize, l: &[f64], b: &[f64], x: &mut [f64]) -> bool {
    if l.len() != p * p || b.len() != p || x.len() != p {
        return false;
    }
    for i in 0..p {
        let mut s = b[i];
        for k in 0..i {
            s -= l[i * p + k] * x[k];
        }
        x[i] = s / l[i * p + i];
    }
    for i in (0..p).rev() {
        let mut s = x[i];
        for k in (i + 1)..p {
            s -= l[k * p + i] * x[k];
        }
        x[i] = s / l[i * p + i];
    }
    true
}

fn local_mvn_log_pdf(x: &[f64], mean: &[f64], cov: &[f64], p: usize) -> Option<f64> {
    if x.len() != p || mean.len() != p || cov.len() != p * p || p == 0 {
        return None;
    }
    let mut l = vec![0.0; p * p];
    if !local_cholesky_factor(p, cov, &mut l) {
        return None;
    }
    let mut log_det = 0.0;
    for i in 0..p {
        let d = l[i * p + i];
        if d <= 0.0 {
            return None;
        }
        log_det += d.ln();
    }
    log_det *= 2.0;
    let diff: Vec<f64> = x.iter().zip(mean).map(|(a, b)| a - b).collect();
    let mut sol = vec![0.0; p];
    if !local_cholesky_solve(p, &l, &diff, &mut sol) {
        return None;
    }
    let maha: f64 = diff.iter().zip(sol.iter()).map(|(d, s)| d * s).sum();
    Some(-0.5 * (p as f64 * LOCAL_LN_2PI + log_det + maha))
}

fn local_mvn_pdf(x: &[f64], mean: &[f64], cov: &[f64], p: usize) -> Option<f64> {
    local_mvn_log_pdf(x, mean, cov, p).map(f64::exp)
}

fn local_mvn_sample(mean: &[f64], cov: &[f64], p: usize, seed: u64) -> Option<Vec<f64>> {
    if mean.len() != p || cov.len() != p * p || p == 0 {
        return None;
    }
    let mut l = vec![0.0; p * p];
    if !local_cholesky_factor(p, cov, &mut l) {
        return None;
    }
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    let mut unit = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 11) as f64) / ((1u64 << 53) as f64)
    };
    let mut gaussian = || {
        let u1 = unit().max(1e-12);
        let u2 = unit();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    };
    let z: Vec<f64> = (0..p).map(|_| gaussian()).collect();
    let mut y = vec![0.0; p];
    for i in 0..p {
        let mut s = mean[i];
        for j in 0..=i {
            s += l[i * p + j] * z[j];
        }
        y[i] = s;
    }
    Some(y)
}

fn local_mvn_mle(data: &[f64], n: usize, p: usize) -> Option<(Vec<f64>, Vec<f64>)> {
    if n < 2 || p == 0 || data.len() != n * p {
        return None;
    }
    let mut mean = vec![0.0; p];
    for j in 0..p {
        let mut s = 0.0;
        for i in 0..n {
            s += data[i * p + j];
        }
        mean[j] = s / n as f64;
    }
    let mut cov = vec![0.0; p * p];
    for i in 0..n {
        for a in 0..p {
            let da = data[i * p + a] - mean[a];
            for b in 0..p {
                cov[a * p + b] += da * (data[i * p + b] - mean[b]);
            }
        }
    }
    for v in cov.iter_mut() {
        *v /= n as f64;
    }
    Some((mean, cov))
}

fn parse_mvn_x_mean_cov(values: &[f64], p: usize) -> Option<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    if p == 0 || values.len() < 2 * p {
        return None;
    }
    let x = values[..p].to_vec();
    let mean = values[p..2 * p].to_vec();
    let need = p * p;
    let cov = if values.len() >= 2 * p + need {
        values[2 * p..2 * p + need].to_vec()
    } else {
        identity_cov(p)
    };
    Some((x, mean, cov))
}

fn parse_mvn_mean_cov(values: &[f64], p: usize) -> Option<(Vec<f64>, Vec<f64>)> {
    if p == 0 || values.len() < p {
        return None;
    }
    let mean = values[..p].to_vec();
    let need = p * p;
    let cov = if values.len() >= p + need {
        values[p..p + need].to_vec()
    } else {
        identity_cov(p)
    };
    Some((mean, cov))
}

fn local_validate_probability(p: &[f64]) -> bool {
    if p.is_empty() {
        return false;
    }
    let mut sum = 0.0;
    for &pi in p {
        if pi < 0.0 || !pi.is_finite() {
            return false;
        }
        sum += pi;
    }
    sum > 0.0 && (sum - 1.0).abs() <= LOCAL_PROB_NORM_TOL
}

fn local_simplex_project(p: &[f64]) -> Option<Vec<f64>> {
    if p.is_empty() || p.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let mut out: Vec<f64> = p.iter().map(|&v| v.max(0.0)).collect();
    let sum: f64 = out.iter().sum();
    if !(sum > 0.0) {
        return None;
    }
    for v in out.iter_mut() {
        *v /= sum;
    }
    Some(out)
}

fn local_fisher_distance(p: &[f64], q: &[f64]) -> Option<f64> {
    if p.is_empty() || p.len() != q.len() {
        return None;
    }
    let mut dot = 0.0;
    for i in 0..p.len() {
        if !p[i].is_finite() || !q[i].is_finite() {
            return None;
        }
        let sp = p[i].max(0.0).sqrt();
        let sq = q[i].max(0.0).sqrt();
        dot += sp * sq;
    }
    Some(dot.clamp(-1.0, 1.0).acos())
}

fn local_neg_entropy(p: &[f64]) -> Option<f64> {
    if p.is_empty() {
        return None;
    }
    let mut sum = 0.0;
    for &pi in p {
        if !pi.is_finite() {
            return None;
        }
        if pi > 0.0 {
            sum += pi * pi.ln();
        }
    }
    Some(sum)
}

fn local_simplex_project_idempotent(p: &[f64]) -> bool {
    let Some(q1) = local_simplex_project(p) else {
        return false;
    };
    let Some(q2) = local_simplex_project(&q1) else {
        return false;
    };
    q1.len() == q2.len()
        && q1
            .iter()
            .zip(q2.iter())
            .all(|(a, b)| (a - b).abs() <= 1e-7)
}

fn local_fisher_inner_product(p: &[f64], u: &[f64], v: &[f64]) -> Option<f64> {
    if p.is_empty() || p.len() != u.len() || p.len() != v.len() {
        return None;
    }
    let mut sum = 0.0;
    for i in 0..p.len() {
        if !p[i].is_finite() || !u[i].is_finite() || !v[i].is_finite() {
            return None;
        }
        if p[i] <= 0.0 {
            continue;
        }
        sum += u[i] * v[i] / p[i];
    }
    Some(sum)
}

fn local_neg_entropy_grad(p: &[f64]) -> Option<Vec<f64>> {
    if p.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(p.len());
    for &pi in p {
        if !pi.is_finite() {
            return None;
        }
        out.push(if pi > 0.0 {
            pi.ln() + 1.0
        } else {
            f64::NEG_INFINITY
        });
    }
    Some(out)
}

fn local_kl_bregman_form(p: &[f64], q: &[f64]) -> Option<f64> {
    if p.is_empty() || p.len() != q.len() {
        return None;
    }
    let psi_p = local_neg_entropy(p)?;
    let psi_q = local_neg_entropy(q)?;
    let grad_q = local_neg_entropy_grad(q)?;
    let mut inner = 0.0;
    for i in 0..p.len() {
        if q[i] > 0.0 {
            inner += grad_q[i] * (p[i] - q[i]);
        }
    }
    Some(psi_p - psi_q - inner)
}

/// Natural-log KL for statistical-manifold sketches (distinct from bits `local_kl_divergence`).
fn local_manifold_kl_divergence(p: &[f64], q: &[f64]) -> Option<f64> {
    if p.is_empty() || p.len() != q.len() {
        return None;
    }
    let mut sum = 0.0;
    for i in 0..p.len() {
        if !p[i].is_finite() || !q[i].is_finite() {
            return None;
        }
        if p[i] > 0.0 {
            if q[i] <= 0.0 {
                return Some(f64::INFINITY);
            }
            sum += p[i] * (p[i].ln() - q[i].ln());
        }
    }
    Some(sum)
}

fn local_bregman_pythagorean_test(
    p: &[f64],
    q_star: &[f64],
    q: &[f64],
) -> Option<(f64, f64, f64)> {
    let kl_pq = local_manifold_kl_divergence(p, q)?;
    let kl_pqstar = local_manifold_kl_divergence(p, q_star)?;
    let kl_qstar_q = local_manifold_kl_divergence(q_star, q)?;
    Some((kl_pq, kl_pqstar, kl_qstar_q))
}

fn local_probability_hash(p: &[f64]) -> Option<u64> {
    if p.is_empty() || p.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let mut hash: u64 = 0xcbf29ce484222325;
    for &pi in p {
        let bits = (pi as f32).to_bits() as u64;
        hash ^= bits;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Some(hash)
}

/// Tiny fixed-seed offline bootstrap mean sketch (Host defaults to 1000 iterations).
fn local_bootstrap_mean_summary(
    values: &[f64],
    iterations: usize,
    seed: u64,
) -> Option<(f64, usize)> {
    let n = values.len();
    if n == 0 || iterations == 0 {
        return None;
    }
    let iters = iterations.min(256);
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    let mut sum_means = 0.0;
    for _ in 0..iters {
        let mut acc = 0.0;
        for _ in 0..n {
            // xorshift64*
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let idx = (state as usize) % n;
            acc += values[idx];
        }
        sum_means += acc / n as f64;
    }
    Some((sum_means / iters as f64, iters))
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: serde_json::Value,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        &format!("Running {cap_id}…"),
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_invoke(cap_id, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(cap_id, &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

fn need_numbers(document: &Document, label: &str) -> Option<Vec<f64>> {
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    if values.is_empty() {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document that contains numbers.",
            "error",
        );
        return None;
    }
    Some(values)
}

/// `Statistics.sum`
pub(super) fn run_sheet_sum(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some(value) = local_sum(&values) else {
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.sum",
        format!("sum of {} values: {value}", values.len()),
        json!({ "values": values }),
    );
}

/// `Statistics.skewness`
pub(super) fn run_sheet_skewness(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some(value) = local_skewness(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document that contains numbers.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.skewness",
        format!("skewness of {} values: {value:.6}", values.len()),
        json!({ "values": values }),
    );
}

/// `Statistics.kurtosis`
pub(super) fn run_sheet_kurtosis(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some(value) = local_kurtosis(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document that contains numbers.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.kurtosis",
        format!("excess kurtosis of {} values: {value:.6}", values.len()),
        json!({ "values": values }),
    );
}

/// `Statistics.quantile` — `data-q` (default 0.95).
pub(super) fn run_sheet_quantile(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let q = numeric_attr(selected_container(document).as_ref(), "data-q").unwrap_or(0.95);
    let Some(value) = local_quantile(&values, q) else {
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.quantile",
        format!("q={q} of {} values: {value}", values.len()),
        json!({ "values": values, "q": q }),
    );
}

/// `Statistics.iqr`
pub(super) fn run_sheet_iqr(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some(value) = local_iqr(&values) else {
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.iqr",
        format!("IQR of {} values: {value}", values.len()),
        json!({ "values": values }),
    );
}

/// `Statistics.mode`
pub(super) fn run_sheet_mode(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((value, count)) = local_mode(&values) else {
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.mode",
        format!("mode of {} values: {value} (count {count})", values.len()),
        json!({ "values": values }),
    );
}

/// `Statistics.trimmed_mean` — `data-proportion` (default 0.1).
pub(super) fn run_sheet_trimmed_mean(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let proportion =
        numeric_attr(selected_container(document).as_ref(), "data-proportion").unwrap_or(0.1);
    let Some(value) = local_trimmed_mean(&values, proportion) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need numbers and data-proportion in [0, 0.5).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.trimmed_mean",
        format!(
            "trimmed mean (p={proportion}) of {} values: {value}",
            values.len()
        ),
        json!({ "values": values, "proportion": proportion }),
    );
}

/// `Statistics.median_abs_deviation` — scaled by default (`data-scaled=0` to disable).
pub(super) fn run_sheet_mad(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let scaled = numeric_attr(selected_container(document).as_ref(), "data-scaled")
        .map(|v| v != 0.0)
        .unwrap_or(true);
    let Some(value) = local_mad(&values, scaled) else {
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.median_abs_deviation",
        format!(
            "MAD (scaled={scaled}) of {} values: {value}",
            values.len()
        ),
        json!({ "values": values, "scaled": scaled }),
    );
}

/// `Statistics.pearson` — first half vs second half of sheet numbers.
pub(super) fn run_sheet_pearson(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((x, y)) = split_pair_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least four numbers (two equal-length series halves).",
            "error",
        );
        return;
    };
    let Some(r) = local_pearson(&x, &y) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Pearson undefined for these series.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.pearson",
        format!("Pearson r over {} paired values: {r:.6}", x.len()),
        json!({ "x": x, "y": y }),
    );
}

/// `Statistics.covariance` — first half vs second half; sample by default.
pub(super) fn run_sheet_covariance(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((x, y)) = split_pair_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least four numbers (two equal-length series halves).",
            "error",
        );
        return;
    };
    let Some(cov) = local_covariance(&x, &y, true) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Covariance undefined for these series.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.covariance",
        format!("sample covariance over {} pairs: {cov:.6}", x.len()),
        json!({ "x": x, "y": y, "sample": true }),
    );
}

/// `Statistics.z_score_outliers` — `data-threshold` (default 3.0).
pub(super) fn run_sheet_z_score_outliers(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let threshold =
        numeric_attr(selected_container(document).as_ref(), "data-threshold").unwrap_or(3.0);
    let Some(count) = local_z_score_outlier_count(&values, threshold) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least two numbers with non-zero spread.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.z_score_outliers",
        format!(
            "z-score outliers (|z|>{threshold}) among {} values: {count}",
            values.len()
        ),
        json!({ "values": values, "threshold": threshold }),
    );
}

/// `Statistics.argmax`
pub(super) fn run_sheet_argmax(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((index, value)) = local_argmax(&values) else {
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.argmax",
        format!("argmax of {} values: index {index} (value {value})", values.len()),
        json!({ "values": values }),
    );
}

/// `Statistics.binomial_pmf` — `data-k` / `data-n` / `data-p` (defaults: first number, 10, 0.5).
pub(super) fn run_sheet_binomial_pmf(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let k = numeric_attr(container.as_ref(), "data-k")
        .or_else(|| values.first().copied())
        .unwrap_or(0.0)
        .floor()
        .max(0.0) as u32;
    let n = numeric_attr(container.as_ref(), "data-n")
        .or_else(|| values.get(1).copied())
        .unwrap_or(10.0)
        .floor()
        .max(0.0) as u32;
    let p = numeric_attr(container.as_ref(), "data-p")
        .or_else(|| values.get(2).copied())
        .unwrap_or(0.5);
    let Some(pmf) = local_binomial_pmf(k, n, p) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need valid binomial params (k≤n, p in [0,1]; optional data-k/data-n/data-p).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.binomial_pmf",
        format!("Binomial PMF P(K={k}|n={n},p={p}): {pmf:.6}"),
        json!({ "k": k, "n": n, "p": p }),
    );
}

/// `Statistics.binomial_cdf` — same param sources as binomial_pmf.
pub(super) fn run_sheet_binomial_cdf(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let k = numeric_attr(container.as_ref(), "data-k")
        .or_else(|| values.first().copied())
        .unwrap_or(0.0)
        .floor()
        .max(0.0) as u32;
    let n = numeric_attr(container.as_ref(), "data-n")
        .or_else(|| values.get(1).copied())
        .unwrap_or(10.0)
        .floor()
        .max(0.0) as u32;
    let p = numeric_attr(container.as_ref(), "data-p")
        .or_else(|| values.get(2).copied())
        .unwrap_or(0.5);
    let Some(cdf) = local_binomial_cdf(k, n, p) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need valid binomial params (p in [0,1]; optional data-k/data-n/data-p).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.binomial_cdf",
        format!("Binomial CDF P(K≤{k}|n={n},p={p}): {cdf:.6}"),
        json!({ "k": k, "n": n, "p": p }),
    );
}

/// `Statistics.beta_pdf` — x from first sheet number; `data-alpha` / `data-beta` (default 2,2).
pub(super) fn run_sheet_beta_pdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(2.0);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(2.0);
    let Some(pdf) = local_beta_pdf(x, alpha, beta) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need x in (0,1) and positive alpha/beta (optional data-alpha/data-beta).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.beta_pdf",
        format!("Beta PDF at x={x} (α={alpha}, β={beta}): {pdf:.6}"),
        json!({ "x": x, "alpha": alpha, "beta": beta }),
    );
}

/// `Statistics.chi_squared_pdf` — x from first number; `data-k` (default 2).
pub(super) fn run_sheet_chi_squared_pdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let x = values[0];
    let k = numeric_attr(selected_container(document).as_ref(), "data-k")
        .or_else(|| values.get(1).copied())
        .unwrap_or(2.0);
    let Some(pdf) = local_chi_squared_pdf(x, k) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need non-negative x and positive df (optional data-k).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.chi_squared_pdf",
        format!("χ² PDF at x={x} (k={k}): {pdf:.6}"),
        json!({ "x": x, "k": k }),
    );
}

/// `Statistics.chi_squared_cdf` — x from first number; `data-k` (default 2).
pub(super) fn run_sheet_chi_squared_cdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let x = values[0];
    let k = numeric_attr(selected_container(document).as_ref(), "data-k")
        .or_else(|| values.get(1).copied())
        .unwrap_or(2.0);
    let Some(cdf) = local_chi_squared_cdf(x, k) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need x and positive df (optional data-k).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.chi_squared_cdf",
        format!("χ² CDF at x={x} (k={k}): {cdf:.6}"),
        json!({ "x": x, "k": k }),
    );
}

/// `Statistics.chi_squared_quantile` — p from first number or `data-p`; `data-k` (default 2).
pub(super) fn run_sheet_chi_squared_quantile(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .or_else(|| values.first().copied())
        .unwrap_or(0.95);
    let k = numeric_attr(container.as_ref(), "data-k")
        .or_else(|| values.get(1).copied())
        .unwrap_or(2.0);
    let Some(q) = local_chi_squared_quantile(p, k) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need p in (0,1) and positive df (optional data-p/data-k).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.chi_squared_quantile",
        format!("χ² quantile p={p} (k={k}): {q:.6}"),
        json!({ "p": p, "k": k }),
    );
}

/// `Statistics.autocorrelation` — `data-lag` (default 1).
pub(super) fn run_sheet_autocorrelation(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let lag = numeric_attr(selected_container(document).as_ref(), "data-lag")
        .unwrap_or(1.0)
        .floor()
        .max(0.0) as usize;
    let Some(r) = local_autocorrelation(&values, lag) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a non-constant series longer than lag (optional data-lag).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.autocorrelation",
        format!("autocorrelation lag={lag} over {} values: {r:.6}", values.len()),
        json!({ "values": values, "lag": lag }),
    );
}

/// `Statistics.bootstrap_means` — `data-iterations` (default 64 local / Host 1000) and `data-seed`.
pub(super) fn run_sheet_bootstrap_means(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let iterations = numeric_attr(container.as_ref(), "data-iterations")
        .unwrap_or(64.0)
        .floor()
        .max(1.0) as u64;
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .unwrap_or(42.0)
        .floor()
        .max(0.0) as u64;
    let Some((mean_of_means, local_iters)) =
        local_bootstrap_mean_summary(&values, iterations as usize, seed)
    else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least one number for bootstrap means.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.bootstrap_means",
        format!(
            "bootstrap mean sketch ({local_iters} of {iterations} iters, seed={seed}) over {} values: {mean_of_means:.6}",
            values.len()
        ),
        json!({ "values": values, "iterations": iterations, "seed": seed }),
    );
}

/// `Statistics.normal_pdf` — x from first number; `data-mu` / `data-sigma` (default 0, 1).
pub(super) fn run_sheet_normal_pdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let mu = numeric_attr(container.as_ref(), "data-mu")
        .or_else(|| values.get(1).copied())
        .unwrap_or(0.0);
    let sigma = numeric_attr(container.as_ref(), "data-sigma")
        .or_else(|| values.get(2).copied())
        .unwrap_or(1.0);
    let Some(pdf) = local_normal_pdf(x, mu, sigma) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x and positive sigma (optional data-mu/data-sigma).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.normal_pdf",
        format!("Normal PDF at x={x} (μ={mu}, σ={sigma}): {pdf:.6}"),
        json!({ "x": x, "mu": mu, "sigma": sigma }),
    );
}

/// `Statistics.normal_cdf` — same param sources as normal_pdf.
pub(super) fn run_sheet_normal_cdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let mu = numeric_attr(container.as_ref(), "data-mu")
        .or_else(|| values.get(1).copied())
        .unwrap_or(0.0);
    let sigma = numeric_attr(container.as_ref(), "data-sigma")
        .or_else(|| values.get(2).copied())
        .unwrap_or(1.0);
    let Some(cdf) = local_normal_cdf(x, mu, sigma) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x and positive sigma (optional data-mu/data-sigma).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.normal_cdf",
        format!("Normal CDF at x={x} (μ={mu}, σ={sigma}): {cdf:.6}"),
        json!({ "x": x, "mu": mu, "sigma": sigma }),
    );
}

/// `Statistics.normal_quantile` — p from first number or `data-p`; `data-mu` / `data-sigma`.
pub(super) fn run_sheet_normal_quantile(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .or_else(|| values.first().copied())
        .unwrap_or(0.975);
    let mu = numeric_attr(container.as_ref(), "data-mu")
        .or_else(|| values.get(1).copied())
        .unwrap_or(0.0);
    let sigma = numeric_attr(container.as_ref(), "data-sigma")
        .or_else(|| values.get(2).copied())
        .unwrap_or(1.0);
    let Some(q) = local_normal_quantile(p, mu, sigma) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need p in (0,1) and positive sigma (optional data-p/data-mu/data-sigma).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.normal_quantile",
        format!("Normal quantile p={p} (μ={mu}, σ={sigma}): {q:.6}"),
        json!({ "p": p, "mu": mu, "sigma": sigma }),
    );
}

/// `Statistics.standard_normal_cdf` — z from first sheet number or `data-z`.
pub(super) fn run_sheet_standard_normal_cdf(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let z = numeric_attr(container.as_ref(), "data-z")
        .or_else(|| values.first().copied())
        .unwrap_or(0.0);
    if !z.is_finite() {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a finite z (first sheet number or data-z).",
            "error",
        );
        return;
    }
    let cdf = local_standard_normal_cdf(z);
    invoke_dual(
        document,
        label,
        "Statistics.standard_normal_cdf",
        format!("Standard Normal CDF Φ({z}): {cdf:.6}"),
        json!({ "z": z }),
    );
}

/// `Statistics.poisson_pmf` — `data-k` / `data-lambda` (defaults: first/second numbers, or 1 / 1).
pub(super) fn run_sheet_poisson_pmf(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let k = numeric_attr(container.as_ref(), "data-k")
        .or_else(|| values.first().copied())
        .unwrap_or(1.0)
        .floor()
        .max(0.0) as u32;
    let lambda = numeric_attr(container.as_ref(), "data-lambda")
        .or_else(|| values.get(1).copied())
        .unwrap_or(1.0);
    let Some(pmf) = local_poisson_pmf(k, lambda) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need k≥0 and λ≥0 (optional data-k/data-lambda).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.poisson_pmf",
        format!("Poisson PMF P(K={k}|λ={lambda}): {pmf:.6}"),
        json!({ "k": k, "lambda": lambda }),
    );
}

/// `Statistics.poisson_cdf` — same param sources as poisson_pmf.
pub(super) fn run_sheet_poisson_cdf(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let k = numeric_attr(container.as_ref(), "data-k")
        .or_else(|| values.first().copied())
        .unwrap_or(1.0)
        .floor()
        .max(0.0) as u32;
    let lambda = numeric_attr(container.as_ref(), "data-lambda")
        .or_else(|| values.get(1).copied())
        .unwrap_or(1.0);
    let Some(cdf) = local_poisson_cdf(k, lambda) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need k≥0 and λ≥0 (optional data-k/data-lambda).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.poisson_cdf",
        format!("Poisson CDF P(K≤{k}|λ={lambda}): {cdf:.6}"),
        json!({ "k": k, "lambda": lambda }),
    );
}

/// `Statistics.exponential_pdf` — x from first number; `data-rate` (default 1).
pub(super) fn run_sheet_exponential_pdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let x = values[0];
    let rate = numeric_attr(selected_container(document).as_ref(), "data-rate")
        .or_else(|| values.get(1).copied())
        .unwrap_or(1.0);
    let Some(pdf) = local_exponential_pdf(x, rate) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x and positive rate (optional data-rate).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.exponential_pdf",
        format!("Exponential PDF at x={x} (rate={rate}): {pdf:.6}"),
        json!({ "x": x, "rate": rate }),
    );
}

/// `Statistics.exponential_cdf` — same param sources as exponential_pdf.
pub(super) fn run_sheet_exponential_cdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let x = values[0];
    let rate = numeric_attr(selected_container(document).as_ref(), "data-rate")
        .or_else(|| values.get(1).copied())
        .unwrap_or(1.0);
    let Some(cdf) = local_exponential_cdf(x, rate) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x and positive rate (optional data-rate).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.exponential_cdf",
        format!("Exponential CDF at x={x} (rate={rate}): {cdf:.6}"),
        json!({ "x": x, "rate": rate }),
    );
}

/// `Statistics.spearman` — first half vs second half of sheet numbers.
pub(super) fn run_sheet_spearman(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((x, y)) = split_pair_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least four numbers (two equal-length series halves).",
            "error",
        );
        return;
    };
    let Some(r) = local_spearman(&x, &y) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Spearman undefined for these series.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.spearman",
        format!("Spearman ρ over {} paired values: {r:.6}", x.len()),
        json!({ "x": x, "y": y }),
    );
}

/// `Statistics.kendall` — first half vs second half of sheet numbers.
pub(super) fn run_sheet_kendall(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((x, y)) = split_pair_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least four numbers (two equal-length series halves).",
            "error",
        );
        return;
    };
    let Some(tau) = local_kendall(&x, &y) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Kendall τ undefined for these series.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.kendall",
        format!("Kendall τ over {} paired values: {tau:.6}", x.len()),
        json!({ "x": x, "y": y }),
    );
}

/// `Statistics.winsorized_mean` — `data-proportion` (default 0.1).
pub(super) fn run_sheet_winsorized_mean(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let proportion =
        numeric_attr(selected_container(document).as_ref(), "data-proportion").unwrap_or(0.1);
    let Some(value) = local_winsorized_mean(&values, proportion) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need numbers and data-proportion in [0, 0.5).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.winsorized_mean",
        format!(
            "winsorized mean (p={proportion}) of {} values: {value}",
            values.len()
        ),
        json!({ "values": values, "proportion": proportion }),
    );
}

/// `Statistics.erf` — x from first sheet number or `data-x`.
pub(super) fn run_sheet_erf(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let x = numeric_attr(container.as_ref(), "data-x")
        .or_else(|| values.first().copied())
        .unwrap_or(0.0);
    if !x.is_finite() {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a finite x (first sheet number or data-x).",
            "error",
        );
        return;
    }
    let v = local_erf_approx(x);
    invoke_dual(
        document,
        label,
        "Statistics.erf",
        format!("erf({x}): {v:.6}"),
        json!({ "x": x }),
    );
}

/// `Statistics.erfc` — x from first sheet number or `data-x`.
pub(super) fn run_sheet_erfc(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let x = numeric_attr(container.as_ref(), "data-x")
        .or_else(|| values.first().copied())
        .unwrap_or(0.0);
    if !x.is_finite() {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a finite x (first sheet number or data-x).",
            "error",
        );
        return;
    }
    let v = local_erfc_approx(x);
    invoke_dual(
        document,
        label,
        "Statistics.erfc",
        format!("erfc({x}): {v:.6}"),
        json!({ "x": x }),
    );
}

/// `Statistics.uniform_pdf` — x from first number; `data-a` / `data-b` (default 0, 1).
pub(super) fn run_sheet_uniform_pdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let a = numeric_attr(container.as_ref(), "data-a")
        .or_else(|| values.get(1).copied())
        .unwrap_or(0.0);
    let b = numeric_attr(container.as_ref(), "data-b")
        .or_else(|| values.get(2).copied())
        .unwrap_or(1.0);
    let Some(pdf) = local_uniform_pdf(x, a, b) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x and a < b (optional data-a/data-b).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.uniform_pdf",
        format!("Uniform PDF at x={x} on [{a}, {b}]: {pdf:.6}"),
        json!({ "x": x, "a": a, "b": b }),
    );
}

/// `Statistics.laplace_pdf` — x from first number; `data-mu` / `data-b` (default 0, 1).
pub(super) fn run_sheet_laplace_pdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let mu = numeric_attr(container.as_ref(), "data-mu")
        .or_else(|| values.get(1).copied())
        .unwrap_or(0.0);
    let b = numeric_attr(container.as_ref(), "data-b")
        .or_else(|| values.get(2).copied())
        .unwrap_or(1.0);
    let Some(pdf) = local_laplace_pdf(x, mu, b) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x and positive b (optional data-mu/data-b).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.laplace_pdf",
        format!("Laplace PDF at x={x} (μ={mu}, b={b}): {pdf:.6}"),
        json!({ "x": x, "mu": mu, "b": b }),
    );
}

/// `Statistics.standard_pdf` — z from first sheet number or `data-z`.
pub(super) fn run_sheet_standard_pdf(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let z = numeric_attr(container.as_ref(), "data-z")
        .or_else(|| values.first().copied())
        .unwrap_or(0.0);
    let Some(pdf) = local_standard_pdf(z) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a finite z (first sheet number or data-z).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.standard_pdf",
        format!("Standard Normal PDF φ({z}): {pdf:.6}"),
        json!({ "z": z }),
    );
}

/// `Statistics.uniform_cdf` — x from first number; `data-a` / `data-b` (default 0, 1).
pub(super) fn run_sheet_uniform_cdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let a = numeric_attr(container.as_ref(), "data-a")
        .or_else(|| values.get(1).copied())
        .unwrap_or(0.0);
    let b = numeric_attr(container.as_ref(), "data-b")
        .or_else(|| values.get(2).copied())
        .unwrap_or(1.0);
    let Some(cdf) = local_uniform_cdf(x, a, b) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x and a < b (optional data-a/data-b).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.uniform_cdf",
        format!("Uniform CDF at x={x} on [{a}, {b}]: {cdf:.6}"),
        json!({ "x": x, "a": a, "b": b }),
    );
}

/// `Statistics.laplace_cdf` — x from first number; `data-mu` / `data-b` (default 0, 1).
pub(super) fn run_sheet_laplace_cdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let mu = numeric_attr(container.as_ref(), "data-mu")
        .or_else(|| values.get(1).copied())
        .unwrap_or(0.0);
    let b = numeric_attr(container.as_ref(), "data-b")
        .or_else(|| values.get(2).copied())
        .unwrap_or(1.0);
    let Some(cdf) = local_laplace_cdf(x, mu, b) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x and positive b (optional data-mu/data-b).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.laplace_cdf",
        format!("Laplace CDF at x={x} (μ={mu}, b={b}): {cdf:.6}"),
        json!({ "x": x, "mu": mu, "b": b }),
    );
}

/// `Statistics.lognormal_pdf` — x from first number; `data-mu` / `data-sigma` (default 0, 1).
pub(super) fn run_sheet_lognormal_pdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let mu = numeric_attr(container.as_ref(), "data-mu")
        .or_else(|| values.get(1).copied())
        .unwrap_or(0.0);
    let sigma = numeric_attr(container.as_ref(), "data-sigma")
        .or_else(|| values.get(2).copied())
        .unwrap_or(1.0);
    let Some(pdf) = local_lognormal_pdf(x, mu, sigma) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need positive x and positive sigma (optional data-mu/data-sigma).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.lognormal_pdf",
        format!("Lognormal PDF at x={x} (μ={mu}, σ={sigma}): {pdf:.6}"),
        json!({ "x": x, "mu": mu, "sigma": sigma }),
    );
}

/// `Statistics.lognormal_cdf` — x from first number; `data-mu` / `data-sigma` (default 0, 1).
pub(super) fn run_sheet_lognormal_cdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let mu = numeric_attr(container.as_ref(), "data-mu")
        .or_else(|| values.get(1).copied())
        .unwrap_or(0.0);
    let sigma = numeric_attr(container.as_ref(), "data-sigma")
        .or_else(|| values.get(2).copied())
        .unwrap_or(1.0);
    let Some(cdf) = local_lognormal_cdf(x, mu, sigma) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x≥0 and positive sigma (optional data-mu/data-sigma).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.lognormal_cdf",
        format!("Lognormal CDF at x={x} (μ={mu}, σ={sigma}): {cdf:.6}"),
        json!({ "x": x, "mu": mu, "sigma": sigma }),
    );
}

/// `Statistics.standard_quantile` — p from first sheet number or `data-p` (default 0.975).
pub(super) fn run_sheet_standard_quantile(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .or_else(|| values.first().copied())
        .unwrap_or(0.975);
    let Some(z) = local_standard_quantile(p) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need p in (0, 1) (first sheet number or data-p).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.standard_quantile",
        format!("Standard Normal quantile Φ⁻¹({p}): {z:.6}"),
        json!({ "p": p }),
    );
}

/// `Statistics.ln_gamma` — x from first sheet number or `data-x`.
pub(super) fn run_sheet_ln_gamma(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let x = numeric_attr(container.as_ref(), "data-x")
        .or_else(|| values.first().copied())
        .unwrap_or(1.0);
    let Some(v) = local_ln_gamma_stirling(x) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need positive finite x (first sheet number or data-x).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.ln_gamma",
        format!("ln Γ({x}): {v:.6}"),
        json!({ "x": x }),
    );
}

/// `Statistics.gamma_fn` — x from first sheet number or `data-x`.
pub(super) fn run_sheet_gamma_fn(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let x = numeric_attr(container.as_ref(), "data-x")
        .or_else(|| values.first().copied())
        .unwrap_or(1.0);
    let Some(v) = local_gamma_fn(x) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need positive finite x (first sheet number or data-x).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.gamma_fn",
        format!("Γ({x}): {v:.6}"),
        json!({ "x": x }),
    );
}

/// `Statistics.weibull_pdf` — x from first number; `data-shape` / `data-scale` (default 1, 1).
pub(super) fn run_sheet_weibull_pdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let shape = numeric_attr(container.as_ref(), "data-shape")
        .or_else(|| values.get(1).copied())
        .unwrap_or(1.0);
    let scale = numeric_attr(container.as_ref(), "data-scale")
        .or_else(|| values.get(2).copied())
        .unwrap_or(1.0);
    let Some(pdf) = local_weibull_pdf(x, shape, scale) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x and positive shape/scale (optional data-shape/data-scale).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.weibull_pdf",
        format!("Weibull PDF at x={x} (shape={shape}, scale={scale}): {pdf:.6}"),
        json!({ "x": x, "shape": shape, "scale": scale }),
    );
}

/// `Statistics.gamma_pdf` — x from first number; `data-shape` / `data-scale` (default 1, 1).
pub(super) fn run_sheet_gamma_pdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let shape = numeric_attr(container.as_ref(), "data-shape")
        .or_else(|| values.get(1).copied())
        .unwrap_or(1.0);
    let scale = numeric_attr(container.as_ref(), "data-scale")
        .or_else(|| values.get(2).copied())
        .unwrap_or(1.0);
    let Some(pdf) = local_gamma_pdf(x, shape, scale) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x and positive shape/scale (optional data-shape/data-scale).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.gamma_pdf",
        format!("Gamma PDF at x={x} (shape={shape}, scale={scale}): {pdf:.6}"),
        json!({ "x": x, "shape": shape, "scale": scale }),
    );
}

/// `Statistics.two_sided_p` — z from first sheet number or `data-z`.
pub(super) fn run_sheet_two_sided_p(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let z = numeric_attr(container.as_ref(), "data-z")
        .or_else(|| values.first().copied())
        .unwrap_or(0.0);
    let Some(p) = local_two_sided_p(z) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a finite z (first sheet number or data-z).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.two_sided_p",
        format!("Two-sided p from z={z}: {p:.6}"),
        json!({ "z": z }),
    );
}

/// `Statistics.chi_squared_upper_p` — x from first number; `data-k` (default 2).
pub(super) fn run_sheet_chi_squared_upper_p(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let k = numeric_attr(container.as_ref(), "data-k")
        .or_else(|| values.get(1).copied())
        .unwrap_or(2.0);
    let Some(p) = local_chi_squared_upper_p(x, k) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x≥0 and positive k (optional data-k).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.chi_squared_upper_p",
        format!("χ² upper-tail p at x={x} (k={k}): {p:.6}"),
        json!({ "x": x, "k": k }),
    );
}

/// `Statistics.students_t_pdf` — t from first number; `data-nu` (default 10).
pub(super) fn run_sheet_students_t_pdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let t = values[0];
    let nu = numeric_attr(container.as_ref(), "data-nu")
        .or_else(|| values.get(1).copied())
        .unwrap_or(10.0);
    let Some(pdf) = local_students_t_pdf(t, nu) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite t and positive nu (optional data-nu).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.students_t_pdf",
        format!("Student's t PDF at t={t} (ν={nu}): {pdf:.6}"),
        json!({ "t": t, "nu": nu }),
    );
}

/// `Statistics.fisher_f_pdf` — x from first number; `data-d1` / `data-d2` (default 5, 10).
pub(super) fn run_sheet_fisher_f_pdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let d1 = numeric_attr(container.as_ref(), "data-d1")
        .or_else(|| values.get(1).copied())
        .unwrap_or(5.0);
    let d2 = numeric_attr(container.as_ref(), "data-d2")
        .or_else(|| values.get(2).copied())
        .unwrap_or(10.0);
    let Some(pdf) = local_fisher_f_pdf(x, d1, d2) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x and positive d1/d2 (optional data-d1/data-d2).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.fisher_f_pdf",
        format!("Fisher F PDF at x={x} (d1={d1}, d2={d2}): {pdf:.6}"),
        json!({ "x": x, "d1": d1, "d2": d2 }),
    );
}

/// `Statistics.gammp` — a from first number or `data-a`; x from second or `data-x`.
pub(super) fn run_sheet_gammp(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let a = numeric_attr(container.as_ref(), "data-a")
        .or_else(|| values.first().copied())
        .unwrap_or(1.0);
    let x = numeric_attr(container.as_ref(), "data-x")
        .or_else(|| values.get(1).copied())
        .unwrap_or(1.0);
    let Some(v) = local_gammp(a, x) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need positive a and finite x≥0 (data-a/data-x or first two numbers).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.gammp",
        format!("P({a}, {x}) = gammp: {v:.6}"),
        json!({ "a": a, "x": x }),
    );
}

/// `Statistics.gammq` — same param sources as gammp.
pub(super) fn run_sheet_gammq(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let a = numeric_attr(container.as_ref(), "data-a")
        .or_else(|| values.first().copied())
        .unwrap_or(1.0);
    let x = numeric_attr(container.as_ref(), "data-x")
        .or_else(|| values.get(1).copied())
        .unwrap_or(1.0);
    let Some(v) = local_gammq(a, x) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need positive a and finite x≥0 (data-a/data-x or first two numbers).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.gammq",
        format!("Q({a}, {x}) = gammq: {v:.6}"),
        json!({ "a": a, "x": x }),
    );
}

/// `Statistics.betai` — a/b/x from sheet numbers or `data-a` / `data-b` / `data-x`.
pub(super) fn run_sheet_betai(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let a = numeric_attr(container.as_ref(), "data-a")
        .or_else(|| values.first().copied())
        .unwrap_or(1.0);
    let b = numeric_attr(container.as_ref(), "data-b")
        .or_else(|| values.get(1).copied())
        .unwrap_or(1.0);
    let x = numeric_attr(container.as_ref(), "data-x")
        .or_else(|| values.get(2).copied())
        .unwrap_or(0.5);
    let Some(v) = local_betai(a, b, x) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need positive a,b and x in [0,1] (data-a/data-b/data-x or first three numbers).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.betai",
        format!("I_{x}({a}, {b}) = betai: {v:.6}"),
        json!({ "a": a, "b": b, "x": x }),
    );
}

/// `Statistics.students_t_cdf` — t from first number; `data-nu` (default 10).
pub(super) fn run_sheet_students_t_cdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let t = values[0];
    let nu = numeric_attr(container.as_ref(), "data-nu")
        .or_else(|| values.get(1).copied())
        .unwrap_or(10.0);
    let Some(cdf) = local_students_t_cdf(t, nu) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite t and positive nu (optional data-nu).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.students_t_cdf",
        format!("Student's t CDF at t={t} (ν={nu}): {cdf:.6}"),
        json!({ "t": t, "nu": nu }),
    );
}

/// `Statistics.students_t_two_sided_p` — t from first number; `data-nu` (default 10).
pub(super) fn run_sheet_students_t_two_sided_p(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let t = values[0];
    let nu = numeric_attr(container.as_ref(), "data-nu")
        .or_else(|| values.get(1).copied())
        .unwrap_or(10.0);
    let Some(p) = local_students_t_two_sided_p(t, nu) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite t and positive nu (optional data-nu).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.students_t_two_sided_p",
        format!("Student's t two-sided p at t={t} (ν={nu}): {p:.6}"),
        json!({ "t": t, "nu": nu }),
    );
}

/// `Statistics.students_t_upper_p` — t from first number; `data-nu` (default 10).
pub(super) fn run_sheet_students_t_upper_p(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let t = values[0];
    let nu = numeric_attr(container.as_ref(), "data-nu")
        .or_else(|| values.get(1).copied())
        .unwrap_or(10.0);
    let Some(p) = local_students_t_upper_p(t, nu) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite t and positive nu (optional data-nu).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.students_t_upper_p",
        format!("Student's t upper-tail p at t={t} (ν={nu}): {p:.6}"),
        json!({ "t": t, "nu": nu }),
    );
}

/// `Statistics.students_t_quantile` — p from first number or `data-p`; `data-nu` (default 10).
pub(super) fn run_sheet_students_t_quantile(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .or_else(|| values.first().copied())
        .unwrap_or(0.975);
    let nu = numeric_attr(container.as_ref(), "data-nu")
        .or_else(|| values.get(1).copied())
        .unwrap_or(10.0);
    let Some(q) = local_students_t_quantile(p, nu) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need p in (0,1) and positive nu (optional data-p/data-nu).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.students_t_quantile",
        format!("Student's t quantile p={p} (ν={nu}): {q:.6}"),
        json!({ "p": p, "nu": nu }),
    );
}

/// `Statistics.fisher_f_cdf` — x from first number; `data-d1` / `data-d2` (default 5, 10).
pub(super) fn run_sheet_fisher_f_cdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let d1 = numeric_attr(container.as_ref(), "data-d1")
        .or_else(|| values.get(1).copied())
        .unwrap_or(5.0);
    let d2 = numeric_attr(container.as_ref(), "data-d2")
        .or_else(|| values.get(2).copied())
        .unwrap_or(10.0);
    let Some(cdf) = local_fisher_f_cdf(x, d1, d2) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x and positive d1/d2 (optional data-d1/data-d2).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.fisher_f_cdf",
        format!("Fisher F CDF at x={x} (d1={d1}, d2={d2}): {cdf:.6}"),
        json!({ "x": x, "d1": d1, "d2": d2 }),
    );
}

/// `Statistics.fisher_f_upper_p` — x from first number; `data-d1` / `data-d2` (default 5, 10).
pub(super) fn run_sheet_fisher_f_upper_p(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = values[0];
    let d1 = numeric_attr(container.as_ref(), "data-d1")
        .or_else(|| values.get(1).copied())
        .unwrap_or(5.0);
    let d2 = numeric_attr(container.as_ref(), "data-d2")
        .or_else(|| values.get(2).copied())
        .unwrap_or(10.0);
    let Some(p) = local_fisher_f_upper_p(x, d1, d2) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x and positive d1/d2 (optional data-d1/data-d2).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.fisher_f_upper_p",
        format!("Fisher F upper-tail p at x={x} (d1={d1}, d2={d2}): {p:.6}"),
        json!({ "x": x, "d1": d1, "d2": d2 }),
    );
}

/// `Statistics.fisher_f_quantile` — p from first number or `data-p`; `data-d1` / `data-d2`.
pub(super) fn run_sheet_fisher_f_quantile(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .or_else(|| values.first().copied())
        .unwrap_or(0.95);
    let d1 = numeric_attr(container.as_ref(), "data-d1")
        .or_else(|| values.get(1).copied())
        .unwrap_or(5.0);
    let d2 = numeric_attr(container.as_ref(), "data-d2")
        .or_else(|| values.get(2).copied())
        .unwrap_or(10.0);
    let Some(q) = local_fisher_f_quantile(p, d1, d2) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need p in (0,1) and positive d1/d2 (optional data-p/data-d1/data-d2).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.fisher_f_quantile",
        format!("Fisher F quantile p={p} (d1={d1}, d2={d2}): {q:.6}"),
        json!({ "p": p, "d1": d1, "d2": d2 }),
    );
}

/// `Statistics.tukey_fences` — `data-k` (default 1.5).
pub(super) fn run_sheet_tukey_fences(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let k = numeric_attr(selected_container(document).as_ref(), "data-k").unwrap_or(1.5);
    let Some((lo, hi)) = local_tukey_fences(&values, k) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least four numbers and non-negative k (optional data-k).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.tukey_fences",
        format!(
            "Tukey fences (k={k}) over {} values: lower={lo}, upper={hi}",
            values.len()
        ),
        json!({ "values": values, "k": k }),
    );
}

/// `Statistics.empirical_cdf` — samples from sheet; `x` from `data-x` or last number.
pub(super) fn run_sheet_empirical_cdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let x = numeric_attr(container.as_ref(), "data-x")
        .or_else(|| values.last().copied())
        .unwrap_or(0.0);
    let samples = if numeric_attr(container.as_ref(), "data-x").is_some() || values.len() == 1 {
        values.clone()
    } else {
        values[..values.len() - 1].to_vec()
    };
    let Some(cdf) = local_empirical_cdf(&samples, x) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite samples and x (optional data-x).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.empirical_cdf",
        format!(
            "Empirical CDF at x={x} over {} samples: {cdf:.6}",
            samples.len()
        ),
        json!({ "samples": samples, "x": x }),
    );
}

/// `Statistics.entropy` — sheet numbers as an unnormalized probability mass.
pub(super) fn run_sheet_entropy(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some(h) = local_entropy(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a non-empty positive mass vector for entropy.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.entropy",
        format!("Shannon entropy over {} masses: {h:.6} bits", values.len()),
        json!({ "p": values }),
    );
}

/// `Statistics.kl_divergence` — first/second halves as p and q.
pub(super) fn run_sheet_kl_divergence(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((p, q)) = split_pair_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least four numbers (two equal-length series halves).",
            "error",
        );
        return;
    };
    let Some(d) = local_kl_divergence(&p, &q) else {
        super::interactions::show_tool_status(
            document,
            label,
            "KL divergence undefined for these distributions.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.kl_divergence",
        format!("KL(p‖q) over {} bins: {d:.6} bits", p.len()),
        json!({ "p": p, "q": q }),
    );
}

/// `Statistics.cross_entropy` — first/second halves as p and q.
pub(super) fn run_sheet_cross_entropy(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((p, q)) = split_pair_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least four numbers (two equal-length series halves).",
            "error",
        );
        return;
    };
    let Some(h) = local_cross_entropy(&p, &q) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Cross-entropy undefined for these distributions.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.cross_entropy",
        format!("H(p,q) over {} bins: {h:.6} bits", p.len()),
        json!({ "p": p, "q": q }),
    );
}

/// `Statistics.entropy_from_counts` — non-negative floored sheet numbers as counts.
pub(super) fn run_sheet_entropy_from_counts(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let counts: Vec<u64> = values
        .iter()
        .map(|&v| v.floor().max(0.0) as u64)
        .collect();
    let Some(h) = local_entropy_from_counts(&counts) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need non-empty non-negative counts for entropy_from_counts.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.entropy_from_counts",
        format!(
            "Entropy from {} counts: {h:.6} bits",
            counts.len()
        ),
        json!({ "counts": counts }),
    );
}

/// `Statistics.moving_average` — `data-window` (default 3); local sketch shows last MA.
pub(super) fn run_sheet_moving_average(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let window = numeric_attr(selected_container(document).as_ref(), "data-window")
        .unwrap_or(3.0)
        .floor()
        .max(1.0) as u64;
    let Some((last, count)) = local_moving_average_last(&values, window as usize) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need window in 1..=n (optional data-window).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.moving_average",
        format!(
            "Moving average (window={window}) over {} values: last={last:.6} ({count} points)",
            values.len()
        ),
        json!({ "values": values, "window": window }),
    );
}

/// `Statistics.modified_z_score_outliers` — `data-threshold` (default 3.5).
pub(super) fn run_sheet_modified_z_score_outliers(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let threshold =
        numeric_attr(selected_container(document).as_ref(), "data-threshold").unwrap_or(3.5);
    let Some(count) = local_modified_z_outlier_count(&values, threshold) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least two numbers with non-zero MAD.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.modified_z_score_outliers",
        format!(
            "Modified z-score outliers (|z|>{threshold}) among {} values: {count}",
            values.len()
        ),
        json!({ "values": values, "threshold": threshold }),
    );
}

/// `Statistics.iqr_outliers` — `data-k` (default 1.5).
pub(super) fn run_sheet_iqr_outliers(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let k = numeric_attr(selected_container(document).as_ref(), "data-k").unwrap_or(1.5);
    let Some(count) = local_iqr_outlier_count(&values, k) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least four numbers and non-negative k (optional data-k).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.iqr_outliers",
        format!(
            "IQR outliers (k={k}) among {} values: {count}",
            values.len()
        ),
        json!({ "values": values, "k": k }),
    );
}

/// `Statistics.exponential_smoothing` — `data-alpha` (default 0.3); local sketch shows last s_t.
pub(super) fn run_sheet_exponential_smoothing(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let alpha = numeric_attr(selected_container(document).as_ref(), "data-alpha").unwrap_or(0.3);
    let Some((last, n)) = local_exponential_smoothing_last(&values, alpha) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need numbers and data-alpha in (0, 1].",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.exponential_smoothing",
        format!("Exponential smoothing (α={alpha}) over {n} values: last={last:.6}"),
        json!({ "values": values, "alpha": alpha }),
    );
}

/// `Statistics.adf_proxy`
pub(super) fn run_sheet_adf_proxy(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some(stat) = local_adf_proxy(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least three numbers for ADF proxy.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.adf_proxy",
        format!(
            "ADF stationarity proxy over {} values: {stat:.6}",
            values.len()
        ),
        json!({ "series": values }),
    );
}

/// `Statistics.histogram` — `data-bins` (default 10).
pub(super) fn run_sheet_histogram(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let bins = numeric_attr(selected_container(document).as_ref(), "data-bins")
        .unwrap_or(10.0)
        .floor()
        .clamp(1.0, 256.0) as u64;
    let Some((occupied, min_v, max_v, width)) = local_histogram_summary(&values, bins as usize)
    else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need numbers and data-bins in 1..=256.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.histogram",
        format!(
            "Histogram (bins={bins}) over {} values: {occupied} occupied, [{min_v:.4},{max_v:.4}] width={width:.4}",
            values.len()
        ),
        json!({ "values": values, "bins": bins }),
    );
}

/// `Statistics.ks_1sample` — one-sample KS vs Uniform(0,1).
pub(super) fn run_sheet_ks_1sample(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((d, p)) = local_ks_1sample(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least one finite number for KS.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.ks_1sample",
        format!(
            "KS (Uniform[0,1]) over {} values: D={d:.6}, p≈{p:.6}",
            values.len()
        ),
        json!({ "values": values }),
    );
}

/// `Statistics.grubbs_test` — `data-alpha` (default 0.05).
pub(super) fn run_sheet_grubbs_test(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let alpha = numeric_attr(selected_container(document).as_ref(), "data-alpha").unwrap_or(0.05);
    let Some((index, g, crit, is_outlier)) = local_grubbs_test(&values, alpha) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need ≥3 numbers with non-zero spread and data-alpha in (0,1).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.grubbs_test",
        format!(
            "Grubbs (α={alpha}) over {} values: G={g:.4} crit={crit:.4} idx={index} outlier={is_outlier}",
            values.len()
        ),
        json!({ "values": values, "alpha": alpha }),
    );
}

/// `Statistics.one_sample_t` — `data-mu` (default 0).
pub(super) fn run_sheet_one_sample_t(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let mu = numeric_attr(selected_container(document).as_ref(), "data-mu").unwrap_or(0.0);
    let Some((t, p, df)) = local_one_sample_t(&values, mu) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least two numbers for one-sample t.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.one_sample_t",
        format!(
            "One-sample t (μ₀={mu}) over {} values: t={t:.4}, p={p:.6}, df={df}",
            values.len()
        ),
        json!({ "values": values, "mu": mu }),
    );
}

/// `Statistics.two_sample_t` — series halves; `data-equal-var` (default true).
pub(super) fn run_sheet_two_sample_t(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((a, b)) = split_pair_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least four numbers (two equal-length series halves).",
            "error",
        );
        return;
    };
    let equal_var = numeric_attr(selected_container(document).as_ref(), "data-equal-var")
        .map(|v| v != 0.0)
        .unwrap_or(true);
    let Some((t, p, df, diff)) = local_two_sample_t(&a, &b, equal_var) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need ≥2 numbers in each half for two-sample t.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.two_sample_t",
        format!(
            "Two-sample t (equal_var={equal_var}): t={t:.4}, p={p:.6}, df={df:.2}, Δμ={diff:.4}"
        ),
        json!({ "a": a, "b": b, "equal_var": equal_var }),
    );
}

/// `Statistics.paired_t` — series halves as paired observations.
pub(super) fn run_sheet_paired_t(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((a, b)) = split_pair_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least four numbers (two equal-length paired series halves).",
            "error",
        );
        return;
    };
    let Some((t, p, df)) = local_paired_t(&a, &b) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need ≥2 paired observations for paired t.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.paired_t",
        format!("Paired t over {} pairs: t={t:.4}, p={p:.6}, df={df}", a.len()),
        json!({ "a": a, "b": b }),
    );
}

/// `Statistics.linear_regression` — series halves as x and y.
pub(super) fn run_sheet_linear_regression(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((x, y)) = split_pair_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least six numbers (two equal-length series halves, n≥3).",
            "error",
        );
        return;
    };
    let Some((slope, intercept, r2, n)) = local_linear_regression(&x, &y) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need n≥3 with non-zero x variance for linear regression.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.linear_regression",
        format!("OLS (n={n}): y = {intercept:.4} + {slope:.4}·x, R²={r2:.4}"),
        json!({ "x": x, "y": y }),
    );
}

/// `Statistics.chi_square_gof` — series halves as observed / expected.
pub(super) fn run_sheet_chi_square_gof(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((observed, expected)) = split_pair_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least four numbers (observed then expected halves).",
            "error",
        );
        return;
    };
    let Some((stat, p, dof)) = local_chi_square_gof(&observed, &expected) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need matching halves with positive expected counts.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.chi_square_gof",
        format!("χ² GOF (k={}): χ²={stat:.4}, p={p:.6}, df={dof}", observed.len()),
        json!({ "observed": observed, "expected": expected }),
    );
}

/// `Statistics.chi_square_independence` — reshape sheet via `data-cols` (default 2).
pub(super) fn run_sheet_chi_square_independence(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let cols = numeric_attr(selected_container(document).as_ref(), "data-cols")
        .unwrap_or(2.0)
        .floor()
        .max(2.0) as usize;
    let Some(table) = reshape_table(&values, cols) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a ≥2×2 contingency table (optional data-cols).",
            "error",
        );
        return;
    };
    let Some((stat, p, dof)) = local_chi_square_independence(&table) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Contingency table invalid for χ² independence.",
            "error",
        );
        return;
    };
    let rows = table.len();
    invoke_dual(
        document,
        label,
        "Statistics.chi_square_independence",
        format!("χ² independence ({rows}×{cols}): χ²={stat:.4}, p={p:.6}, df={dof}"),
        json!({ "table": table }),
    );
}

/// `Statistics.correlation_p_value` — Pearson of halves, or `data-r` / `data-n`.
pub(super) fn run_sheet_correlation_p_value(document: &Document, label: &str) {
    let container = selected_container(document);
    let (r, n) = if let (Some(r), Some(n)) = (
        numeric_attr(container.as_ref(), "data-r"),
        numeric_attr(container.as_ref(), "data-n"),
    ) {
        (r, n.floor().max(0.0) as usize)
    } else {
        let Some(values) = need_numbers(document, label) else {
            return;
        };
        let Some((x, y)) = split_pair_series(&values) else {
            super::interactions::show_tool_status(
                document,
                label,
                "Need series halves or data-r and data-n (≥3).",
                "error",
            );
            return;
        };
        let Some(r) = local_pearson(&x, &y) else {
            super::interactions::show_tool_status(
                document,
                label,
                "Need n≥2 with finite Pearson r.",
                "error",
            );
            return;
        };
        (r, x.len())
    };
    let Some(p) = local_correlation_p_value(r, n) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need n≥3 for correlation p-value.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.correlation_p_value",
        format!("Correlation p-value (r={r:.4}, n={n}): p={p:.6}"),
        json!({ "r": r, "n": n as u64 }),
    );
}

/// `Statistics.friedman` — reshape via `data-treatments` (default 3).
pub(super) fn run_sheet_friedman(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let treatments = numeric_attr(selected_container(document).as_ref(), "data-treatments")
        .unwrap_or(3.0)
        .floor()
        .max(2.0) as usize;
    let Some(blocks) = split_blocks(&values, treatments) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need ≥2 blocks of ≥2 treatments (optional data-treatments).",
            "error",
        );
        return;
    };
    let Some((chi, p, df)) = local_friedman(&blocks) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Friedman input invalid.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.friedman",
        format!(
            "Friedman ({} blocks × {treatments}): χ²={chi:.4}, p={p:.6}, df={df}",
            blocks.len()
        ),
        json!({ "groups": blocks }),
    );
}

/// `Statistics.ljung_box` — ACF from sheet; `data-h` (default min(10, n-1)).
pub(super) fn run_sheet_ljung_box(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let n = values.len();
    if n < 3 {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least three numbers for Ljung–Box.",
            "error",
        );
        return;
    }
    let h_default = (n - 1).min(10) as f64;
    let h = numeric_attr(selected_container(document).as_ref(), "data-h")
        .unwrap_or(h_default)
        .floor()
        .clamp(1.0, (n - 1) as f64) as usize;
    let mut acf = Vec::with_capacity(h);
    for lag in 1..=h {
        let Some(r) = local_autocorrelation(&values, lag) else {
            super::interactions::show_tool_status(
                document,
                label,
                "Could not form ACF for Ljung–Box.",
                "error",
            );
            return;
        };
        acf.push(r);
    }
    let Some(q) = local_ljung_box(&acf, n, h) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need data-h in 1..=n-1 for Ljung–Box.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.ljung_box",
        format!("Ljung–Box (n={n}, h={h}): Q={q:.4}"),
        json!({ "acf": acf, "n": n as u64, "h": h as u64 }),
    );
}

/// `Statistics.mann_whitney_u` — series halves as independent samples.
pub(super) fn run_sheet_mann_whitney_u(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((x, y)) = split_pair_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least four numbers (two sample halves).",
            "error",
        );
        return;
    };
    let Some((u, p, n1, n2)) = local_mann_whitney_u(&x, &y) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need non-empty sample halves for Mann–Whitney U.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.mann_whitney_u",
        format!("Mann–Whitney U (n1={n1}, n2={n2}): U={u:.4}, p={p:.6}"),
        json!({ "x": x, "y": y }),
    );
}

/// `Statistics.mcnemar` — discordant counts from first two numbers or `data-b`/`data-c`.
pub(super) fn run_sheet_mcnemar(document: &Document, label: &str) {
    let container = selected_container(document);
    let (b, c) = if let (Some(b), Some(c)) = (
        numeric_attr(container.as_ref(), "data-b"),
        numeric_attr(container.as_ref(), "data-c"),
    ) {
        (b.floor().max(0.0) as u64, c.floor().max(0.0) as u64)
    } else {
        let Some(values) = need_numbers(document, label) else {
            return;
        };
        if values.len() < 2 {
            super::interactions::show_tool_status(
                document,
                label,
                "Need two non-negative counts (b, c) or data-b/data-c.",
                "error",
            );
            return;
        }
        (
            values[0].floor().max(0.0) as u64,
            values[1].floor().max(0.0) as u64,
        )
    };
    let Some((stat, p, dof)) = local_mcnemar(b, c) else {
        super::interactions::show_tool_status(
            document,
            label,
            "McNemar needs b + c > 0.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.mcnemar",
        format!("McNemar (b={b}, c={c}): χ²={stat:.4}, p={p:.6}, df={dof}"),
        json!({ "b": b, "c": c }),
    );
}

/// `Statistics.mutual_information` — floored series halves as discrete labels.
pub(super) fn run_sheet_mutual_information(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((xa, ya)) = split_pair_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need at least four numbers (two discrete label halves).",
            "error",
        );
        return;
    };
    let x: Vec<u64> = xa.iter().map(|&v| v.floor().max(0.0) as u64).collect();
    let y: Vec<u64> = ya.iter().map(|&v| v.floor().max(0.0) as u64).collect();
    let xs: Vec<usize> = x.iter().map(|&v| v as usize).collect();
    let ys: Vec<usize> = y.iter().map(|&v| v as usize).collect();
    let Some(mi) = local_mutual_information_discrete(&xs, &ys) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need matching non-empty discrete halves for MI.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.mutual_information",
        format!("Mutual information over {} pairs: {mi:.6} bits", x.len()),
        json!({ "x": x, "y": y }),
    );
}

/// `Statistics.one_way_anova` — equal groups via `data-groups` (default 2).
pub(super) fn run_sheet_one_way_anova(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let k = numeric_attr(selected_container(document).as_ref(), "data-groups")
        .unwrap_or(2.0)
        .floor()
        .max(2.0) as usize;
    let Some(groups) = split_k_groups(&values, k) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need ≥2 non-empty equal groups (optional data-groups).",
            "error",
        );
        return;
    };
    let Some((f, p, dfb, dfw)) = local_one_way_anova(&groups) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need total n > groups for one-way ANOVA.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.one_way_anova",
        format!("One-way ANOVA ({k} groups): F={f:.4}, p={p:.6}, df=({dfb},{dfw})"),
        json!({ "groups": groups }),
    );
}

/// `Statistics.mahalanobis_sq` — `data-dim` (default 2): x, mean, then inv_cov (or I).
pub(super) fn run_sheet_mahalanobis_sq(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let d = numeric_attr(selected_container(document).as_ref(), "data-dim")
        .unwrap_or(2.0)
        .floor()
        .max(1.0) as usize;
    let Some((x, mean, inv_cov)) = parse_mahalanobis_args(&values, d) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need ≥2·dim numbers (x then mean; optional inv_cov, else I).",
            "error",
        );
        return;
    };
    let Some(d2) = local_mahalanobis_sq(&x, &mean, &inv_cov) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Mahalanobis dimension mismatch.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.mahalanobis_sq",
        format!("Mahalanobis² (dim={d}): {d2:.6}"),
        json!({ "x": x, "mean": mean, "inv_cov": inv_cov }),
    );
}

/// `Statistics.mvn_log_pdf` — `data-dim` (default 2): x, mean, then cov (or I).
pub(super) fn run_sheet_mvn_log_pdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let p = numeric_attr(selected_container(document).as_ref(), "data-dim")
        .unwrap_or(2.0)
        .floor()
        .max(1.0) as usize;
    let Some((x, mean, cov)) = parse_mvn_x_mean_cov(&values, p) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need ≥2·dim numbers (x then mean; optional cov, else I).",
            "error",
        );
        return;
    };
    let Some(lp) = local_mvn_log_pdf(&x, &mean, &cov, p) else {
        super::interactions::show_tool_status(
            document,
            label,
            "MVN log-pdf: shape mismatch or non-PD covariance.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.mvn_log_pdf",
        format!("MVN log-pdf (p={p}): {lp:.6}"),
        json!({ "x": x, "mean": mean, "cov": cov, "p": p as u64 }),
    );
}

/// `Statistics.mvn_pdf` — same layout as mvn_log_pdf.
pub(super) fn run_sheet_mvn_pdf(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let p = numeric_attr(selected_container(document).as_ref(), "data-dim")
        .unwrap_or(2.0)
        .floor()
        .max(1.0) as usize;
    let Some((x, mean, cov)) = parse_mvn_x_mean_cov(&values, p) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need ≥2·dim numbers (x then mean; optional cov, else I).",
            "error",
        );
        return;
    };
    let Some(dens) = local_mvn_pdf(&x, &mean, &cov, p) else {
        super::interactions::show_tool_status(
            document,
            label,
            "MVN pdf: shape mismatch or non-PD covariance.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.mvn_pdf",
        format!("MVN pdf (p={p}): {dens:.6}"),
        json!({ "x": x, "mean": mean, "cov": cov, "p": p as u64 }),
    );
}

/// `Statistics.mvn_sample` — `data-dim` / `data-seed`: mean then cov (or I).
pub(super) fn run_sheet_mvn_sample(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let p = numeric_attr(container.as_ref(), "data-dim")
        .unwrap_or(2.0)
        .floor()
        .max(1.0) as usize;
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .unwrap_or(0.0)
        .floor()
        .max(0.0) as u64;
    let Some((mean, cov)) = parse_mvn_mean_cov(&values, p) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need ≥dim numbers (mean; optional cov, else I).",
            "error",
        );
        return;
    };
    let Some(sample) = local_mvn_sample(&mean, &cov, p, seed) else {
        super::interactions::show_tool_status(
            document,
            label,
            "MVN sample: shape mismatch or non-PD covariance.",
            "error",
        );
        return;
    };
    let preview: String = sample
        .iter()
        .take(4)
        .map(|v| format!("{v:.4}"))
        .collect::<Vec<_>>()
        .join(", ");
    invoke_dual(
        document,
        label,
        "Statistics.mvn_sample",
        format!("MVN sample (p={p}, seed={seed}): [{preview}]"),
        json!({ "mean": mean, "cov": cov, "p": p as u64, "seed": seed }),
    );
}

/// `Statistics.mvn_mle` — row-major `n×p` via `data-dim` / optional `data-n`.
pub(super) fn run_sheet_mvn_mle(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let container = selected_container(document);
    let p = numeric_attr(container.as_ref(), "data-dim")
        .unwrap_or(2.0)
        .floor()
        .max(1.0) as usize;
    let n = numeric_attr(container.as_ref(), "data-n")
        .map(|v| v.floor().max(2.0) as usize)
        .unwrap_or_else(|| {
            if p == 0 {
                0
            } else {
                values.len() / p
            }
        });
    if n < 2 || values.len() < n * p {
        super::interactions::show_tool_status(
            document,
            label,
            "Need n≥2 rows of length dim (optional data-dim/data-n).",
            "error",
        );
        return;
    }
    let data = values[..n * p].to_vec();
    let Some((mean, _cov)) = local_mvn_mle(&data, n, p) else {
        super::interactions::show_tool_status(
            document,
            label,
            "MVN MLE: insufficient data or shape mismatch.",
            "error",
        );
        return;
    };
    let mean_preview: String = mean
        .iter()
        .take(4)
        .map(|v| format!("{v:.4}"))
        .collect::<Vec<_>>()
        .join(", ");
    invoke_dual(
        document,
        label,
        "Statistics.mvn_mle",
        format!("MVN MLE (n={n}, p={p}) mean≈[{mean_preview}]"),
        json!({ "data": data, "n": n as u64, "p": p as u64 }),
    );
}

/// `Statistics.validate_probability` — sheet numbers as a probability mass.
pub(super) fn run_sheet_validate_probability(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    if !local_validate_probability(&values) {
        super::interactions::show_tool_status(
            document,
            label,
            "Need non-negative masses that sum to 1 (±1e-6).",
            "error",
        );
        return;
    }
    invoke_dual(
        document,
        label,
        "Statistics.validate_probability",
        format!("Probability mass of {} entries validates (sums to 1).", values.len()),
        json!({ "p": values }),
    );
}

/// `Statistics.simplex_project` — Euclidean projection onto the simplex.
pub(super) fn run_sheet_simplex_project(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some(q) = local_simplex_project(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a non-empty finite mass vector with positive clipped sum.",
            "error",
        );
        return;
    };
    let preview: String = q
        .iter()
        .take(4)
        .map(|v| format!("{v:.4}"))
        .collect::<Vec<_>>()
        .join(", ");
    invoke_dual(
        document,
        label,
        "Statistics.simplex_project",
        format!("Simplex projection ({}): [{preview}]", q.len()),
        json!({ "p": values }),
    );
}

/// `Statistics.fisher_distance` — first/second halves as simplex points p, q.
pub(super) fn run_sheet_fisher_distance(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((p, q)) = split_pair_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need an even count (≥2) of numbers (p then q halves).",
            "error",
        );
        return;
    };
    let Some(d) = local_fisher_distance(&p, &q) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Fisher distance needs equal-length finite masses.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.fisher_distance",
        format!("Fisher–Rao distance over {}-simplex: {d:.6}", p.len()),
        json!({ "p": p, "q": q }),
    );
}

/// `Statistics.neg_entropy` — `Σ pᵢ ln(pᵢ)` over sheet masses.
pub(super) fn run_sheet_neg_entropy(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some(v) = local_neg_entropy(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a non-empty finite mass vector for neg-entropy.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.neg_entropy",
        format!("Neg-entropy over {} masses: {v:.6}", values.len()),
        json!({ "p": values }),
    );
}

/// `Statistics.simplex_project_idempotent` — project(project(p)) ≈ project(p).
pub(super) fn run_sheet_simplex_project_idempotent(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let ok = local_simplex_project_idempotent(&values);
    invoke_dual(
        document,
        label,
        "Statistics.simplex_project_idempotent",
        format!(
            "Simplex project idempotent over {} masses: {ok}",
            values.len()
        ),
        json!({ "p": values }),
    );
}

/// `Statistics.fisher_inner_product` — thirds as p, u, v on the simplex.
pub(super) fn run_sheet_fisher_inner_product(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((p, u, v)) = split_triple_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a multiple of 3 (≥6) numbers (p, then u, then v).",
            "error",
        );
        return;
    };
    let Some(ip) = local_fisher_inner_product(&p, &u, &v) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Fisher inner product needs equal-length finite p/u/v.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.fisher_inner_product",
        format!("Fisher inner product dim={}: {ip:.6}", p.len()),
        json!({ "p": p, "u": u, "v": v }),
    );
}

/// `Statistics.neg_entropy_grad` — ∇ψ(p)_i = ln(pᵢ) + 1.
pub(super) fn run_sheet_neg_entropy_grad(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some(grad) = local_neg_entropy_grad(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a non-empty finite mass vector for neg-entropy gradient.",
            "error",
        );
        return;
    };
    let preview: String = grad
        .iter()
        .take(4)
        .map(|v| {
            if v.is_finite() {
                format!("{v:.4}")
            } else {
                "−∞".into()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    invoke_dual(
        document,
        label,
        "Statistics.neg_entropy_grad",
        format!("Neg-entropy grad ({}): [{preview}]", grad.len()),
        json!({ "p": values }),
    );
}

/// `Statistics.kl_bregman_form` — KL via Bregman generator ψ = neg-entropy.
pub(super) fn run_sheet_kl_bregman_form(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((p, q)) = split_pair_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need an even count (≥4) of numbers (p then q halves).",
            "error",
        );
        return;
    };
    let Some(v) = local_kl_bregman_form(&p, &q) else {
        super::interactions::show_tool_status(
            document,
            label,
            "KL Bregman form needs equal-length finite masses.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.kl_bregman_form",
        format!("KL Bregman over {}-simplex: {v:.6}", p.len()),
        json!({ "p": p, "q": q }),
    );
}

/// `Statistics.bregman_pythagorean_test` — thirds as p, q★, q.
pub(super) fn run_sheet_bregman_pythagorean_test(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some((p, q_star, q)) = split_triple_series(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a multiple of 3 (≥6) numbers (p, then q★, then q).",
            "error",
        );
        return;
    };
    let Some((kl_pq, kl_pqstar, kl_qstar_q)) =
        local_bregman_pythagorean_test(&p, &q_star, &q)
    else {
        super::interactions::show_tool_status(
            document,
            label,
            "Bregman–Pythagorean test needs equal-length finite masses.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.bregman_pythagorean_test",
        format!(
            "Bregman–Pythagorean dim={}: KL(p‖q)={kl_pq:.6}, KL(p‖q★)={kl_pqstar:.6}, KL(q★‖q)={kl_qstar_q:.6}",
            p.len()
        ),
        json!({ "p": p, "q_star": q_star, "q": q }),
    );
}

/// `Statistics.probability_hash` — FNV-1a over f32 bit patterns.
pub(super) fn run_sheet_probability_hash(document: &Document, label: &str) {
    let Some(values) = need_numbers(document, label) else {
        return;
    };
    let Some(hash) = local_probability_hash(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a non-empty finite mass vector for probability hash.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Statistics.probability_hash",
        format!("Probability hash over {} masses: 0x{hash:016x}", values.len()),
        json!({ "p": values }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_sum_of_three() {
        assert_eq!(local_sum(&[1.0, 2.0, 3.0]), Some(6.0));
        assert_eq!(local_sum(&[]), None);
    }

    #[test]
    fn local_skewness_symmetric_near_zero() {
        let s = local_skewness(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap();
        assert!(s.abs() < 1e-9);
    }

    #[test]
    fn local_kurtosis_defined() {
        let k = local_kurtosis(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap();
        assert!(k.is_finite());
    }

    #[test]
    fn local_quantile_median() {
        assert_eq!(local_quantile(&[1.0, 2.0, 3.0, 4.0], 0.5), Some(2.5));
    }

    #[test]
    fn local_iqr_basic() {
        let i = local_iqr(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap();
        assert!(i > 0.0);
    }

    #[test]
    fn local_mode_picks_frequent() {
        let (v, c) = local_mode(&[1.0, 2.0, 2.0, 3.0]).unwrap();
        assert_eq!(v, 2.0);
        assert_eq!(c, 2);
    }

    #[test]
    fn local_trimmed_mean_rejects_bad_proportion() {
        assert!(local_trimmed_mean(&[1.0, 2.0, 3.0], 0.5).is_none());
        assert!(local_trimmed_mean(&[1.0, 2.0, 3.0], 0.0).is_some());
    }

    #[test]
    fn local_mad_scaled_positive() {
        let mad = local_mad(&[1.0, 2.0, 3.0, 4.0, 5.0], true).unwrap();
        assert!(mad > 0.0);
    }

    #[test]
    fn local_pearson_perfect_positive() {
        let r = local_pearson(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]).unwrap();
        assert!((r - 1.0).abs() < 1e-9);
    }

    #[test]
    fn local_covariance_sample() {
        let c = local_covariance(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0], true).unwrap();
        assert!((c - 2.0).abs() < 1e-9);
    }

    #[test]
    fn split_pair_series_needs_four() {
        assert!(split_pair_series(&[1.0, 2.0, 3.0]).is_none());
        let (x, y) = split_pair_series(&[1.0, 2.0, 3.0, 4.0]).unwrap();
        assert_eq!(x, vec![1.0, 2.0]);
        assert_eq!(y, vec![3.0, 4.0]);
    }

    #[test]
    fn local_z_score_flags_spike() {
        let values = [10.0, 11.0, 9.0, 10.5, 9.5, 10.2, 100.0];
        let count = local_z_score_outlier_count(&values, 2.0)
            .expect("z-score sketch should be defined");
        assert_eq!(
            count, 1,
            "expected the 100.0 spike to be the sole |z|>2 outlier"
        );
    }

    #[test]
    fn local_argmax_picks_first_on_tie() {
        let (idx, v) = local_argmax(&[1.0, 5.0, 5.0, 2.0]).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(v, 5.0);
    }

    #[test]
    fn local_binomial_pmf_fair_coin() {
        let p = local_binomial_pmf(1, 2, 0.5).unwrap();
        assert!((p - 0.5).abs() < 1e-12);
    }

    #[test]
    fn local_binomial_cdf_covers_all() {
        let c = local_binomial_cdf(2, 2, 0.5).unwrap();
        assert!((c - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_beta_pdf_uniform() {
        let p = local_beta_pdf(0.3, 1.0, 1.0).unwrap();
        assert!((p - 1.0).abs() < 1e-6);
    }

    #[test]
    fn local_chi_squared_pdf_k2_at_zero() {
        assert_eq!(local_chi_squared_pdf(0.0, 2.0), Some(0.5));
    }

    #[test]
    fn local_chi_squared_cdf_k2_matches_exp() {
        let x = 2.0;
        let c = local_chi_squared_cdf(x, 2.0).unwrap();
        assert!((c - (1.0 - (-x / 2.0).exp())).abs() < 1e-12);
    }

    #[test]
    fn local_chi_squared_quantile_k2_roundtrip() {
        let p = 0.9;
        let q = local_chi_squared_quantile(p, 2.0).unwrap();
        let back = local_chi_squared_cdf(q, 2.0).unwrap();
        assert!((back - p).abs() < 1e-9);
    }

    #[test]
    fn local_autocorrelation_lag_zero_is_one() {
        let r = local_autocorrelation(&[1.0, 2.0, 3.0, 4.0], 0).unwrap();
        assert!((r - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_bootstrap_means_finite() {
        let (m, n) = local_bootstrap_mean_summary(&[1.0, 2.0, 3.0], 32, 7).unwrap();
        assert!(m.is_finite());
        assert_eq!(n, 32);
    }

    #[test]
    fn local_normal_pdf_standard_at_zero() {
        let p = local_normal_pdf(0.0, 0.0, 1.0).unwrap();
        let expected = 1.0 / (2.0 * std::f64::consts::PI).sqrt();
        assert!((p - expected).abs() < 1e-12);
    }

    #[test]
    fn local_normal_cdf_median() {
        let c = local_normal_cdf(0.0, 0.0, 1.0).unwrap();
        assert!((c - 0.5).abs() < 1e-6);
    }

    #[test]
    fn local_normal_quantile_roundtrip() {
        let p = 0.975;
        let q = local_normal_quantile(p, 0.0, 1.0).unwrap();
        let back = local_normal_cdf(q, 0.0, 1.0).unwrap();
        assert!((back - p).abs() < 1e-4);
    }

    #[test]
    fn local_standard_normal_cdf_zero() {
        assert!((local_standard_normal_cdf(0.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn local_poisson_pmf_lambda_one() {
        let p = local_poisson_pmf(0, 1.0).unwrap();
        assert!((p - (-1.0f64).exp()).abs() < 1e-12);
    }

    #[test]
    fn local_poisson_cdf_covers_mass() {
        let c = local_poisson_cdf(20, 1.0).unwrap();
        assert!((c - 1.0).abs() < 1e-9);
    }

    #[test]
    fn local_exponential_pdf_at_zero() {
        let p = local_exponential_pdf(0.0, 2.0).unwrap();
        assert!((p - 2.0).abs() < 1e-12);
    }

    #[test]
    fn local_exponential_cdf_unit() {
        let c = local_exponential_cdf(0.0, 1.0).unwrap();
        assert!((c - 0.0).abs() < 1e-12);
        let c2 = local_exponential_cdf(1.0, 1.0).unwrap();
        assert!((c2 - (1.0 - (-1.0f64).exp())).abs() < 1e-12);
    }

    #[test]
    fn local_spearman_perfect_positive() {
        let r = local_spearman(&[1.0, 2.0, 3.0], &[10.0, 20.0, 30.0]).unwrap();
        assert!((r - 1.0).abs() < 1e-9);
    }

    #[test]
    fn local_kendall_perfect_positive() {
        let t = local_kendall(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]).unwrap();
        assert!((t - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_winsorized_mean_basic() {
        let m = local_winsorized_mean(&[1.0, 2.0, 3.0, 4.0, 100.0], 0.2).unwrap();
        assert!(m.is_finite());
        assert!(m < 100.0);
    }

    #[test]
    fn local_erf_erfc_at_zero() {
        assert!((local_erf_approx(0.0)).abs() < 1e-6);
        assert!((local_erfc_approx(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn local_uniform_pdf_unit_interval() {
        assert!((local_uniform_pdf(0.5, 0.0, 1.0).unwrap() - 1.0).abs() < 1e-12);
        assert_eq!(local_uniform_pdf(2.0, 0.0, 1.0), Some(0.0));
    }

    #[test]
    fn local_laplace_pdf_at_mu() {
        let p = local_laplace_pdf(0.0, 0.0, 1.0).unwrap();
        assert!((p - 0.5).abs() < 1e-12);
    }

    #[test]
    fn local_standard_pdf_at_zero() {
        let p = local_standard_pdf(0.0).unwrap();
        let expected = 1.0 / (2.0 * std::f64::consts::PI).sqrt();
        assert!((p - expected).abs() < 1e-12);
    }

    #[test]
    fn local_uniform_cdf_unit_interval() {
        assert!((local_uniform_cdf(0.25, 0.0, 1.0).unwrap() - 0.25).abs() < 1e-12);
        assert_eq!(local_uniform_cdf(-1.0, 0.0, 1.0), Some(0.0));
        assert_eq!(local_uniform_cdf(2.0, 0.0, 1.0), Some(1.0));
    }

    #[test]
    fn local_laplace_cdf_at_mu() {
        let c = local_laplace_cdf(0.0, 0.0, 1.0).unwrap();
        assert!((c - 0.5).abs() < 1e-12);
    }

    #[test]
    fn local_lognormal_pdf_positive() {
        let p = local_lognormal_pdf(1.0, 0.0, 1.0).unwrap();
        let expected = 1.0 / (2.0 * std::f64::consts::PI).sqrt();
        assert!((p - expected).abs() < 1e-12);
    }

    #[test]
    fn local_lognormal_cdf_at_one() {
        let c = local_lognormal_cdf(1.0, 0.0, 1.0).unwrap();
        assert!((c - 0.5).abs() < 1e-6);
    }

    #[test]
    fn local_standard_quantile_median() {
        let z = local_standard_quantile(0.5).unwrap();
        assert!(z.abs() < 1e-6);
    }

    #[test]
    fn local_ln_gamma_and_gamma_fn_at_one() {
        let ln = local_ln_gamma_stirling(1.0).unwrap();
        assert!(ln.abs() < 1e-9);
        let g = local_gamma_fn(1.0).unwrap();
        assert!((g - 1.0).abs() < 1e-9);
    }

    #[test]
    fn local_weibull_pdf_unit_at_one() {
        let p = local_weibull_pdf(1.0, 1.0, 1.0).unwrap();
        assert!((p - (-1.0f64).exp()).abs() < 1e-12);
    }

    #[test]
    fn local_gamma_pdf_unit_at_one() {
        let p = local_gamma_pdf(1.0, 1.0, 1.0).unwrap();
        assert!((p - (-1.0f64).exp()).abs() < 1e-9);
    }

    #[test]
    fn local_two_sided_p_at_zero() {
        let p = local_two_sided_p(0.0).unwrap();
        // Offline erf sketch is approximate at 0; Host Φ(0)=½ → p=1.
        assert!((p - 1.0).abs() < 1e-3);
        let tail = local_two_sided_p(1.96).unwrap();
        assert!(tail < 0.06 && tail > 0.04);
    }

    #[test]
    fn local_chi_squared_upper_p_k2() {
        let p = local_chi_squared_upper_p(0.0, 2.0).unwrap();
        assert!((p - 1.0).abs() < 1e-9);
    }

    #[test]
    fn local_students_t_pdf_at_zero() {
        let p = local_students_t_pdf(0.0, 10.0).unwrap();
        assert!(p > 0.0 && p.is_finite());
    }

    #[test]
    fn local_fisher_f_pdf_positive() {
        let p = local_fisher_f_pdf(1.0, 5.0, 10.0).unwrap();
        assert!(p > 0.0 && p.is_finite());
    }

    #[test]
    fn local_gammp_a_one() {
        let p = local_gammp(1.0, 1.0).unwrap();
        assert!((p - (1.0 - (-1.0f64).exp())).abs() < 1e-12);
    }

    #[test]
    fn local_gammq_complements_gammp() {
        let p = local_gammp(1.0, 0.5).unwrap();
        let q = local_gammq(1.0, 0.5).unwrap();
        assert!((p + q - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_betai_midpoint_symmetric() {
        let v = local_betai(2.0, 2.0, 0.5).unwrap();
        assert!((v - 0.5).abs() < 1e-6);
    }

    #[test]
    fn local_students_t_cdf_at_zero() {
        let c = local_students_t_cdf(0.0, 10.0).unwrap();
        assert!((c - 0.5).abs() < 1e-4);
    }

    #[test]
    fn local_students_t_two_sided_p_at_zero() {
        let p = local_students_t_two_sided_p(0.0, 10.0).unwrap();
        assert!((p - 1.0).abs() < 1e-3);
    }

    #[test]
    fn local_students_t_upper_p_complements_cdf() {
        let t = 1.5;
        let nu = 10.0;
        let cdf = local_students_t_cdf(t, nu).unwrap();
        let up = local_students_t_upper_p(t, nu).unwrap();
        assert!((cdf + up - 1.0).abs() < 1e-9);
    }

    #[test]
    fn local_students_t_quantile_median() {
        let q = local_students_t_quantile(0.5, 10.0).unwrap();
        assert!(q.abs() < 1e-4);
    }

    #[test]
    fn local_fisher_f_cdf_at_zero() {
        assert_eq!(local_fisher_f_cdf(0.0, 5.0, 10.0), Some(0.0));
    }

    #[test]
    fn local_fisher_f_upper_p_complements_cdf() {
        let x = 1.0;
        let cdf = local_fisher_f_cdf(x, 5.0, 10.0).unwrap();
        let up = local_fisher_f_upper_p(x, 5.0, 10.0).unwrap();
        assert!((cdf + up - 1.0).abs() < 1e-9);
    }

    #[test]
    fn local_fisher_f_quantile_positive() {
        let q = local_fisher_f_quantile(0.95, 5.0, 10.0).unwrap();
        assert!(q > 0.0 && q.is_finite());
        let back = local_fisher_f_cdf(q, 5.0, 10.0).unwrap();
        assert!((back - 0.95).abs() < 1e-3);
    }

    #[test]
    fn local_tukey_fences_basic() {
        let (lo, hi) = local_tukey_fences(&[1.0, 2.0, 3.0, 4.0, 5.0, 100.0], 1.5).unwrap();
        assert!(lo < hi);
        assert!(hi < 100.0 || lo < 1.0);
    }

    #[test]
    fn local_entropy_uniform_two() {
        let h = local_entropy(&[1.0, 1.0]).unwrap();
        assert!((h - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_kl_identical_zero() {
        let d = local_kl_divergence(&[0.5, 0.5], &[1.0, 1.0]).unwrap();
        assert!(d.abs() < 1e-12);
    }

    #[test]
    fn local_cross_entropy_uniform() {
        let h = local_cross_entropy(&[0.5, 0.5], &[0.5, 0.5]).unwrap();
        assert!((h - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_empirical_cdf_mid() {
        let c = local_empirical_cdf(&[1.0, 2.0, 3.0, 4.0], 2.5).unwrap();
        assert!((c - 0.5).abs() < 1e-12);
    }

    #[test]
    fn local_entropy_from_counts_uniform() {
        let h = local_entropy_from_counts(&[2, 2]).unwrap();
        assert!((h - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_moving_average_last_window2() {
        let (last, count) = local_moving_average_last(&[1.0, 2.0, 3.0, 4.0], 2).unwrap();
        assert_eq!(count, 3);
        assert!((last - 3.5).abs() < 1e-12);
    }

    #[test]
    fn local_modified_z_flags_spike() {
        let count =
            local_modified_z_outlier_count(&[1.0, 1.1, 0.9, 1.0, 1.05, 50.0], 3.5).unwrap();
        assert!(count >= 1);
    }

    #[test]
    fn local_iqr_outlier_count_flags_spike() {
        let count = local_iqr_outlier_count(&[1.0, 2.0, 3.0, 4.0, 5.0, 100.0], 1.5).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn local_exponential_smoothing_last_known() {
        let (last, n) = local_exponential_smoothing_last(&[1.0, 2.0, 3.0], 0.5).unwrap();
        assert_eq!(n, 3);
        assert!((last - 2.25).abs() < 1e-12);
    }

    #[test]
    fn local_adf_proxy_finite() {
        let s = local_adf_proxy(&[1.0, 1.5, 1.2, 1.8, 1.4]).unwrap();
        assert!(s.is_finite());
    }

    #[test]
    fn local_histogram_summary_bins() {
        let (occupied, min_v, max_v, _) =
            local_histogram_summary(&[1.0, 2.0, 3.0, 4.0, 5.0], 4).unwrap();
        assert!(occupied >= 1);
        assert!((min_v - 1.0).abs() < 1e-12);
        assert!((max_v - 5.0).abs() < 1e-12);
    }

    #[test]
    fn local_ks_1sample_uniformish() {
        let (d, p) = local_ks_1sample(&[0.1, 0.3, 0.5, 0.7, 0.9]).unwrap();
        assert!(d >= 0.0 && d <= 1.0);
        assert!(p > 0.0 && p <= 1.0);
    }

    #[test]
    fn local_grubbs_flags_spike() {
        let (idx, g, crit, is_out) =
            local_grubbs_test(&[1.0, 1.1, 0.9, 1.0, 1.05, 50.0], 0.05).unwrap();
        assert_eq!(idx, 5);
        assert!(g > crit);
        assert!(is_out);
    }

    #[test]
    fn local_one_sample_t_at_mean() {
        let (t, p, df) = local_one_sample_t(&[1.0, 2.0, 3.0, 4.0, 5.0], 3.0).unwrap();
        assert!(t.abs() < 1e-9);
        assert!((p - 1.0).abs() < 1e-3);
        assert_eq!(df, 4);
    }

    #[test]
    fn local_two_sample_t_identical() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [1.0, 2.0, 3.0, 4.0];
        let (t, p, _, diff) = local_two_sample_t(&a, &b, true).unwrap();
        assert!(t.abs() < 1e-9);
        assert!(diff.abs() < 1e-12);
        assert!((p - 1.0).abs() < 1e-3);
    }

    #[test]
    fn local_paired_t_zero_diff() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [1.0, 2.0, 3.0, 4.0];
        let (t, p, df) = local_paired_t(&a, &b).unwrap();
        assert!(t.abs() < 1e-9);
        assert!((p - 1.0).abs() < 1e-3);
        assert_eq!(df, 3);
    }

    #[test]
    fn local_linear_regression_perfect() {
        let x = [1.0, 2.0, 3.0, 4.0];
        let y = [2.0, 4.0, 6.0, 8.0];
        let (slope, intercept, r2, n) = local_linear_regression(&x, &y).unwrap();
        assert_eq!(n, 4);
        assert!((slope - 2.0).abs() < 1e-12);
        assert!(intercept.abs() < 1e-12);
        assert!((r2 - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_chi_square_gof_fairish() {
        let (stat, p, dof) = local_chi_square_gof(
            &[9.0, 11.0, 10.0, 12.0, 8.0, 10.0],
            &[10.0; 6],
        )
        .unwrap();
        assert_eq!(dof, 5.0);
        assert!(stat.is_finite());
        assert!(p > 0.4);
    }

    #[test]
    fn local_chi_square_independence_proportional() {
        let table = vec![vec![10.0, 20.0], vec![20.0, 40.0]];
        let (stat, p, dof) = local_chi_square_independence(&table).unwrap();
        assert_eq!(dof, 1.0);
        assert!(stat < 1e-9);
        assert!((p - 1.0).abs() < 1e-6);
    }

    #[test]
    fn local_correlation_p_value_perfect() {
        assert_eq!(local_correlation_p_value(1.0, 10), Some(0.0));
        assert!(local_correlation_p_value(0.0, 2).is_none());
    }

    #[test]
    fn local_ljung_box_zero_acf() {
        let q = local_ljung_box(&[0.0, 0.0, 0.0], 20, 3).unwrap();
        assert!(q.abs() < 1e-12);
    }

    #[test]
    fn local_mann_whitney_identical() {
        let (u, p, n1, n2) =
            local_mann_whitney_u(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]).unwrap();
        assert_eq!(n1, 3);
        assert_eq!(n2, 3);
        assert!((u - 4.5).abs() < 1e-9);
        assert!(p > 0.5);
    }

    #[test]
    fn local_mcnemar_disagreement() {
        let (stat, p, dof) = local_mcnemar(30, 5).unwrap();
        assert_eq!(dof, 1.0);
        assert!(stat > 15.0);
        assert!(p < 0.001);
        assert!(local_mcnemar(0, 0).is_none());
    }

    #[test]
    fn local_mutual_information_dependent() {
        let x = [0usize, 0, 1, 1, 0, 1, 0, 1];
        let y = x;
        let mi = local_mutual_information_discrete(&x, &y).unwrap();
        assert!((mi - 1.0).abs() < 1e-9);
    }

    #[test]
    fn local_one_way_anova_identical_groups() {
        let groups = vec![
            vec![1.0, 2.0, 3.0],
            vec![1.0, 2.0, 3.0],
        ];
        let (f, p, dfb, dfw) = local_one_way_anova(&groups).unwrap();
        assert_eq!(dfb, 1.0);
        assert_eq!(dfw, 4.0);
        assert!(f.abs() < 1e-12);
        assert!(p > 0.9);
    }

    #[test]
    fn local_friedman_ordering() {
        let blocks = vec![
            vec![1.0, 2.0, 3.0],
            vec![1.1, 2.2, 3.3],
            vec![0.9, 2.1, 3.1],
            vec![1.0, 2.5, 3.4],
            vec![1.2, 2.0, 3.0],
        ];
        let (chi, p, df) = local_friedman(&blocks).unwrap();
        assert_eq!(df, 2.0);
        assert!(chi > 5.0);
        assert!(p < 0.1);
    }

    #[test]
    fn local_mahalanobis_sq_identity() {
        let d2 = local_mahalanobis_sq(&[1.0, 2.0], &[0.0, 0.0], &[1.0, 0.0, 0.0, 1.0]).unwrap();
        assert!((d2 - 5.0).abs() < 1e-12);
    }

    #[test]
    fn local_mvn_pdf_identity_at_mean() {
        let dens = local_mvn_pdf(&[0.0, 0.0], &[0.0, 0.0], &[1.0, 0.0, 0.0, 1.0], 2).unwrap();
        let expected = 1.0 / (2.0 * std::f64::consts::PI);
        assert!((dens - expected).abs() < 1e-12);
    }

    #[test]
    fn local_mvn_log_pdf_peaks_at_mean() {
        let mean = [1.0, 2.0];
        let cov = [1.0, 0.3, 0.3, 1.0];
        let at = local_mvn_log_pdf(&mean, &mean, &cov, 2).unwrap();
        let away = local_mvn_log_pdf(&[3.0, -1.0], &mean, &cov, 2).unwrap();
        assert!(at > away);
    }

    #[test]
    fn local_mvn_mle_two_points() {
        let data = [0.0, 0.0, 2.0, 4.0];
        let (mean, cov) = local_mvn_mle(&data, 2, 2).unwrap();
        assert!((mean[0] - 1.0).abs() < 1e-12);
        assert!((mean[1] - 2.0).abs() < 1e-12);
        assert!((cov[0] - 1.0).abs() < 1e-12);
        assert!((cov[3] - 4.0).abs() < 1e-12);
    }

    #[test]
    fn local_mvn_sample_deterministic() {
        let a = local_mvn_sample(&[0.0, 0.0], &[1.0, 0.0, 0.0, 1.0], 2, 7).unwrap();
        let b = local_mvn_sample(&[0.0, 0.0], &[1.0, 0.0, 0.0, 1.0], 2, 7).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn local_validate_probability_ok_and_reject() {
        assert!(local_validate_probability(&[0.5, 0.5]));
        assert!(!local_validate_probability(&[0.5, 0.6]));
    }

    #[test]
    fn local_simplex_project_normalises() {
        let q = local_simplex_project(&[1.0, 1.0, 0.0]).unwrap();
        assert!((q.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        assert!((q[0] - 0.5).abs() < 1e-12);
        assert!((q[1] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn local_fisher_distance_self_zero() {
        let d = local_fisher_distance(&[0.3, 0.3, 0.4], &[0.3, 0.3, 0.4]).unwrap();
        assert!(d.abs() < 1e-6, "Fisher distance to self must be ~0, got {d}");
    }

    #[test]
    fn local_neg_entropy_uniform_two() {
        let v = local_neg_entropy(&[0.5, 0.5]).unwrap();
        assert!((v - 0.5f64.ln()).abs() < 1e-12);
    }

    #[test]
    fn local_simplex_project_idempotent_true() {
        assert!(local_simplex_project_idempotent(&[1.0, 1.0, 0.0]));
    }

    #[test]
    fn local_fisher_inner_product_unit() {
        let ip = local_fisher_inner_product(&[0.5, 0.5], &[1.0, -1.0], &[1.0, -1.0]).unwrap();
        assert!((ip - 4.0).abs() < 1e-12, "got {ip}");
    }

    #[test]
    fn local_neg_entropy_grad_uniform() {
        let g = local_neg_entropy_grad(&[0.5, 0.5]).unwrap();
        let expect = 0.5f64.ln() + 1.0;
        assert!((g[0] - expect).abs() < 1e-12);
        assert!((g[1] - expect).abs() < 1e-12);
    }

    #[test]
    fn local_kl_bregman_self_zero() {
        let v = local_kl_bregman_form(&[0.2, 0.5, 0.3], &[0.2, 0.5, 0.3]).unwrap();
        assert!(v.abs() < 1e-10, "got {v}");
    }

    #[test]
    fn local_bregman_pythagorean_parts() {
        let (kl_pq, kl_pqstar, kl_qstar_q) = local_bregman_pythagorean_test(
            &[0.5, 0.5],
            &[0.5, 0.5],
            &[0.25, 0.75],
        )
        .unwrap();
        assert!(kl_pqstar.abs() < 1e-12);
        assert!((kl_pq - (kl_pqstar + kl_qstar_q)).abs() < 1e-9);
    }

    #[test]
    fn local_probability_hash_stable() {
        let h1 = local_probability_hash(&[0.25, 0.75]).unwrap();
        let h2 = local_probability_hash(&[0.25, 0.75]).unwrap();
        assert_eq!(h1, h2);
        assert_ne!(h1, 0);
    }

    #[test]
    fn split_triple_series_needs_six() {
        assert!(split_triple_series(&[1.0, 2.0, 3.0, 4.0, 5.0]).is_none());
        let (a, b, c) = split_triple_series(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
        assert_eq!(a, vec![1.0, 2.0]);
        assert_eq!(b, vec![3.0, 4.0]);
        assert_eq!(c, vec![5.0, 6.0]);
    }
}
