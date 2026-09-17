//! Dual-path Tool Chest actions for curated `NumberTheory.*` ALL_BOUND ids
//! (wave 16 + wave 18 remainder).
//!
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.
//! Local sketches mirror Host scalar algorithms for small inputs (CPU; Host path authoritative).

use serde_json::json;
use web_sys::{Document, Element};

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn need_container(document: &Document, label: &str, message: &str) -> Option<Element> {
    match selected_container(document) {
        Some(container) => Some(container),
        None => {
            super::interactions::show_tool_status(document, label, message, "error");
            None
        }
    }
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
}

fn surface_u64(container: &Element, name: &str, default: u64) -> u64 {
    numeric_attr(Some(container), name)
        .filter(|v| v.is_finite() && *v >= 0.0 && *v == v.floor())
        .map(|v| v as u64)
        .unwrap_or(default)
}

fn surface_i64(container: &Element, name: &str, default: i64) -> i64 {
    numeric_attr(Some(container), name)
        .filter(|v| v.is_finite() && *v == v.floor())
        .map(|v| v as i64)
        .unwrap_or(default)
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

// ── Local sketches (match Host for typical small surface values) ─────

fn local_gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn local_lcm(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 {
        return 0;
    }
    a / local_gcd(a, b) * b
}

fn local_is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n % 2 == 0 {
        return n == 2;
    }
    let mut d = 3u64;
    while d.saturating_mul(d) <= n {
        if n % d == 0 {
            return false;
        }
        d += 2;
    }
    true
}

fn local_next_prime(n: u64) -> u64 {
    let mut c = n.saturating_add(1);
    if c <= 2 {
        return 2;
    }
    if c % 2 == 0 {
        c += 1;
    }
    loop {
        if local_is_prime(c) {
            return c;
        }
        c = c.saturating_add(2);
    }
}

fn local_mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus <= 1 {
        return 0;
    }
    let m = modulus as u128;
    let mut result: u128 = 1;
    base %= modulus;
    let mut b = base as u128;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result * b % m;
        }
        b = b * b % m;
        exp >>= 1;
    }
    result as u64
}

fn local_extended_gcd(a: i64, b: i64) -> (i64, i64, i64) {
    if b == 0 {
        return (a.abs(), if a < 0 { -1 } else { 1 }, 0);
    }
    let (g, x, y) = local_extended_gcd(b, a % b);
    (g, y, x - (a / b) * y)
}

fn local_mod_inverse(a: u64, m: u64) -> Option<u64> {
    if m == 0 {
        return None;
    }
    let (g, x, _) = local_extended_gcd((a % m) as i64, m as i64);
    if g != 1 {
        return None;
    }
    Some(((x % m as i64 + m as i64) % m as i64) as u64)
}

fn local_factorial(n: u64) -> Option<u128> {
    let mut acc: u128 = 1;
    for k in 2..=n as u128 {
        acc = acc.checked_mul(k)?;
    }
    Some(acc)
}

fn local_binomial(n: u64, k: u64) -> Option<u128> {
    if k > n {
        return Some(0);
    }
    let k = k.min(n - k);
    let mut result: u128 = 1;
    for i in 0..k {
        result = result.checked_mul((n - i) as u128)?;
        result /= (i + 1) as u128;
    }
    Some(result)
}

fn local_euler_totient(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut result = n;
    let mut rem = n;
    let mut p = 2u64;
    while p.saturating_mul(p) <= rem {
        if rem % p == 0 {
            while rem % p == 0 {
                rem /= p;
            }
            result = result / p * (p - 1);
        }
        p += if p == 2 { 1 } else { 2 };
    }
    if rem > 1 {
        result = result / rem * (rem - 1);
    }
    result
}

fn local_divisor_count(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut count = 0u64;
    let mut d = 1u64;
    while d.saturating_mul(d) <= n {
        if n % d == 0 {
            count += 1;
            let other = n / d;
            if other != d {
                count += 1;
            }
        }
        d += 1;
    }
    count
}

