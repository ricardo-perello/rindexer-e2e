### Rindexer E2E Test Plan

This document outlines a comprehensive checklist of end-to-end tests for Rindexer. Items marked [Covered] are already implemented in the current suite; [New] are recommended additions.

### Prioritized execution order (start here)
1. Storage: PostgreSQL end-to-end (enable, insert rows, idempotent migrations)
2. Health and observability: `/health` readiness + log-based completion [Covered]
3. GraphQL API: start service [Covered], basic queries, pagination/filtering
4. Historical restart/checkpoint: resume without duplicates after crash/restart [Covered]
5. Reorg and resilience: reorg rewind/reindex; RPC transient failures/backoff
6. Live robustness: burst/backpressure; slow sink (CSV/DB) stability
7. Configuration validation: invalid YAML, missing ABI, bad block ranges
8. Event correctness: include_events strictness, topics/data types, ordering, dedupe
9. Multi-contract/network: many contracts on one/multi networks; mixed live+historic
10. CLI workflows: `new`, `add contract`, `codegen`, `delete`, `phantom`
11. Performance/scale: sustained throughput target; bounded memory on large backfill
12. Forked/mainnet realism: run on Anvil fork; verify against known logs
13. Housekeeping/lifecycle: clean shutdown, safe reruns, clear diagnostics
14. Developer ergonomics: helpful errors for missing/wrong binaries/versions

### Core boot and connectivity
- [Covered] Minimal config boots indexer and stays healthy
- [New] Start `graphql` and `all` services without contracts

### Configuration parsing and validation
- [Covered] Invalid YAML fails with clear error
- [New] Missing ABI path yields actionable error
- [New] Bad `start_block`/`end_block` ranges are rejected
- [New] Multiple networks are parsed and normalized

### Contract and ABI discovery
- [Covered] ABI loaded and `Transfer` registered into outputs
- [New] Multiple ABIs across contracts are registered
- [New] Non-indexed events in ABI are ignored without crashing

### Historical indexing
- [Covered] Deployment `Transfer` (zero address) appears in CSV
- [New] Partial range backfill: `start_block > 0` honors bounds
- [New] Multiple contracts backfill concurrently
- [New] Restart continues from checkpoint without duplication

### Live indexing
- [Covered] Steady flow: ≥1 new events written
- [Covered] Higher frequency: ≥2 new events without loss
- [New] Backpressure: burst traffic doesn’t drop events
- [New] Slow sink (CSV/DB) does not stall or crash

### Event filters and data correctness
- [New] `include_events` limits output strictly to configured events
- [New] Indexed args (topics) and data fields match ABI types
- [New] Log ordering preserved within a block and across blocks
- [New] Duplicates not produced on reorg/retry

### Multi-contract and multi-network
- [New] Multiple contracts on one network
- [New] Same contract on multiple networks
- [New] Mixed live + historical tasks in one run

### Storage backends
- [Covered] CSV writer creates dirs/files and appends rows
- [New] PostgreSQL enabled: tables created, rows inserted
- [New] Postgres schema migration safe to rerun; idempotent
- [New] Storage toggle switches respected (csv off, pg on, etc.)

### Health and observability
- [Covered] `/health` returns healthy (readiness) once server is up
- [Covered] Log-based sync-complete used to assert completion when range is bounded
- [New] `/health.indexing` shows active_tasks during backfill; 0 when done (env permitting)

### GraphQL API
- [New] Service starts and exposes schema
- [New] Basic queries return indexed data (transfer by tx/hash/address)
- [New] Pagination and filtering semantics correct
- [New] GraphQL while indexing (eventual consistency, no crashes)

### CLI workflows
- [New] `new` creates project layout for no-code
- [New] `add contract` augments YAML correctly
- [New] `codegen` produces artifacts without panics
- [New] `delete` removes data safely
- [New] `phantom` runs and leaves config/data in valid state

### Resilience and fault tolerance
- [New] RPC transient failure: retry/backoff and recovery
- [New] Network partition: progress pauses and resumes without data loss
- [New] Reorg handling: correct rewinds and reindex
- [New] Crash mid-run: restart resumes at last checkpoint without dupes

### Performance and scale
- [New] Sustained throughput target (e.g., N events/s) on live feed
- [New] Memory remains bounded during large backfill
- [New] Large ABI (many events) doesn’t degrade correctness

### Forked/mainnet realism
- [New] Run on Anvil fork at known block range (real contract + ABI)
- [New] Deterministic verification against known tx logs

### Housekeeping and lifecycle
- [New] Clean shutdown of services (signals) flushes buffers
- [New] Re-running with same output dir/app DB is safe (no corruption)
- [New] File/permission errors produce clear diagnostics

### Developer ergonomics
- [New] Helpful errors when binaries missing or wrong versions
- [New] Clear logs at INFO; DEBUG adds ABI/decoding details without PII


