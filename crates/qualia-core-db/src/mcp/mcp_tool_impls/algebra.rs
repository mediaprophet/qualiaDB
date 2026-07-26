use super::*;

pub fn matrix_operation(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::linear_algebra::{DataType, LinearAlgebraLibrary};

    let v = parse_tool_args(args)?;
    let op = json_str(&v, "op", "multiply");
    let mut lib = LinearAlgebraLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;

    if let Some(matrices) = v.get("matrices").and_then(Value::as_array) {
        for m in matrices {
            let (id, rows, cols, data) = parse_matrix_def(m)?;
            lib.create_matrix(id, rows, cols, DataType::Float64, data)
                .map_err(|_| McpSystemError::InvalidParameters)?;
        }
    } else {
        let (id_a, rows_a, cols_a, data_a) = parse_matrix_def(
            v.get("left")
                .or(v.get("a"))
                .ok_or(McpSystemError::InvalidParameters)?,
        )?;
        lib.create_matrix(id_a.clone(), rows_a, cols_a, DataType::Float64, data_a)
            .map_err(|_| McpSystemError::InvalidParameters)?;
        if op == "multiply" || op == "solve" {
            let (id_b, rows_b, cols_b, data_b) = parse_matrix_def(
                v.get("right")
                    .or(v.get("b"))
                    .ok_or(McpSystemError::InvalidParameters)?,
            )?;
            lib.create_matrix(id_b, rows_b, cols_b, DataType::Float64, data_b)
                .map_err(|_| McpSystemError::InvalidParameters)?;
        }
    }

    let result_id = v
        .get("result_id")
        .and_then(Value::as_str)
        .unwrap_or("result")
        .to_string();

    let result = match op {
        "transpose" => {
            let input = v
                .get("input_id")
                .and_then(Value::as_str)
                .or_else(|| {
                    v.get("left")
                        .and_then(|l| l.get("id"))
                        .and_then(Value::as_str)
                })
                .unwrap_or("A");
            lib.matrix_transpose(input, &result_id)
        }
        "solve" => {
            let matrix_id = v
                .get("matrix_id")
                .and_then(Value::as_str)
                .or_else(|| {
                    v.get("left")
                        .and_then(|l| l.get("id"))
                        .and_then(Value::as_str)
                })
                .unwrap_or("A");
            let rhs_id = v
                .get("rhs_id")
                .and_then(Value::as_str)
                .or_else(|| {
                    v.get("right")
                        .and_then(|r| r.get("id"))
                        .and_then(Value::as_str)
                })
                .unwrap_or("B");
            lib.solve_linear_system(matrix_id, rhs_id, &result_id)
        }
        "inverse" => {
            let input = v
                .get("input_id")
                .and_then(Value::as_str)
                .or_else(|| {
                    v.get("left")
                        .and_then(|l| l.get("id"))
                        .and_then(Value::as_str)
                })
                .unwrap_or("A");
            lib.matrix_inverse(input, &result_id)
        }
        _ => {
            let left = v
                .get("left_id")
                .and_then(Value::as_str)
                .or_else(|| {
                    v.get("left")
                        .and_then(|l| l.get("id"))
                        .and_then(Value::as_str)
                })
                .unwrap_or("A");
            let right = v
                .get("right_id")
                .and_then(Value::as_str)
                .or_else(|| {
                    v.get("right")
                        .and_then(|r| r.get("id"))
                        .and_then(Value::as_str)
                })
                .unwrap_or("B");
            let alpha = json_f64(&v, "alpha", 1.0);
            let beta = json_f64(&v, "beta", 0.0);
            lib.matrix_multiply(left, right, &result_id, alpha, beta)
        }
    }
    .map_err(|_| McpSystemError::InvalidParameters)?;

    Ok(json!({
        "op": op,
        "result_id": result_id,
        "rows": result.result.rows,
        "cols": result.result.cols,
        "data": result.result.data,
        "execution_time_ms": result.execution_time
    })
    .to_string())
}

/// Find all roots of a polynomial given DESCENDING coefficients `[cₙ, …, c₁, c₀]`.
pub fn algebra_solve_polynomial(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::linear_algebra::polynomial_roots;
    let v = parse_tool_args(args)?;
    let coeffs = json_f64_array(&v, "coeffs")?;
    let roots = polynomial_roots(&coeffs).map_err(|_| McpSystemError::InvalidParameters)?;
    let out: Vec<Value> = roots
        .iter()
        .map(|r| json!({ "re": r.re, "im": r.im }))
        .collect();
    Ok(json!({
        "degree": coeffs.len().saturating_sub(1),
        "roots": out
    })
    .to_string())
}

