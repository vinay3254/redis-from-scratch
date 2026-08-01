# Redis Clone — Milestone 10: AOF Persistence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Note:** This milestone's code already exists (commit `f073176`). This plan documents what it must deliver so the implementation can be verified against it, rather than rewritten from scratch. **A real bug was found and fixed at this milestone's boundary with Milestone 9** — see Task 3 below, which is the regression test for it. Do not skip Task 3.

**Goal:** Every successful write command is appended to `appendonly.aof` as its original RESP-encoded command; on boot, that log is replayed to reconstruct state.

**Architecture:** `Aof` (`src/persistence/aof.rs`) wraps a `File` opened in append mode. `commands::dispatch` (and separately, `commands::tx::exec` for transactions) appends the *original* `RespFrame` — not a re-serialized/normalized version — for any command in the `is_write_command` allowlist, but only after the command actually succeeded (an error response is never logged). On boot, `Aof::replay` reads the file back, re-parses each RESP frame, and calls `dispatch_mutating` (the same dispatch core used for live commands, minus the AOF-append and pub/sub side effects) to rebuild state.

**Tech Stack:** `std::fs::OpenOptions` (`create(true).append(true).read(true)`), reusing the Milestone 2 `RespFrame` parser for replay.

## Global Constraints

- Language: Rust, no external crates.
- Only commands in `is_write_command` (`SET`, `DEL`, `EXPIRE`, `PEXPIRE`, `LPUSH`, `RPUSH`, `LPOP`, `RPOP`, `HSET`, `HDEL`, `SADD`, `SREM`, `ZADD`) are logged — read commands (`GET`, `LRANGE`, etc.) and `PING`/`ECHO` never touch the AOF.
- A command that returned an error is never appended — replaying the log must not attempt commands that never actually mutated state the first time.
- **On boot, AOF and RDB are not both replayed cumulatively.** If `appendonly.aof` exists, it alone is replayed into a fresh `Db` (it's a complete history since the file's creation, so it fully reconstructs state on its own). `dump.rdb` is only consulted as a fallback when no AOF file exists. This is the fixed version of the logic — the original code loaded RDB *then* unconditionally replayed the full AOF on top, which silently duplicated every non-idempotent write (`LPUSH`/`RPUSH`) on every restart after a `SAVE`. Real Redis has the same principle: when both persistence mechanisms are present, it doesn't apply both cumulatively.
- Code has no comments.

---

### Task 1: Aof struct — append and replay

**Files:**
- Modify: `src/persistence/aof.rs`

**Interfaces:**
- Produces: `pub struct Aof { ... }`, `Aof::open(path: &str) -> std::io::Result<Self>`, `Aof::append(&self, frame: &RespFrame) -> std::io::Result<()>`, `Aof::replay(path: &str, db: &mut Db) -> std::io::Result<()>` (associated function, not a method — it takes a fresh path and an external `&mut Db` rather than operating on `self`, since replay happens once at boot before any `Aof` instance is even needed for live appends).

- [ ] **Step 1: Confirm Aof::open uses create+append+read, and append flushes**

```rust
pub fn open(path: &str) -> std::io::Result<Self> {
    let file = OpenOptions::new().create(true).append(true).read(true).open(path)?;
    Ok(Aof { file: Arc::new(Mutex::new(file)) })
}

pub fn append(&self, frame: &RespFrame) -> std::io::Result<()> {
    let bytes = frame.serialize();
    let mut f = self.file.lock().unwrap();
    f.write_all(&bytes)?;
    f.flush()
}
```

