# Redis Clone — Milestone 9: RDB Persistence Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement RDB binary snapshot persistence (`src/persistence/rdb.rs`), `SAVE`/`BGSAVE` commands, and automatic DB loading on startup.

**Architecture:**
- `src/persistence/rdb.rs`: Serialization/deserialization logic for custom RDB binary snapshot format.
- `src/commands/generic.rs`: Handlers for `SAVE` and `BGSAVE`.
- `src/main.rs`: Bootstraps DB from `dump.rdb` if file exists on disk.

**Tech Stack:** Rust standard library (`std::fs::File`, `std::io::{Read, Write}`). No external crates.

## Global Constraints

- No external crates.
- Concurrency model: thread-per-connection sharing `Arc<Mutex<Db>>`.
- Code has no comments.
- Code must pass `cargo test`.

---

### Task 1: Create `src/persistence/rdb.rs` snapshot engine

- [ ] **Step 1: Write `src/persistence/rdb.rs` with `dump_db` and `load_db`**
- [ ] **Step 2: Add unit tests for RDB binary roundtrips**

---

### Task 2: Implement `SAVE` and `BGSAVE` commands

- [ ] **Step 1: Implement `save` and `bgsave` handlers in `src/commands/generic.rs`**
- [ ] **Step 2: Register handlers in `src/commands/mod.rs` dispatcher**
- [ ] **Step 3: Add unit tests for `SAVE` and `BGSAVE`**

---

### Task 3: Load `dump.rdb` on startup in `src/main.rs`

- [ ] **Step 1: Update `src/main.rs` boot sequence to load `dump.rdb` if present**
- [ ] **Step 2: Verify with `cargo test`**

---

### Task 4: Push branch and open GitHub PR

- [ ] **Step 1: Push `phase-9/rdb-persistence` to GitHub**
- [ ] **Step 2: Open PR #9 against `phase-8/zsets`**
