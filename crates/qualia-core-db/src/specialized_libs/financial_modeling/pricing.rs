use super::*;

/// Pricing engine
pub struct PricingEngine {
    pricing_models: HashMap<String, PricingModel>,
    market_data: MarketData,
    valuation_engine: ValuationEngine,
}

/// Pricing models
#[derive(Debug, Clone)]
pub struct PricingModel {
    pub model_id: String,
    pub model_type: PricingModelType,
    pub parameters: PricingModelParameters,
}

/// Pricing model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PricingModelType {
    BlackScholes,
    Binomial,
    MonteCarlo,
    FiniteDifference,
    Analytical,
}

/// Pricing model parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingModelParameters {
    pub risk_free_rate: f64,
    pub volatility: f64,
    pub dividend_yield: f64,
    pub time_to_maturity: f64,
}

/// Valuation engine
pub struct ValuationEngine {
    valuation_methods: HashMap<String, ValuationMethod>,
    discount_rates: HashMap<String, f64>,
    cash_flow_projections: HashMap<String, CashFlowProjection>,
}

/// Valuation methods
#[derive(Debug, Clone)]
pub struct ValuationMethod {
    pub method_id: String,
    pub method_type: ValuationMethodType,
    pub parameters: ValuationMethodParameters,
}

/// Valuation method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValuationMethodType {
    DCF,
    DDM,
    Multiples,
    AssetBased,
    OptionPricing,
}

/// Valuation method parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationMethodParameters {
    pub discount_rate: f64,
    pub growth_rate: f64,
    pub terminal_growth: f64,
    pub multiples: HashMap<String, f64>,
}

/// Cash flow projections
#[derive(Debug, Clone)]
pub struct CashFlowProjection {
    pub projection_id: String,
    pub cash_flows: Vec<CashFlow>,
    pub assumptions: Vec<Assumption>,
}

/// Cash flows
#[derive(Debug, Clone)]
pub struct CashFlow {
    pub period: u32,
    pub amount: f64,
    pub cash_flow_type: CashFlowType,
}

/// Cash flow types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CashFlowType {
    Operating,
    Investing,
    Financing,
    Free,
}

/// Assumptions
#[derive(Debug, Clone)]
pub struct Assumption {
    pub assumption_id: String,
    pub assumption_name: String,
    pub assumption_value: f64,
    pub justification: String,
}

