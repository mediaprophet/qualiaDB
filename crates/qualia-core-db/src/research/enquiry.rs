//! Research enquiry — purpose, scope, constraints, questions.

use std::collections::BTreeMap;

/// A research constraint (e.g. temporal, geographic, methodological).
#[derive(Debug, Clone)]
pub struct ResearchConstraint {
    pub constraint_type: String,
    pub value: String,
    pub description: String,
}

/// A research question linked to the enquiry.
#[derive(Debug, Clone)]
pub struct ResearchQuestion {
    pub id: String,
    pub text: String,
    pub linked_questions: Vec<String>,
    pub answered: bool,
}

/// A research enquiry with scope, constraints, and questions.
#[derive(Debug, Clone)]
pub struct ResearchEnquiry {
    pub id: String,
    pub purpose: String,
    pub scope: Vec<String>,
    pub constraints: Vec<ResearchConstraint>,
    pub questions: Vec<ResearchQuestion>,
    pub metadata: BTreeMap<String, String>,
}

impl ResearchEnquiry {
    pub fn new(id: &str, purpose: &str) -> Self {
        Self {
            id: id.to_string(),
            purpose: purpose.to_string(),
            scope: Vec::new(),
            constraints: Vec::new(),
            questions: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn set_purpose(&mut self, purpose: &str) {
        self.purpose = purpose.to_string();
    }

    pub fn define_scope(&mut self, scope: Vec<String>) {
        self.scope = scope;
    }

    pub fn add_constraint(&mut self, constraint: ResearchConstraint) {
        self.constraints.push(constraint);
    }

    pub fn add_question(&mut self, question: ResearchQuestion) {
        self.questions.push(question);
    }

    pub fn link_questions(&mut self, q1: &str, q2: &str) {
        for q in &mut self.questions {
            if q.id == q1 && !q.linked_questions.contains(&q2.to_string()) {
                q.linked_questions.push(q2.to_string());
            }
            if q.id == q2 && !q.linked_questions.contains(&q1.to_string()) {
                q.linked_questions.push(q1.to_string());
            }
        }
    }

    pub fn question_count(&self) -> usize {
        self.questions.len()
    }

    pub fn answered_count(&self) -> usize {
        self.questions.iter().filter(|q| q.answered).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enquiry_creation() {
        let eq = ResearchEnquiry::new("r1", "Investigate causal links");
        assert_eq!(eq.id, "r1");
        assert_eq!(eq.purpose, "Investigate causal links");
        assert!(eq.questions.is_empty());
    }

    #[test]
    fn enquiry_add_question() {
        let mut eq = ResearchEnquiry::new("r1", "Test");
        eq.add_question(ResearchQuestion {
            id: "q1".into(),
            text: "What happened?".into(),
            linked_questions: vec![],
            answered: false,
        });
        assert_eq!(eq.question_count(), 1);
    }

    #[test]
    fn enquiry_link_questions() {
        let mut eq = ResearchEnquiry::new("r1", "Test");
        eq.add_question(ResearchQuestion {
            id: "q1".into(),
            text: "Why?".into(),
            linked_questions: vec![],
            answered: false,
        });
        eq.add_question(ResearchQuestion {
            id: "q2".into(),
            text: "How?".into(),
            linked_questions: vec![],
            answered: false,
        });
        eq.link_questions("q1", "q2");
        assert!(eq.questions[0].linked_questions.contains(&"q2".to_string()));
        assert!(eq.questions[1].linked_questions.contains(&"q1".to_string()));
    }

    #[test]
    fn enquiry_scope_and_constraints() {
        let mut eq = ResearchEnquiry::new("r1", "Test");
        eq.define_scope(vec!["2020-2024".into(), "EU".into()]);
        eq.add_constraint(ResearchConstraint {
            constraint_type: "temporal".into(),
            value: "2020-2024".into(),
            description: "Year range".into(),
        });
        assert_eq!(eq.scope.len(), 2);
        assert_eq!(eq.constraints.len(), 1);
    }
}
