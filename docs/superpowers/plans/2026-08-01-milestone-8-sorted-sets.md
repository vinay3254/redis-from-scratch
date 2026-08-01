# Redis Clone — Milestone 8: Sorted Sets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.
>
> **Note:** This milestone's code already exists (commit `62094d1`). This plan documents what it must deliver so the implementation can be verified against it, rather than rewritten from scratch. This is the milestone that most needs careful scrutiny: the skip list is hand-rolled with raw pointers and `unsafe` blocks, which is exactly where subtle bugs hide.

**Goal:** `ZADD`, `ZRANGE` (with optional `WITHSCORES`), `ZSCORE` operating on a new `Value::ZSet` variant, backed by a hand-rolled skip list ordered by (score, member) so range queries are efficient without a full sort on every read.

**Architecture:** `ZSet` (in `db.rs`) pairs a `HashMap<Vec<u8>, f64>` (member → score, for O(1) `ZSCORE`) with a `SkipList` (`src/skiplist.rs`, member ordering, for O(log n) insert/remove and O(range) `ZRANGE`). `ZADD` on an existing member removes the old `(score, member)` node from the skip list before inserting the new one — a skip list is ordered by key, so changing a member's score means its position has to move, not just its stored value.

**Tech Stack:** Hand-rolled skip list using raw pointers (`*mut Node`) and `unsafe` — no `unsafe`-free alternative (like `Rc<RefCell<>>`) was used, which is a legitimate but riskier choice for a learning project. `MAX_LEVEL` is fixed at 16, `random_level()` uses a simple xorshift-style PRNG (`rand_simple`), not a crate.

## Global Constraints