`append` calls `.flush()` after every single write — this is a durability-over-throughput choice (real Redis's `appendfsync always` mode, the safest but slowest option) rather than batching. Confirm this is intentional in the current design, not an oversight — for a learning project prioritizing correctness, always-flush is the right default; if performance ever becomes a concern, that's a deliberate future tradeoff, not a bug to silently fix here.

- [ ] **Step 2: Confirm replay parses frames incrementally, not by loading the whole file into one buffer and calling parse once**

```rust
pub fn replay(path: &str, db: &mut Db) -> std::io::Result<()> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    let mut read_buf = [0u8; 512];

    loop {
        let bytes_read = reader.read(&mut read_buf)?;
        if bytes_read == 0 {
            break;
        }
        buffer.extend_from_slice(&read_buf[..bytes_read]);

        loop {
            match RespFrame::parse(&buffer) {
                Ok(Some((frame, consumed))) => {
                    crate::commands::dispatch_mutating(frame, db);
                    buffer.drain(..consumed);
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }
    }
    Ok(())
}
```

This mirrors the same incremental-parse-and-drain pattern used in `main.rs`'s connection loop (Milestone 2/3), reused here for replay instead of a live socket. Confirm a parse error (`Err(_)`) breaks the inner loop rather than propagating — a single malformed trailing record (e.g. from a crash mid-write) shouldn't prevent replaying everything that came before it.

- [ ] **Step 3: Run unit test**

```bash
cargo test persistence::aof::tests::test_aof_append_and_replay
```

Expected: pass.

---

### Task 2: AOF-append wiring in dispatch and transactions

**Files:**
- Modify: `src/commands/mod.rs`
- Modify: `src/commands/tx.rs`

**Interfaces:**
- Consumes: `Aof::append` (Task 1).
- Produces: `pub fn is_write_command(cmd_name: &str) -> bool`.

- [ ] **Step 1: Confirm dispatch only appends on success, for write commands, after executing**

```rust
let response = {
    let mut db_guard = db.lock().unwrap();
    dispatch_mutating(raw_frame.clone(), &mut db_guard)
};

if let RespFrame::Error(_) = &response {
} else if is_write_command(&cmd_name) {
    if let Some(a) = aof {
        a.append(&raw_frame).ok();
    }
}
```

The append happens strictly after `dispatch_mutating` returns and only when the response isn't an error — confirm this ordering (execute-then-log, not log-then-execute), since logging a command that then fails would corrupt future replays.

- [ ] **Step 2: Confirm the same success/write-command gating exists independently in commands::tx::exec**

`MULTI`/`EXEC` bypasses the normal `dispatch` function (it needs to hold the DB lock across the whole queued batch, not per-command), so it has its own copy of the same logic:

```rust
let result = super::dispatch_mutating(frame.clone(), &mut db_guard);
if let RespFrame::Error(_) = &result {
} else if super::is_write_command(&cmd_name) {
    if let Some(a) = aof {
        a.append(&frame).ok();
    }
}
```

Confirm this exists in `src/commands/tx.rs::exec` and matches the same success/write-command gating as Task 2 Step 1 — since this is a second, independently-maintained copy of the same rule (not a shared helper), it's a spot where the two copies could drift out of sync if one is edited without the other.

- [ ] **Step 3: Run unit tests**

```bash
cargo test commands::tests commands::tx::tests
```

Expected: pass.

---

### Task 3: Boot-time load ordering (regression test for the RDB/AOF double-apply bug)

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `Aof::replay`, `persistence::rdb::load_db` (Milestone 9).

- [ ] **Step 1: Confirm main() prefers AOF and only falls back to RDB**

```rust
let mut db_instance = Db::new();

if Path::new("appendonly.aof").exists() {
    Aof::replay("appendonly.aof", &mut db_instance).ok();
} else if Path::new("dump.rdb").exists() {
    if let Ok(loaded_db) = persistence::rdb::load_db("dump.rdb") {
        db_instance = loaded_db;
    }
}
```

This is the fixed version (commit `c552716`). If you're checking this against an older checkout, the broken version instead unconditionally loaded RDB *then* replayed AOF on top regardless of whether RDB was loaded — confirm the current code does NOT do that.

- [ ] **Step 2: Regression-test the exact bug that was found — isolated repro**

```bash
# from a clean state (no dump.rdb, no appendonly.aof)
redis-cli -p 6380 RPUSH testlist a b c
redis-cli -p 6380 SAVE
redis-cli -p 6380 LRANGE testlist 0 -1
```

Expected at this point: `a b c` (3 elements).

Stop the server, restart it, then:

```bash
redis-cli -p 6380 LRANGE testlist 0 -1
```

Expected: still `a b c` (3 elements) — **not** `a b c a b c` (6 elements). If you see 6 elements, the bug has regressed; check Step 1's boot logic first.

- [ ] **Step 3: Verify the RDB-only fallback path still works when no AOF file exists**

```bash
# stop the server
rm appendonly.aof
# keep dump.rdb
# restart the server
redis-cli -p 6380 LRANGE testlist 0 -1
```

Expected: still `a b c` — proving the RDB fallback branch (Task 3 Step 1's `else if`) is reachable and correct on its own, not just dead code now that AOF is preferred.

---

## Milestone 10 Done

Once Task 3's manual verification passes (both the regression repro and the RDB-fallback check), report back to confirm before Milestone 11's plan (Pub/Sub) is written.
