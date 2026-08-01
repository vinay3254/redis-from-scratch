# Redis Clone — Milestone 5: Lists Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Note:** This milestone's code already exists (commit `1442a95`). This plan documents what it must deliver so the implementation can be verified against it, rather than rewritten from scratch.

**Goal:** `LPUSH`, `RPUSH`, `LPOP`, `RPOP`, `LRANGE` operating on a new `Value::List` variant, with real Redis's push/pop/range semantics (including negative indices).

**Architecture:** `Value` gains a `List(VecDeque<Vec<u8>>)` variant. `VecDeque` gives O(1) push/pop at both ends, which is what `LPUSH`/`RPUSH`/`LPOP`/`RPOP` need. A shared `normalize_indices` helper in `db.rs` converts Redis-style possibly-negative `start`/`stop` into a concrete `(usize, usize)` range (or `None` if the range is empty), reused by `LRANGE` here and `ZRANGE` in Milestone 8.

**Tech Stack:** `std::collections::VecDeque`.

## Global Constraints

- Language: Rust, no external crates.
- `LPUSH key v1 v2 v3` pushes each value to the head *in argument order*, meaning the final order is `v3, v2, v1, ...` at the front (each push puts the new value before the previous one) — this matches real Redis, not a naive "prepend the whole list" semantic.
- Operating on a key that holds a non-list `Value` returns `WRONGTYPE`, not a panic or silent overwrite.
- A list that becomes empty (last element popped) removes the key entirely, matching real Redis (`LLEN` on a nonexistent key is `0`, not an empty list object).
- Code has no comments.

---

### Task 1: List storage and operations in `Db`

**Files:**
- Modify: `src/db.rs`

**Interfaces:**
- Produces: `Value::List(VecDeque<Vec<u8>>)`, `Db::lpush(&mut self, key: &[u8], elements: &[Vec<u8>]) -> Result<usize, ()>`, `Db::rpush(...)`, `Db::lpop(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, ()>`, `Db::rpop(...)`, `Db::lrange(&mut self, key: &[u8], start: i64, stop: i64) -> Result<Vec<Vec<u8>>, ()>`, `normalize_indices(len: usize, start: i64, stop: i64) -> Option<(usize, usize)>` (free function).
- `Err(())` signals WRONGTYPE to the caller (the command layer converts it to the actual RESP error).

- [ ] **Step 1: Confirm normalize_indices handles negative indices and edge cases**

```rust
fn normalize_indices(len: usize, start: i64, stop: i64) -> Option<(usize, usize)> {
    if len == 0 {
        return None;
    }
    let l = len as i64;
    let mut s = if start < 0 { l + start } else { start };
    let mut e = if stop < 0 { l + stop } else { stop };

    if s < 0 {
        s = 0;
    }
    if e < 0 {
        return None;
    }

    if s >= l {
        return None;
    }
    if e >= l {
        e = l - 1;
    }

    if s > e {
        return None;
    }

    Some((s as usize, e as usize))
}
```

Trace through `normalize_indices(3, 0, -1)` by hand: `l=3`, `s=0`, `e = 3 + (-1) = 2`, no clamping needed, returns `Some((0, 2))` — the whole list, which is the most common `LRANGE key 0 -1` call.

- [ ] **Step 2: Confirm lpush/rpush create-if-missing and reject wrong type**

```rust
pub fn lpush(&mut self, key: &[u8], elements: &[Vec<u8>]) -> Result<usize, ()> {
    self.check_expired(key);
    let list = match self.entries.get_mut(key) {
        Some(Value::List(l)) => l,
        Some(_) => return Err(()),
        None => {
            self.entries.insert(key.to_vec(), Value::List(VecDeque::new()));
            match self.entries.get_mut(key) {
                Some(Value::List(l)) => l,
                _ => unreachable!(),
            }
        }
    };
    for elem in elements {
        list.push_front(elem.clone());
    }
    Ok(list.len())
}
```

