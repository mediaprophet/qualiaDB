//! Combinatorial functions: factorials, binomial coefficients, integer partitions,
//! Stirling numbers (both kinds) and Catalan numbers. Exact integer results; the
//! overflow-prone ones return `Option<u128>` (`None` past the `u128` ceiling) rather
//! than wrapping silently — fail closed.

/// `n!` as `u128`. `None` for `n ≥ 35` (`35!` overflows `u128`).
pub fn factorial(n: u64) -> Option<u128> {
    let mut acc: u128 = 1;
    for k in 2..=n as u128 {
        acc = acc.checked_mul(k)?;
    }
    Some(acc)
}

/// Binomial coefficient `C(n, k)` via the multiplicative formula (integer at every
/// step). `0` for `k > n`; `None` on `u128` overflow.
pub fn binomial(n: u64, k: u64) -> Option<u128> {
    if k > n {
        return Some(0);
    }
    let k = k.min(n - k); // symmetry C(n,k) = C(n,n−k)
    let mut result: u128 = 1;
    for i in 0..k {
        result = result.checked_mul((n - i) as u128)?;
        result /= (i + 1) as u128; // exact: C(n,i+1) is integer
    }
    Some(result)
}

/// The number of integer partitions `p(n)` (ways to write `n` as a sum of positive
/// integers, order-independent). DP in `O(n²)`; `u64` holds `p(n)` comfortably into the
/// hundreds. `p(0) = 1`.
pub fn partitions(n: u64) -> u64 {
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

/// Stirling numbers of the **second** kind `S(n, k)` — partitions of an `n`-set into
/// `k` non-empty subsets. `S(0,0) = 1`. `None` on overflow.
pub fn stirling_second(n: u64, k: u64) -> Option<u128> {
    let (n, k) = (n as usize, k as usize);
    if k > n {
        return Some(0);
    }
    let mut prev = vec![0u128; k + 1];
    prev[0] = 1; // S(0,0)
    for i in 1..=n {
        let mut cur = vec![0u128; k + 1];
        for j in 1..=k.min(i) {
            // S(i,j) = j·S(i−1,j) + S(i−1,j−1)
            let a = (j as u128).checked_mul(prev[j])?;
            cur[j] = a.checked_add(prev[j - 1])?;
        }
        prev = cur;
    }
    Some(prev[k])
}

/// **Unsigned** Stirling numbers of the **first** kind `c(n, k)` — permutations of `n`
/// elements with exactly `k` cycles. `c(0,0) = 1`. `None` on overflow.
pub fn stirling_first(n: u64, k: u64) -> Option<u128> {
    let (n, k) = (n as usize, k as usize);
    if k > n {
        return Some(0);
    }
    let mut prev = vec![0u128; k + 1];
    prev[0] = 1; // c(0,0)
    for i in 1..=n {
        let mut cur = vec![0u128; k + 1];
        for j in 1..=k.min(i) {
            // c(i,j) = (i−1)·c(i−1,j) + c(i−1,j−1)
            let a = ((i - 1) as u128).checked_mul(prev[j])?;
            cur[j] = a.checked_add(prev[j - 1])?;
        }
        prev = cur;
    }
    Some(prev[k])
}

/// The `n`-th Catalan number `C(2n, n)/(n+1)`. `None` on overflow.
pub fn catalan(n: u64) -> Option<u128> {
    let c = binomial(2 * n, n)?;
    Some(c / (n as u128 + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factorial_and_overflow_guard() {
        assert_eq!(factorial(0), Some(1));
        assert_eq!(factorial(5), Some(120));
        assert_eq!(factorial(20), Some(2_432_902_008_176_640_000));
        assert_eq!(factorial(34), Some(295232799039604140847618609643520000000));
        assert_eq!(factorial(35), None); // overflows u128 — fail closed
    }

    #[test]
    fn binomial_known_values() {
        assert_eq!(binomial(5, 2), Some(10));
        assert_eq!(binomial(10, 0), Some(1));
        assert_eq!(binomial(10, 10), Some(1));
        assert_eq!(binomial(52, 5), Some(2_598_960)); // poker hands
        assert_eq!(binomial(3, 5), Some(0)); // k > n
    }

    #[test]
    fn partition_counts() {
        assert_eq!(partitions(0), 1);
        assert_eq!(partitions(4), 5); // 4,3+1,2+2,2+1+1,1+1+1+1
        assert_eq!(partitions(5), 7);
        assert_eq!(partitions(10), 42);
    }

    #[test]
    fn stirling_numbers() {
        // S(4,2) = 7
        assert_eq!(stirling_second(4, 2), Some(7));
        // Σ_k S(n,k) = Bell number; B(3) = 5: S(3,1)+S(3,2)+S(3,3) = 1+3+1.
        let bell3: u128 = (1..=3).map(|k| stirling_second(3, k).unwrap()).sum();
        assert_eq!(bell3, 5);
        // c(4,2) = 11 (unsigned Stirling first kind)
        assert_eq!(stirling_first(4, 2), Some(11));
        // Σ_k c(n,k) = n!
        let sum: u128 = (0..=4).map(|k| stirling_first(4, k).unwrap()).sum();
        assert_eq!(sum, factorial(4).unwrap());
    }

    #[test]
    fn catalan_numbers() {
        // 1, 1, 2, 5, 14, 42, …
        let seq: Vec<u128> = (0..6).map(|n| catalan(n).unwrap()).collect();
        assert_eq!(seq, vec![1, 1, 2, 5, 14, 42]);
    }
}
