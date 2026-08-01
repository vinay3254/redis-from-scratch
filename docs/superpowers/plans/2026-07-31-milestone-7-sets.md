# Redis Clone — Milestone 7: Sets Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement Redis Set data structure (`Value::Set(HashSet<Vec<u8>>)`) and set commands (`SADD`, `SREM`, `SMEMBERS`, `SISMEMBER`).

**Architecture:**
- `src/db.rs`: `Value::Set(HashSet<Vec<u8>>)`, set helper methods (`sadd`, `srem`, `smembers`, `sismember`).
- `src/commands/set.rs`: Handlers for set commands.
- `src/commands/mod.rs`: Register commands in `dispatch()`.

**Tech Stack:** Rust standard library (`std::collections::HashSet`). No external crates.

## Global Constraints

- No external crates.
- Concurrency model: thread-per-connection sharing `Arc<Mutex<Db>>`.
- Code has no comments.
- Code must pass `cargo test`.

---

### Task 1: Update `src/db.rs` with `Value::Set` and set operations

- [x] **Step 1: Update `Value` enum and implement `sadd`, `srem`, `smembers`, `sismember` in `Db`**
- [x] **Step 2: Add unit tests for set operations in `src/db.rs`**

---

### Task 2: Implement set commands in `src/commands/set.rs`

- [x] **Step 1: Create `src/commands/set.rs` with command handlers**
- [x] **Step 2: Register handlers in `src/commands/mod.rs` dispatcher**
- [x] **Step 3: Add unit tests for set commands in `src/commands/mod.rs`**
- [x] **Step 4: Verify all tests pass with `cargo test`**