impl PricingEngine {
    pub fn new() -> Self {
        Self {
            pricing_models: HashMap::new(),
            market_data: MarketData::new(),
            valuation_engine: ValuationEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.valuation_engine.initialize()?;
        Ok(())
    }

    pub fn validate_option_parameters(
        &self,
        params: &OptionParameters,
    ) -> Result<(), FinancialError> {
        if params.underlying_price <= 0.0 {
            return Err(FinancialError::ValidationError(
                "Underlying price must be positive".to_string(),
            ));
        }
        if params.strike <= 0.0 {
            return Err(FinancialError::ValidationError(
                "Strike price must be positive".to_string(),
            ));
        }
        if params.time_to_maturity < 0.0 {
            return Err(FinancialError::ValidationError(
                "Time to maturity must be non-negative".to_string(),
            ));
        }
        if params.volatility < 0.0 {
            return Err(FinancialError::ValidationError(
                "Volatility must be non-negative".to_string(),
            ));
        }
        Ok(())
    }

    pub fn price_option(&self, params: &OptionParameters) -> Result<OptionPrice, FinancialError> {
        // Price option using Black-Scholes
        let option_price = self.black_scholes_price(params)?;
        Ok(option_price)
    }

    fn black_scholes_price(
        &self,
        params: &OptionParameters,
    ) -> Result<OptionPrice, FinancialError> {
        let s = params.underlying_price;
        let k = params.strike;
        let r = params.risk_free_rate;
        let sigma = params.volatility;
        let t = params.time_to_maturity;

        // Edge case: zero time to expiry -> option is worth its intrinsic value
        // (no time value remains). Greeks collapse to their intrinsic boundary.
        if t <= 0.0 {
            return Ok(self.intrinsic_price(params));
        }

        // Edge case: zero volatility -> payoff is deterministic. The terminal
        // price is S*exp(rT), so the discounted call payoff is max(S - K*exp(-rT), 0)
        // and the put payoff is max(K*exp(-rT) - S, 0). Greeks are zero except
        // delta, which is the step function at the strike.
        if sigma <= 0.0 {
            let disc = (-r * t).exp();
            let fwd = s - k * disc;
            let (price, delta) = match params.option_type {
                OptionType::Call => (fwd.max(0.0), if fwd > 0.0 { 1.0 } else { 0.0 }),
                OptionType::Put => ((-fwd).max(0.0), if fwd < 0.0 { -1.0 } else { 0.0 }),
            };
            return Ok(OptionPrice {
                price,
                delta,
                gamma: 0.0,
                theta: 0.0,
                vega: 0.0,
                rho: 0.0,
            });
        }

        // Edge case: zero underlying price -> call is worthless, put is the
        // discounted strike.
        if s <= 0.0 {
            let disc = (-r * t).exp();
            return Ok(match params.option_type {
                OptionType::Call => OptionPrice {
                    price: 0.0,
                    delta: 0.0,
                    gamma: 0.0,
                    theta: 0.0,
                    vega: 0.0,
                    rho: 0.0,
                },
                OptionType::Put => OptionPrice {
                    price: k * disc,
                    delta: -1.0,
                    gamma: 0.0,
                    theta: r * k * disc,
                    vega: 0.0,
                    rho: -t * k * disc,
                },
            });
        }

        // Standard Black-Scholes formula.
        let sqrt_t = t.sqrt();
        let d1 = ((s / k).ln() + (r + 0.5 * sigma * sigma) * t) / (sigma * sqrt_t);
        let d2 = d1 - sigma * sqrt_t;
        let disc = (-r * t).exp();
        let pdf_d1 = self.normal_pdf(d1);

        let (price, delta) = match params.option_type {
            OptionType::Call => {
                let p = s * self.normal_cdf(d1) - k * disc * self.normal_cdf(d2);
                (p, self.normal_cdf(d1))
            }
            OptionType::Put => {
                let p = k * disc * self.normal_cdf(-d2) - s * self.normal_cdf(-d1);
                (p, self.normal_cdf(d1) - 1.0)
            }
        };

        // Gamma and Vega are identical for calls and puts.
        let gamma = pdf_d1 / (s * sigma * sqrt_t);
        let vega = s * pdf_d1 * sqrt_t;

        let theta = self.calculate_theta(params, d1, d2, pdf_d1);
        let rho = self.calculate_rho(params, d2, disc);

        Ok(OptionPrice {
            price,
            delta,
            gamma,
            theta,
            vega,
            rho,
        })
    }

    /// Intrinsic value at expiry (T=0): call = max(S-K, 0), put = max(K-S, 0).
    /// Delta is the step at the strike; other Greeks are zero.
    fn intrinsic_price(&self, params: &OptionParameters) -> OptionPrice {
        let intrinsic = match params.option_type {
            OptionType::Call => (params.underlying_price - params.strike).max(0.0),
            OptionType::Put => (params.strike - params.underlying_price).max(0.0),
        };
        let delta = match params.option_type {
            OptionType::Call => {
                if params.underlying_price > params.strike {
                    1.0
                } else {
                    0.0
                }
            }
            OptionType::Put => {
                if params.underlying_price < params.strike {
                    -1.0
                } else {
                    0.0
                }
            }
        };
        OptionPrice {
            price: intrinsic,
            delta,
            gamma: 0.0,
            theta: 0.0,
            vega: 0.0,
            rho: 0.0,
        }
    }

    fn normal_cdf(&self, x: f64) -> f64 {
        // Abramowitz and Stegun approximation for normal CDF (max error 7.5e-8)
        let t = 1.0 / (1.0 + 0.2316419 * x.abs());
        let d = 0.3989422819 * (-x * x / 2.0).exp();
        let p = d
            * t
            * (0.3193815306
                + t * (-0.3565637813
                    + t * (1.7814779372 + t * (-1.8212559978 + t * 1.3302744929))));
        if x >= 0.0 {
            1.0 - p
        } else {
            p
        }
    }

    fn normal_pdf(&self, x: f64) -> f64 {
        (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
    }

    fn calculate_theta(&self, params: &OptionParameters, _d1: f64, d2: f64, pdf_d1: f64) -> f64 {
        // Theta per calendar day (divided by 365). The annualized theta is the
        // standard Black-Scholes expression; reporting per-day matches how the
        // Greek is conventionally quoted.
        let sqrt_t = params.time_to_maturity.sqrt();
        let disc = (-params.risk_free_rate * params.time_to_maturity).exp();
        let annualized = match params.option_type {
            OptionType::Call => {
                -(params.underlying_price * pdf_d1 * params.volatility) / (2.0 * sqrt_t)
                    - params.risk_free_rate * params.strike * disc * self.normal_cdf(d2)
            }
            OptionType::Put => {
                -(params.underlying_price * pdf_d1 * params.volatility) / (2.0 * sqrt_t)
                    + params.risk_free_rate * params.strike * disc * self.normal_cdf(-d2)
            }
        };
        annualized / 365.0
    }

    fn calculate_rho(&self, params: &OptionParameters, d2: f64, disc: f64) -> f64 {
        match params.option_type {
            OptionType::Call => {
                params.strike * params.time_to_maturity * disc * self.normal_cdf(d2)
            }
            OptionType::Put => {
                -params.strike * params.time_to_maturity * disc * self.normal_cdf(-d2)
            }
        }
    }

    pub fn add_pricing_model(&mut self, model: PricingModel) {
        self.pricing_models.insert(model.model_id.clone(), model);
    }

    pub fn get_pricing_model(&self, model_id: &str) -> Option<&PricingModel> {
        self.pricing_models.get(model_id)
    }

    pub fn list_pricing_models(&self) -> Vec<String> {
        self.pricing_models.keys().cloned().collect()
    }

    pub fn market_data(&self) -> &MarketData {
        &self.market_data
    }

    pub fn market_data_mut(&mut self) -> &mut MarketData {
        &mut self.market_data
    }
}

impl ValuationEngine {
    pub fn new() -> Self {
        Self {
            valuation_methods: HashMap::new(),
            discount_rates: HashMap::new(),
            cash_flow_projections: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }

    pub fn add_valuation_method(&mut self, method: ValuationMethod) {
        self.valuation_methods
            .insert(method.method_id.clone(), method);
    }

    pub fn get_valuation_method(&self, method_id: &str) -> Option<&ValuationMethod> {
        self.valuation_methods.get(method_id)
    }

    pub fn list_valuation_methods(&self) -> Vec<String> {
        self.valuation_methods.keys().cloned().collect()
    }

    pub fn set_discount_rate(&mut self, name: &str, rate: f64) {
        self.discount_rates.insert(name.to_string(), rate);
    }

    pub fn get_discount_rate(&self, name: &str) -> Option<&f64> {
        self.discount_rates.get(name)
    }

    pub fn add_cash_flow_projection(&mut self, projection: CashFlowProjection) {
        self.cash_flow_projections
            .insert(projection.projection_id.clone(), projection);
    }

    pub fn get_cash_flow_projection(&self, projection_id: &str) -> Option<&CashFlowProjection> {
        self.cash_flow_projections.get(projection_id)
    }
}
