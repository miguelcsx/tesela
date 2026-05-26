//! AIP-style evaluation harness for agents, actions, and model providers.
//!
//! A [`Suite`] is a JSON-loadable collection of [`Case`]s.  A [`Runner`]
//! invokes a [`Target`] function for each case, applies a [`Scorer`], and
//! aggregates a [`Report`].
//!
//! Scorers are pure functions: `fn(case: &Case, got: &serde_json::Value) -> Score`.

use tesela_core::Error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Case and Suite
// ---------------------------------------------------------------------------

/// A single evaluation case: input + optional expected output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    /// Unique identifier within the suite.
    pub id: String,
    /// Optional tags for filtering (e.g. `["smoke", "regression"]`).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Input fed to the target function.
    pub input: BTreeMap<String, serde_json::Value>,
    /// Expected output for comparison by the scorer.
    #[serde(default)]
    pub expected: BTreeMap<String, serde_json::Value>,
}

/// A named collection of [`Case`]s.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suite {
    /// Suite name.
    pub name: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Cases to run.
    pub cases: Vec<Case>,
}

impl Suite {
    /// Load a suite from a JSON file on disk.
    pub fn load_file(path: &std::path::Path) -> Result<Self, Error> {
        let raw =
            std::fs::read(path).map_err(|e| Error::internal(format!("suite file read: {}", e)))?;
        serde_json::from_slice(&raw)
            .map_err(|e| Error::validation(format!("suite JSON parse: {}", e)))
    }

    /// Parse a suite from a JSON byte slice.
    pub fn from_json(json: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(json)
            .map_err(|e| Error::validation(format!("suite JSON parse: {}", e)))
    }

    /// Filter cases by tag — returns only cases that contain at least one of `tags`.
    /// If `tags` is empty, all cases are returned.
    pub fn filter_tags(&self, tags: &[&str]) -> Self {
        if tags.is_empty() {
            return self.clone();
        }
        Self {
            name: self.name.clone(),
            description: self.description.clone(),
            cases: self
                .cases
                .iter()
                .filter(|c| tags.iter().any(|t| c.tags.iter().any(|ct| ct == t)))
                .cloned()
                .collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// Target + Scorer + Score
// ---------------------------------------------------------------------------

/// The function under test.
pub type Target =
    Box<dyn Fn(&Case) -> Result<BTreeMap<String, serde_json::Value>, Error> + Send + Sync>;

/// Evaluates one result.  Score is in `[0.0, 1.0]`.
pub type Scorer = Box<dyn Fn(&Case, &BTreeMap<String, serde_json::Value>) -> Score + Send + Sync>;

/// Per-case scoring outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    /// Numeric score in `[0.0, 1.0]`.
    pub value: f64,
    /// `true` when `value >= threshold` (threshold defined by the scorer).
    pub pass: bool,
    /// Human-readable explanation.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Result + Report
// ---------------------------------------------------------------------------

/// Per-case execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseResult {
    /// The case that was run.
    pub case: Case,
    /// Actual output from the target, if successful.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub got: Option<BTreeMap<String, serde_json::Value>>,
    /// Score assigned by the scorer.
    pub score: Score,
    /// Error message if the target raised an error.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error: String,
    /// Wall-clock latency in milliseconds.
    pub latency_ms: u64,
}

/// Aggregated results across an entire suite run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Suite name.
    pub suite: String,
    /// ISO 8601 start timestamp.
    pub started_at: String,
    /// ISO 8601 end timestamp.
    pub ended_at: String,
    /// Per-case results.
    pub results: Vec<CaseResult>,
    /// Mean score across all cases.
    pub mean_score: f64,
    /// Number of passing cases.
    pub pass_count: usize,
    /// Number of failing cases.
    pub fail_count: usize,
}

impl Report {
    /// Return `true` when every case passed.
    pub fn all_passed(&self) -> bool {
        self.fail_count == 0
    }

    /// Serialize the report to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

/// Runs a [`Suite`] against a [`Target`] and scores each result.
pub struct Runner {
    target: Target,
    scorer: Scorer,
}

impl Runner {
    /// Create a runner with a target function and scorer.
    pub fn new(target: Target, scorer: Scorer) -> Self {
        Self { target, scorer }
    }

