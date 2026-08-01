# Redis Clone — Milestone 3: Command Dispatcher & Basic Commands Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement shared `Db` store, command dispatcher module, and six core Redis commands (`PING`, `ECHO`, `SET`, `GET`, `DEL`, `EXISTS`).

**Architecture:**
- `src/db.rs`: Shared thread-safe in-memory database wrapped in `Arc<Mutex<Db>>`.
- `src/commands/`:
  - `mod.rs`: Main command dispatcher. Converts frame into command name + arguments and routes to command handlers.
  - `string.rs`: `PING`, `ECHO`, `SET`, `GET`.
  - `generic.rs`: `DEL`, `EXISTS`.
- `src/main.rs`: Shares `Arc<Mutex<Db>>` across connection threads and passes parsed frames to dispatcher.

**Tech Stack:** Rust standard library (`std`). No external crates.

## Global Constraints

- No external crates.
- Concurrency model: thread-per-connection sharing `Arc<Mutex<Db>>`.
- Code has no comments.
- Code must pass `cargo test`.

---

### Task 1: Create `src/db.rs` with in-memory database store

**Files:**
- Create: `src/db.rs`

- [x] **Step 1: Write `src/db.rs` with `Value` enum and `Db` struct**
- [x] **Step 2: Add unit tests in `src/db.rs`**

---

### Task 2: Create command dispatcher and handlers in `src/commands/`

**Files:**
- Create: `src/commands/mod.rs`
- Create: `src/commands/string.rs`
- Create: `src/commands/generic.rs`

- [x] **Step 1: Write string commands (`PING`, `ECHO`, `SET`, `GET`)**
- [x] **Step 2: Write generic commands (`DEL`, `EXISTS`)**
- [x] **Step 3: Write dispatcher module and unit tests**

---

### Task 3: Integrate command dispatcher into `src/main.rs`

**Files:**
- Modify: `src/main.rs`

- [x] **Step 1: Share `Arc<Mutex<Db>>` across connection threads and execute dispatched commands**
- [x] **Step 2: Run unit tests with `cargo test`**
- [x] **Step 3: Build server with `cargo check`**