/// Determinant / eigenvalues / symmetric eigensystem / SVD of a row-major matrix.
pub fn algebra_matrix_analyze(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::linear_algebra::{
        determinant, eigen_symmetric, eigenvalues_general, svd,
    };
    let v = parse_tool_args(args)?;
    let op = json_str(&v, "op", "determinant");
    let rows = json_u64(&v, "rows", 0) as usize;
    let cols = json_u64(&v, "cols", 0) as usize;
    let data = json_f64_array(&v, "data")?;
    match op {
        "determinant" => {
            let d = determinant(rows, &data).map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(json!({ "op": op, "determinant": d }).to_string())
        }
        "eigenvalues" => {
            let e =
                eigenvalues_general(rows, &data).map_err(|_| McpSystemError::InvalidParameters)?;
            let out: Vec<Value> = e
                .iter()
                .map(|z| json!({ "re": z.re, "im": z.im }))
                .collect();
            Ok(json!({ "op": op, "eigenvalues": out }).to_string())
        }
        "eigen_symmetric" => {
            let (vals, vecs) =
                eigen_symmetric(rows, &data).map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(
                json!({ "op": op, "n": rows, "eigenvalues": vals, "eigenvectors": vecs })
                    .to_string(),
            )
        }
        "svd" => {
            let s = svd(rows, cols, &data).map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(json!({
                "op": op, "rows": rows, "cols": cols,
                "singular_values": s.singular_values, "u": s.u, "v": s.v
            })
            .to_string())
        }
        _ => Err(McpSystemError::InvalidParameters),
    }
}

pub fn cas(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::symbolic_algebra as sym;
    let v = parse_tool_args(args)?;
    let op = json_str(&v, "op", "simplify");
    match op {
        "differentiate" => {
            let expr_s = json_str(&v, "expr", "");
            let wrt = json_str(&v, "var", "x");
            let e = sym::parse(expr_s).map_err(|_| McpSystemError::InvalidParameters)?;
            let d = sym::simplify(&sym::differentiate(&e, wrt));
            Ok(
                json!({ "op": op, "input": expr_s, "var": wrt, "derivative": d.to_string() })
                    .to_string(),
            )
        }
        "simplify" => {
            let expr_s = json_str(&v, "expr", "");
            let e = sym::parse(expr_s).map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(
                json!({ "op": op, "input": expr_s, "simplified": sym::simplify(&e).to_string() })
                    .to_string(),
            )
        }
        "evaluate" => {
            let expr_s = json_str(&v, "expr", "");
            let e = sym::parse(expr_s).map_err(|_| McpSystemError::InvalidParameters)?;
            let mut env = std::collections::HashMap::new();
            if let Some(obj) = v.get("env").and_then(Value::as_object) {
                for (k, val) in obj {
                    if let Some(f) = val.as_f64() {
                        env.insert(k.clone(), f);
                    }
                }
            }
            let value = e.eval(&env).ok_or(McpSystemError::InvalidParameters)?;
            Ok(json!({ "op": op, "input": expr_s, "value": value }).to_string())
        }
        "solve_quadratic" => {
            let a = json_f64(&v, "a", 1.0);
            let b = json_f64(&v, "b", 0.0);
            let cc = json_f64(&v, "c", 0.0);
            let roots: Vec<String> = sym::solve_quadratic_symbolic(a, b, cc)
                .iter()
                .map(|r| r.to_string())
                .collect();
            Ok(json!({ "op": op, "a": a, "b": b, "c": cc, "roots": roots }).to_string())
        }
        "expand" => {
            let expr_s = json_str(&v, "expr", "");
            let e = sym::parse(expr_s).map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(
                json!({ "op": op, "input": expr_s, "expanded": sym::expand(&e).to_string() })
                    .to_string(),
            )
        }
        "factor" => {
            let a = json_f64(&v, "a", 1.0);
            let b = json_f64(&v, "b", 0.0);
            let cc = json_f64(&v, "c", 0.0);
            let varname = json_str(&v, "var", "x");
            match sym::factor_quadratic(a, b, cc, varname) {
                Some(f) => Ok(
                    json!({ "op": op, "a": a, "b": b, "c": cc, "factored": f.to_string() })
                        .to_string(),
                ),
                None => Ok(
                    json!({ "op": op, "a": a, "b": b, "c": cc, "factored": Value::Null,
                    "note": "no real factorisation (negative discriminant or a = 0)" })
                    .to_string(),
                ),
            }
        }
        _ => Err(McpSystemError::InvalidParameters),
    }
}
