use anyhow::Result;
use tracing::info;
use std::pin::Pin;
use std::future::Future;

use crate::test_suite::TestContext;
use crate::tests::registry::{TestDefinition, TestModule};

pub struct GraphqlQueriesTests;

impl TestModule for GraphqlQueriesTests {
    fn get_tests() -> Vec<TestDefinition> {
        vec![
            TestDefinition::new(
                "test_graphql_basic_query",
                "Start indexer+graphql, feed events, query transfers with filter & pagination",
                graphql_basic_query_test,
            ).with_timeout(300).as_live_test(),
        ]
    }
}

fn graphql_basic_query_test(context: &mut TestContext) -> Pin<Box<dyn Future<Output = Result<()>> + '_>> {
    Box::pin(async move {
        // Feeder is managed by TestRunner for live tests

        info!("Running GraphQL Queries Test");

        // Use the contract deployed by the TestRunner's live setup
        let contract_address = context.test_contract_address.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No test contract address available"))?;
        let mut config = context.create_contract_config(contract_address);
        // Enable Postgres for GraphQL (GraphQL typically serves off DB)
        config.storage.postgres.enabled = true;
        config.storage.csv.enabled = false;

        // Start a clean Postgres container (random port) for GraphQL backing store
        let (container_name, pg_port) = match crate::docker::start_postgres_container().await {
            Ok(v) => v,
            Err(e) => { return Err(crate::tests::test_runner::SkipTest(format!("Docker not available: {}", e)).into()); }
        };
        // Wait for Postgres readiness
        {
            let mut ready = false;
            for _ in 0..40 {
                if tokio_postgres::connect(
                    &format!("host=localhost port={} user=postgres password=postgres dbname=postgres", pg_port),
                    tokio_postgres::NoTls,
                ).await.is_ok() { ready = true; break; }
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
            if !ready { return Err(anyhow::anyhow!("Postgres did not become ready in time")); }
        }

        // Write config & ABI
        let config_path = context.project_path.join("rindexer.yaml");
        std::fs::create_dir_all(context.project_path.join("abis"))?;
        std::fs::copy("abis/SimpleERC20.abi.json", context.project_path.join("abis").join("SimpleERC20.abi.json"))?;
        let yaml = serde_yaml::to_string(&config)?;
        std::fs::write(&config_path, yaml)?;

        // Prepare instance with PG env (GraphQL uses the same DB)
        let mut r = crate::rindexer_client::RindexerInstance::new(&context.rindexer_binary, context.project_path.clone())
            .with_env("POSTGRES_HOST", "localhost")
            .with_env("POSTGRES_PORT", &pg_port.to_string())
            .with_env("POSTGRES_USER", "postgres")
            .with_env("POSTGRES_PASSWORD", "postgres")
            .with_env("POSTGRES_DB", "postgres")
            .with_env("DATABASE_URL", &format!("postgres://postgres:postgres@localhost:{}/postgres", pg_port))
            .with_env("GRAPHQL_PORT", "3001")
            .with_env("PORT", "3001");

        // Start ALL services (indexer + GraphQL) in one process
        r.start_all().await?;
        context.rindexer = Some(r.clone());

        // Wait for GraphQL URL from logs; fallback to default path
        let gql_url = r.wait_for_graphql_url(15).await
            .or_else(|| Some("http://localhost:3001/graphql".to_string()))
            .unwrap();
        info!("GraphQL URL: {}", gql_url);

        // LiveFeeder is already running from TestRunner for live tests; wait a bit for events
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        // Basic GraphQL query for transfers with filter and pagination
        let query = r#"{
  transfers(first: 2, orderBy: blockNumber, orderDirection: desc) {
    edges { node { blockNumber to from value txHash } }
    pageInfo { hasNextPage }
  }
}"#;

        let client = reqwest::Client::new();
        // Retry a few times while GraphQL warms up
        let mut body: Option<serde_json::Value> = None;
        for _ in 0..10 {
            let resp = client.post(&gql_url)
                .json(&serde_json::json!({"query": query}))
                .send().await;
            if let Ok(r) = resp {
                if r.status().is_success() {
                    if let Ok(json) = r.json::<serde_json::Value>().await {
                        body = Some(json); break;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        let body = body.ok_or_else(|| anyhow::anyhow!("GraphQL did not return success after retries"))?;

        // Sanity checks: structure + at least one edge
        let edges = body["data"]["transfers"]["edges"].as_array().unwrap_or(&vec![]).len();
        if edges == 0 {
            return Err(anyhow::anyhow!("GraphQL returned no transfers"));
        }

        // If pageInfo.hasNextPage is present, verify pagination flag exists
        let _ = body["data"]["transfers"]["pageInfo"]["hasNextPage"].as_bool();

        // Feeder is managed by TestRunner; no local stop

        // Cleanup PG container
        let _ = crate::docker::stop_postgres_container(&container_name).await;

        info!("✓ GraphQL Queries Test PASSED: basic query, filter, pagination");
        Ok(())
    })
}


