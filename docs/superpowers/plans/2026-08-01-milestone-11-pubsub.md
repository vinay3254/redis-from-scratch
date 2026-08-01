# Redis Clone — Milestone 11: Pub/Sub Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.
>
> **Note:** This milestone's code already exists (commit `9b58bd3`). This plan documents what it must deliver so the implementation can be verified against it, rather than rewritten from scratch. This is the milestone with the trickiest concurrency: a subscribed connection needs to both keep reading commands from its own socket *and* receive asynchronously-published messages from other connections' threads.

**Goal:** `SUBSCRIBE`, `PUBLISH`, `UNSUBSCRIBE`, with real Redis's wire format for subscription confirmations and message delivery.

**Architecture:** `PubSub` (`src/pubsub.rs`) maps channel name → `{client_id → mpsc::Sender<RespFrame>}`, plus the reverse index (`client_id → set of subscribed channels`) needed for cleanup. Each connection thread creates its own `mpsc::channel()` when it starts; once it issues `SUBSCRIBE`, the sending half is registered with `PubSub` under its `client_id`, and the connection's own read loop switches into a polling mode: instead of blocking indefinitely on `stream.read`, it sets a 100ms socket read timeout and, each iteration, drains any pending messages from its `mpsc::Receiver` before checking for more input on the socket. This is how one thread does both jobs without a second thread per connection.

**Tech Stack:** `std::sync::mpsc` (one channel per connection), `stream.set_read_timeout` for the non-blocking-ish poll loop.

## Global Constraints

- Language: Rust, no external crates.
- Subscription confirmation and message delivery use real Redis's exact RESP array shapes: `SUBSCRIBE` replies with one `["subscribe", channel, subscription_count]` array *per channel requested*; a delivered message is `["message", channel, payload]`; `UNSUBSCRIBE` replies with `["unsubscribe", channel, remaining_count]` per channel (or a single `["unsubscribe", nil, 0]` if called with no arguments and the client had no subscriptions).
- `PUBLISH` returns the number of subscribers that received the message (matching real Redis's return value), and prunes any subscriber whose channel `Send` fails (a dead/disconnected client) rather than leaving stale entries.
- A client's subscriptions must be fully cleaned up on disconnect — both directions of the map (`channels` and `client_subscriptions`) need entries removed, or channels would accumulate phantom subscriber counts forever.
- Code has no comments.

---

### Task 1: PubSub broker — subscribe, publish, unsubscribe, client cleanup

**Files:**
- Modify: `src/pubsub.rs`

**Interfaces:**
- Produces: `pub struct PubSub { ... }`, `PubSub::new() -> Self`, `generate_client_id(&mut self) -> u64`, `publish(&mut self, channel: &[u8], message: &[u8]) -> usize`, `subscribe(&mut self, client_id: u64, requested_channels: &[Vec<u8>], tx: mpsc::Sender<RespFrame>) -> Vec<RespFrame>`, `unsubscribe(&mut self, client_id: u64, requested_channels: &[Vec<u8>]) -> Vec<RespFrame>`, `remove_client(&mut self, client_id: u64)`.

- [x] **Step 1: Confirm publish's exact message frame shape and dead-subscriber pruning**

```rust
pub fn publish(&mut self, channel: &[u8], message: &[u8]) -> usize {
    let mut receivers_count = 0;
    if let Some(subscribers) = self.channels.get_mut(channel) {
        let msg_frame = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"message".to_vec())),
            RespFrame::BulkString(Some(channel.to_vec())),
            RespFrame::BulkString(Some(message.to_vec())),
        ]));
        let mut dead_clients = Vec::new();
        for (&client_id, tx) in subscribers.iter() {
            if tx.send(msg_frame.clone()).is_ok() {
                receivers_count += 1;
            } else {
                dead_clients.push(client_id);
            }
        }
        for dead_id in dead_clients {
            subscribers.remove(&dead_id);
        }
    }
    receivers_count
}
```

Confirm dead clients are collected during iteration and removed *after* (not during, which would be a mutable-borrow-while-iterating error in Rust anyway, but worth confirming the two-pass structure is there for the right reason).

- [x] **Step 2: Confirm subscribe returns one confirmation frame per channel with the running subscription count**

```rust
pub fn subscribe(&mut self, client_id: u64, requested_channels: &[Vec<u8>], tx: mpsc::Sender<RespFrame>) -> Vec<RespFrame> {
    let mut responses = Vec::with_capacity(requested_channels.len());
    let client_subs = self.client_subscriptions.entry(client_id).or_insert_with(HashSet::new);
    for ch in requested_channels {
        client_subs.insert(ch.clone());
        self.channels.entry(ch.clone()).or_insert_with(HashMap::new).insert(client_id, tx.clone());
        let count = client_subs.len();
        responses.push(RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"subscribe".to_vec())),
            RespFrame::BulkString(Some(ch.clone())),
            RespFrame::Integer(count as i64),
        ])));
    }
    responses
}
```

The `count` in each confirmation is the client's *total* subscription count after that channel is added (so `SUBSCRIBE ch1 ch2` yields counts `1` then `2`), not the channel's subscriber count — confirm this matches real Redis's semantics (the count is from the subscribing client's perspective).

- [x] **Step 3: Confirm unsubscribe with no arguments unsubscribes from everything, and the empty-subscription-list edge case**

