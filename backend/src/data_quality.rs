use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityStatus {
    Pass,
    Fail(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectationResult {
    pub table_name: String,
    pub expectation_name: String,
    pub status: QualityStatus,
}

/// Trait for defining a data quality expectation
pub trait DataExpectation {
    fn name(&self) -> &str;
    fn validate(&self, data: &[serde_json::Value]) -> ExpectationResult;
}

/// Example: Ensures no nulls in a specific field
pub struct NotNullExpectation {
    pub field: String,
    pub table_name: String,
}

impl DataExpectation for NotNullExpectation {
    fn name(&self) -> &str {
        "not_null"
    }

    fn validate(&self, data: &[serde_json::Value]) -> ExpectationResult {
        for row in data {
            if row.get(&self.field).is_none() || row[&self.field].is_null() {
                return ExpectationResult {
                    table_name: self.table_name.clone(),
                    expectation_name: self.name().to_string(),
                    status: QualityStatus::Fail(format!(
                        "Null value found in field: {}",
                        self.field
                    )),
                };
            }
        }
        ExpectationResult {
            table_name: self.table_name.clone(),
            expectation_name: self.name().to_string(),
            status: QualityStatus::Pass,
        }
    }
}

/// Service to run all expectations against a dataset
pub struct DataQualityService {
    expectations: Vec<Box<dyn DataExpectation>>,
}

impl DataQualityService {
    pub fn new() -> Self {
        Self {
            expectations: vec![],
        }
    }

    pub fn add_expectation(&mut self, exp: Box<dyn DataExpectation>) {
        self.expectations.push(exp);
    }

    /// Runs all expectations and returns true if all critical checks pass
    pub fn run_all(&self, data: &[serde_json::Value]) -> (bool, Vec<ExpectationResult>) {
        let mut all_passed = true;
        let mut results = vec![];

        for exp in &self.expectations {
            let result = exp.validate(data);
            if let QualityStatus::Fail(_) = result.status {
                all_passed = false;
            }
            results.push(result);
        }

        (all_passed, results)
    }
}
