# Redis Clone — Milestone 5: Lists Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement Redis List data structure (`Value::List(VecDeque<Vec<u8>>)`) and list commands (`LPUSH`, `RPUSH`, `LPOP`, `RPOP`, `LRANGE`).

**Architecture:**
- `src/db.rs`: `Value::List(VecDeque<Vec<u8>>)`, list helper methods (`lpush`, `rpush`, `lpop`, `rpop`, `lrange`) with passive expiry and type checking.
- `src/commands/list.rs`: Handlers for list commands.
- `src/commands/mod.rs`: Register commands in `dispatch()`.

**Tech Stack:** Rust standard library (`std::collections::VecDeque`). No external crates.

## Global Constraints

- No external crates.
- Concurrency model: thread-per-connection sharing `Arc<Mutex<Db>>`.
- Code has no comments.
- Code must pass `cargo test`.

---

### Task 1: Update `src/db.rs` with `Value::List` and list operations

- [x] **Step 1: Update `Value` enum and implement `lpush`, `rpush`, `lpop`, `rpop`, `lrange` in `Db`**
- [x] **Step 2: Add unit tests for list operations in `src/db.rs`**

---

### Task 2: Implement list commands in `src/commands/list.rs`

- [x] **Step 1: Create `src/commands/list.rs` with command handlers**
- [x] **Step 2: Register handlers in `src/commands/mod.rs` dispatcher**
- [x] **Step 3: Add unit tests for list commands in `src/commands/mod.rs`**
- [x] **Step 4: Verify all tests pass with `cargo test`**
