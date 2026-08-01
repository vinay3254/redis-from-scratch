# Redis Clone — Milestone 11: Pub/Sub Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement Pub/Sub message broker registry (`src/pubsub.rs`), channel subscription management, `PUBLISH`, `SUBSCRIBE`, `UNSUBSCRIBE` commands, and push connection loops.

**Architecture:**
- `src/pubsub.rs`: `PubSub` registry (`publish`, `subscribe`, `unsubscribe`).
- `src/commands/pubsub.rs`: `PUBLISH` command handler.
- `src/commands/mod.rs`: Dispatcher handling Pub/Sub commands.
- `src/main.rs`: Connection handler entering subscribed push mode.

**Tech Stack:** Rust standard library (`std::sync::mpsc`, `std::collections::HashMap`). No external crates.

## Global Constraints

- No external crates.
- Concurrency model: thread-per-connection sharing `Arc<Mutex<Db>>` and `Arc<Mutex<PubSub>>`.
- Code has no comments.
- Code must pass `cargo test`.

---

### Task 1: Create `src/pubsub.rs` engine

- [x] **Step 1: Write `src/pubsub.rs` with `PubSub` struct, `publish`, `subscribe`, `unsubscribe`**
- [x] **Step 2: Add unit tests for `PubSub`**

---

### Task 2: Implement `PUBLISH` command and connection subscribed mode

- [x] **Step 1: Create `src/commands/pubsub.rs` with `PUBLISH` handler**
- [x] **Step 2: Update `src/commands/mod.rs` dispatcher**
- [x] **Step 3: Update `src/main.rs` connection loop for push delivery**
- [x] **Step 4: Verify with `cargo test`**

---

### Task 3: Push branch and open GitHub PR

- [x] **Step 1: Push `phase-11/pubsub` to GitHub**
- [x] **Step 2: Open PR #11 against `phase-10/aof-persistence`**
