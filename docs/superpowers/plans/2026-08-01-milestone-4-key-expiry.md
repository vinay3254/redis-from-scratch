# Redis Clone — Milestone 4: Key Expiry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.
>
> **Note:** This milestone's code already exists (commit `b0429a3`). This plan documents what it must deliver so the implementation can be verified against it, rather than rewritten from scratch.

**Goal:** `EXPIRE`, `PEXPIRE`, and `TTL` commands, with both passive expiration (checked on every key access) and active expiration (a background sweep that removes expired keys even if nobody touches them).

**Architecture:** `Db` (`src/db.rs`) gains an `expirations: HashMap<Vec<u8>, Instant>` map alongside `entries`. Every read/write method (`get`, `set`, `del`, `exists`, and all the data-type-specific accessors added in later milestones) calls a private `check_expired` first, which lazily removes the key if its `Instant` has passed — this is the passive path. Separately, `main.rs` spawns a background thread that sleeps 100ms and calls `Db::purge_expired()` in a loop — this is the active path, matching real Redis's default active-expire-cycle frequency of 10Hz (100ms).

**Tech Stack:** `std::time::{Duration, Instant}`. No wall-clock (`SystemTime`) — `Instant` is monotonic, which is what you want for a duration-based TTL, though it means expirations don't survive being written as absolute epoch times without conversion (see Milestone 9 for how RDB persists them as millisecond durations instead).

## Global Constraints

- Language: Rust, no external crates.
- `TTL` returns `-2` if the key doesn't exist (or has just expired), `-1` if the key exists but has no expiration, and the remaining whole seconds otherwise (matches real Redis's three-way return).
- Active sweep runs on its own thread against the same `Arc<Mutex<Db>>` everything else uses — it takes the same global lock, so it can momentarily block command dispatch; that's an accepted tradeoff of the single-lock design.
- Code has no comments.

---

### Task 1: Passive expiration in `Db`

**Files:**
- Modify: `src/db.rs`

**Interfaces:**
- Produces: `Db::check_expired(&mut self, key: &[u8]) -> bool` (private), `Db::set_expire(&mut self, key: &[u8], duration: Duration) -> bool`, `Db::ttl(&mut self, key: &[u8]) -> i64`, `Db::purge_expired(&mut self) -> usize`.
- Consumes: `Db::entries` (Task in Milestone 3).

- [x] **Step 1: Confirm the expirations map and check_expired**

Read `src/db.rs`. Confirm:

```rust
pub struct Db {
    entries: HashMap<Vec<u8>, Value>,
    expirations: HashMap<Vec<u8>, Instant>,
}
```

and

```rust
fn check_expired(&mut self, key: &[u8]) -> bool {
    if let Some(&expire_at) = self.expirations.get(key) {
        if Instant::now() >= expire_at {
            self.entries.remove(key);
            self.expirations.remove(key);
            return true;
        }
    }
    false
}
```

Confirm every read path (`get`, `hget`, `lrange`, `sismember`, `zscore`, etc.) calls `check_expired` before looking at `entries`, and that `set` clears any prior expiration (`self.expirations.remove(&key)`) — otherwise a `SET` on an already-expiring key would incorrectly keep the old TTL.

- [x] **Step 2: Confirm set_expire and ttl**

```rust
pub fn set_expire(&mut self, key: &[u8], duration: Duration) -> bool {
    if self.check_expired(key) || !self.entries.contains_key(key) {
        return false;
    }
    self.expirations.insert(key.to_vec(), Instant::now() + duration);
    true
}

pub fn ttl(&mut self, key: &[u8]) -> i64 {
    if self.check_expired(key) || !self.entries.contains_key(key) {
        return -2;
    }
    match self.expirations.get(key) {
        Some(&expire_at) => {
            let now = Instant::now();
            if now >= expire_at {
                self.entries.remove(key);
                self.expirations.remove(key);
                -2
            } else {
                (expire_at - now).as_secs() as i64
            }
        }
        None => -1,
    }
}
```

