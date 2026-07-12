use super::*;


pub fn ml_inference(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::machine_learning::{
        InferenceParameters, MachineLearningLibrary, Precision,
    };

    let v = parse_tool_args(args)?;
    let model_id = v
        .get("model_id")
        .and_then(Value::as_str)
        .or_else(|| v.get("model").and_then(Value::as_str))
        .unwrap_or("mcp_model")
        .to_string();
    let model_path = v
        .get("model_path")
        .and_then(Value::as_str)
        .unwrap_or("in-memory");

    let input_data = if let Ok(bytes) = json_u8_array(&v, "input_data") {
        bytes
    } else if let Some(s) = v.get("input_hex").and_then(Value::as_str) {
        hex_decode(s)?
    } else {
        vec![0u8; v.get("input_size").and_then(Value::as_u64).unwrap_or(64) as usize]
    };

    let mut lib = MachineLearningLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;
    lib.load_model(model_id.clone(), model_path)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    let params = InferenceParameters {
        batch_size: v.get("batch_size").and_then(Value::as_u64).unwrap_or(1) as usize,
        sequence_length: v
            .get("sequence_length")
            .and_then(Value::as_u64)
            .unwrap_or(input_data.len() as u64) as usize,
        temperature: v.get("temperature").and_then(Value::as_f64).or(Some(0.7)),
        top_k: v
            .get("top_k")
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .or(Some(1)),
        top_p: v.get("top_p").and_then(Value::as_f64).or(Some(1.0)),
        max_tokens: v
            .get("max_tokens")
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .or(Some(10)),
        precision: Precision::FP32,
    };

    let r = lib
        .run_inference(&model_id, &input_data, params)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    Ok(json!({
        "model_id": model_id,
        "result_id": r.result.result_id,
        "confidence": r.result.confidence,
        "output_size": r.result.output_data.len(),
        "execution_time_ms": r.execution_time
    })
    .to_string())
}