/// Trial-division factorization for small sketch inputs (Host may use Pollard's rho).
fn local_prime_factors(mut n: u64) -> Vec<(u64, u32)> {
    if n < 2 {
        return Vec::new();
    }
    let mut out: Vec<(u64, u32)> = Vec::new();
    let mut push = |p: u64| {
        if let Some(last) = out.last_mut() {
            if last.0 == p {
                last.1 += 1;
                return;
            }
        }
        out.push((p, 1));
    };
    while n % 2 == 0 {
        push(2);
        n /= 2;
    }
    let mut p = 3u64;
    while p.saturating_mul(p) <= n {
        while n % p == 0 {
            push(p);
            n /= p;
        }
        p += 2;
    }
    if n > 1 {
        push(n);
    }
    out
}

fn local_divisors(n: u64) -> Vec<u64> {
    if n == 0 {
        return Vec::new();
    }
    let mut divs = vec![1u64];
    for (p, e) in local_prime_factors(n) {
        let mut pk = 1u64;
        let base = divs.clone();
        for _ in 0..e {
            pk *= p;
            for &d in &base {
                divs.push(d * pk);
            }
        }
    }
    divs.sort_unstable();
    divs
}

fn local_mobius(n: u64) -> i8 {
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }
    let factors = local_prime_factors(n);
    if factors.iter().any(|&(_, e)| e > 1) {
        return 0;
    }
    if factors.len() % 2 == 0 {
        1
    } else {
        -1
    }
}

fn local_divisor_sum(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut sum = 1u64;
    for (p, e) in local_prime_factors(n) {
        let mut term = 1u64;
        let mut pk = 1u64;
        for _ in 0..e {
            pk *= p;
            term += pk;
        }
        sum *= term;
    }
    sum
}

fn local_partitions(n: u64) -> u64 {
    let n = n as usize;
    let mut dp = vec![0u64; n + 1];
    dp[0] = 1;
    for coin in 1..=n {
        for amount in coin..=n {
            dp[amount] = dp[amount].wrapping_add(dp[amount - coin]);
        }
    }
    dp[n]
}

fn local_catalan(n: u64) -> Option<u128> {
    let c = local_binomial(2 * n, n)?;
    Some(c / (n as u128 + 1))
}

fn local_stirling_second(n: u64, k: u64) -> Option<u128> {
    let (n, k) = (n as usize, k as usize);
    if k > n {
        return Some(0);
    }
    let mut prev = vec![0u128; k + 1];
    prev[0] = 1;
    for i in 1..=n {
        let mut cur = vec![0u128; k + 1];
        for j in 1..=k.min(i) {
            let a = (j as u128).checked_mul(prev[j])?;
            cur[j] = a.checked_add(prev[j - 1])?;
        }
        prev = cur;
    }
    Some(prev[k])
}

fn local_stirling_first(n: u64, k: u64) -> Option<u128> {
    let (n, k) = (n as usize, k as usize);
    if k > n {
        return Some(0);
    }
    let mut prev = vec![0u128; k + 1];
    prev[0] = 1;
    for i in 1..=n {
        let mut cur = vec![0u128; k + 1];
        for j in 1..=k.min(i) {
            let a = ((i - 1) as u128).checked_mul(prev[j])?;
            cur[j] = a.checked_add(prev[j - 1])?;
        }
        prev = cur;
    }
    Some(prev[k])
}

fn local_crt(r1: u64, m1: u64, r2: u64, m2: u64) -> Option<(u64, u64)> {
    let inv = local_mod_inverse(m1 % m2, m2)?;
    let m = m1.checked_mul(m2)?;
    let diff = (r2 as i128 - r1 as i128).rem_euclid(m2 as i128) as u128;
    let t = diff * inv as u128 % m2 as u128;
    let x = (r1 as u128 + m1 as u128 * t) % m as u128;
    Some((x as u64, m))
}

fn format_factors(factors: &[(u64, u32)]) -> String {
    if factors.is_empty() {
        return "[]".into();
    }
    factors
        .iter()
        .map(|(p, e)| format!("{p}^{e}"))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// `NumberTheory.gcd` — list `[a, b]`.
pub(super) fn run_nt_gcd(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-a and data-b before computing gcd.",
    ) else {
        return;
    };
    let a = surface_u64(&container, "data-a", 48);
    let b = surface_u64(&container, "data-b", 18);
    let value = local_gcd(a, b);
    invoke_dual(
        document,
        label,
        "NumberTheory.gcd",
        format!("Local gcd sketch: gcd({a}, {b}) → {value}. Connect QualiaDB for a live gcd."),
        json!([a, b]),
    );
}

