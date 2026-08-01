# Redis Clone — Milestone 6: Hashes Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement Redis Hash data structure (`Value::Hash(HashMap<Vec<u8>, Vec<u8>>)`) and hash commands (`HSET`, `HGET`, `HGETALL`, `HDEL`).

**Architecture:**
- `src/db.rs`: `Value::Hash(HashMap<Vec<u8>, Vec<u8>>)`, hash helper methods (`hset`, `hget`, `hgetall`, `hdel`).
- `src/commands/hash.rs`: Handlers for hash commands.
- `src/commands/mod.rs`: Register commands in `dispatch()`.

**Tech Stack:** Rust standard library (`std::collections::HashMap`). No external crates.

## Global Constraints

- No external crates.
- Concurrency model: thread-per-connection sharing `Arc<Mutex<Db>>`.
- Code has no comments.
- Code must pass `cargo test`.

---

### Task 1: Update `src/db.rs` with `Value::Hash` and hash operations

- [x] **Step 1: Update `Value` enum and implement `hset`, `hget`, `hgetall`, `hdel` in `Db`**
- [x] **Step 2: Add unit tests for hash operations in `src/db.rs`**

---

### Task 2: Implement hash commands in `src/commands/hash.rs`

- [x] **Step 1: Create `src/commands/hash.rs` with command handlers**
- [x] **Step 2: Register handlers in `src/commands/mod.rs` dispatcher**
- [x] **Step 3: Add unit tests for hash commands in `src/commands/mod.rs`**
- [x] **Step 4: Verify all tests pass with `cargo test`**
