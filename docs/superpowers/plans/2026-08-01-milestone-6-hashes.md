# Redis Clone — Milestone 6: Hashes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Note:** This milestone's code already exists (commit `aebd2fe`). This plan documents what it must deliver so the implementation can be verified against it, rather than rewritten from scratch.

**Goal:** `HSET`, `HGET`, `HGETALL`, `HDEL` operating on a new `Value::Hash` variant.

**Architecture:** `Value` gains a `Hash(HashMap<Vec<u8>, Vec<u8>>)` variant — a nested map keyed by field name. `HSET` accepts a variadic list of field/value pairs (real Redis behavior since Redis 4.0), so the command layer groups the flat argument list into pairs before calling into `Db`.

**Tech Stack:** `std::collections::HashMap`.

## Global Constraints

- Language: Rust, no external crates.
- `HSET key f1 v1 f2 v2 ...` returns the count of *new* fields created (fields that already existed and were merely overwritten don't count) — matches real Redis.
- A hash that becomes empty (last field deleted via `HDEL`) removes the key entirely, same rule as lists in Milestone 5.
- Wrong-type key access returns `WRONGTYPE`.
- Code has no comments.

---

### Task 1: Hash storage and operations in `Db`

**Files:**
- Modify: `src/db.rs`

**Interfaces:**
- Produces: `Value::Hash(HashMap<Vec<u8>, Vec<u8>>)`, `Db::hset(&mut self, key: &[u8], pairs: &[(Vec<u8>, Vec<u8>)]) -> Result<usize, ()>`, `Db::hget(&mut self, key: &[u8], field: &[u8]) -> Result<Option<Vec<u8>>, ()>`, `Db::hgetall(&mut self, key: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>, ()>`, `Db::hdel(&mut self, key: &[u8], fields: &[Vec<u8>]) -> Result<usize, ()>`.

- [ ] **Step 1: Confirm hset counts only newly-created fields**

```rust
pub fn hset(&mut self, key: &[u8], pairs: &[(Vec<u8>, Vec<u8>)]) -> Result<usize, ()> {
    self.check_expired(key);
    let hash = match self.entries.get_mut(key) {
        Some(Value::Hash(h)) => h,
        Some(_) => return Err(()),
        None => {
            self.entries.insert(key.to_vec(), Value::Hash(HashMap::new()));
            match self.entries.get_mut(key) {
                Some(Value::Hash(h)) => h,
                _ => unreachable!(),
            }
        }
    };
    let mut created = 0;
    for (field, val) in pairs {
        if hash.insert(field.clone(), val.clone()).is_none() {
            created += 1;
        }
    }
    Ok(created)
}
```

The `HashMap::insert` return value (`None` means no prior entry) is exactly the signal needed to distinguish "created" from "overwritten" — confirm this logic is present, not just always incrementing `created`.

- [ ] **Step 2: Confirm hget/hgetall/hdel, and hdel's empty-hash cleanup**

```rust
pub fn hdel(&mut self, key: &[u8], fields: &[Vec<u8>]) -> Result<usize, ()> {
    self.check_expired(key);
    match self.entries.get_mut(key) {
        Some(Value::Hash(h)) => {
            let mut removed = 0;
            for field in fields {
                if h.remove(field).is_some() {
                    removed += 1;
                }
            }
            if h.is_empty() {
                self.entries.remove(key);
                self.expirations.remove(key);
            }
            Ok(removed)
        }
        Some(_) => Err(()),
        None => Ok(0),
    }
}
```

`hget`/`hgetall` on a missing key return `Ok(None)`/`Ok(Vec::new())` respectively (empty, not an error) — only wrong-typed keys are errors.

- [ ] **Step 3: Run unit tests**

```bash
cargo test db::tests::test_hash_operations
```

Expected: pass.

---

### Task 2: HSET/HGET/HGETALL/HDEL commands + manual verification

**Files:**
- Modify: `src/commands/hash.rs`
- Modify: `src/commands/mod.rs` (wire into dispatch match)

**Interfaces:**
- Produces: `pub fn hset(db: &mut Db, args: &[Vec<u8>]) -> RespFrame`, `hget`, `hgetall`, `hdel`.

- [ ] **Step 1: Confirm HSET's arity check requires an even number of field/value args**

```rust
pub fn hset(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return RespFrame::Error("ERR wrong number of arguments for 'hset' command".into());
    }
    let key = &args[0];
    let mut pairs = Vec::with_capacity((args.len() - 1) / 2);
    for i in (1..args.len()).step_by(2) {
        pairs.push((args[i].clone(), args[i + 1].clone()));
    }
    match db.hset(key, &pairs) {
        Ok(count) => RespFrame::Integer(count as i64),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}
```

`args.len() < 3` rejects `HSET key` (no field/value pair at all); the modulo check rejects an odd count like `HSET key f1 v1 f2` (dangling field with no value).

- [ ] **Step 2: Confirm HGETALL flattens pairs into a single RESP array**

```rust
pub fn hgetall(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 1 {
        return RespFrame::Error("ERR wrong number of arguments for 'hgetall' command".into());
    }
    match db.hgetall(&args[0]) {
        Ok(pairs) => {
            let mut frames = Vec::with_capacity(pairs.len() * 2);
            for (field, val) in pairs {
                frames.push(RespFrame::BulkString(Some(field)));
                frames.push(RespFrame::BulkString(Some(val)));
            }
            RespFrame::Array(Some(frames))
        }
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}
```

This matches real Redis's flat `[field1, value1, field2, value2, ...]` wire format for `HGETALL`.

- [ ] **Step 3: Run unit tests**

```bash
cargo test commands::tests::test_hash_commands
```

Expected: pass.

- [ ] **Step 4: Manually verify field creation count, overwrite, and HGETALL**

```bash
redis-cli -p 6380 HSET myhash f1 v1 f2 v2
redis-cli -p 6380 HSET myhash f1 newval f3 v3
redis-cli -p 6380 HGET myhash f1
redis-cli -p 6380 HGETALL myhash
```

Expected: first `HSET` returns `2` (both new); second returns `1` (only `f3` is new, `f1` was overwritten); `HGET f1` shows `newval`; `HGETALL` shows all three fields with current values.

- [ ] **Step 5: Manually verify HDEL empty-hash cleanup and arity errors**

```bash
redis-cli -p 6380 HDEL myhash f1 f2 f3
redis-cli -p 6380 EXISTS myhash
redis-cli -p 6380 HSET onlykey
```

Expected: `HDEL` removes all 3 fields (returns `3`), `EXISTS myhash` then returns `0` (key removed since the hash became empty), and `HSET onlykey` with no field/value pairs is an arity error.

---

## Milestone 6 Done

Once Task 2's manual verification passes, report back to confirm before Milestone 7's plan (Sets) is written.