/// `NumberTheory.lcm` — list `[a, b]`.
pub(super) fn run_nt_lcm(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-a and data-b before computing lcm.",
    ) else {
        return;
    };
    let a = surface_u64(&container, "data-a", 48);
    let b = surface_u64(&container, "data-b", 18);
    let value = local_lcm(a, b);
    invoke_dual(
        document,
        label,
        "NumberTheory.lcm",
        format!("Local lcm sketch: lcm({a}, {b}) → {value}. Connect QualiaDB for a live lcm."),
        json!([a, b]),
    );
}

/// `NumberTheory.is_prime` — scalar `n`.
pub(super) fn run_nt_is_prime(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before testing primality.",
    ) else {
        return;
    };
    let n = surface_u64(&container, "data-n", 17);
    let value = local_is_prime(n);
    invoke_dual(
        document,
        label,
        "NumberTheory.is_prime",
        format!(
            "Local is_prime sketch: n={n} → {value}. Connect QualiaDB for a live Miller–Rabin check."
        ),
        json!(n),
    );
}

/// `NumberTheory.factorial` — `{ n }`.
pub(super) fn run_nt_factorial(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before computing a factorial.",
    ) else {
        return;
    };
    let n = surface_u64(&container, "data-n", 10);
    let sketch = match local_factorial(n) {
        Some(v) => format!("{v}"),
        None => "overflow".into(),
    };
    invoke_dual(
        document,
        label,
        "NumberTheory.factorial",
        format!(
            "Local factorial sketch: {n}! → {sketch}. Connect QualiaDB for a live factorial."
        ),
        json!({ "n": n }),
    );
}

/// `NumberTheory.binomial` — `{ n, k }`.
pub(super) fn run_nt_binomial(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n and data-k before C(n,k).",
    ) else {
        return;
    };
    let n = surface_u64(&container, "data-n", 10);
    let k = surface_u64(&container, "data-k", 3);
    let sketch = match local_binomial(n, k) {
        Some(v) => format!("{v}"),
        None => "overflow".into(),
    };
    invoke_dual(
        document,
        label,
        "NumberTheory.binomial",
        format!(
            "Local binomial sketch: C({n}, {k}) → {sketch}. Connect QualiaDB for a live binomial."
        ),
        json!({ "n": n, "k": k }),
    );
}

/// `NumberTheory.euler_totient` — `{ n }`.
pub(super) fn run_nt_euler_totient(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before φ(n).",
    ) else {
        return;
    };
    let n = surface_u64(&container, "data-n", 12);
    let value = local_euler_totient(n);
    invoke_dual(
        document,
        label,
        "NumberTheory.euler_totient",
        format!(
            "Local Euler totient sketch: φ({n}) → {value}. Connect QualiaDB for a live euler_totient."
        ),
        json!({ "n": n }),
    );
}

/// `NumberTheory.mod_pow` — `{ base, exp, modulus }`.
pub(super) fn run_nt_mod_pow(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-base, data-exp, and data-modulus before mod_pow.",
    ) else {
        return;
    };
    let base = surface_u64(&container, "data-base", 2);
    let exp = surface_u64(&container, "data-exp", 10);
    let modulus = surface_u64(&container, "data-modulus", 1000);
    let value = local_mod_pow(base, exp, modulus);
    invoke_dual(
        document,
        label,
        "NumberTheory.mod_pow",
        format!(
            "Local mod_pow sketch: {base}^{exp} mod {modulus} → {value}. Connect QualiaDB for a live mod_pow."
        ),
        json!({ "base": base, "exp": exp, "modulus": modulus }),
    );
}

/// `NumberTheory.next_prime` — `{ n }`.
pub(super) fn run_nt_next_prime(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before next_prime.",
    ) else {
        return;
    };
    let n = surface_u64(&container, "data-n", 14);
    let value = local_next_prime(n);
    invoke_dual(
        document,
        label,
        "NumberTheory.next_prime",
        format!(
            "Local next_prime sketch: next ≥ {n}+1 → {value}. Connect QualiaDB for a live next_prime."
        ),
        json!({ "n": n }),
    );
}

/// `NumberTheory.mod_inverse` — `{ a, modulus }`.
pub(super) fn run_nt_mod_inverse(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-a and data-modulus before mod_inverse.",
    ) else {
        return;
    };
    let a = surface_u64(&container, "data-a", 3);
    let modulus = surface_u64(&container, "data-modulus", 11);
    let sketch = match local_mod_inverse(a, modulus) {
        Some(v) => format!("{v}"),
        None => "none".into(),
    };
    invoke_dual(
        document,
        label,
        "NumberTheory.mod_inverse",
        format!(
            "Local mod_inverse sketch: {a}⁻¹ mod {modulus} → {sketch}. Connect QualiaDB for a live mod_inverse."
        ),
        json!({ "a": a, "modulus": modulus }),
    );
}