`rpush` is identical except `push_back`. Confirm both exist with this shape.

- [ ] **Step 3: Confirm lpop/rpop clean up empty lists**

```rust
pub fn lpop(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, ()> {
    self.check_expired(key);
    match self.entries.get_mut(key) {
        Some(Value::List(l)) => {
            let item = l.pop_front();
            if l.is_empty() {
                self.entries.remove(key);
                self.expirations.remove(key);
            }
            Ok(item)
        }
        Some(_) => Err(()),
        None => Ok(None),
    }
}
```

Confirm `rpop` mirrors this with `pop_back`, and that popping from a nonexistent key returns `Ok(None)` (not an error) — only a *wrong-typed* key is an error.

- [ ] **Step 4: Run unit tests**

```bash
cargo test db::tests::test_list_operations
```

Expected: pass, covering push order, `lrange`, and pop-to-empty behavior.

---

### Task 2: LPUSH/RPUSH/LPOP/RPOP/LRANGE commands + manual verification

**Files:**
- Modify: `src/commands/list.rs`
- Modify: `src/commands/mod.rs` (wire into dispatch match)

**Interfaces:**
- Produces: `pub fn lpush(db: &mut Db, args: &[Vec<u8>]) -> RespFrame`, `rpush`, `lpop`, `rpop`, `lrange`.

- [ ] **Step 1: Confirm arity checks and WRONGTYPE mapping**

```rust
const WRONG_TYPE_ERR: &str =
    "WRONGTYPE Operation against a key holding the wrong kind of value";

pub fn lpush(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'lpush' command".into());
    }
    match db.lpush(&args[0], &args[1..]) {
        Ok(len) => RespFrame::Integer(len as i64),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}
```

`lpush`/`rpush` require at least 2 args (key + one value); `lpop`/`rpop` require exactly 1 (just the key); `lrange` requires exactly 3 (key, start, stop) and returns `ERR value is not an integer or out of range` if either bound fails to parse as `i64`.

- [ ] **Step 2: Confirm LRANGE serializes to a RESP array of bulk strings**

```rust
pub fn lrange(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    // ...arity + parse checks...
    match db.lrange(&args[0], start, stop) {
        Ok(elements) => {
            let frames = elements.into_iter().map(|e| RespFrame::BulkString(Some(e))).collect();
            RespFrame::Array(Some(frames))
        }
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}
```

- [ ] **Step 3: Run unit tests**

```bash
cargo test commands::tests::test_list_commands
```

Expected: pass.

- [ ] **Step 4: Manually verify push order, pop, range, and negative indices**

```bash
redis-cli -p 6380 LPUSH mylist a b c
redis-cli -p 6380 RPUSH mylist x y
redis-cli -p 6380 LRANGE mylist 0 -1
redis-cli -p 6380 LRANGE mylist -2 -1
redis-cli -p 6380 LPOP mylist
redis-cli -p 6380 RPOP mylist
redis-cli -p 6380 LRANGE mylist 0 -1
```

Expected: after `LPUSH a b c` then `RPUSH x y`, the list is `c b a x y` (LPUSH reverses insertion order at the head). `LRANGE 0 -1` shows all 5; `LRANGE -2 -1` shows the last two (`x y`). After one `LPOP` and one `RPOP`, the list is `b a x`.

- [ ] **Step 5: Manually verify WRONGTYPE and empty-list cleanup**

```bash
redis-cli -p 6380 SET strkey hello
redis-cli -p 6380 LPUSH strkey x
redis-cli -p 6380 RPUSH emptylist z
redis-cli -p 6380 LPOP emptylist
redis-cli -p 6380 EXISTS emptylist
```

Expected: `WRONGTYPE` error on `LPUSH strkey x`; after popping `emptylist` down to zero elements, `EXISTS emptylist` returns `0` (key was removed, not left behind as an empty list).

---

## Milestone 5 Done

Once Task 2's manual verification passes, report back to confirm before Milestone 6's plan (Hashes) is written.
