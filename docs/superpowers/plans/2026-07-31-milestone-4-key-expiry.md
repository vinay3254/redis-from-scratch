# Redis Clone — Milestone 4: Key Expiry Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement key expiration (`EXPIRE`, `PEXPIRE`, `TTL`) with passive and active deletion mechanisms in `src/db.rs`, `src/commands/generic.rs`, and `src/main.rs`.

**Architecture:**
- `src/db.rs`: `HashMap<Vec<u8>, Instant>` tracks expiration times. Passive check on access; active sweep method `purge_expired()`.
- `src/commands/generic.rs`: Handlers for `EXPIRE`, `PEXPIRE`, and `TTL`.
- `src/main.rs`: Active expiration thread waking every 100ms.

**Tech Stack:** Rust standard library (`std::time::Instant`, `std::time::Duration`). No external crates.

## Global Constraints

- No external crates.
- Concurrency model: thread-per-connection sharing `Arc<Mutex<Db>>` + 1 background active expiration thread.
- Code has no comments.
- Code must pass `cargo test`.

---

### Task 1: Update `src/db.rs` with expiration map and passive/active sweep logic

- [x] **Step 1: Update `Db` struct and methods to track expiration, passive check, and active sweep**
- [x] **Step 2: Add unit tests for key expiry in `src/db.rs`**

---

### Task 2: Implement `EXPIRE`, `PEXPIRE`, `TTL` commands

- [x] **Step 1: Add handlers in `src/commands/generic.rs`**
- [x] **Step 2: Register commands in `src/commands/mod.rs` dispatcher**
- [x] **Step 3: Add unit tests for expiry commands**

---

### Task 3: Background Active Expiration Thread in `src/main.rs`

- [x] **Step 1: Spawn background thread in `src/main.rs` looping every 100ms**
- [x] **Step 2: Verify with `cargo test`**
