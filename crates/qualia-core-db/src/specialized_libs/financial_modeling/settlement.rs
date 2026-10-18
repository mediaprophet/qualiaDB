use super::*;

/// Settlement engine
pub struct SettlementEngine {
    settlement_methods: HashMap<String, SettlementMethod>,
    clearing_house: ClearingHouse,
    settlement_validator: SettlementValidator,
}

/// Settlement methods
#[derive(Debug, Clone)]
pub struct SettlementMethod {
    pub method_id: String,
    pub method_name: String,
    pub method_type: SettlementMethodType,
    pub settlement_cycle: u32,
}

/// Settlement method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettlementMethodType {
    TPlus0,
    TPlus1,
    TPlus2,
    TPlus3,
    Continuous,
}

/// Clearing house
pub struct ClearingHouse {
    pub house_id: String,
    pub house_name: String,
    pub margin_requirements: MarginRequirements,
    pub risk_management: RiskManagement,
}

/// Margin requirements
#[derive(Debug, Clone)]
pub struct MarginRequirements {
    pub initial_margin: f64,
    pub maintenance_margin: f64,
    pub variation_margin: f64,
}

/// Risk management
#[derive(Debug, Clone)]
pub struct RiskManagement {
    pub position_limits: HashMap<String, f64>,
    pub stress_scenarios: Vec<String>,
    pub collateral_requirements: CollateralRequirements,
}

/// Collateral requirements
#[derive(Debug, Clone)]
pub struct CollateralRequirements {
    pub haircuts: HashMap<String, f64>,
    pub concentration_limits: HashMap<String, f64>,
    pub eligible_collateral: Vec<String>,
}

/// Settlement validator
pub struct SettlementValidator {
    validation_rules: Vec<SettlementValidationRule>,
    compliance_checker: SettlementComplianceChecker,
}

/// Settlement validation rules
#[derive(Debug, Clone)]
pub struct SettlementValidationRule {
    pub rule_id: String,
    pub rule_type: SettlementValidationRuleType,
    pub condition: String,
    pub action: SettlementValidationAction,
}

/// Settlement validation rule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettlementValidationRuleType {
    Funds,
    Securities,
    Timing,
    Compliance,
}

/// Settlement validation actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettlementValidationAction {
    Approve,
    Reject,
    Hold,
    Escalate,
}

/// Settlement compliance checker
pub struct SettlementComplianceChecker {
    compliance_rules: Vec<SettlementComplianceRule>,
    regulatory_requirements: Vec<RegulatoryRequirement>,
}

/// Settlement compliance rules
#[derive(Debug, Clone)]
pub struct SettlementComplianceRule {
    pub rule_id: String,
    pub rule_name: String,
    pub conditions: Vec<SettlementComplianceCondition>,
    pub actions: Vec<SettlementComplianceAction>,
}

/// Settlement compliance conditions
#[derive(Debug, Clone)]
pub struct SettlementComplianceCondition {
    pub condition_id: String,
    pub field: String,
    pub operator: ComparisonOperator,
    pub value: SettlementComplianceValue,
}

/// Settlement compliance values
#[derive(Debug, Clone)]
pub enum SettlementComplianceValue {
    String(String),
    Number(f64),
    Boolean(bool),
}

/// Settlement compliance actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettlementComplianceAction {
    Approve,
    Reject,
    Flag,
    Escalate,
}

impl SettlementEngine {
    /// Creates a `SettlementEngine` with empty registries / default configuration.
    pub fn new() -> Self {
        Self {
            settlement_methods: HashMap::new(),
            clearing_house: ClearingHouse::new(),
            settlement_validator: SettlementValidator::new(),
        }
    }

    /// Readies the `SettlementEngine` sub-components; registers/populates any built-in defaults.
    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }

    /// Registers `settlement method` in the registry (keyed by its id).
    pub fn add_settlement_method(&mut self, method: SettlementMethod) {
        self.settlement_methods
            .insert(method.method_id.clone(), method);
    }

    /// Looks up `settlement method` by id; `None` if not registered.
    pub fn get_settlement_method(&self, method_id: &str) -> Option<&SettlementMethod> {
        self.settlement_methods.get(method_id)
    }

    /// Iterates over all registered `settlement methods` entries.
    pub fn list_settlement_methods(&self) -> Vec<String> {
        self.settlement_methods.keys().cloned().collect()
    }

    /// Returns a reference to the inner `clearing_house` sub-component.
    pub fn clearing_house(&self) -> &ClearingHouse {
        &self.clearing_house
    }

    /// Mutable access to the inner `clearing_house` sub-component.
    pub fn clearing_house_mut(&mut self) -> &mut ClearingHouse {
        &mut self.clearing_house
    }

    /// Returns a reference to the inner `settlement_validator` sub-component.
    pub fn settlement_validator(&self) -> &SettlementValidator {
        &self.settlement_validator
    }

    /// Mutable access to the inner `settlement_validator` sub-component.
    pub fn settlement_validator_mut(&mut self) -> &mut SettlementValidator {
        &mut self.settlement_validator
    }
}

