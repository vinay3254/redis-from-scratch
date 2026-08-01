# Redis Clone — Milestone 12: Basic Transactions Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement Redis transaction management (`src/commands/tx.rs`), connection command queueing, `MULTI`, `EXEC`, `DISCARD`, and atomic batch execution.

**Architecture:**
- `src/commands/tx.rs`: Atomic transaction execution engine.
- `src/main.rs`: Connection handler managing `in_transaction` state and `tx_queue`.
- `src/commands/mod.rs`: Transaction module registration.

**Tech Stack:** Rust standard library. No external crates.

## Global Constraints

- No external crates.
- Concurrency model: thread-per-connection sharing `Arc<Mutex<Db>>`, `Arc<Mutex<PubSub>>`, and `Arc<Aof>`.
- Code has no comments.
- Code must pass `cargo test`.

---

### Task 1: Create `src/commands/tx.rs` engine

- [x] **Step 1: Write `src/commands/tx.rs` with `exec` helper**
- [x] **Step 2: Update `src/commands/mod.rs` to export `tx`**

---

### Task 2: Implement transaction state loop in `src/main.rs`

- [x] **Step 1: Update `handle_connection` in `src/main.rs` to track `in_transaction` and `tx_queue`**
- [x] **Step 2: Handle `MULTI`, `EXEC`, `DISCARD`, and command queueing (`+QUEUED\r\n`)**
- [x] **Step 3: Add unit tests for transactions**
- [x] **Step 4: Verify with `cargo test`**

---

### Task 3: Push branch and open final GitHub PR

- [x] **Step 1: Push `phase-12/transactions` to GitHub**
- [x] **Step 2: Open PR #12 against `phase-11/pubsub`**
