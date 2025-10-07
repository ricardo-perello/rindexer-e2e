use std::time::Duration;

#[derive(Debug, Clone)]
pub enum TestResult {
    Passed,
    Failed(String),
    Timeout,
    Skipped(String),
}

impl TestResult {
    pub fn is_success(&self) -> bool {
        matches!(self, TestResult::Passed)
    }
}

#[derive(Debug)]
pub struct TestInfo {
    pub name: String,
    pub result: TestResult,
    pub duration: Duration,
    pub error_message: Option<String>,
}

impl TestInfo {
    pub fn new(name: String, result: TestResult, duration: Duration) -> Self {
        let error_message = match &result {
            TestResult::Failed(msg) => Some(msg.clone()),
            TestResult::Timeout => Some("Test timed out".to_string()),
            TestResult::Skipped(msg) => Some(msg.clone()),
            TestResult::Passed => None,
        };

        Self { name, result, duration, error_message }
    }
}

pub struct TestSuite {
    pub name: String,
    pub tests: Vec<TestInfo>,
    pub duration: Duration,
}

impl TestSuite {
    pub fn new(name: String) -> Self {
        Self { name, tests: Vec::new(), duration: Duration::ZERO }
    }

    pub fn add_test(&mut self, test: TestInfo) {
        self.duration += test.duration;
        self.tests.push(test);
    }

    pub fn passed_count(&self) -> usize {
        self.tests.iter().filter(|t| t.result.is_success()).count()
    }

    pub fn failed_count(&self) -> usize {
        self.tests.iter().filter(|t| matches!(t.result, TestResult::Failed(_))).count()
    }

    pub fn timeout_count(&self) -> usize {
        self.tests.iter().filter(|t| matches!(t.result, TestResult::Timeout)).count()
    }

    pub fn skipped_count(&self) -> usize {
        self.tests.iter().filter(|t| matches!(t.result, TestResult::Skipped(_))).count()
    }

    pub fn total_count(&self) -> usize {
        self.tests.len()
    }

    pub fn print_summary(&self) {
        println!();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        let passed = self.passed_count();
        let failed = self.failed_count();
        let timeout = self.timeout_count();
        let skipped = self.skipped_count();
        let total = self.total_count();

        if failed == 0 && timeout == 0 {
            println!("[SUCCESS] Test Suites: 1 passed, 1 total");
            println!("[SUCCESS] Tests:       {} passed, {} total", passed, total);
        } else {
            println!(
                "[ERROR] Test Suites: {} failed, 1 total",
                if failed > 0 || timeout > 0 { 1 } else { 0 }
            );
            println!(
                "[ERROR] Tests:       {} failed, {} passed, {} total",
                failed + timeout,
                passed,
                total
            );
        }

        if skipped > 0 {
            println!("[SKIP] Skipped:     {}", skipped);
        }

        println!("[TIME] Time:        {:.2}s", self.duration.as_secs_f64());

        if failed > 0 || timeout > 0 {
            println!();
            println!("Failed Tests:");
            for test in &self.tests {
                if let TestResult::Failed(msg) = &test.result {
                    println!("  [ERROR] {} - {}", test.name, msg);
                } else if let TestResult::Timeout = &test.result {
                    println!("  [TIMEOUT] {} - Test timed out after {} seconds", test.name, test.duration.as_secs());
                }
            }
        }

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        if failed == 0 && timeout == 0 {
            println!("🎉 All tests passed!");
        } else {
            println!("💥 Some tests failed. See details above.");
        }
    }
}
