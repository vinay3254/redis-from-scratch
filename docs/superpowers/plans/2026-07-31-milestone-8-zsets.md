# Redis Clone — Milestone 8: Sorted Sets Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement hand-rolled Skip List (`src/skiplist.rs`), `ZSet` data structure, and sorted set commands (`ZADD`, `ZRANGE`, `ZSCORE`).

**Architecture:**
- `src/skiplist.rs`: Hand-rolled Skip List implementation.
- `src/db.rs`: `ZSet` struct (`HashMap<Vec<u8>, f64>` + `SkipList`), `Value::ZSet`, zset helper methods (`zadd`, `zscore`, `zrange`).
- `src/commands/zset.rs`: Handlers for `ZADD`, `ZRANGE`, `ZSCORE`.
- `src/commands/mod.rs`: Register commands in `dispatch()`.

**Tech Stack:** Rust standard library (`std`). No external crates.

## Global Constraints

- No external crates.
- Concurrency model: thread-per-connection sharing `Arc<Mutex<Db>>`.
- Code has no comments.
- Code must pass `cargo test`.

---

### Task 1: Create `src/skiplist.rs` with hand-rolled Skip List

- [ ] **Step 1: Write `src/skiplist.rs` struct, node framing, insertion, deletion, and range queries**
- [ ] **Step 2: Add unit tests in `src/skiplist.rs`**

---

### Task 2: Update `src/db.rs` with `ZSet` and `Value::ZSet`

- [ ] **Step 1: Implement `ZSet` and update `Db` with `zadd`, `zscore`, `zrange`**
- [ ] **Step 2: Add unit tests for `ZSet` in `src/db.rs`**

---

### Task 3: Implement sorted set commands in `src/commands/zset.rs`

- [ ] **Step 1: Create `src/commands/zset.rs` with command handlers**
- [ ] **Step 2: Register handlers in `src/commands/mod.rs` dispatcher**
- [ ] **Step 3: Add unit tests for zset commands in `src/commands/mod.rs`**
- [ ] **Step 4: Verify all tests pass with `cargo test`**

---

### Task 4: Push branch and open GitHub PR

- [ ] **Step 1: Push `phase-8/zsets` to GitHub**
- [ ] **Step 2: Open PR #8 against `phase-7/sets`**