pub fn financial_model(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::financial_modeling::{
        Asset, AssetType, FinancialModelingLibrary, OptionParameters, OptionType, Portfolio,
    };

    let v = parse_tool_args(args)?;
    let op = json_str(&v, "op", "option");
    let mut lib = FinancialModelingLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;

    if op == "risk" {
        let mut portfolio = Portfolio::new();
        if let Some(id) = v.get("portfolio_id").and_then(Value::as_str) {
            portfolio.portfolio_id = id.to_string();
        }
        if let Some(assets) = v.get("assets").and_then(Value::as_array) {
            portfolio.assets = assets
                .iter()
                .filter_map(|a| {
                    Some(Asset {
                        asset_id: a.get("asset_id")?.as_str()?.to_string(),
                        symbol: a.get("symbol")?.as_str()?.to_string(),
                        asset_type: AssetType::Stock,
                        quantity: json_f64(a, "quantity", 0.0),
                        average_cost: json_f64(a, "average_cost", 0.0),
                        current_price: json_f64(a, "current_price", 0.0),
                        market_value: json_f64(a, "market_value", 0.0),
                        currency: a
                            .get("currency")
                            .and_then(Value::as_str)
                            .unwrap_or("USD")
                            .to_string(),
                        exchange: a
                            .get("exchange")
                            .and_then(Value::as_str)
                            .unwrap_or("NASDAQ")
                            .to_string(),
                        last_updated: 0,
                        // Optional real price history (oldest first) so risk
                        // metrics can be computed from genuine returns; empty when
                        // not supplied (risk computation then refuses, never fakes).
                        price_history: a
                            .get("price_history")
                            .and_then(Value::as_array)
                            .map(|arr| arr.iter().filter_map(Value::as_f64).collect())
                            .unwrap_or_default(),
                    })
                })
                .collect();
        }
        portfolio.cash_balance = json_f64(&v, "cash_balance", portfolio.cash_balance);
        let created = lib
            .create_portfolio(portfolio)
            .map_err(|_| McpSystemError::InvalidParameters)?;
        let pid = created.result.portfolio_id;
        let r = lib
            .calculate_portfolio_risk(&pid)
            .map_err(|_| McpSystemError::InvalidParameters)?;
        return Ok(json!({
            "op": "risk",
            "portfolio_id": pid,
            "var_95": r.result.var_95,
            "sharpe_ratio": r.result.sharpe_ratio,
            "sortino_ratio": r.result.sortino_ratio,
            "max_drawdown": r.result.max_drawdown,
            "overall_risk_score": r.result.overall_risk_score
        })
        .to_string());
    }

    if op == "bond" {
        use crate::specialized_libs::computational_economics::fixed_income::coupon_bond_price;
        let face = json_f64(&v, "face", 100.0);
        let c_rate = json_f64(&v, "coupon_rate", 0.05);
        let y = json_f64(&v, "yield", 0.06);
        let n = v.get("periods").and_then(Value::as_u64).unwrap_or(5) as u32;
        let price = coupon_bond_price(face, c_rate, y, n as f64, 1).unwrap_or(f64::NAN);
        return Ok(json!({
            "op": "bond",
            "price": price,
            "face": face,
            "yield": y,
            "assumptions": "flat yield, actual/actual-ish periods, no daycount, no credit risk (fixed_income kernel)",
            "data_sufficiency": "synthetic schedule; supply cash flows for real use"
        }).to_string());
    }
    if op == "forensic_demo" {
        use crate::specialized_libs::computational_economics::forensic_economics::{
            generate_synthetic_persona_trace, HealthWelfareState, NquinVector,
        };
        let steps = v.get("steps").and_then(Value::as_u64).unwrap_or(20) as usize;
        let mut states = [HealthWelfareState::Stable; 64];
        let mut nqs = [NquinVector::ZERO; 64];
        let shocks: Vec<f64> = v.get("shocks").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_else(|| vec![-0.05; steps]);
        let _ = generate_synthetic_persona_trace(4242, steps.min(64), 0.4, &shocks, &mut states, &mut nqs);
        let final_l1 = nqs[steps.min(64) - 1].l1_norm();
        return Ok(json!({
            "op": "forensic_demo",
            "final_harm_l1": final_l1,
            "absorbing": states[steps.min(64) - 1] as u8 == 4,
            "assumptions": "synthetic WellFair-style persona, memory effects, 5-dim nquin, deterministic seed 4242",
            "evidence": "none (demo); real use requires user-sovereign event stream + consent/provenance"
        }).to_string());
    }

    if op == "welfare" {
        use crate::specialized_libs::computational_economics::welfare::gini_coefficient;
        let inc: Vec<f64> = v.get("incomes").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
        let g = gini_coefficient(&inc).unwrap_or(f64::NAN);
        return Ok(json!({
            "op": "welfare",
            "gini": g,
            "assumptions": "gini from incomes list; pair with deontic review"
        }).to_string());
    }

    let option_type = if json_str(&v, "option_type", "call") == "put" {
        OptionType::Put
    } else {
        OptionType::Call
    };
    let params = OptionParameters {
        underlying_price: json_f64(&v, "underlying_price", 100.0),
        strike: json_f64(&v, "strike", 105.0),
        time_to_maturity: json_f64(&v, "time_to_maturity", 0.25),
        risk_free_rate: json_f64(&v, "risk_free_rate", 0.05),
        volatility: json_f64(&v, "volatility", 0.2),
        option_type,
    };
    let r = lib
        .price_option(params)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    Ok(json!({
        "op": "option",
        "price": r.result.price,
        "delta": r.result.delta,
        "gamma": r.result.gamma,
        "theta": r.result.theta,
        "vega": r.result.vega,
        "rho": r.result.rho,
        "assumptions": "European exercise, Black-Scholes-Merton, constant vol, no dividends (computational_economics::derivatives fallback)"
    })
    .to_string())
}