`set_expire` returns `false` (matching Redis's `EXPIRE` returning `0`) when the key doesn't exist.

- [x] **Step 3: Confirm the active sweep**

```rust
pub fn purge_expired(&mut self) -> usize {
    let now = Instant::now();
    let expired_keys: Vec<Vec<u8>> = self
        .expirations
        .iter()
        .filter_map(|(k, &expire_at)| if now >= expire_at { Some(k.clone()) } else { None })
        .collect();

    let count = expired_keys.len();
    for key in expired_keys {
        self.entries.remove(&key);
        self.expirations.remove(&key);
    }
    count
}
```

This is a full-scan sweep (not real Redis's probabilistic sampling), which is fine at this scale — note it as a simplification, not a bug.

- [x] **Step 4: Run unit tests**

```bash
cargo test db::tests::test_expiry db::tests::test_purge_expired
```

Expected: both pass, including the sleep-then-assert-expired pattern.

---

### Task 2: EXPIRE/PEXPIRE/TTL commands + active sweep thread + manual verification

**Files:**
- Modify: `src/commands/generic.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Produces: `pub fn expire(db: &mut Db, args: &[Vec<u8>]) -> RespFrame`, `pub fn pexpire(...)`, `pub fn ttl(...)`.

- [x] **Step 1: Confirm the command handlers**

```rust
pub fn expire(db: &mut Db, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'expire' command".into());
    }
    let seconds: u64 = match std::str::from_utf8(&args[1]).ok().and_then(|s| s.parse().ok()) {
        Some(s) => s,
        None => return RespFrame::Error("ERR value is not an integer or out of range".into()),
    };
    let success = db.set_expire(&args[0], Duration::from_secs(seconds));
    RespFrame::Integer(if success { 1 } else { 0 })
}
```

`pexpire` is identical but parses milliseconds and calls `Duration::from_millis`. `ttl` is a thin wrapper returning `RespFrame::Integer(db.ttl(&args[0]))`.

- [x] **Step 2: Confirm the active-sweep thread in main()**

Read `src/main.rs`. Confirm:

```rust
let db_active_expire = Arc::clone(&db);
thread::spawn(move || loop {
    thread::sleep(Duration::from_millis(100));
    let mut db_guard = db_active_expire.lock().unwrap();
    db_guard.purge_expired();
});
```

spawned before the `TcpListener::bind` call, so it's running for the lifetime of the process.

- [x] **Step 3: Manually verify passive expiration**

```bash
redis-cli -p 6380 SET k v
redis-cli -p 6380 PEXPIRE k 200
redis-cli -p 6380 TTL k
# wait ~0.3s
redis-cli -p 6380 GET k
redis-cli -p 6380 EXISTS k
```

Expected: `TTL` shows `0` (rounds down from ~0.2s), `GET` after the wait returns `(nil)`, `EXISTS` returns `0`.

- [x] **Step 4: Manually verify active sweep (key removed without being accessed)**

There's no `DBSIZE`/`KEYS` command in this clone to observe internal key count directly, so verify indirectly: set a short-lived key, wait well past its expiry and past the 100ms sweep interval, then restart the server and confirm `dump.rdb`/AOF replay don't resurrect it (this is exercised concretely in Milestone 9's plan). For now, confirm via logs/behavior that a `GET` immediately after the sweep interval has elapsed returns `(nil)` without needing multiple probes — this alone confirms `check_expired` and `purge_expired` agree on the same expiry instant.

- [x] **Step 5: Manually verify TTL edge cases**

```bash
redis-cli -p 6380 SET permanent v
redis-cli -p 6380 TTL permanent
redis-cli -p 6380 TTL nonexistent
redis-cli -p 6380 EXPIRE nonexistent 10
```

Expected: `-1` (no expiry set), `-2` (key doesn't exist), `0` (EXPIRE on missing key fails).

---

## Milestone 4 Done

Once Task 2's manual verification passes, report back to confirm before Milestone 5's plan (Lists) is written.
