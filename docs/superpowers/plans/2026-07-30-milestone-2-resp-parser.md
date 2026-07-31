# Redis Clone — Milestone 2: RESP2 Parser and Serializer Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a complete RESP2 protocol parser and serializer in Rust (`src/resp.rs`) and integrate it into `src/main.rs` so the TCP server parses incoming client frames and echoes them back as RESP.

**Architecture:** `src/resp.rs` provides `RespFrame` representing the 5 RESP2 data types, along with `parse()` and `serialize()`. `src/main.rs` maintains a per-connection read buffer, parsing incoming frames and echoing serialized RESP.

**Tech Stack:** Rust standard library (`std`). No external crates.

## Global Constraints

- No external crates.
- Concurrency model: thread-per-connection.
- Code has no comments.
- Code must pass `cargo test`.

---

### Task 1: Create `src/resp.rs` with `RespFrame` parser and serializer

**Files:**
- Create: `src/resp.rs`

- [ ] **Step 1: Write `src/resp.rs` frame definition, parser, serializer, and unit tests**

- [ ] **Step 2: Run unit tests**

```bash
cargo test
```

Expected: All tests in `src/resp.rs` pass.

---

### Task 2: Integrate RESP parser into `src/main.rs` server loop

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update connection handler to buffer bytes, parse frames, and echo RESP frames back**

- [ ] **Step 2: Build and run server**

```bash
cargo check
```

---

### Task 3: Manual Verification

- [ ] **Step 1: Test with redis-cli**

Run `redis-cli -p 6380 PING` or interactive `redis-cli -p 6380` and send commands to verify echo behavior.