- Language: Rust, no external crates — including no `rand` crate; the PRNG is hand-rolled.
- `ZADD key score1 member1 score2 member2 ...` returns the count of *newly added* members (score updates to existing members don't count) — matches real Redis.
- Ordering is by score ascending; ties break by member bytes asc (lexicographic) — this tie-break rule is a design choice, not incidental, since two equal-score members need a deterministic order for `ZRANGE` to be stable.
- Skip-list memory is manually managed (`Box::into_raw`/`Box::from_raw`); a bug here is a memory-safety bug (use-after-free, double-free, leak), not just a wrong-answer bug — treat any changes to `skiplist.rs` with extra suspicion and prefer stress-testing over code reading alone.
- Wrong-type key access returns `WRONGTYPE`.
- Code has no comments.

---

### Task 1: Skip list correctness and stress testing

**Files:**
- Modify: `src/skiplist.rs` (review only, no changes expected)

**Interfaces:**
- Produces: `SkipList::new() -> Self`, `insert(&mut self, score: f64, member: Vec<u8>) -> bool`, `remove(&mut self, score: f64, member: &[u8]) -> bool`, `get_range(&self, start: usize, stop: usize) -> Vec<(Vec<u8>, f64)>`, `len(&self) -> usize`. Also implements `Drop` (must free every node, no leaks) and `Clone` (must produce an independent copy, not aliased raw pointers).

- [x] **Step 1: Confirm insert doesn't allow duplicate (score, member) pairs**

Read `src/skiplist.rs`. Confirm `insert` checks whether the immediate successor at level 0 already matches `(score, member)` before creating a new node:

```rust
unsafe {
    let next = (&(*curr).forward)[0];
    if !next.is_null() && (&(*next)).score == score && (&(*next)).member == member {
        return false;
    }
}
```

This prevents `ZADD` from creating two nodes for the same member if called twice with the same score (the `Db::zadd` layer in Task 2 handles the *changed*-score case by calling `remove` first, but this guard protects the skip list itself against direct duplicate inserts).

- [x] **Step 2: Confirm remove correctly relinks every level and frees the node exactly once**

Confirm the `remove` function's relinking loop only rewrites `forward[i]` on levels where `update[i]`'s forward pointer actually pointed at the removed node (not unconditionally, which would corrupt unrelated chains), and that `Box::from_raw(target)` is called exactly once per removed node.

- [x] **Step 3: Confirm Drop frees every node without double-freeing the head**

```rust
impl Drop for SkipList {
    fn drop(&mut self) {
        let mut curr = unsafe { (&(*self.head).forward)[0] };
        while !curr.is_null() {
            let next = unsafe { (&(*curr).forward)[0] };
            unsafe { let _ = Box::from_raw(curr); }
            curr = next;
        }
        unsafe { let _ = Box::from_raw(self.head); }
    }
}
```

Walks the level-0 chain (which threads through every node regardless of its height) freeing each one, then frees the sentinel head separately — confirm this ordering (data nodes first, then head) is what's there.

- [x] **Step 4: Run the existing unit test, then stress-test with more data than the built-in test covers**

```bash
cargo test skiplist::tests
```

Expected: `test_skiplist_basic` passes.

Then run a heavier manual stress check to catch anything the small built-in test wouldn't (e.g. `MAX_LEVEL` handling at scale, level promotion under many inserts). Add a temporary test (do not commit it if it passes cleanly — this step is exploratory verification, not a permanent addition unless it reveals something):

```rust
#[test]
fn test_skiplist_stress() {
    let mut sl = SkipList::new();
    for i in 0..2000 {
        sl.insert(i as f64, format!("m{}", i).into_bytes());
    }
    assert_eq!(sl.len(), 2000);
    let all = sl.get_range(0, 1999);
    assert_eq!(all.len(), 2000);
    for i in 0..1999 {
        assert!(all[i].1 <= all[i + 1].1);
    }
    for i in (0..2000).step_by(2) {
        assert!(sl.remove(i as f64, format!("m{}", i).into_bytes().as_slice()));
    }
    assert_eq!(sl.len(), 1000);
}
```

Run it with `cargo test test_skiplist_stress -- --nocapture`, then run the full suite once more (`cargo test`) to make sure nothing else regressed. If it passes, this confirms the skip list holds up well beyond the 3-node happy path the shipped test covers.

---

### Task 2: ZSet storage in `Db`

**Files:**
- Modify: `src/db.rs`

**Interfaces:**
- Produces: `pub struct ZSet { pub dict: HashMap<Vec<u8>, f64>, pub skiplist: SkipList }`, `Value::ZSet(ZSet)`, `Db::zadd(&mut self, key: &[u8], pairs: &[(f64, Vec<u8>)]) -> Result<usize, ()>`, `Db::zscore(&mut self, key: &[u8], member: &[u8]) -> Result<Option<f64>, ()>`, `Db::zrange(&mut self, key: &[u8], start: i64, stop: i64) -> Result<Vec<(Vec<u8>, f64)>, ()>`.

- [x] **Step 1: Confirm zadd handles both new members and score updates**

```rust
pub fn zadd(&mut self, key: &[u8], pairs: &[(f64, Vec<u8>)]) -> Result<usize, ()> {
    self.check_expired(key);
    let zset = match self.entries.get_mut(key) {
        Some(Value::ZSet(z)) => z,
        Some(_) => return Err(()),
        None => {
            self.entries.insert(key.to_vec(), Value::ZSet(ZSet::new()));
            match self.entries.get_mut(key) {
                Some(Value::ZSet(z)) => z,
                _ => unreachable!(),
            }
        }
    };
    let mut added = 0;
    for (score, member) in pairs {
        if let Some(&old_score) = zset.dict.get(member) {
            zset.skiplist.remove(old_score, member);
            zset.dict.insert(member.clone(), *score);
            zset.skiplist.insert(*score, member.clone());
        } else {
            zset.dict.insert(member.clone(), *score);
            zset.skiplist.insert(*score, member.clone());
            added += 1;
        }
    }
    Ok(added)
}
```

The critical detail: when a member's score changes, the *old* `(old_score, member)` node must be removed from the skip list before the *new* `(score, member)` node is inserted — otherwise the skip list would end up with two entries for the same member (one stale). Confirm this remove-then-insert ordering is present, not just a blind insert.

- [x] **Step 2: Confirm zrange reuses normalize_indices from Milestone 5**

```rust
pub fn zrange(&mut self, key: &[u8], start: i64, stop: i64) -> Result<Vec<(Vec<u8>, f64)>, ()> {
    self.check_expired(key);
    match self.entries.get(key) {
        Some(Value::ZSet(z)) => match normalize_indices(z.skiplist.len(), start, stop) {
            Some((s, e)) => Ok(z.skiplist.get_range(s, e)),
            None => Ok(Vec::new()),
        },
        Some(_) => Err(()),
        None => Ok(Vec::new()),
    }
}
```

- [x] **Step 3: Run unit tests**

```bash
cargo test db::tests::test_zset_operations
```

Expected: pass.

---

### Task 3: ZADD/ZRANGE/ZSCORE commands + manual verification

**Files:**
- Modify: `src/commands/zset.rs`
- Modify: `src/commands/mod.rs` (wire into dispatch match)

**Interfaces:**
- Produces: `pub fn zadd(db: &mut Db, args: &[Vec<u8>]) -> RespFrame`, `zscore`, `zrange`.

- [x] **Step 1: Confirm ZADD parses scores as floats and validates pair count**

```rust
pub fn zadd(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return RespFrame::Error("ERR wrong number of arguments for 'zadd' command".into());
    }
    let key = &args[0];
    let mut pairs = Vec::with_capacity((args.len() - 1) / 2);
    for i in (1..args.len()).step_by(2) {
        let score: f64 = match std::str::from_utf8(&args[i]).ok().and_then(|s| s.parse().ok()) {
            Some(s) => s,
            None => return RespFrame::Error("ERR value is not a valid float".into()),
        };
        pairs.push((score, args[i + 1].clone()));
    }
    match db.zadd(key, &pairs) {
        Ok(count) => RespFrame::Integer(count as i64),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}
```

- [x] **Step 2: Confirm ZRANGE's optional WITHSCORES flag**

```rust
let with_scores = if args.len() == 4 {
    match std::str::from_utf8(&args[3]) {
        Ok(s) if s.to_uppercase() == "WITHSCORES" => true,
        _ => return RespFrame::Error("ERR syntax error".into()),
    }
} else {
    false
};
```

A 4th argument that isn't (case-insensitively) `WITHSCORES` is a syntax error, not silently ignored.

- [x] **Step 3: Confirm ZSCORE serializes the float as a bulk string**

```rust
pub fn zscore(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'zscore' command".into());
    }
    match db.zscore(&args[0], &args[1]) {
        Ok(Some(score)) => RespFrame::BulkString(Some(score.to_string().into_bytes())),
        Ok(None) => RespFrame::BulkString(None),
        Err(_) => RespFrame::Error(WRONG_TYPE_ERR.into()),
    }
}
```

Real Redis also returns scores as strings (RESP2 has no float type), so `score.to_string()` is the right approach, not a shortcut.

- [x] **Step 4: Run unit tests**

```bash
cargo test commands::tests::test_zset_commands
```

Expected: pass.

- [x] **Step 5: Manually verify ordering, WITHSCORES, and score updates**

```bash
redis-cli -p 6380 ZADD myzset 3 c 1 a 2 b
redis-cli -p 6380 ZRANGE myzset 0 -1
redis-cli -p 6380 ZRANGE myzset 0 -1 WITHSCORES
redis-cli -p 6380 ZSCORE myzset b
redis-cli -p 6380 ZADD myzset 0.5 c
redis-cli -p 6380 ZRANGE myzset 0 -1
```

Expected: despite insertion order `c, a, b`, `ZRANGE 0 -1` returns `a b c` (sorted by score). `WITHSCORES` interleaves member/score pairs. After re-scoring `c` to `0.5`, `ZRANGE` returns `c a b` (c moved to the front) — this specifically exercises the remove-then-reinsert path from Task 2 Step 1.

- [x] **Step 6: Manually verify ZADD's new-vs-updated return count**

```bash
redis-cli -p 6380 ZADD myzset 99 c 100 newmember
```

Expected: returns `1` — `c` already existed (score update, not counted), `newmember` is new (counted).

---

## Milestone 8 Done

Once Task 3's manual verification passes, report back to confirm before Milestone 9's plan (RDB persistence) is written.
