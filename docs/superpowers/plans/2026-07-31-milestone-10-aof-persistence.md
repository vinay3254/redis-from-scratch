# Redis Clone — Milestone 10: AOF Persistence Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement Append-Only File (AOF) persistence (`src/persistence/aof.rs`), write logger for mutating commands, and AOF replay on boot.

**Architecture:**
- `src/persistence/aof.rs`: `Aof` logger (`open`, `append`, `replay`).
- `src/commands/mod.rs`: Log write command RESP frames to `Aof`.
- `src/main.rs`: Bootstraps DB from `appendonly.aof` if present.

**Tech Stack:** Rust standard library (`std::fs::OpenOptions`, `std::io::{Read, Write}`). No external crates.

## Global Constraints

- No external crates.
- Concurrency model: thread-per-connection sharing `Arc<Mutex<Db>>` and `Arc<Aof>`.
- Code has no comments.
- Code must pass `cargo test`.

---

### Task 1: Create `src/persistence/aof.rs` engine

- [x] **Step 1: Write `src/persistence/aof.rs` with `Aof` struct, `open`, `append`, and `replay`**
- [x] **Step 2: Add unit tests for AOF logging and replay**

---

### Task 2: Integrate AOF logging into `src/commands/mod.rs` and `src/main.rs`

- [x] **Step 1: Update `dispatch` to log write commands to `Aof`**
- [x] **Step 2: Update `main.rs` to replay `appendonly.aof` on boot**
- [x] **Step 3: Run unit tests with `cargo test`**

---

### Task 3: Push branch and open GitHub PR

- [x] **Step 1: Push `phase-10/aof-persistence` to GitHub**
- [x] **Step 2: Open PR #10 against `phase-9/rdb-persistence`**
