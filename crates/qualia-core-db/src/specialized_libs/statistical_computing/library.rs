use super::*;

impl StatisticalComputingLibrary {
    /// Create new statistical computing library
    pub fn new() -> Self {
        Self {
            data_storage: StatisticalDataStorage::new(),
            computation_engine: StatisticalComputationEngine::new(),
            privacy_engine: StatisticalPrivacyEngine::new(),
            analysis_engine: StatisticalAnalysisEngine::new(),
            performance_monitor: StatisticalPerformanceMonitor::new(),
        }
    }

    /// Initialize the library
    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        // Initialize storage
        self.data_storage.initialize()?;

        // Initialize computation engine
        self.computation_engine.initialize()?;

        // Initialize privacy engine
        self.privacy_engine.initialize()?;

        // Initialize analysis engine
        self.analysis_engine.initialize()?;

        Ok(())
    }

    /// Create a new dataset
    pub fn create_dataset(
        &mut self,
        dataset_id: String,
        data: Vec<Vec<DataValue>>,
        column_names: Vec<String>,
        column_types: Vec<DataType>,
        privacy_level: PrivacyLevel,
    ) -> Result<Dataset, StatisticalError> {
        // Validate input
        if data.is_empty() {
            return Err(StatisticalError::InvalidData(
                "Dataset cannot be empty".to_string(),
            ));
        }
        if column_names.len() != column_types.len() {
            return Err(StatisticalError::InvalidData(
                "Column names and types must match".to_string(),
            ));
        }
        if data.iter().any(|row| row.len() != column_names.len()) {
            return Err(StatisticalError::InvalidData(
                "All rows must have same number of columns".to_string(),
            ));
        }

        // Create metadata
        let metadata = DatasetMetadata {
            dataset_id: dataset_id.clone(),
            dataset_type: DatasetType::Mixed,
            dimensions: DatasetDimensions {
                rows: data.len(),
                columns: column_names.len(),
                time_steps: None,
                features: Some(column_names.len()),
            },
            data_types: column_types.clone(),
            sample_size: data.len(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_updated: 0,
            access_count: 0,
            privacy_level,
        };

        // Create dataset
        let dataset = Dataset {
            dataset_id: dataset_id.clone(),
            metadata,
            data,
            column_names,
            column_types,
        };

        // Store dataset
        self.data_storage.store_dataset(dataset.clone())?;

        Ok(dataset)
    }

    /// Compute mean of a column
    pub fn mean(
        &mut self,
        dataset_id: &str,
        column: &str,
        privacy_preserved: bool,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let start_time = std::time::Instant::now();

        // Get dataset
        let dataset = self.data_storage.get_dataset(dataset_id)?;

        // Find column index
        let column_index = dataset
            .column_names
            .iter()
            .position(|name| name == column)
            .ok_or_else(|| StatisticalError::InvalidColumn(column.to_string()))?;

        // Validate column type
        if !matches!(
            dataset.column_types[column_index],
            DataType::Float32 | DataType::Float64
        ) {
            return Err(StatisticalError::InvalidOperation(
                "Mean can only be computed on numeric columns".to_string(),
            ));
        }

        // Extract column data
        let mut values = Vec::new();
        for row in &dataset.data {
            match &row[column_index] {
                DataValue::Float(value) => values.push(*value),
                DataValue::Integer(value) => values.push(*value as f64),
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            }
        }

        if values.is_empty() {
            return Err(StatisticalError::InvalidData(
                "No valid data in column".to_string(),
            ));
        }

        // Compute mean via the engine's canonical statistics solver (Modality-First:
        // no inline re-implementation). `values` is the caller-owned slice.
        let mean = crate::solvers::statistics::mean(&values)
            .ok_or_else(|| StatisticalError::InvalidData("No valid data in column".to_string()))?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Apply privacy if requested. Sensitivity is calibrated via the
        // differential-privacy sensitivity analyzer (mean sensitivity = 1/n)
        // instead of the previous hardcoded 1.0, so noise scales with the
        // actual query sensitivity.
        let (final_mean, privacy_cost) = if privacy_preserved {
            let sensitivity = {
                let analyzer = &mut self
                    .privacy_engine
                    .differential_privacy
                    .sensitivity_analyzer;
                analyzer.get_sensitivity("mean", &values).unwrap_or(1.0)
            };
            let (noisy_mean, cost) = self.privacy_engine.add_laplace_noise(mean, sensitivity)?;
            (noisy_mean, cost)
        } else {
            (mean, 0.0)
        };

        // Update performance metrics
        self.performance_monitor
            .record_operation("mean", execution_time, 0, privacy_cost);

        Ok(StatisticalAnalysisResult {
            result: final_mean,
            execution_time,
            memory_usage: 0,
            sample_size: values.len(),
            confidence_level: 0.95,
            privacy_preserved,
            privacy_cost,
        })
    }

    /// Compute median of a column
    pub fn median(
        &mut self,
        dataset_id: &str,
        column: &str,
        privacy_preserved: bool,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let start_time = std::time::Instant::now();

        // Get dataset
        let dataset = self.data_storage.get_dataset(dataset_id)?;

        // Find column index
        let column_index = dataset
            .column_names
            .iter()
            .position(|name| name == column)
            .ok_or_else(|| StatisticalError::InvalidColumn(column.to_string()))?;

        // Validate column type
        if !matches!(
            dataset.column_types[column_index],
            DataType::Float32 | DataType::Float64
        ) {
            return Err(StatisticalError::InvalidOperation(
                "Median can only be computed on numeric columns".to_string(),
            ));
        }

        // Extract column data
        let mut values = Vec::new();
        for row in &dataset.data {
            match &row[column_index] {
                DataValue::Float(value) => values.push(*value),
                DataValue::Integer(value) => values.push(*value as f64),
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            }
        }

        if values.is_empty() {
            return Err(StatisticalError::InvalidData(
                "No valid data in column".to_string(),
            ));
        }

        // Compute median via the engine's canonical statistics solver (sorts the
        // caller-owned buffer in place; no inline re-implementation).
        let median = crate::solvers::statistics::median_in_place(&mut values)
            .ok_or_else(|| StatisticalError::InvalidData("No valid data in column".to_string()))?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Apply privacy if requested
        let (final_median, privacy_cost) = if privacy_preserved {
            let (noisy_median, cost) = self.privacy_engine.add_laplace_noise(median, 1.0)?;
            (noisy_median, cost)
        } else {
            (median, 0.0)
        };

        // Update performance metrics
        self.performance_monitor
            .record_operation("median", execution_time, 0, privacy_cost);

        Ok(StatisticalAnalysisResult {
            result: final_median,
            execution_time,
            memory_usage: 0,
            sample_size: values.len(),
            confidence_level: 0.95,
            privacy_preserved,
            privacy_cost,
        })
    }

    /// Compute variance of a column
    pub fn variance(
        &mut self,
        dataset_id: &str,
        column: &str,
        sample: bool,
        privacy_preserved: bool,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let start_time = std::time::Instant::now();

        // Get dataset
        let dataset = self.data_storage.get_dataset(dataset_id)?;

        // Find column index
        let column_index = dataset
            .column_names
            .iter()
            .position(|name| name == column)
            .ok_or_else(|| StatisticalError::InvalidColumn(column.to_string()))?;

        // Validate column type
        if !matches!(
            dataset.column_types[column_index],
            DataType::Float32 | DataType::Float64
        ) {
            return Err(StatisticalError::InvalidOperation(
                "Variance can only be computed on numeric columns".to_string(),
            ));
        }

        // Extract column data
        let mut values = Vec::new();
        for row in &dataset.data {
            match &row[column_index] {
                DataValue::Float(value) => values.push(*value),
                DataValue::Integer(value) => values.push(*value as f64),
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            }
        }

        if values.is_empty() {
            return Err(StatisticalError::InvalidData(
                "No valid data in column".to_string(),
            ));
        }

        // Compute variance via the engine's canonical statistics solver
        // (Modality-First: no inline re-implementation).
        let variance = crate::solvers::statistics::variance(&values, sample)
            .ok_or_else(|| StatisticalError::InvalidData("No valid data in column".to_string()))?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Apply privacy if requested
        let (final_variance, privacy_cost) = if privacy_preserved {
            let (noisy_variance, cost) = self.privacy_engine.add_laplace_noise(variance, 1.0)?;
            (noisy_variance, cost)
        } else {
            (variance, 0.0)
        };

        // Update performance metrics
        self.performance_monitor
            .record_operation("variance", execution_time, 0, privacy_cost);

        Ok(StatisticalAnalysisResult {
            result: final_variance,
            execution_time,
            memory_usage: 0,
            sample_size: values.len(),
            confidence_level: 0.95,
            privacy_preserved,
            privacy_cost,
        })
    }

    /// Compute correlation between two columns
    pub fn correlation(
        &mut self,
        dataset_id: &str,
        column1: &str,
        column2: &str,
        method: CorrelationMethod,
        privacy_preserved: bool,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let start_time = std::time::Instant::now();

        // Get dataset
        let dataset = self.data_storage.get_dataset(dataset_id)?;

        // Find column indices
        let column1_index = dataset
            .column_names
            .iter()
            .position(|name| name == column1)
            .ok_or_else(|| StatisticalError::InvalidColumn(column1.to_string()))?;

        let column2_index = dataset
            .column_names
            .iter()
            .position(|name| name == column2)
            .ok_or_else(|| StatisticalError::InvalidColumn(column2.to_string()))?;

        // Validate column types
        if !matches!(
            dataset.column_types[column1_index],
            DataType::Float32 | DataType::Float64
        ) {
            return Err(StatisticalError::InvalidOperation(
                "Correlation can only be computed on numeric columns".to_string(),
            ));
        }
        if !matches!(
            dataset.column_types[column2_index],
            DataType::Float32 | DataType::Float64
        ) {
            return Err(StatisticalError::InvalidOperation(
                "Correlation can only be computed on numeric columns".to_string(),
            ));
        }

        // Extract column data
        let mut x_values = Vec::new();
        let mut y_values = Vec::new();

        for row in &dataset.data {
            let x_val = match &row[column1_index] {
                DataValue::Float(value) => *value,
                DataValue::Integer(value) => *value as f64,
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            };

            let y_val = match &row[column2_index] {
                DataValue::Float(value) => *value,
                DataValue::Integer(value) => *value as f64,
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            };

            x_values.push(x_val);
            y_values.push(y_val);
        }

        if x_values.len() < 2 {
            return Err(StatisticalError::InvalidData(
                "Insufficient data for correlation".to_string(),
            ));
        }

        // Compute correlation based on method
        let correlation = match method {
            CorrelationMethod::Pearson => self.pearson_correlation(&x_values, &y_values)?,
            CorrelationMethod::Spearman => self.spearman_correlation(&x_values, &y_values)?,
            CorrelationMethod::Kendall => self.kendall_correlation(&x_values, &y_values)?,
            _ => {
                return Err(StatisticalError::InvalidOperation(
                    "Correlation method not supported".to_string(),
                ))
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Apply privacy if requested
        let (final_correlation, privacy_cost) = if privacy_preserved {
            let (noisy_correlation, cost) =
                self.privacy_engine.add_laplace_noise(correlation, 0.1)?;
            (noisy_correlation.clamp(-1.0, 1.0), cost)
        } else {
            (correlation, 0.0)
        };

        // Update performance metrics
        self.performance_monitor
            .record_operation("correlation", execution_time, 0, privacy_cost);

        Ok(StatisticalAnalysisResult {
            result: final_correlation,
            execution_time,
            memory_usage: 0,
            sample_size: x_values.len(),
            confidence_level: 0.95,
            privacy_preserved,
            privacy_cost,
        })
    }

    /// Perform t-test
    pub fn t_test(
        &mut self,
        dataset_id: &str,
        column: &str,
        hypothesis_type: HypothesisType,
        privacy_preserved: bool,
    ) -> Result<StatisticalAnalysisResult<TTestResult>, StatisticalError> {
        let start_time = std::time::Instant::now();

        // Get dataset
        let dataset = self.data_storage.get_dataset(dataset_id)?;

        // Find column index
        let column_index = dataset
            .column_names
            .iter()
            .position(|name| name == column)
            .ok_or_else(|| StatisticalError::InvalidColumn(column.to_string()))?;

        // Validate column type
        if !matches!(
            dataset.column_types[column_index],
            DataType::Float32 | DataType::Float64
        ) {
            return Err(StatisticalError::InvalidOperation(
                "T-test can only be computed on numeric columns".to_string(),
            ));
        }

        // Extract column data
        let mut values = Vec::new();
        for row in &dataset.data {
            match &row[column_index] {
                DataValue::Float(value) => values.push(*value),
                DataValue::Integer(value) => values.push(*value as f64),
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            }
        }

        if values.len() < 2 {
            return Err(StatisticalError::InvalidData(
                "Insufficient data for t-test".to_string(),
            ));
        }

        // Compute t-test based on hypothesis type
        let t_test_result = match hypothesis_type {
            HypothesisType::OneSample => self.one_sample_t_test(&values, 0.0)?,
            HypothesisType::TwoSample => {
                return Err(StatisticalError::InvalidOperation(
                    "Two-sample t-test requires two datasets".to_string(),
                ))
            }
            HypothesisType::Paired => {
                return Err(StatisticalError::InvalidOperation(
                    "Paired t-test requires paired data".to_string(),
                ))
            }
            HypothesisType::Independent => {
                return Err(StatisticalError::InvalidOperation(
                    "Independent t-test requires two samples".to_string(),
                ))
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Apply privacy if requested
        let (final_result, privacy_cost) = if privacy_preserved {
            let (noisy_t_statistic, cost) = self
                .privacy_engine
                .add_laplace_noise(t_test_result.t_statistic, 1.0)?;
            let noisy_result = TTestResult {
                t_statistic: noisy_t_statistic,
                p_value: t_test_result.p_value,
                degrees_of_freedom: t_test_result.degrees_of_freedom,
                confidence_interval: t_test_result.confidence_interval,
            };
            (noisy_result, cost)
        } else {
            (t_test_result, 0.0)
        };

        // Update performance metrics
        self.performance_monitor
            .record_operation("t_test", execution_time, 0, privacy_cost);

        Ok(StatisticalAnalysisResult {
            result: final_result,
            execution_time,
            memory_usage: 0,
            sample_size: values.len(),
            confidence_level: 0.95,
            privacy_preserved,
            privacy_cost,
        })
    }

    /// Generate histogram
    pub fn histogram(
        &mut self,
        dataset_id: &str,
        column: &str,
        bins: usize,
        privacy_preserved: bool,
    ) -> Result<StatisticalAnalysisResult<HistogramResult>, StatisticalError> {
        let start_time = std::time::Instant::now();

        // Get dataset
        let dataset = self.data_storage.get_dataset(dataset_id)?;

        // Find column index
        let column_index = dataset
            .column_names
            .iter()
            .position(|name| name == column)
            .ok_or_else(|| StatisticalError::InvalidColumn(column.to_string()))?;

        // Validate column type
        if !matches!(
            dataset.column_types[column_index],
            DataType::Float32 | DataType::Float64
        ) {
            return Err(StatisticalError::InvalidOperation(
                "Histogram can only be computed on numeric columns".to_string(),
            ));
        }

        // Extract column data
        let mut values = Vec::new();
        for row in &dataset.data {
            match &row[column_index] {
                DataValue::Float(value) => values.push(*value),
                DataValue::Integer(value) => values.push(*value as f64),
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            }
        }

        if values.is_empty() {
            return Err(StatisticalError::InvalidData(
                "No valid data in column".to_string(),
            ));
        }

        // Compute histogram
        let histogram_result = self.compute_histogram(&values, bins)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Apply privacy if requested
        let (final_result, privacy_cost) = if privacy_preserved {
            let (noisy_counts, cost) = self
                .privacy_engine
                .add_histogram_noise(&histogram_result.counts)?;
            let noisy_result = HistogramResult {
                bins: histogram_result.bins,
                counts: noisy_counts,
                min_value: histogram_result.min_value,
                max_value: histogram_result.max_value,
                bin_width: histogram_result.bin_width,
            };
            (noisy_result, cost)
        } else {
            (histogram_result, 0.0)
        };

        // Update performance metrics
        self.performance_monitor
            .record_operation("histogram", execution_time, 0, privacy_cost);

        Ok(StatisticalAnalysisResult {
            result: final_result,
            execution_time,
            memory_usage: 0,
            sample_size: values.len(),
            confidence_level: 0.95,
            privacy_preserved,
            privacy_cost,
        })
    }

    // ========================================================================
    // Wired capability methods.
    //
    // Each declared `StatisticalOperation` is a genuine, MCP-reachable
    // computation. Every method below marshals the caller's dataset into a
    // slice and delegates to the canonical numeric kernels in
    // `crate::solvers` (Modality-First Composition — no inline re-derivation);
    // the descriptive/correlation/regression/hypothesis kernels live in
    // `solvers::statistics`, the learning models in `solvers::learning`, and
    // polynomial least-squares in `solvers::interpolation`.
    // ========================================================================

    /// Extract one numeric column as an owned `Vec<f64>` (Integer widened,
    /// Null rows skipped). Errors on unknown column, non-numeric cell, or empty.
    fn numeric_column(&self, dataset_id: &str, column: &str) -> Result<Vec<f64>, StatisticalError> {
        let dataset = self.data_storage.get_dataset(dataset_id)?;
        let idx = dataset
            .column_names
            .iter()
            .position(|n| n == column)
            .ok_or_else(|| StatisticalError::InvalidColumn(column.to_string()))?;
        let mut out = Vec::with_capacity(dataset.data.len());
        for row in &dataset.data {
            match &row[idx] {
                DataValue::Float(v) => out.push(*v),
                DataValue::Integer(v) => out.push(*v as f64),
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            }
        }
        if out.is_empty() {
            return Err(StatisticalError::InvalidData(
                "No valid data in column".to_string(),
            ));
        }
        Ok(out)
    }

    /// Extract several numeric columns row-aligned into a row-major `n × p`
    /// matrix, skipping any row where one of the requested cells is Null (so
    /// every returned row is complete). Returns `(data, n, p)`.
    fn numeric_matrix(
        &self,
        dataset_id: &str,
        columns: &[&str],
    ) -> Result<(Vec<f64>, usize, usize), StatisticalError> {
        let dataset = self.data_storage.get_dataset(dataset_id)?;
        let p = columns.len();
        if p == 0 {
            return Err(StatisticalError::InvalidOperation(
                "no columns given".to_string(),
            ));
        }
        let mut idxs = Vec::with_capacity(p);
        for c in columns {
            idxs.push(
                dataset
                    .column_names
                    .iter()
                    .position(|n| n == c)
                    .ok_or_else(|| StatisticalError::InvalidColumn((*c).to_string()))?,
            );
        }
        let mut data = Vec::with_capacity(dataset.data.len() * p);
        let mut n = 0;
        'rows: for row in &dataset.data {
            let mut tmp = [0.0f64; 0].to_vec();
            tmp.reserve(p);
            for &ix in &idxs {
                match &row[ix] {
                    DataValue::Float(v) => tmp.push(*v),
                    DataValue::Integer(v) => tmp.push(*v as f64),
                    DataValue::Null => continue 'rows,
                    _ => {
                        return Err(StatisticalError::InvalidOperation(
                            "Non-numeric data in column".to_string(),
                        ))
                    }
                }
            }
            data.extend_from_slice(&tmp);
            n += 1;
        }
        if n == 0 {
            return Err(StatisticalError::InvalidData(
                "No complete rows across the requested columns".to_string(),
            ));
        }
        Ok((data, n, p))
    }

    /// Split feature columns + a trailing label column into `(x, y, n, p)`,
    /// row-aligned (rows with a Null in any of them are dropped together).
    fn features_and_label(
        &self,
        dataset_id: &str,
        feature_columns: &[&str],
        label_column: &str,
    ) -> Result<(Vec<f64>, Vec<f64>, usize, usize), StatisticalError> {
        let mut cols: Vec<&str> = feature_columns.to_vec();
        cols.push(label_column);
        let (mat, n, p_all) = self.numeric_matrix(dataset_id, &cols)?;
        let p = p_all - 1;
        let mut x = Vec::with_capacity(n * p);
        let mut y = Vec::with_capacity(n);
        for r in 0..n {
            x.extend_from_slice(&mat[r * p_all..r * p_all + p]);
            y.push(mat[r * p_all + p]);
        }
        Ok((x, y, n, p))
    }

    fn scalar_result(&self, value: f64, n: usize) -> StatisticalAnalysisResult<f64> {
        StatisticalAnalysisResult {
            result: value,
            execution_time: 0,
            memory_usage: 0,
            sample_size: n,
            confidence_level: 0.95,
            privacy_preserved: false,
            privacy_cost: 0.0,
        }
    }

    /// Standard deviation (`sample = true` → Bessel-corrected).
    pub fn standard_deviation(
        &self,
        dataset_id: &str,
        column: &str,
        sample: bool,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let v = self.numeric_column(dataset_id, column)?;
        let sd = crate::solvers::statistics::std_dev(&v, sample)
            .ok_or_else(|| StatisticalError::InvalidData("empty column".to_string()))?;
        Ok(self.scalar_result(sd, v.len()))
    }

    /// Sample skewness (Fisher-Pearson).
    pub fn skewness(
        &self,
        dataset_id: &str,
        column: &str,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let v = self.numeric_column(dataset_id, column)?;
        let s = crate::solvers::statistics::skewness(&v)
            .ok_or_else(|| StatisticalError::InvalidData("skewness undefined".to_string()))?;
        Ok(self.scalar_result(s, v.len()))
    }

    /// Excess kurtosis.
    pub fn kurtosis(
        &self,
        dataset_id: &str,
        column: &str,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let v = self.numeric_column(dataset_id, column)?;
        let k = crate::solvers::statistics::kurtosis(&v)
            .ok_or_else(|| StatisticalError::InvalidData("kurtosis undefined".to_string()))?;
        Ok(self.scalar_result(k, v.len()))
    }

    /// Modal value + its frequency.
    pub fn mode(&self, dataset_id: &str, column: &str) -> Result<ModeResult, StatisticalError> {
        let mut v = self.numeric_column(dataset_id, column)?;
        let n = v.len();
        let (value, count) = crate::solvers::statistics::mode_in_place(&mut v)
            .ok_or_else(|| StatisticalError::InvalidData("empty column".to_string()))?;
        Ok(ModeResult {
            value,
            count,
            sample_size: n,
        })
    }

    /// The `q`-quantile (`q ∈ [0,1]`) via linear interpolation between order
    /// statistics. `percentile` is the same with `p ∈ [0,100]`.
    pub fn quantile(
        &self,
        dataset_id: &str,
        column: &str,
        q: f64,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let mut v = self.numeric_column(dataset_id, column)?;
        let n = v.len();
        let val = crate::solvers::statistics::quantile_in_place(&mut v, q).ok_or_else(|| {
            StatisticalError::InvalidOperation("quantile requires q in [0,1]".to_string())
        })?;
        Ok(self.scalar_result(val, n))
    }

    /// Covariance between two columns (`sample = true` → divide by n-1).
    pub fn covariance(
        &self,
        dataset_id: &str,
        column_x: &str,
        column_y: &str,
        sample: bool,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let (mat, n, _p) = self.numeric_matrix(dataset_id, &[column_x, column_y])?;
        let x: Vec<f64> = (0..n).map(|r| mat[r * 2]).collect();
        let y: Vec<f64> = (0..n).map(|r| mat[r * 2 + 1]).collect();
        let cov = crate::solvers::statistics::covariance(&x, &y, sample)
            .ok_or_else(|| StatisticalError::InvalidData("covariance undefined".to_string()))?;
        Ok(self.scalar_result(cov, n))
    }

    /// Ordinary-least-squares simple linear regression `y ~ x` with full
    /// inferential statistics (slope/intercept, R², standard errors, t, p).
    pub fn linear_regression(
        &self,
        dataset_id: &str,
        column_x: &str,
        column_y: &str,
    ) -> Result<crate::solvers::statistics::LinearRegression, StatisticalError> {
        let (mat, n, _p) = self.numeric_matrix(dataset_id, &[column_x, column_y])?;
        let x: Vec<f64> = (0..n).map(|r| mat[r * 2]).collect();
        let y: Vec<f64> = (0..n).map(|r| mat[r * 2 + 1]).collect();
        crate::solvers::statistics::simple_linear_regression(&x, &y).ok_or_else(|| {
            StatisticalError::InvalidData(
                "regression undefined (n<3 or zero variance in x)".to_string(),
            )
        })
    }

    /// Polynomial regression `y ~ poly(x, degree)` by least squares (normal
    /// equations). Returns ascending-power coefficients and in-sample R².
    pub fn polynomial_regression(
        &self,
        dataset_id: &str,
        column_x: &str,
        column_y: &str,
        degree: usize,
    ) -> Result<PolynomialFit, StatisticalError> {
        let (mat, n, _p) = self.numeric_matrix(dataset_id, &[column_x, column_y])?;
        let x: Vec<f64> = (0..n).map(|r| mat[r * 2]).collect();
        let y: Vec<f64> = (0..n).map(|r| mat[r * 2 + 1]).collect();
        let coeffs = crate::solvers::interpolation::least_squares::poly_fit(&x, &y, degree)
            .map_err(|e| StatisticalError::InvalidOperation(format!("poly_fit: {:?}", e)))?;
        // In-sample R² from the fitted coefficients.
        let y_mean = crate::solvers::statistics::mean(&y).unwrap_or(0.0);
        let mut ss_res = 0.0;
        let mut ss_tot = 0.0;
        for i in 0..n {
            let yhat = crate::solvers::interpolation::least_squares::poly_eval(&coeffs, x[i]);
            ss_res += (y[i] - yhat) * (y[i] - yhat);
            ss_tot += (y[i] - y_mean) * (y[i] - y_mean);
        }
        let r_squared = if ss_tot > 0.0 {
            1.0 - ss_res / ss_tot
        } else {
            0.0
        };
        Ok(PolynomialFit {
            degree,
            coefficients: coeffs,
            r_squared,
            n,
        })
    }

    /// Binary logistic regression by IRLS (`label` in {0,1}), with Wald
    /// standard errors, z-statistics and p-values on the coefficients.
    pub fn logistic_regression(
        &self,
        dataset_id: &str,
        feature_columns: &[&str],
        label_column: &str,
        fit_intercept: bool,
    ) -> Result<crate::solvers::learning::glm::GlmModel, StatisticalError> {
        let (x, y, n, p) = self.features_and_label(dataset_id, feature_columns, label_column)?;
        crate::solvers::learning::glm::fit_logistic(&x, &y, n, p, fit_intercept)
            .map_err(|e| StatisticalError::InvalidOperation(format!("logistic: {:?}", e)))
    }

    /// One-way ANOVA across the named columns (each column is a group). Groups
    /// may have unequal lengths; Null cells are dropped per column.
    pub fn anova(
        &self,
        dataset_id: &str,
        group_columns: &[&str],
    ) -> Result<crate::solvers::statistics::AnovaResult, StatisticalError> {
        if group_columns.len() < 2 {
            return Err(StatisticalError::InvalidOperation(
                "ANOVA needs at least two groups".to_string(),
            ));
        }
        let groups: Vec<Vec<f64>> = group_columns
            .iter()
            .map(|c| self.numeric_column(dataset_id, c))
            .collect::<Result<_, _>>()?;
        let refs: Vec<&[f64]> = groups.iter().map(|g| g.as_slice()).collect();
        crate::solvers::statistics::one_way_anova(&refs).ok_or_else(|| {
            StatisticalError::InvalidData("ANOVA undefined for these groups".to_string())
        })
    }

    /// Chi-square goodness-of-fit test: observed counts vs. expected counts.
    /// If `expected_column` is `None`, a uniform expectation is used.
    pub fn chi_square_gof(
        &self,
        dataset_id: &str,
        observed_column: &str,
        expected_column: Option<&str>,
    ) -> Result<crate::solvers::statistics::ChiSquareResult, StatisticalError> {
        let observed = self.numeric_column(dataset_id, observed_column)?;
        let expected = match expected_column {
            Some(c) => self.numeric_column(dataset_id, c)?,
            None => {
                let total: f64 = observed.iter().sum();
                let u = total / observed.len() as f64;
                vec![u; observed.len()]
            }
        };
        crate::solvers::statistics::chi_square_gof(&observed, &expected).ok_or_else(|| {
            StatisticalError::InvalidOperation("chi-square GoF undefined".to_string())
        })
    }

    /// Chi-square test of independence over a contingency table whose columns
    /// are the named columns (each row of the table is one dataset row).
    pub fn chi_square_independence(
        &self,
        dataset_id: &str,
        columns: &[&str],
    ) -> Result<crate::solvers::statistics::ChiSquareResult, StatisticalError> {
        let (mat, n, p) = self.numeric_matrix(dataset_id, columns)?;
        let rows: Vec<&[f64]> = (0..n).map(|r| &mat[r * p..(r + 1) * p]).collect();
        crate::solvers::statistics::chi_square_independence(&rows).ok_or_else(|| {
            StatisticalError::InvalidOperation("chi-square independence undefined".to_string())
        })
    }

    /// Autocorrelation of a column at the given lag (biased estimator).
    pub fn autocorrelation(
        &self,
        dataset_id: &str,
        column: &str,
        lag: usize,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let v = self.numeric_column(dataset_id, column)?;
        let r = crate::solvers::statistics::autocorrelation(&v, lag).ok_or_else(|| {
            StatisticalError::InvalidOperation(
                "autocorrelation undefined (lag>=n or constant series)".to_string(),
            )
        })?;
        Ok(self.scalar_result(r, v.len()))
    }

    /// Simple moving average of a column with the given window.
    pub fn moving_average(
        &self,
        dataset_id: &str,
        column: &str,
        window: usize,
    ) -> Result<Vec<f64>, StatisticalError> {
        let v = self.numeric_column(dataset_id, column)?;
        if window == 0 || window > v.len() {
            return Err(StatisticalError::InvalidOperation(
                "window must be in 1..=n".to_string(),
            ));
        }
        let mut out = vec![0.0; v.len() - window + 1];
        crate::solvers::statistics::moving_average_into(&v, window, &mut out).ok_or_else(|| {
            StatisticalError::InvalidOperation("moving average failed".to_string())
        })?;
        Ok(out)
    }

    /// Single exponential smoothing of a column with factor `alpha ∈ (0,1]`.
    pub fn exponential_smoothing(
        &self,
        dataset_id: &str,
        column: &str,
        alpha: f64,
    ) -> Result<Vec<f64>, StatisticalError> {
        let v = self.numeric_column(dataset_id, column)?;
        let mut out = vec![0.0; v.len()];
        crate::solvers::statistics::exponential_smoothing_into(&v, alpha, &mut out).ok_or_else(
            || StatisticalError::InvalidOperation("alpha must be in (0,1]".to_string()),
        )?;
        Ok(out)
    }

    /// K-means clustering over the named feature columns. Returns the fitted
    /// model (centroids, per-point labels, inertia, convergence).
    pub fn kmeans(
        &self,
        dataset_id: &str,
        feature_columns: &[&str],
        k: usize,
        max_iter: usize,
        seed: u64,
    ) -> Result<crate::solvers::learning::clustering::kmeans::KMeansModel, StatisticalError> {
        let (x, n, p) = self.numeric_matrix(dataset_id, feature_columns)?;
        crate::solvers::learning::clustering::kmeans::fit(&x, n, p, k, max_iter, seed)
            .map_err(|e| StatisticalError::InvalidOperation(format!("kmeans: {:?}", e)))
    }

    /// Soft-margin linear SVM over the named feature columns with a boolean
    /// `label` column (non-zero = positive class). Returns a fit summary with
    /// support-vector count and in-sample accuracy.
    pub fn linear_svm(
        &self,
        dataset_id: &str,
        feature_columns: &[&str],
        label_column: &str,
        c: f64,
    ) -> Result<SvmFitResult, StatisticalError> {
        let (x, y, n, p) = self.features_and_label(dataset_id, feature_columns, label_column)?;
        let labels: Vec<bool> = y.iter().map(|&v| v != 0.0).collect();
        let model = crate::solvers::learning::classification::svm::fit(
            &x,
            &labels,
            n,
            p,
            c,
            crate::solvers::learning::classification::svm::Kernel::Linear,
            100,
            1e-3,
        )
        .map_err(|e| StatisticalError::InvalidOperation(format!("svm: {:?}", e)))?;
        let mut correct = 0usize;
        for i in 0..n {
            if model.predict_row(&x[i * p..(i + 1) * p]) == labels[i] {
                correct += 1;
            }
        }
        Ok(SvmFitResult {
            n_support_vectors: model.n_support_vectors(),
            train_accuracy: correct as f64 / n as f64,
            n,
            n_features: p,
        })
    }

    /// Random-forest fit over the named feature columns and a target column.
    /// `classifier = true` fits a classification forest (integer labels) and
    /// reports in-sample accuracy; otherwise a regression forest reporting R².
    pub fn random_forest(
        &self,
        dataset_id: &str,
        feature_columns: &[&str],
        target_column: &str,
        n_trees: usize,
        classifier: bool,
        seed: u64,
    ) -> Result<RandomForestFitResult, StatisticalError> {
        use crate::solvers::learning::trees::decision_tree::TreeParams;
        use crate::solvers::learning::trees::random_forest::RandomForest;
        let (x, y, n, p) = self.features_and_label(dataset_id, feature_columns, target_column)?;
        let params = TreeParams::default();
        let (metric, model_predict): (f64, Vec<f64>) = if classifier {
            let labels: Vec<usize> = y.iter().map(|&v| v.round().max(0.0) as usize).collect();
            let rf = RandomForest::fit_classifier(&x, &labels, n, p, n_trees, params, seed)
                .map_err(|e| StatisticalError::InvalidOperation(format!("forest: {:?}", e)))?;
            let mut correct = 0usize;
            for i in 0..n {
                if rf.predict_class(&x[i * p..(i + 1) * p]) == labels[i] {
                    correct += 1;
                }
            }
            (correct as f64 / n as f64, vec![])
        } else {
            let rf = RandomForest::fit_regressor(&x, &y, n, p, n_trees, params, seed)
                .map_err(|e| StatisticalError::InvalidOperation(format!("forest: {:?}", e)))?;
            let preds: Vec<f64> = (0..n)
                .map(|i| rf.predict_row(&x[i * p..(i + 1) * p]))
                .collect();
            let y_mean = crate::solvers::statistics::mean(&y).unwrap_or(0.0);
            let mut ss_res = 0.0;
            let mut ss_tot = 0.0;
            for i in 0..n {
                ss_res += (y[i] - preds[i]) * (y[i] - preds[i]);
                ss_tot += (y[i] - y_mean) * (y[i] - y_mean);
            }
            let r2 = if ss_tot > 0.0 {
                1.0 - ss_res / ss_tot
            } else {
                0.0
            };
            (r2, preds)
        };
        let _ = model_predict;
        Ok(RandomForestFitResult {
            n_trees,
            classifier,
            train_metric: metric,
            n,
            n_features: p,
        })
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> SystemMetrics {
        self.performance_monitor.get_system_metrics()
    }

    /// List all datasets
    pub fn list_datasets(&self) -> Vec<String> {
        self.data_storage.list_datasets()
    }

    /// Get dataset information
    pub fn get_dataset_info(&self, dataset_id: &str) -> Option<DatasetMetadata> {
        self.data_storage.get_dataset_metadata(dataset_id)
    }

    // Internal methods

    /// Compute Pearson correlation
    fn pearson_correlation(&self, x: &[f64], y: &[f64]) -> Result<f64, StatisticalError> {
        // Modality-First: the math lives in the engine's statistics solver.
        crate::solvers::statistics::pearson(x, y).ok_or_else(|| {
            StatisticalError::InvalidData("Invalid data for correlation".to_string())
        })
    }

    /// Compute Spearman correlation
    fn spearman_correlation(&self, x: &[f64], y: &[f64]) -> Result<f64, StatisticalError> {
        // Convert to ranks
        let x_ranked = self.rank_values(x);
        let y_ranked = self.rank_values(y);

        // Compute Pearson correlation on ranks
        self.pearson_correlation(&x_ranked, &y_ranked)
    }

    /// Compute Kendall correlation
    fn kendall_correlation(&self, x: &[f64], y: &[f64]) -> Result<f64, StatisticalError> {
        // Modality-First: the math lives in the engine's statistics solver.
        crate::solvers::statistics::kendall(x, y).ok_or_else(|| {
            StatisticalError::InvalidData("Invalid data for correlation".to_string())
        })
    }

    /// Rank values
    fn rank_values(&self, values: &[f64]) -> Vec<f64> {
        // Modality-First: ranking lives in the engine. The wrapper owns the
        // scratch/output buffers (heap is fine at this composition boundary).
        let n = values.len();
        let mut idx = vec![0usize; n];
        let mut ranks = vec![0.0; n];
        let _ = crate::solvers::statistics::rank_into(values, &mut idx, &mut ranks);
        ranks
    }

    /// One sample t-test
    fn one_sample_t_test(&self, values: &[f64], mu: f64) -> Result<TTestResult, StatisticalError> {
        let n = values.len();
        if n < 2 {
            return Err(StatisticalError::InvalidData(
                "Insufficient data for t-test".to_string(),
            ));
        }

        let t = crate::solvers::statistics::one_sample_t(values, mu).ok_or_else(|| {
            StatisticalError::InvalidData("Insufficient data for t-test".to_string())
        })?;
        Ok(TTestResult {
            t_statistic: t.t_statistic,
            p_value: t.p_value,
            degrees_of_freedom: t.degrees_of_freedom,
            confidence_interval: t.confidence_interval,
        })
    }

    /// Compute histogram
    fn compute_histogram(
        &self,
        values: &[f64],
        bins: usize,
    ) -> Result<HistogramResult, StatisticalError> {
        if values.is_empty() {
            return Err(StatisticalError::InvalidData(
                "No data for histogram".to_string(),
            ));
        }

        // Modality-First: binning lives in the engine; the wrapper owns the
        // counts buffer and builds the domain result.
        let mut counts = vec![0u32; bins];
        let range = crate::solvers::statistics::histogram_into(values, &mut counts)
            .ok_or_else(|| StatisticalError::InvalidData("No data for histogram".to_string()))?;
        Ok(HistogramResult {
            bins,
            counts,
            min_value: range.min,
            max_value: range.max,
            bin_width: range.bin_width,
        })
    }
}

// Supporting implementations
