# Redis Clone — Milestone 3: Command Dispatcher + PING/ECHO/SET/GET/DEL/EXISTS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.
>
> **Note:** This milestone's code already exists in the repo (commit `71f69d5`, later folded into `src/db.rs` / `src/commands/` as later milestones extended the same files). This plan documents what Milestone 3 is supposed to deliver so the existing implementation can be checked against it task-by-task, rather than rewritten from scratch.

**Goal:** A shared, thread-safe key-value store reachable from every connection, plus a command dispatcher that parses a RESP array into a command name + arguments and routes it to a handler for `PING`, `ECHO`, `SET`, `GET`, `DEL`, `EXISTS`.

**Architecture:** A `Db` struct (currently `src/db.rs`) owns a `HashMap<Vec<u8>, Value>`, wrapped in `Arc<Mutex<Db>>` and shared across all connection threads (this is where Milestone 1's per-connection isolation ends and shared state begins). `src/commands/mod.rs::dispatch` takes a parsed `RespFrame::Array`, extracts the command name and arguments as raw bytes, uppercases the command name for case-insensitive matching, and routes to a handler function. Handlers return a `RespFrame` that the connection loop serializes and writes back.

**Tech Stack:** Rust standard library only (`std::collections::HashMap`, `std::sync::{Arc, Mutex}`). Builds on Milestone 2's `RespFrame` type.

## Global Constraints

- Language: Rust, no external Redis crates.
- Concurrency model: thread-per-connection, shared mutable state behind `Arc<Mutex<Db>>` (single global lock — no per-key locking at this milestone).
- No automated test suite required for milestone sign-off, but the existing code does carry `#[cfg(test)]` unit tests; keep them passing.
- Code has no comments.
- Redis command names are case-insensitive (`set`, `SET`, `SeT` all valid) — the dispatcher uppercases before matching.
- Unknown commands return `-ERR unknown command '<name>'`.
- Wrong argument count returns `-ERR wrong number of arguments for '<cmd>' command` (lowercase command name in the message, matching real Redis).
- Server continues listening on port `6380` (per Milestone 1's port-conflict resolution).

---

### Task 1: Shared `Db` store with String values

**Files:**
- Modify: `src/db.rs`

**Interfaces:**
- Consumes: nothing new (pure data structure).
- Produces: `pub struct Db { ... }`, `pub enum Value { String(Vec<u8>), ... }`, `Db::new() -> Db`, `Db::get(&mut self, key: &[u8]) -> Option<&Value>`, `Db::set(&mut self, key: Vec<u8>, value: Value)`, `Db::del(&mut self, keys: &[Vec<u8>]) -> usize`, `Db::exists(&mut self, keys: &[Vec<u8>]) -> usize`. Later milestones add more `Value` variants and more `Db` methods to this same file — don't remove existing ones.

- [x] **Step 1: Confirm the current `Db`/`Value` definitions satisfy this milestone**

Read `src/db.rs`. Confirm it defines:

```rust
pub enum Value {
    String(Vec<u8>),
    // later milestones add List/Hash/Set/ZSet variants here
}

pub struct Db {
    entries: HashMap<Vec<u8>, Value>,
    // later milestones add an `expirations` map here
}
```

with `get`, `set`, `del`, `exists` methods matching the signatures above. If any is missing or has a different signature, that's a gap against this milestone — fix it before continuing.

- [x] **Step 2: Run the existing unit tests for `Db`**

Run:

```bash
cargo test db::tests
```

Expected: `test_db_operations` (and any other `db::tests::*` present) pass, exercising `set`/`get`/`exists`/`del`.

- [x] **Step 3: Commit (only if Step 1 required a fix)**

```bash
git add src/db.rs
git commit -m "Ensure Db store satisfies Milestone 3 requirements"
```

If Step 1 required no changes, skip this commit — there's nothing new to record.

---

### Task 2: Command dispatcher skeleton + PING/ECHO

**Files:**
- Modify: `src/commands/mod.rs`
- Modify: `src/commands/string.rs`

**Interfaces:**
- Consumes: `RespFrame` (from Milestone 2), `Db` (from Task 1).
- Produces: `pub fn dispatch(frame: RespFrame, db: Arc<Mutex<Db>>, ...) -> RespFrame` (later milestones add more parameters — `pubsub`, `aof` — to this same signature, so check by matching first 2 params, not exact arity), `pub fn ping(args: &[Vec<u8>]) -> RespFrame`, `pub fn echo(args: &[Vec<u8>]) -> RespFrame`.

- [x] **Step 1: Confirm argument extraction from the RESP array**

Read `src/commands/mod.rs`. Confirm `dispatch` (or a helper it calls) does the following before matching on command name:
1. Rejects anything that isn't `RespFrame::Array(Some(elements))` with at least one element, returning `RespFrame::Error("ERR command must be a non-empty array".into())`.
2. Extracts each array element's bytes (`BulkString(Some(bytes))` primarily; `SimpleString` should also work since RESP2 clients may send either).
3. Uppercases the first element as the command name; the rest are `cmd_args`.

- [x] **Step 2: Confirm PING and ECHO handlers**

Read `src/commands/string.rs`. Confirm:

```rust
pub fn ping(args: &[Vec<u8>]) -> RespFrame {
    match args.len() {
        0 => RespFrame::SimpleString("PONG".into()),
        1 => RespFrame::BulkString(Some(args[0].clone())),
        _ => RespFrame::Error("ERR wrong number of arguments for 'ping' command".into()),
    }
}

pub fn echo(args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 1 {
        return RespFrame::Error("ERR wrong number of arguments for 'echo' command".into());
    }
    RespFrame::BulkString(Some(args[0].clone()))
}
```

`PING` with zero args returns a simple string `PONG`; `PING <msg>` echoes `<msg>` as a bulk string (real Redis behavior) — confirm both arms exist.

- [x] **Step 3: Build and run unit tests**

Run:

```bash
cargo build && cargo test commands::tests::test_ping_echo
```

Expected: builds clean, test passes.

- [x] **Step 4: Manually verify with redis-cli**

With the server running (`cargo run`, or the built binary, listening on 6380):

```bash
redis-cli -p 6380 PING
redis-cli -p 6380 PING "hello there"
redis-cli -p 6380 ECHO "hello there"
redis-cli -p 6380 ECHO
```

Expected: `PONG`, `"hello there"`, `"hello there"`, and an arity error for the bare `ECHO`.

---

### Task 3: SET/GET commands

**Files:**
- Modify: `src/commands/string.rs`
- Modify: `src/commands/mod.rs` (wire `SET`/`GET` into the dispatch match)

**Interfaces:**
- Consumes: `Db::get`/`Db::set` from Task 1.
- Produces: `pub fn set(db: &mut Db, args: &[Vec<u8>]) -> RespFrame`, `pub fn get(db: &mut Db, args: &[Vec<u8>]) -> RespFrame`.

- [x] **Step 1: Confirm SET**

```rust
pub fn set(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'set' command".into());
    }
    db.set(args[0].clone(), Value::String(args[1].clone()));
    RespFrame::SimpleString("OK".into())
}
```

- [x] **Step 2: Confirm GET, including the WRONGTYPE and nil cases**

```rust
pub fn get(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 1 {
        return RespFrame::Error("ERR wrong number of arguments for 'get' command".into());
    }
    match db.get(&args[0]) {
        Some(Value::String(val)) => RespFrame::BulkString(Some(val.clone())),
        Some(_) => RespFrame::Error(
            "WRONGTYPE Operation against a key holding the wrong kind of value".into(),
        ),
        None => RespFrame::BulkString(None),
    }
}
```

Note the `Some(_)` arm only becomes reachable once a non-String `Value` variant exists (added in later milestones) — it's fine for it to be unreachable in practice at this exact point in history, as long as the match compiles.

- [x] **Step 3: Build and test**

Run:

```bash
cargo build && cargo test commands::tests::test_set_get_del_exists
```

Expected: builds clean, test passes.

- [x] **Step 4: Manually verify with redis-cli**

```bash
redis-cli -p 6380 SET greeting hello
redis-cli -p 6380 GET greeting
redis-cli -p 6380 GET nonexistent
redis-cli -p 6380 SET onlyonearg
```

Expected: `OK`, `"hello"`, `(nil)`, arity error on the last one.

---

### Task 4: DEL/EXISTS commands

**Files:**
- Modify: `src/commands/generic.rs`
- Modify: `src/commands/mod.rs` (wire `DEL`/`EXISTS` into the dispatch match)

**Interfaces:**
- Consumes: `Db::del`/`Db::exists` from Task 1.
- Produces: `pub fn del(db: &mut Db, args: &[Vec<u8>]) -> RespFrame`, `pub fn exists(db: &mut Db, args: &[Vec<u8>]) -> RespFrame`. Both accept a variadic key list, matching real Redis (`DEL k1 k2 k3`, `EXISTS k1 k2 k2` counts `k2` twice if present).

- [x] **Step 1: Confirm DEL and EXISTS accept multiple keys**

```rust
pub fn del(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.is_empty() {
        return RespFrame::Error("ERR wrong number of arguments for 'del' command".into());
    }
    let count = db.del(args);
    RespFrame::Integer(count as i64)
}

pub fn exists(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.is_empty() {
        return RespFrame::Error("ERR wrong number of arguments for 'exists' command".into());
    }
    let count = db.exists(args);
    RespFrame::Integer(count as i64)
}
```

- [x] **Step 2: Build and test**

```bash
cargo build && cargo test commands::tests::test_set_get_del_exists
```

Expected: builds clean, test passes (same test as Task 3, since it exercises the full SET/GET/DEL/EXISTS chain).

- [x] **Step 3: Manually verify with redis-cli, including multi-key behavior**

```bash
redis-cli -p 6380 SET a 1
redis-cli -p 6380 SET b 2
redis-cli -p 6380 EXISTS a b c
redis-cli -p 6380 EXISTS a a
redis-cli -p 6380 DEL a b c
redis-cli -p 6380 EXISTS a b
```

Expected: `2` (a and b exist, c doesn't), `2` (a counted twice), `2` (only a and b were actually deleted, c didn't exist), `0`.

- [x] **Step 4: Verify unknown-command handling end-to-end**

```bash
redis-cli -p 6380 FOOBAR
```

Expected: `(error) ERR unknown command 'FOOBAR'`.

---

## Milestone 3 Done

Once Task 4's manual verification passes (including the unknown-command check), this milestone is confirmed against spec. Report back to confirm before Milestone 4's plan (key expiry: `EXPIRE`/`PEXPIRE`/`TTL`, passive + active expiration) is written.
