# Redis Clone — Milestone 7: Sets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.
>
> **Note:** This milestone's code already exists (commit `0f13bf6`). This plan documents what it must deliver so the implementation can be verified against it, rather than rewritten from scratch.

**Goal:** `SADD`, `SREM`, `SMEMBERS`, `SISMEMBER` operating on a new `Value::Set` variant.

**Architecture:** `Value` gains a `Set(HashSet<Vec<u8>>)` variant. Set membership operations map directly onto `HashSet`'s own `insert`/`remove`/`contains`, which already give the right idempotency (`SADD` of an existing member is a no-op, matching Redis's set semantics) without extra bookkeeping.

**Tech Stack:** `std::collections::HashSet`.

## Global Constraints

- Language: Rust, no external crates.
- `SADD` returns the count of members *actually added* (re-adding an existing member doesn't count) — mirrors `HashSet::insert`'s boolean return directly.
- Member order from `SMEMBERS` is unspecified (real Redis doesn't guarantee set order either) — don't test for a specific order, only for set membership.
- A set that becomes empty (last member removed via `SREM`) removes the key entirely.
- Wrong-type key access returns `WRONGTYPE`.
- Code has no comments.

---

### Task 1: Set storage and operations in `Db`

**Files:**
- Modify: `src/db.rs`

**Interfaces:**
- Produces: `Value::Set(HashSet<Vec<u8>>)`, `Db::sadd(&mut self, key: &[u8], members: &[Vec<u8>]) -> Result<usize, ()>`, `Db::srem(...)`, `Db::smembers(&mut self, key: &[u8]) -> Result<Vec<Vec<u8>>, ()>`, `Db::sismember(&mut self, key: &[u8], member: &[u8]) -> Result<bool, ()>`.

- [x] **Step 1: Confirm sadd uses HashSet::insert's boolean to count new members**

```rust
pub fn sadd(&mut self, key: &[u8], members: &[Vec<u8>]) -> Result<usize, ()> {
    self.check_expired(key);
    let set = match self.entries.get_mut(key) {
        Some(Value::Set(s)) => s,
        Some(_) => return Err(()),
        None => {
            self.entries.insert(key.to_vec(), Value::Set(HashSet::new()));
            match self.entries.get_mut(key) {
                Some(Value::Set(s)) => s,
                _ => unreachable!(),
            }
        }
    };
    let mut added = 0;
    for member in members {
        if set.insert(member.clone()) {
            added += 1;
        }
    }
    Ok(added)
}
```

- [x] **Step 2: Confirm srem's empty-set cleanup and sismember's boolean result**

```rust
pub fn sismember(&mut self, key: &[u8], member: &[u8]) -> Result<bool, ()> {
    self.check_expired(key);
    match self.entries.get(key) {
        Some(Value::Set(s)) => Ok(s.contains(member)),
        Some(_) => Err(()),
        None => Ok(false),
    }
}
```

`sismember` on a nonexistent key returns `Ok(false)` (not an error) — same "missing key is empty, wrong type is an error" pattern as every other data type.

- [x] **Step 3: Run unit tests**

```bash
cargo test db::tests::test_set_operations
```

Expected: pass.

---

### Task 2: SADD/SREM/SMEMBERS/SISMEMBER commands + manual verification

**Files:**
- Modify: `src/commands/set.rs`
- Modify: `src/commands/mod.rs` (wire into dispatch match)

**Interfaces:**
- Produces: `pub fn sadd(db: &mut Db, args: &[Vec<u8>]) -> RespFrame`, `srem`, `smembers`, `sismember`.

- [x] **Step 1: Confirm SISMEMBER maps bool to RESP integer 0/1**

```rust
pub fn sismember(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'sismember' command".into());
    }
    match db.sismember(&args[0], &args[1]) {
        Ok(true) => RespFrame::Integer(1),
        Ok(false) => RespFrame::Integer(0),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}
```

- [x] **Step 2: Confirm SADD/SREM require at least one member (arity ≥ 2 total args)**

```rust
pub fn sadd(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'sadd' command".into());
    }
    match db.sadd(&args[0], &args[1..]) {
        Ok(count) => RespFrame::Integer(count as i64),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}
```

- [x] **Step 3: Run unit tests**

```bash
cargo test commands::tests::test_set_commands
```

Expected: pass.

- [x] **Step 4: Manually verify add-count, duplicate no-op, membership, and removal**

```bash
redis-cli -p 6380 SADD myset a b c
redis-cli -p 6380 SADD myset a
redis-cli -p 6380 SISMEMBER myset a
redis-cli -p 6380 SISMEMBER myset zzz
redis-cli -p 6380 SMEMBERS myset
redis-cli -p 6380 SREM myset a b c
redis-cli -p 6380 EXISTS myset
```

Expected: first `SADD` returns `3`, second returns `0` (duplicate, no-op), `SISMEMBER a` is `1`, `SISMEMBER zzz` is `0`, `SMEMBERS` shows all 3 in unspecified order, and after removing all members `EXISTS myset` is `0`.

- [x] **Step 5: Manually verify WRONGTYPE**

```bash
redis-cli -p 6380 SET strkey v
redis-cli -p 6380 SADD strkey x
```

Expected: `WRONGTYPE` error.

---

## Milestone 7 Done

Once Task 2's manual verification passes, report back to confirm before Milestone 8's plan (Sorted Sets) is written.