```rust
let channels_to_unsub: Vec<Vec<u8>> = if requested_channels.is_empty() {
    if let Some(subs) = self.client_subscriptions.get(&client_id) {
        subs.iter().cloned().collect()
    } else {
        Vec::new()
    }
} else {
    requested_channels.to_vec()
};

if channels_to_unsub.is_empty() {
    responses.push(RespFrame::Array(Some(vec![
        RespFrame::BulkString(Some(b"unsubscribe".to_vec())),
        RespFrame::BulkString(None),
        RespFrame::Integer(0),
    ])));
    return responses;
}
```

`UNSUBSCRIBE` (bare, no channel args) when the client has zero active subscriptions returns a single frame with a nil channel and count `0` — confirm this special case exists rather than returning an empty array (real Redis always sends at least one unsubscribe confirmation, even a vacuous one).

- [x] **Step 4: Confirm remove_client cleans up both directions of the map**

```rust
pub fn remove_client(&mut self, client_id: u64) {
    if let Some(subs) = self.client_subscriptions.remove(&client_id) {
        for ch in subs {
            if let Some(subscribers) = self.channels.get_mut(&ch) {
                subscribers.remove(&client_id);
                if subscribers.is_empty() {
                    self.channels.remove(&ch);
                }
            }
        }
    }
}
```

Confirm a channel with zero remaining subscribers is removed from `self.channels` entirely (not left behind as an empty `HashMap`), so `self.channels` doesn't grow unboundedly across many short-lived subscriptions.

- [x] **Step 5: Run unit test**

```bash
cargo test pubsub::tests::test_pubsub_basic
```

Expected: pass.

---

### Task 2: SUBSCRIBE/PUBLISH/UNSUBSCRIBE wired into the connection loop

**Files:**
- Modify: `src/commands/pubsub.rs`
- Modify: `src/main.rs` (connection handling loop)

**Interfaces:**
- Produces: `pub fn publish(pubsub: &mut PubSub, args: &[Vec<u8>]) -> RespFrame` (command-layer wrapper).
- Consumes: `PubSub` methods (Task 1).

- [x] **Step 1: Confirm the per-connection read-timeout + channel-drain poll loop**

Read `src/main.rs`'s `handle_connection`. Confirm:

```rust
let (tx, rx) = std::sync::mpsc::channel::<RespFrame>();
let mut is_subscribed = false;
stream.set_read_timeout(Some(Duration::from_millis(100))).ok();

loop {
    if is_subscribed {
        while let Ok(msg) = rx.try_recv() {
            if stream.write_all(&msg.serialize()).is_err() {
                // clean up and return
            }
        }
    }

    match stream.read(&mut read_buf) {
        Ok(0) => { /* clean up and return */ }
        Ok(n) => { buffer.extend_from_slice(&read_buf[..n]); }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {}
        Err(_) => { /* clean up and return */ }
    }
    // ...parse and dispatch any complete frames in buffer...
}
```

The read timeout is set unconditionally at connection start (not only after subscribing) — confirm this, since it means every connection (subscribed or not) polls in ~100ms slices rather than blocking forever on `read`. This is a deliberate tradeoff: it adds up to 100ms of latency to any command on an otherwise-idle connection, in exchange for letting a single thread handle both inbound commands and inbound pub/sub messages without needing a second thread or an async runtime.

- [x] **Step 2: Confirm SUBSCRIBE/UNSUBSCRIBE are intercepted before reaching the generic dispatch function**

Confirm `handle_connection` special-cases `cmd_name == "SUBSCRIBE"` and `cmd_name == "UNSUBSCRIBE"` directly (calling `pubsub.lock().unwrap().subscribe(...)` / `.unsubscribe(...)`) rather than routing them through `commands::dispatch`, since they need direct access to this connection's own `tx`/`client_id`/`is_subscribed` flag, which `dispatch` has no way to reach.

- [x] **Step 3: Confirm client_id is generated once per connection and cleaned up on every exit path**

Confirm `ps.remove_client(client_id)` is called on every path out of the connection loop (socket closed, write error, parse error) — not just the graceful `Ok(0)` disconnect path. A client that drops mid-write should still be unregistered from `PubSub`, or `publish` would keep trying (and failing) to send to it until the next publish prunes it.

- [x] **Step 4: Run unit tests**

```bash
cargo test commands::tests pubsub::tests
```

Expected: pass.

- [x] **Step 5: Manually verify subscribe confirmation and message delivery across two connections**

`redis-cli`'s own `SUBSCRIBE` blocks in the foreground waiting for messages, so use two terminals:

Terminal A:
```bash
redis-cli -p 6380 SUBSCRIBE ch1
```

Expected immediately: `1) "subscribe" 2) "ch1" 3) (integer) 1`.

Terminal B:
```bash
redis-cli -p 6380 PUBLISH ch1 "hello subscribers"
```

Expected: `(integer) 1` (one subscriber received it). Terminal A should then print the delivered message: `1) "message" 2) "ch1" 3) "hello subscribers"`.

- [x] **Step 6: Manually verify PUBLISH with zero subscribers, and multi-channel UNSUBSCRIBE**

```bash
redis-cli -p 6380 PUBLISH nosubschannel "anyone there?"
```

Expected: `(integer) 0`.

In Terminal A (still subscribed to `ch1`), press Ctrl+C to disconnect, then from Terminal B:

```bash
redis-cli -p 6380 PUBLISH ch1 "is anyone still listening?"
```

Expected: `(integer) 0` — proving the disconnected client was pruned from `ch1`'s subscriber list, not left as a phantom subscriber.

---

## Milestone 11 Done

Once Task 2's manual verification passes, report back to confirm before Milestone 12's plan (Transactions) is written.
