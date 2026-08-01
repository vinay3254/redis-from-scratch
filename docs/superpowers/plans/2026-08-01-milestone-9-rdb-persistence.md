# Redis Clone — Milestone 9: RDB Persistence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.
>
> **Note:** This milestone's code already exists (commit `481e6a2`). This plan documents what it must deliver so the implementation can be verified against it, rather than rewritten from scratch.

**Goal:** `SAVE` (blocking snapshot) and `BGSAVE` (snapshot on a background thread) write the entire keyspace to `dump.rdb` in a custom binary format; the server loads that file on boot if present.

**Architecture:** A tagged binary format: a fixed header (`REDIS_CLONE_RDB_V1`), then one record per key (a type tag byte, the key bytes, an optional expiry, then type-specific payload), terminated by an `EOF_TAG` sentinel byte. This is **not** byte-compatible with real Redis's RDB format — it's a from-scratch equivalent serving the same purpose (point-in-time binary snapshot), which is what the original spec called for. `Db::snapshot_entries()` produces a consistent view (skipping already-expired keys, converting each key's absolute `Instant` expiry into a `Duration` relative to "now") that `dump_db` writes to disk.

**Tech Stack:** `std::fs::File`, `std::io::{BufReader, BufWriter}`, big-endian integers via `to_be_bytes`/`from_be_bytes` (arbitrary choice, consistent throughout).

## Global Constraints

- Language: Rust, no external crates — no `serde`, no `bincode`. The format is hand-rolled length-prefixed binary.
- Expiry is stored as a **relative** millisecond duration from the moment of the snapshot, not an absolute timestamp — because `Instant` (used internally) has no meaningful cross-process/cross-restart representation; only `Duration` survives a restart correctly.
- An unknown type tag during load is a hard error (`ErrorKind::InvalidData`), not silently skipped — a corrupt or foreign file should fail loudly rather than partially load.
- **This milestone alone is not safe to test in isolation against real usage** — see Milestone 10's plan for the interaction between RDB and AOF on boot, which was the source of a real bug (fixed in commit `c552716`, see below).
- Code has no comments.

---

### Task 1: Snapshot format — write path

**Files:**
- Modify: `src/persistence/rdb.rs`
- Modify: `src/db.rs` (for `snapshot_entries`)

**Interfaces:**
- Produces: `pub fn dump_db(db: &Db, path: &str) -> std::io::Result<()>`, `Db::snapshot_entries(&self) -> Vec<(Vec<u8>, Value, Option<Duration>)>`.

- [x] **Step 1: Confirm snapshot_entries skips expired keys and converts to relative duration**

```rust
pub fn snapshot_entries(&self) -> Vec<(Vec<u8>, Value, Option<Duration>)> {
    let now = Instant::now();
    let mut snapshot = Vec::new();
    for (k, v) in &self.entries {
        if let Some(&expire_at) = self.expirations.get(k) {
            if now >= expire_at {
                continue;
            }
            let remaining = expire_at - now;
            snapshot.push((k.clone(), v.clone(), Some(remaining)));
        } else {
            snapshot.push((k.clone(), v.clone(), None));
        }
    }
    snapshot
}
```

A key whose expiry has already passed is silently excluded (it's logically gone; there's no reason to persist it) rather than persisted with `Some(Duration::ZERO)` or similar, which would be a subtly different (and wrong) signal on reload.

- [x] **Step 2: Confirm dump_db writes one tagged record per key, per Value variant**

Read `src/persistence/rdb.rs`. Confirm every `Value` variant (`String`, `List`, `Hash`, `Set`, `ZSet`) has a corresponding write arm using a distinct type tag (`TYPE_STRING = 1` through `TYPE_ZSET = 5`), each writing: tag byte → key (length-prefixed) → expiry flag+value → type-specific payload (e.g. list: count then each element; hash: count then field/value pairs). Confirm the file ends with a single `EOF_TAG` (`0xFF`) byte after the last record.

- [x] **Step 3: Run the round-trip unit test**

```bash
cargo test persistence::rdb::tests::test_rdb_roundtrip
```

Expected: pass — this test already covers one key of each type plus an expiring string, dumped and reloaded.

---

### Task 2: Snapshot format — load path, plus SAVE/BGSAVE commands

**Files:**
- Modify: `src/persistence/rdb.rs`
- Modify: `src/commands/generic.rs`
- Modify: `src/commands/mod.rs` (wire `BGSAVE` — note it's handled specially in `dispatch`, not through `dispatch_mutating`, since it needs the `Arc<Mutex<Db>>` itself, not a locked guard)

**Interfaces:**
- Produces: `pub fn load_db(path: &str) -> std::io::Result<Db>`, `pub fn save(db: &Db, args: &[Vec<u8>]) -> RespFrame`, `pub fn bgsave(db: Arc<Mutex<Db>>, args: &[Vec<u8>]) -> RespFrame`.

- [x] **Step 1: Confirm load_db validates the header before trusting the rest of the file**

```rust
let mut header = [0u8; 18];
r.read_exact(&mut header)?;
if header != RDB_HEADER {
    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid RDB header"));
}
```

`RDB_HEADER` is `b"REDIS_CLONE_RDB_V1"` (18 bytes) — confirm the buffer size (`18`) actually matches the header's byte length; a mismatch here would silently misparse every file.

- [x] **Step 2: Confirm load_db re-applies expiry via set_expire after inserting the value**

```rust
if let Some(dur) = expiry {
    db.set_expire(&key, dur);
}
```

This runs *after* the type-specific `db.set(key.clone(), Value::...)` call for that record — confirm the ordering, since `set_expire` requires the key to already exist in `entries` (it returns `false` otherwise, per Milestone 4's `Db::set_expire`).

- [x] **Step 3: Confirm SAVE is synchronous and BGSAVE spawns a thread**

```rust
pub fn save(db: &Db, args: &[Vec<u8>]) -> RespFrame {
    if !args.is_empty() {
        return RespFrame::Error("ERR wrong number of arguments for 'save' command".into());
    }
    match dump_db(db, "dump.rdb") {
        Ok(_) => RespFrame::SimpleString("OK".into()),
        Err(e) => RespFrame::Error(format!("ERR failed to save snapshot: {}", e)),
    }
}

pub fn bgsave(db: Arc<Mutex<Db>>, args: &[Vec<u8>]) -> RespFrame {
    if !args.is_empty() {
        return RespFrame::Error("ERR wrong number of arguments for 'bgsave' command".into());
    }
    thread::spawn(move || {
        let db_guard = db.lock().unwrap();
        let _ = dump_db(&db_guard, "dump.rdb");
    });
    RespFrame::SimpleString("Background saving started".into())
}
```

`bgsave` returns immediately with `"Background saving started"` without waiting for the spawned thread — confirm the response doesn't accidentally block on the thread (e.g. via `.join()`), which would defeat the point of it being "background."

- [x] **Step 4: Run unit tests**

```bash
cargo test persistence::rdb::tests
```

Expected: pass.

- [x] **Step 5: Manually verify SAVE + restart round-trips all 5 data types and expiry**

```bash
redis-cli -p 6380 SET str_key hello
redis-cli -p 6380 RPUSH list_key a b c
redis-cli -p 6380 HSET hash_key f1 v1
redis-cli -p 6380 SADD set_key m1 m2
redis-cli -p 6380 ZADD zset_key 1.5 z1
redis-cli -p 6380 EXPIRE str_key 300
redis-cli -p 6380 SAVE
```

Stop the server, restart it (`cargo run` again), then:

```bash
redis-cli -p 6380 GET str_key
redis-cli -p 6380 TTL str_key
redis-cli -p 6380 LRANGE list_key 0 -1
redis-cli -p 6380 HGET hash_key f1
redis-cli -p 6380 SISMEMBER set_key m1
redis-cli -p 6380 ZSCORE zset_key z1
```

Expected: every value survives the restart, and `TTL str_key` shows a number close to (but slightly less than) `300` — proving the expiry was correctly converted to a relative duration on save and reconstituted on load, not reset or lost.

- [x] **Step 6: Manually verify BGSAVE doesn't block the client**

```bash
redis-cli -p 6380 BGSAVE
```

Expected: immediate `Background saving started` reply, not a multi-second pause even with a nontrivial keyspace.

---

## Milestone 9 Done

This milestone's boot-loading logic interacts directly with Milestone 10 (AOF) — do not consider persistence "done" until Milestone 10's plan is also verified, since a real bug was found and fixed exactly at that boundary (see that plan's notes).

Once Task 2's manual verification passes, report back to confirm before Milestone 10's plan (AOF persistence) is written.