impl ClearingHouse {
    /// Creates a `ClearingHouse` with empty registries / default configuration.
    pub fn new() -> Self {
        Self {
            house_id: "default".to_string(),
            house_name: "Default Clearing House".to_string(),
            margin_requirements: MarginRequirements::new(),
            risk_management: RiskManagement::new(),
        }
    }
}

impl MarginRequirements {
    /// Creates a `MarginRequirements` with empty registries / default configuration.
    pub fn new() -> Self {
        Self {
            initial_margin: 0.5,
            maintenance_margin: 0.25,
            variation_margin: 0.1,
        }
    }
}

impl RiskManagement {
    /// Creates a `RiskManagement` with empty registries / default configuration.
    pub fn new() -> Self {
        Self {
            position_limits: HashMap::new(),
            stress_scenarios: Vec::new(),
            collateral_requirements: CollateralRequirements::new(),
        }
    }
}

impl CollateralRequirements {
    /// Creates a `CollateralRequirements` with empty registries / default configuration.
    pub fn new() -> Self {
        Self {
            haircuts: HashMap::new(),
            concentration_limits: HashMap::new(),
            eligible_collateral: Vec::new(),
        }
    }
}

impl SettlementValidator {
    /// Creates a `SettlementValidator` with empty registries / default configuration.
    pub fn new() -> Self {
        Self {
            validation_rules: Vec::new(),
            compliance_checker: SettlementComplianceChecker::new(),
        }
    }

    /// Registers `validation rule` in the registry (keyed by its id).
    pub fn add_validation_rule(&mut self, rule: SettlementValidationRule) {
        self.validation_rules.push(rule);
    }

    /// Iterates over all registered `validation rules` entries.
    pub fn list_validation_rules(&self) -> &[SettlementValidationRule] {
        &self.validation_rules
    }

    /// Returns a reference to the inner `compliance_checker` sub-component.
    pub fn compliance_checker(&self) -> &SettlementComplianceChecker {
        &self.compliance_checker
    }

    /// Mutable access to the inner `compliance_checker` sub-component.
    pub fn compliance_checker_mut(&mut self) -> &mut SettlementComplianceChecker {
        &mut self.compliance_checker
    }
}

impl SettlementComplianceChecker {
    /// Creates a `SettlementComplianceChecker` with empty registries / default configuration.
    pub fn new() -> Self {
        Self {
            compliance_rules: Vec::new(),
            regulatory_requirements: Vec::new(),
        }
    }

    /// Registers `compliance rule` in the registry (keyed by its id).
    pub fn add_compliance_rule(&mut self, rule: SettlementComplianceRule) {
        self.compliance_rules.push(rule);
    }

    /// Iterates over all registered `compliance rules` entries.
    pub fn list_compliance_rules(&self) -> &[SettlementComplianceRule] {
        &self.compliance_rules
    }

    /// Registers `regulatory requirement` in the registry (keyed by its id).
    pub fn add_regulatory_requirement(&mut self, requirement: RegulatoryRequirement) {
        self.regulatory_requirements.push(requirement);
    }

    /// Iterates over all registered `regulatory requirements` entries.
    pub fn list_regulatory_requirements(&self) -> &[RegulatoryRequirement] {
        &self.regulatory_requirements
    }
}

impl SettlementMethod {
    /// Creates a `SettlementMethod` with empty registries / default configuration.
    pub fn new() -> Self {
        Self {
            method_id: "settle_1".to_string(),
            method_name: "T+2".to_string(),
            method_type: SettlementMethodType::TPlus2,
            settlement_cycle: 2,
        }
    }
}

impl SettlementValidationRule {
    /// Creates a `SettlementValidationRule` with empty registries / default configuration.
    pub fn new() -> Self {
        Self {
            rule_id: "rule_1".to_string(),
            rule_type: SettlementValidationRuleType::Funds,
            condition: "sufficient_funds".to_string(),
            action: SettlementValidationAction::Approve,
        }
    }
}

impl SettlementComplianceCondition {
    /// Creates a `SettlementComplianceCondition` with empty registries / default configuration.
    pub fn new() -> Self {
        Self {
            condition_id: "cond_1".to_string(),
            field: "funds".to_string(),
            operator: ComparisonOperator::GreaterThan,
            value: SettlementComplianceValue::Number(0.0),
        }
    }
}

impl SettlementComplianceRule {
    /// Creates a `SettlementComplianceRule` with empty registries / default configuration.
    pub fn new() -> Self {
        Self {
            rule_id: "rule_1".to_string(),
            rule_name: "Funds validation".to_string(),
            conditions: vec![SettlementComplianceCondition::new()],
            actions: vec![SettlementComplianceAction::Approve],
        }
    }
}
