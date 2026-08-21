//! Dynamics analysis — social, economic, spatiotemporal dynamics.

use std::collections::BTreeMap;

/// Social dynamics configuration.
#[derive(Debug, Clone)]
pub struct SocialDynamics {
    pub id: String,
    pub network_type: String,
    pub agents: Vec<String>,
    pub interactions: Vec<(String, String, f64)>, // (agent_a, agent_b, strength)
}

/// Economic dynamics configuration.
#[derive(Debug, Clone)]
pub struct EconomicDynamics {
    pub id: String,
    pub variables: Vec<String>,
    pub relationships: Vec<(String, String, f64)>, // (cause, effect, coefficient)
}

/// Spatiotemporal dynamics configuration.
#[derive(Debug, Clone)]
pub struct SpatiotemporalDynamics {
    pub id: String,
    pub spatial_extent: [f64; 4],  // min_x, min_y, max_x, max_y
    pub temporal_extent: [f64; 2], // start, end
    pub diffusion_rate: f64,
}

/// Analyse a social network — compute centrality measures.
pub fn analyse_social_network(
    agents: &[String],
    interactions: &[(String, String, f64)],
) -> NetworkAnalysis {
    let mut degree: BTreeMap<String, usize> = BTreeMap::new();
    for agent in agents {
        degree.insert(agent.clone(), 0);
    }
    for (a, b, _) in interactions {
        *degree.entry(a.clone()).or_insert(0) += 1;
        *degree.entry(b.clone()).or_insert(0) += 1;
    }
    let max_degree = degree.values().copied().max().unwrap_or(0);
    let avg_degree = if agents.is_empty() {
        0.0
    } else {
        degree.values().sum::<usize>() as f64 / agents.len() as f64
    };
    let most_central = degree
        .iter()
        .max_by_key(|(_, &v)| v)
        .map(|(k, _)| k.clone())
        .unwrap_or_default();
    NetworkAnalysis {
        agent_count: agents.len(),
        interaction_count: interactions.len(),
        max_degree,
        avg_degree,
        most_central_agent: most_central,
    }
}

#[derive(Debug, Clone)]
pub struct NetworkAnalysis {
    pub agent_count: usize,
    pub interaction_count: usize,
    pub max_degree: usize,
    pub avg_degree: f64,
    pub most_central_agent: String,
}

/// Analyse inequality — compute Gini coefficient from values.
pub fn analyse_inequality(values: &[f64]) -> InequalityAnalysis {
    if values.is_empty() {
        return InequalityAnalysis {
            gini: 0.0,
            mean: 0.0,
            min: 0.0,
            max: 0.0,
        };
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len() as f64;
    let sum: f64 = sorted.iter().sum();
    let mean = sum / n;
    let min = sorted[0];
    let max = sorted[sorted.len() - 1];

    // Gini coefficient: (sum of |xi - xj|) / (2 * n * mean)
    let mut abs_diff_sum = 0.0;
    for i in 0..sorted.len() {
        for j in 0..sorted.len() {
            abs_diff_sum += (sorted[i] - sorted[j]).abs();
        }
    }
    let gini = if mean > 0.0 {
        abs_diff_sum / (2.0 * n * mean)
    } else {
        0.0
    };

    InequalityAnalysis {
        gini,
        mean,
        min,
        max,
    }
}

#[derive(Debug, Clone)]
pub struct InequalityAnalysis {
    pub gini: f64,
    pub mean: f64,
    pub min: f64,
    pub max: f64,
}

/// Analyse diffusion — simulate simple diffusion over time steps.
pub fn analyse_diffusion(
    initial_values: &[f64],
    diffusion_rate: f64,
    steps: usize,
) -> Vec<Vec<f64>> {
    if initial_values.is_empty() {
        return Vec::new();
    }
    let mut history = Vec::with_capacity(steps + 1);
    let mut current = initial_values.to_vec();
    history.push(current.clone());
    let n = current.len();
    for _ in 0..steps {
        let mut next = current.clone();
        for i in 0..n {
            let left = if i > 0 { current[i - 1] } else { current[0] };
            let right = if i < n - 1 {
                current[i + 1]
            } else {
                current[n - 1]
            };
            let laplacian = left - 2.0 * current[i] + right;
            next[i] = current[i] + diffusion_rate * laplacian;
        }
        current = next;
        history.push(current.clone());
    }
    history
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyse_social_network_basic() {
        let agents = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let interactions = vec![
            ("a".to_string(), "b".to_string(), 1.0),
            ("b".to_string(), "c".to_string(), 0.5),
            ("a".to_string(), "c".to_string(), 0.3),
        ];
        let analysis = analyse_social_network(&agents, &interactions);
        assert_eq!(analysis.agent_count, 3);
        assert_eq!(analysis.interaction_count, 3);
        assert_eq!(analysis.max_degree, 2);
    }

    #[test]
    fn analyse_inequality_equal() {
        let values = vec![100.0; 5];
        let analysis = analyse_inequality(&values);
        assert!((analysis.gini).abs() < 0.01);
    }

    #[test]
    fn analyse_inequality_unequal() {
        let values = vec![0.0, 0.0, 0.0, 0.0, 100.0];
        let analysis = analyse_inequality(&values);
        assert!(analysis.gini > 0.5);
    }

    #[test]
    fn analyse_diffusion_spreads() {
        let initial = vec![0.0, 0.0, 1.0, 0.0, 0.0];
        let history = analyse_diffusion(&initial, 0.1, 10);
        assert_eq!(history.len(), 11);
        // The peak should have spread — center value should decrease.
        assert!(history[10][2] < history[0][2]);
        // Neighbors should have increased.
        assert!(history[10][1] > history[0][1]);
        assert!(history[10][3] > history[0][3]);
    }

    #[test]
    fn analyse_diffusion_empty() {
        let history = analyse_diffusion(&[], 0.1, 5);
        assert!(history.is_empty());
    }

    #[test]
    fn analyse_inequality_empty() {
        let analysis = analyse_inequality(&[]);
        assert_eq!(analysis.gini, 0.0);
    }
}