    /// Execute all cases in `suite` and return a [`Report`].
    pub fn run(&self, suite: &Suite) -> Report {
        let started = chrono::Utc::now();
        let mut results = Vec::with_capacity(suite.cases.len());

        for case in &suite.cases {
            let t0 = Instant::now();
            let (got, error_str) = match (self.target)(case) {
                Ok(out) => (Some(out), String::new()),
                Err(e) => (None, e.to_string()),
            };

            let score = match &got {
                Some(out) => (self.scorer)(case, out),
                None => Score {
                    value: 0.0,
                    pass: false,
                    reason: error_str.clone(),
                },
            };

            results.push(CaseResult {
                case: case.clone(),
                got,
                score,
                error: error_str,
                latency_ms: t0.elapsed().as_millis() as u64,
            });
        }

        let pass_count = results.iter().filter(|r| r.score.pass).count();
        let fail_count = results.len() - pass_count;
        let mean_score = if results.is_empty() {
            0.0
        } else {
            results.iter().map(|r| r.score.value).sum::<f64>() / results.len() as f64
        };

        let ended = chrono::Utc::now();
        Report {
            suite: suite.name.clone(),
            started_at: started.to_rfc3339(),
            ended_at: ended.to_rfc3339(),
            results,
            mean_score,
            pass_count,
            fail_count,
        }
    }
}

// ---------------------------------------------------------------------------
// Built-in scorers
// ---------------------------------------------------------------------------

/// Scorer that checks exact equality of every key in `case.expected`.
pub fn exact_match_scorer() -> Scorer {
    Box::new(|case: &Case, got: &BTreeMap<String, serde_json::Value>| {
        let mut mismatches = Vec::new();
        for (k, exp) in &case.expected {
            match got.get(k) {
                Some(actual) if actual == exp => {}
                Some(actual) => {
                    mismatches.push(format!("{}: expected {:?} got {:?}", k, exp, actual))
                }
                None => mismatches.push(format!("{}: missing", k)),
            }
        }
        if mismatches.is_empty() {
            Score {
                value: 1.0,
                pass: true,
                reason: String::new(),
            }
        } else {
            Score {
                value: 0.0,
                pass: false,
                reason: mismatches.join("; "),
            }
        }
    })
}

/// Scorer that always passes — useful as a latency / no-crash baseline.
pub fn passthrough_scorer() -> Scorer {
    Box::new(|_case, _got| Score {
        value: 1.0,
        pass: true,
        reason: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_suite() -> Suite {
        Suite {
            name: "add".to_string(),
            description: String::new(),
            cases: vec![
                Case {
                    id: "c1".to_string(),
                    tags: vec!["smoke".to_string()],
                    input: [
                        ("a".to_string(), serde_json::json!(1)),
                        ("b".to_string(), serde_json::json!(2)),
                    ]
                    .into(),
                    expected: [("result".to_string(), serde_json::json!(3))].into(),
                },
                Case {
                    id: "c2".to_string(),
                    tags: vec![],
                    input: [
                        ("a".to_string(), serde_json::json!(10)),
                        ("b".to_string(), serde_json::json!(5)),
                    ]
                    .into(),
                    expected: [("result".to_string(), serde_json::json!(15))].into(),
                },
            ],
        }
    }

    #[test]
    fn runner_exact_match_passes() {
        let suite = simple_suite();
        let target: Target = Box::new(|c: &Case| {
            let a = c.input["a"].as_i64().unwrap_or(0);
            let b = c.input["b"].as_i64().unwrap_or(0);
            Ok([("result".to_string(), serde_json::json!(a + b))].into())
        });
        let runner = Runner::new(target, exact_match_scorer());
        let report = runner.run(&suite);
        assert_eq!(report.pass_count, 2);
        assert_eq!(report.fail_count, 0);
        assert!(report.all_passed());
    }

    #[test]
    fn runner_captures_errors() {
        let suite = simple_suite();
        let target: Target = Box::new(|_| Err(Error::internal("boom")));
        let runner = Runner::new(target, passthrough_scorer());
        let report = runner.run(&suite);
        assert_eq!(report.pass_count, 0);
        assert_eq!(report.fail_count, 2);
    }

    #[test]
    fn filter_tags_works() {
        let suite = simple_suite();
        let filtered = suite.filter_tags(&["smoke"]);
        assert_eq!(filtered.cases.len(), 1);
        assert_eq!(filtered.cases[0].id, "c1");
    }

    #[test]
    fn suite_from_json_roundtrip() {
        let suite = simple_suite();
        let json = serde_json::to_vec(&suite).unwrap();
        let loaded = Suite::from_json(&json).unwrap();
        assert_eq!(loaded.name, suite.name);
        assert_eq!(loaded.cases.len(), suite.cases.len());
    }
}
