//! Sentiment analysis — deterministic symbolic sentiment assessment.

use std::collections::BTreeMap;

/// Sentiment dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SentimentDimension {
    Valence,      // Positive/negative
    Arousal,      // Calm/excited
    Dominance,    // Submissive/dominant
    Authenticity, // Performed/genuine
}

impl SentimentDimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Valence => "valence",
            Self::Arousal => "arousal",
            Self::Dominance => "dominance",
            Self::Authenticity => "authenticity",
        }
    }
}

/// A sentiment assessment of a text.
#[derive(Debug, Clone)]
pub struct SentimentAssessment {
    pub id: String,
    pub text_ref: String,
    pub dimensions: BTreeMap<SentimentDimension, f64>,
    pub performed: bool,
    pub manipulation_detected: bool,
    pub manipulation_indicators: Vec<String>,
}

/// A sentiment trend over time.
#[derive(Debug, Clone)]
pub struct SentimentTrend {
    pub timestamps: Vec<String>,
    pub values: Vec<f64>,
}

impl SentimentTrend {
    pub fn new() -> Self {
        Self {
            timestamps: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn add_point(&mut self, timestamp: &str, value: f64) {
        self.timestamps.push(timestamp.to_string());
        self.values.push(value);
    }

    /// Analyse the trend — direction, volatility, and manipulation indicators.
    pub fn analyse(&self) -> TrendAnalysis {
        if self.values.len() < 2 {
            return TrendAnalysis {
                direction: TrendDirection::Flat,
                volatility: 0.0,
                suspicious_uniformity: false,
            };
        }
        let first = self.values.first().unwrap();
        let last = self.values.last().unwrap();
        let direction = if last > first {
            TrendDirection::Increasing
        } else if last < first {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Flat
        };

        let mean = self.values.iter().sum::<f64>() / self.values.len() as f64;
        let variance =
            self.values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / self.values.len() as f64;
        let volatility = variance.sqrt();

        // Suspicious uniformity — all values within 0.01 of each other.
        let suspicious_uniformity = self.values.iter().all(|v| (v - mean).abs() < 0.01);

        TrendAnalysis {
            direction,
            volatility,
            suspicious_uniformity,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrendAnalysis {
    pub direction: TrendDirection,
    pub volatility: f64,
    pub suspicious_uniformity: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Flat,
}

impl Default for SentimentTrend {
    fn default() -> Self {
        Self::new()
    }
}

impl SentimentAssessment {
    pub fn new(id: &str, text_ref: &str) -> Self {
        Self {
            id: id.to_string(),
            text_ref: text_ref.to_string(),
            dimensions: BTreeMap::new(),
            performed: false,
            manipulation_detected: false,
            manipulation_indicators: Vec::new(),
        }
    }

    pub fn set_dimension(&mut self, dimension: SentimentDimension, value: f64) {
        self.dimensions.insert(dimension, value.clamp(-1.0, 1.0));
    }

    /// Assess sentiment from text using deterministic lexical matching.
    pub fn assess_sentiment(text: &str) -> f64 {
        let positive_words = [
            "good",
            "great",
            "excellent",
            "happy",
            "positive",
            "wonderful",
            "amazing",
            "love",
            "best",
            "beautiful",
        ];
        let negative_words = [
            "bad", "terrible", "awful", "sad", "negative", "horrible", "hate", "worst", "ugly",
            "angry",
        ];
        let lower = text.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        let positive = words
            .iter()
            .filter(|w| {
                positive_words.contains(
                    &w.to_lowercase()
                        .trim_matches(|c: char| !c.is_alphanumeric()),
                )
            })
            .count();
        let negative = words
            .iter()
            .filter(|w| {
                negative_words.contains(
                    &w.to_lowercase()
                        .trim_matches(|c: char| !c.is_alphanumeric()),
                )
            })
            .count();
        let total = positive + negative;
        if total == 0 {
            0.0
        } else {
            (positive as f64 - negative as f64) / total as f64
        }
    }

    /// Detect performed (inauthentic) sentiment — excessive positivity or
    /// uniform emotional language.
    pub fn detect_performed_sentiment(text: &str) -> bool {
        let lower = text.to_lowercase();
        let exclamation_count = lower.matches('!').count();
        let positive_words = [
            "amazing",
            "incredible",
            "awesome",
            "perfect",
            "best ever",
            "love love",
        ];
        let positive_count = positive_words.iter().filter(|w| lower.contains(*w)).count();
        exclamation_count > 3 || positive_count > 2
    }

    /// Detect sentiment manipulation — coordinated messaging patterns.
    pub fn detect_sentiment_manipulation(texts: &[String]) -> Vec<String> {
        let mut indicators = Vec::new();
        if texts.is_empty() {
            return indicators;
        }

        // Check for uniform sentiment across texts (coordinated messaging).
        let sentiments: Vec<f64> = texts.iter().map(|t| Self::assess_sentiment(t)).collect();
        let mean = sentiments.iter().sum::<f64>() / sentiments.len() as f64;
        let all_similar = sentiments.iter().all(|s| (s - mean).abs() < 0.1);
        if all_similar && texts.len() > 3 {
            indicators.push(
                "uniform sentiment across multiple texts — possible coordination".to_string(),
            );
        }

        // Check for excessive positive sentiment.
        if mean > 0.8 {
            indicators.push("excessively positive sentiment — possible astroturfing".to_string());
        }

        // Check for performed sentiment markers.
        let performed_count = texts
            .iter()
            .filter(|t| Self::detect_performed_sentiment(t))
            .count();
        if performed_count > texts.len() / 2 {
            indicators.push("majority of texts show performed sentiment markers".to_string());
        }

        indicators
    }

    /// Map a sentiment network — which entities are associated with which sentiments.
    pub fn map_sentiment_network(
        mentions: &[(String, String, f64)],
    ) -> BTreeMap<String, Vec<(String, f64)>> {
        // (entity, target, sentiment) → entity → [(target, sentiment)]
        let mut network: BTreeMap<String, Vec<(String, f64)>> = BTreeMap::new();
        for (entity, target, sentiment) in mentions {
            network
                .entry(entity.clone())
                .or_default()
                .push((target.clone(), *sentiment));
        }
        network
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assess_positive_sentiment() {
        let score =
            SentimentAssessment::assess_sentiment("This is great and wonderful, I love it!");
        assert!(score > 0.0);
    }

    #[test]
    fn assess_negative_sentiment() {
        let score = SentimentAssessment::assess_sentiment("This is terrible and awful, I hate it.");
        assert!(score < 0.0);
    }

    #[test]
    fn assess_neutral_sentiment() {
        let score = SentimentAssessment::assess_sentiment("The table is brown.");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn detect_performed_sentiment_exclamation() {
        assert!(SentimentAssessment::detect_performed_sentiment(
            "This is amazing!!!"
        ));
    }

    #[test]
    fn detect_performed_sentiment_normal() {
        assert!(!SentimentAssessment::detect_performed_sentiment(
            "This is a normal sentence."
        ));
    }

    #[test]
    fn detect_manipulation_uniform() {
        let texts = vec![
            "This is great and wonderful".to_string(),
            "This is great and wonderful".to_string(),
            "This is great and wonderful".to_string(),
            "This is great and wonderful".to_string(),
        ];
        let indicators = SentimentAssessment::detect_sentiment_manipulation(&texts);
        assert!(!indicators.is_empty());
    }

    #[test]
    fn map_sentiment_network_basic() {
        let mentions = vec![
            ("alice".to_string(), "bob".to_string(), 0.8),
            ("alice".to_string(), "carol".to_string(), -0.3),
        ];
        let network = SentimentAssessment::map_sentiment_network(&mentions);
        assert!(network.contains_key("alice"));
        assert_eq!(network.get("alice").unwrap().len(), 2);
    }

    #[test]
    fn trend_analysis() {
        let mut trend = SentimentTrend::new();
        trend.add_point("t1", 0.2);
        trend.add_point("t2", 0.5);
        trend.add_point("t3", 0.8);
        let analysis = trend.analyse();
        assert_eq!(analysis.direction, TrendDirection::Increasing);
    }

    #[test]
    fn trend_suspicious_uniformity() {
        let mut trend = SentimentTrend::new();
        trend.add_point("t1", 0.5);
        trend.add_point("t2", 0.5);
        trend.add_point("t3", 0.5);
        let analysis = trend.analyse();
        assert!(analysis.suspicious_uniformity);
    }
}