/// `NumberTheory.divisor_count` — `{ n }`.
pub(super) fn run_nt_divisor_count(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before divisor_count.",
    ) else {
        return;
    };
    let n = surface_u64(&container, "data-n", 12);
    let value = local_divisor_count(n);
    invoke_dual(
        document,
        label,
        "NumberTheory.divisor_count",
        format!(
            "Local divisor_count sketch: d({n}) → {value}. Connect QualiaDB for a live divisor_count."
        ),
        json!({ "n": n }),
    );
}

/// `NumberTheory.prime_factors` — `{ n }`.
pub(super) fn run_nt_prime_factors(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before prime_factors.",
    ) else {
        return;
    };
    let n = surface_u64(&container, "data-n", 360);
    let factors = local_prime_factors(n);
    let sketch = format_factors(&factors);
    invoke_dual(
        document,
        label,
        "NumberTheory.prime_factors",
        format!(
            "Local prime_factors sketch: {n} → {sketch}. Connect QualiaDB for a live factorization."
        ),
        json!({ "n": n }),
    );
}

/// `NumberTheory.divisors` — `{ n }`.
pub(super) fn run_nt_divisors(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before listing divisors.",
    ) else {
        return;
    };
    let n = surface_u64(&container, "data-n", 28);
    let divs = local_divisors(n);
    let sketch = format!("{:?}", divs);
    invoke_dual(
        document,
        label,
        "NumberTheory.divisors",
        format!(
            "Local divisors sketch: divisors({n}) → {sketch}. Connect QualiaDB for a live divisor list."
        ),
        json!({ "n": n }),
    );
}

/// `NumberTheory.mobius` — `{ n }`.
pub(super) fn run_nt_mobius(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before μ(n).",
    ) else {
        return;
    };
    let n = surface_u64(&container, "data-n", 30);
    let value = local_mobius(n);
    invoke_dual(
        document,
        label,
        "NumberTheory.mobius",
        format!(
            "Local Möbius sketch: μ({n}) → {value}. Connect QualiaDB for a live mobius."
        ),
        json!({ "n": n }),
    );
}

/// `NumberTheory.divisor_sum` — `{ n }`.
pub(super) fn run_nt_divisor_sum(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before σ(n).",
    ) else {
        return;
    };
    let n = surface_u64(&container, "data-n", 28);
    let value = local_divisor_sum(n);
    invoke_dual(
        document,
        label,
        "NumberTheory.divisor_sum",
        format!(
            "Local divisor_sum sketch: σ({n}) → {value}. Connect QualiaDB for a live divisor_sum."
        ),
        json!({ "n": n }),
    );
}

/// `NumberTheory.partitions` — `{ n }`.
pub(super) fn run_nt_partitions(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before p(n).",
    ) else {
        return;
    };
    let n = surface_u64(&container, "data-n", 10);
    let value = local_partitions(n);
    invoke_dual(
        document,
        label,
        "NumberTheory.partitions",
        format!(
            "Local partitions sketch: p({n}) → {value}. Connect QualiaDB for a live partitions."
        ),
        json!({ "n": n }),
    );
}

/// `NumberTheory.catalan` — `{ n }`.
pub(super) fn run_nt_catalan(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before Catalan C_n.",
    ) else {
        return;
    };
    let n = surface_u64(&container, "data-n", 5);
    let sketch = match local_catalan(n) {
        Some(v) => format!("{v}"),
        None => "overflow".into(),
    };
    invoke_dual(
        document,
        label,
        "NumberTheory.catalan",
        format!(
            "Local Catalan sketch: C_{n} → {sketch}. Connect QualiaDB for a live catalan."
        ),
        json!({ "n": n }),
    );
}

