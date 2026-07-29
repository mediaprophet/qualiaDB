use super::*;

/// Trading engine
pub struct TradingEngine {
    order_manager: OrderManager,
    execution_engine: ExecutionEngine,
    position_manager: PositionManager,
}

/// Position manager
pub struct PositionManager {
    positions: HashMap<String, Position>,
    position_limits: HashMap<String, PositionLimit>,
    margin_calculator: MarginCalculator,
}

/// Positions
#[derive(Debug, Clone)]
pub struct Position {
    pub position_id: String,
    pub portfolio_id: String,
    pub asset_id: String,
    pub quantity: f64,
    pub average_cost: f64,
    pub market_value: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub last_updated: u64,
}

/// Position limits
#[derive(Debug, Clone)]
pub struct PositionLimit {
    pub limit_id: String,
    pub asset_id: String,
    pub max_position: f64,
    pub min_position: f64,
    pub warning_threshold: f64,
}

/// Margin calculator
pub struct MarginCalculator {
    margin_methods: HashMap<String, MarginMethod>,
    margin_requirements: MarginRequirements,
}

/// Margin methods
#[derive(Debug, Clone)]
pub struct MarginMethod {
    pub method_id: String,
    pub method_type: MarginMethodType,
    pub parameters: MarginMethodParameters,
}

/// Margin method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MarginMethodType {
    SPAN,
    TIMS,
    PortfolioMargin,
    RegT,
}

/// Margin method parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginMethodParameters {
    pub volatility_multiplier: f64,
    pub concentration_factor: f64,
    pub stress_period: u32,
}

impl TradingEngine {
    pub fn new() -> Self {
        Self {
            order_manager: OrderManager::new(),
            execution_engine: ExecutionEngine::new(),
            position_manager: PositionManager::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.order_manager.initialize()?;
        self.execution_engine.initialize()?;
        Ok(())
    }

    pub fn validate_order(&self, order: &Order) -> Result<(), FinancialError> {
        if order.quantity <= 0.0 {
            return Err(FinancialError::ValidationError(
                "Order quantity must be positive".to_string(),
            ));
        }
        if let Some(price) = order.price {
            if price <= 0.0 {
                return Err(FinancialError::ValidationError(
                    "Order price must be positive".to_string(),
                ));
            }
        }
        Ok(())
    }

    pub fn execute_trade(&mut self, _order: &Order) -> Result<TradeResult, FinancialError> {
        // NOT IMPLEMENTED — and this one must never fabricate. The previous body returned a
        // `TradeResult { status: Filled, executed_price: order.price.unwrap_or(100.0) }` — a
        // *fake fill* at a fabricated default price for a trade that never executed. Reporting a
        // filled trade that did not happen is dangerous. Real execution requires a broker/exchange
        // connection and order-management — and as a matter of policy this system must not place
        // real orders or move money. It therefore refuses, explicitly.
        Err(FinancialError::NotImplemented(
            "trade execution (execute_trade): no broker/exchange connection; this system does not \
             place orders or move money. Refusing to report a fabricated fill."
                .to_string(),
        ))
    }

    pub fn position_manager(&self) -> &PositionManager {
        &self.position_manager
    }

    pub fn position_manager_mut(&mut self) -> &mut PositionManager {
        &mut self.position_manager
    }
}

impl PositionManager {
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
            position_limits: HashMap::new(),
            margin_calculator: MarginCalculator::new(),
        }
    }

    pub fn add_position(&mut self, position: Position) {
        self.positions
            .insert(position.position_id.clone(), position);
    }

    pub fn get_position(&self, position_id: &str) -> Option<&Position> {
        self.positions.get(position_id)
    }

    pub fn list_positions(&self) -> Vec<String> {
        self.positions.keys().cloned().collect()
    }

    pub fn add_position_limit(&mut self, limit: PositionLimit) {
        self.position_limits.insert(limit.limit_id.clone(), limit);
    }

    pub fn get_position_limit(&self, limit_id: &str) -> Option<&PositionLimit> {
        self.position_limits.get(limit_id)
    }

    pub fn margin_calculator(&self) -> &MarginCalculator {
        &self.margin_calculator
    }

    pub fn margin_calculator_mut(&mut self) -> &mut MarginCalculator {
        &mut self.margin_calculator
    }
}

impl MarginCalculator {
    pub fn new() -> Self {
        Self {
            margin_methods: HashMap::new(),
            margin_requirements: MarginRequirements::new(),
        }
    }

    pub fn add_margin_method(&mut self, method: MarginMethod) {
        self.margin_methods.insert(method.method_id.clone(), method);
    }

    pub fn get_margin_method(&self, method_id: &str) -> Option<&MarginMethod> {
        self.margin_methods.get(method_id)
    }

    pub fn list_margin_methods(&self) -> Vec<String> {
        self.margin_methods.keys().cloned().collect()
    }

    pub fn margin_requirements(&self) -> &MarginRequirements {
        &self.margin_requirements
    }
}
