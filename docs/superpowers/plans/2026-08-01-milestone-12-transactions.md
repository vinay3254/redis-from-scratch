# Redis Clone — Milestone 12: Transactions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Note:** This milestone's code already exists (commit `26ed3d0`), and is the last of the 12 original milestones. This plan documents what it must deliver so the implementation can be verified against it, rather than rewritten from scratch.

**Goal:** `MULTI` (start queuing), `EXEC` (atomically run the queued commands), `DISCARD` (abandon the queue) — per-connection transaction state, atomic execution under a single lock acquisition.

**Architecture:** Each connection thread owns its own `in_transaction: bool` and `tx_queue: Vec<RespFrame>` (local variables in `handle_connection`, not shared state — transactions are inherently per-client). While `in_transaction` is true, every command except `MULTI`/`EXEC`/`DISCARD` is pushed onto `tx_queue` and immediately answered with `QUEUED`, without being executed. `EXEC` takes the queue, locks the shared `Db` *once* for the whole batch (via `commands::tx::exec`), and runs every queued command against that single locked guard — this single lock acquisition is what makes the batch atomic with respect to other connections (no other thread's command can interleave between two commands in the transaction).

**Tech Stack:** No new crates — this is pure control flow over the existing `RespFrame`/`Db`/dispatch machinery.

## Global Constraints

- Language: Rust, no external crates.
- Nesting `MULTI` inside an already-open transaction is an error (`ERR MULTI calls can not be nested`), not silently accepted or silently restarting the queue.
- `DISCARD`/`EXEC` without a preceding `MULTI` is an error (`ERR DISCARD without MULTI` / `ERR EXEC without MULTI`).
- `EXEC`'s reply is a single RESP array containing each queued command's individual reply, in order — matching real Redis's transaction reply shape.
- A command inside the transaction that fails (e.g. `WRONGTYPE`) does **not** abort the rest of the batch — each queued command's result (success or error) is collected independently, matching real Redis (transactions in Redis don't roll back on a per-command error; they're not that kind of transaction).
- Only successful write commands from inside the transaction are appended to the AOF (Milestone 10's rule applies per-command inside `EXEC`, not once for the whole batch) — see Milestone 10's plan, Task 2 Step 2, for why this lives as a second copy of the same gating logic rather than a shared helper.
- Code has no comments.

---

### Task 1: Transaction state machine in the connection loop

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `commands::tx::exec` (Task 2).
- Produces: per-connection `in_transaction: bool`, `tx_queue: Vec<RespFrame>` local state.

- [ ] **Step 1: Confirm MULTI rejects nesting and resets the queue**

```rust
if cmd_name == "MULTI" {
    if in_transaction {
        let err = RespFrame::Error("ERR MULTI calls can not be nested".into());
        let _ = stream.write_all(&err.serialize());
    } else {
        in_transaction = true;
        tx_queue.clear();
        let ok = RespFrame::SimpleString("OK".into());
        let _ = stream.write_all(&ok.serialize());
    }
}
```

`tx_queue.clear()` on a fresh `MULTI` is defensive (it should already be empty if the state machine is correct elsewhere) — confirm it's there anyway, since leftover queued commands from a prior mishandled sequence would otherwise silently execute inside a later transaction.

- [ ] **Step 2: Confirm DISCARD and the "no MULTI in progress" error cases for both DISCARD and EXEC**

```rust
} else if cmd_name == "DISCARD" {
    if !in_transaction {
        let err = RespFrame::Error("ERR DISCARD without MULTI".into());
        let _ = stream.write_all(&err.serialize());
    } else {
        in_transaction = false;
        tx_queue.clear();
        let ok = RespFrame::SimpleString("OK".into());
        let _ = stream.write_all(&ok.serialize());
    }
}
```

Confirm the equivalent `!in_transaction` guard exists for `EXEC` too (`ERR EXEC without MULTI`).

- [ ] **Step 3: Confirm any other command while in_transaction is queued, not executed**

```rust
} else if in_transaction {
    tx_queue.push(frame);
    let queued = RespFrame::SimpleString("QUEUED".into());
    if stream.write_all(&queued.serialize()).is_err() {
        // clean up and return
    }
}
```

This branch must come *before* the normal dispatch branch in the if/else chain (checked only after ruling out `MULTI`/`DISCARD`/`EXEC` specifically) — confirm the ordering, since if it came after dispatch, queued commands would be executed immediately instead of deferred.

- [ ] **Step 4: Confirm EXEC takes the queue, calls tx::exec once, and resets in_transaction before executing**

```rust
} else if cmd_name == "EXEC" {
    if !in_transaction {
        let err = RespFrame::Error("ERR EXEC without MULTI".into());
        let _ = stream.write_all(&err.serialize());
    } else {
        in_transaction = false;
        let queue = std::mem::take(&mut tx_queue);
        let response_frame = commands::tx::exec(queue, Arc::clone(&db), Some(Arc::clone(&pubsub)), Some(&aof));
        if stream.write_all(&response_frame.serialize()).is_err() {
            // clean up and return
        }
    }
}
```

`std::mem::take(&mut tx_queue)` empties the queue and hands ownership of its contents to `exec` in one step — confirm `in_transaction` is reset to `false` here (not left `true`, which would incorrectly keep queuing subsequent commands after the transaction already ran).

---

### Task 2: Atomic batch execution

**Files:**
- Modify: `src/commands/tx.rs`

**Interfaces:**
- Produces: `pub fn exec(tx_queue: Vec<RespFrame>, db: Arc<Mutex<Db>>, pubsub: Option<Arc<Mutex<PubSub>>>, aof: Option<&Aof>) -> RespFrame`.

- [ ] **Step 1: Confirm the DB lock is acquired exactly once, outside the per-command loop**

```rust
pub fn exec(tx_queue: Vec<RespFrame>, db: Arc<Mutex<Db>>, pubsub: Option<Arc<Mutex<PubSub>>>, aof: Option<&Aof>) -> RespFrame {
    let mut results = Vec::with_capacity(tx_queue.len());
    let mut db_guard = db.lock().unwrap();

    for frame in tx_queue {
        // ...
        let result = super::dispatch_mutating(frame.clone(), &mut db_guard);
        // ...
        results.push(result);
    }

    RespFrame::Array(Some(results))
}
```

`db.lock()` happens once before the loop, and every queued command reuses the same `db_guard` via `dispatch_mutating` (not `dispatch`, which would try to lock again and deadlock) — confirm this is the actual call, not `super::dispatch`.

- [ ] **Step 2: Confirm PUBLISH inside a transaction is special-cased the same way EXEC's caller special-cases SUBSCRIBE**

```rust
if cmd_name == "PUBLISH" {
    if let Some(ref ps) = pubsub {
        if let RespFrame::Array(Some(ref elements)) = frame {
            let mut args = Vec::new();
            for elem in elements {
                if let RespFrame::BulkString(Some(bytes)) = elem {
                    args.push(bytes.clone());
                }
            }
            if args.len() >= 3 {
                let mut ps_guard = ps.lock().unwrap();
                let res = super::pubsub::publish(&mut ps_guard, &args[1..]);
                results.push(res);
                continue;
            }
        }
    }
}
```

`PUBLISH` doesn't go through `dispatch_mutating` (which has no `PubSub` access) — confirm this branch exists and `continue`s past the normal dispatch call for that iteration, so `PUBLISH` isn't *also* run through `dispatch_mutating` afterward (which would either double-publish or hit an "unknown command" fallthrough, depending on whether `dispatch_mutating` recognizes `PUBLISH` at all — it should not).

- [ ] **Step 3: Confirm per-command success/failure doesn't abort the batch, and AOF logging happens per-command**

```rust
let result = super::dispatch_mutating(frame.clone(), &mut db_guard);
if let RespFrame::Error(_) = &result {
} else if super::is_write_command(&cmd_name) {
    if let Some(a) = aof {
        a.append(&frame).ok();
    }
}
results.push(result);
```

The loop always continues to the next queued frame regardless of whether `result` was an error — confirm there's no early `return`/`break` on error.

- [ ] **Step 4: Run unit test**

```bash
cargo test commands::tx::tests::test_exec_atomic
```

Expected: pass.

- [ ] **Step 5: Manually verify the full MULTI/EXEC happy path, in a single persistent connection**

`redis-cli`'s interactive/heredoc mode keeps one connection open across multiple lines, which is required here (separate `redis-cli` invocations are separate connections with independent transaction state):

```bash
redis-cli -p 6380 <<'EOF'
MULTI
SET tk1 v1
SET tk2 v2
EXEC
GET tk1
GET tk2
EOF
```

Expected: `OK` (MULTI), `QUEUED` ×2, then `EXEC` returns a 2-element array of `OK OK`, then `v1` and `v2` confirm both writes actually landed.

- [ ] **Step 6: Manually verify DISCARD abandons the queue**

```bash
redis-cli -p 6380 <<'EOF'
MULTI
SET tk3 v3
DISCARD
GET tk3
EOF
```

Expected: `OK`, `QUEUED`, `OK` (DISCARD), then `(nil)` — `tk3` was never actually set.

- [ ] **Step 7: Manually verify the error-without-MULTI and nested-MULTI cases**

```bash
redis-cli -p 6380 EXEC
redis-cli -p 6380 DISCARD
redis-cli -p 6380 <<'EOF'
MULTI
MULTI
DISCARD
EOF
```

Expected: `ERR EXEC without MULTI`, `ERR DISCARD without MULTI`, then `OK` / `ERR MULTI calls can not be nested` / `OK`.

- [ ] **Step 8: Manually verify a failing command inside a transaction doesn't abort the rest**

```bash
redis-cli -p 6380 SET strkey hello
redis-cli -p 6380 <<'EOF'
MULTI
SET goodkey works
LPUSH strkey x
SET anothergoodkey alsoworks
EXEC
GET goodkey
GET anothergoodkey
EOF
```

Expected: the `EXEC` array shows `OK`, then a `WRONGTYPE` error for the `LPUSH`, then `OK` — and both `goodkey`/`anothergoodkey` are set despite the middle command failing.

---

## Milestone 12 Done

This is the last of the 12 originally planned milestones. Once Task 2's manual verification passes, the full plan is verified end-to-end.