/// `NumberTheory.stirling_second` — `{ n, k }`.
pub(super) fn run_nt_stirling_second(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n and data-k before S(n,k).",
    ) else {
        return;
    };
    let n = surface_u64(&container, "data-n", 4);
    let k = surface_u64(&container, "data-k", 2);
    let sketch = match local_stirling_second(n, k) {
        Some(v) => format!("{v}"),
        None => "overflow".into(),
    };
    invoke_dual(
        document,
        label,
        "NumberTheory.stirling_second",
        format!(
            "Local Stirling-2 sketch: S({n}, {k}) → {sketch}. Connect QualiaDB for a live stirling_second."
        ),
        json!({ "n": n, "k": k }),
    );
}

/// `NumberTheory.stirling_first` — `{ n, k }`.
pub(super) fn run_nt_stirling_first(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n and data-k before c(n,k).",
    ) else {
        return;
    };
    let n = surface_u64(&container, "data-n", 4);
    let k = surface_u64(&container, "data-k", 2);
    let sketch = match local_stirling_first(n, k) {
        Some(v) => format!("{v}"),
        None => "overflow".into(),
    };
    invoke_dual(
        document,
        label,
        "NumberTheory.stirling_first",
        format!(
            "Local Stirling-1 sketch: c({n}, {k}) → {sketch}. Connect QualiaDB for a live stirling_first."
        ),
        json!({ "n": n, "k": k }),
    );
}

/// `NumberTheory.extended_gcd` — `{ a, b }` (signed).
pub(super) fn run_nt_extended_gcd(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-a and data-b before extended_gcd.",
    ) else {
        return;
    };
    let a = surface_i64(&container, "data-a", 240);
    let b = surface_i64(&container, "data-b", 46);
    let (g, x, y) = local_extended_gcd(a, b);
    invoke_dual(
        document,
        label,
        "NumberTheory.extended_gcd",
        format!(
            "Local extended_gcd sketch: gcd({a}, {b}) → g={g}, x={x}, y={y}. Connect QualiaDB for a live extended_gcd."
        ),
        json!({ "a": a, "b": b }),
    );
}

/// `NumberTheory.crt` — `{ r1, m1, r2, m2 }`.
pub(super) fn run_nt_crt(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-r1, data-m1, data-r2, data-m2 before CRT.",
    ) else {
        return;
    };
    let r1 = surface_u64(&container, "data-r1", 2);
    let m1 = surface_u64(&container, "data-m1", 3);
    let r2 = surface_u64(&container, "data-r2", 3);
    let m2 = surface_u64(&container, "data-m2", 5);
    let sketch = match local_crt(r1, m1, r2, m2) {
        Some((x, m)) => format!("x={x} mod {m}"),
        None => "none (non-coprime)".into(),
    };
    invoke_dual(
        document,
        label,
        "NumberTheory.crt",
        format!(
            "Local CRT sketch: x≡{r1} (mod {m1}), x≡{r2} (mod {m2}) → {sketch}. Connect QualiaDB for a live crt."
        ),
        json!({ "r1": r1, "m1": m1, "r2": r2, "m2": m2 }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_nt_basics_match_known() {
        assert_eq!(local_gcd(48, 18), 6);
        assert_eq!(local_lcm(4, 6), 12);
        assert!(local_is_prime(17));
        assert!(!local_is_prime(15));
        assert_eq!(local_factorial(5), Some(120));
        assert_eq!(local_binomial(10, 3), Some(120));
        assert_eq!(local_euler_totient(12), 4);
        assert_eq!(local_mod_pow(2, 10, 1000), 24);
        assert_eq!(local_next_prime(14), 17);
        assert_eq!(local_mod_inverse(3, 11), Some(4));
        assert_eq!(local_divisor_count(12), 6);
    }

    #[test]
    fn local_nt_wave18_remainder_match_known() {
        assert_eq!(local_prime_factors(360), vec![(2, 3), (3, 2), (5, 1)]);
        assert_eq!(local_divisors(28), vec![1, 2, 4, 7, 14, 28]);
        assert_eq!(local_mobius(30), -1);
        assert_eq!(local_mobius(4), 0);
        assert_eq!(local_divisor_sum(28), 56);
        assert_eq!(local_partitions(10), 42);
        assert_eq!(local_catalan(5), Some(42));
        assert_eq!(local_stirling_second(4, 2), Some(7));
        assert_eq!(local_stirling_first(4, 2), Some(11));
        let (g, x, y) = local_extended_gcd(240, 46);
        assert_eq!(g, 2);
        assert_eq!(240 * x + 46 * y, g);
        assert_eq!(local_crt(2, 3, 3, 5), Some((8, 15)));
    }
}
