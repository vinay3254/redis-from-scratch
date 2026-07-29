# Redis Clone (Rust) — Design

## Purpose

Build a Redis server clone from scratch in Rust, as a learning project. No existing Redis
client/server libraries — RESP parsing, the data structures, persistence, and networking are
all hand-written. Goal is to internalize how Redis actually works (protocol, concurrency model,
data structures, persistence), validated milestone by milestone against a real `redis-cli`.

## Scope

12 milestones, built and confirmed working one at a time before moving to the next:

1. Raw TCP server accepting multiple concurrent connections
2. RESP2 protocol parser and serializer
3. Command dispatcher + `PING`, `ECHO`, `SET`, `GET`, `DEL`, `EXISTS`
4. Key expiry: `EXPIRE`, `TTL`, `PEXPIRE`, passive + active expiration
5. Lists: `LPUSH`, `RPUSH`, `LPOP`, `RPOP`, `LRANGE`
6. Hashes: `HSET`, `HGET`, `HGETALL`, `HDEL`
7. Sets: `SADD`, `SREM`, `SMEMBERS`, `SISMEMBER`
8. Sorted sets: `ZADD`, `ZRANGE`, `ZSCORE` (hand-rolled skip list)
9. RDB persistence: `SAVE`/`BGSAVE` snapshot (custom format) + load on startup
10. AOF persistence: append writes, replay log on restart
11. Pub/Sub: `SUBSCRIBE`, `PUBLISH`, `UNSUBSCRIBE`
12. Basic transactions: `MULTI`, `EXEC`, `DISCARD`

Out of scope: replication (`REPLICAOF`), Lua scripting (`EVAL`), RESP3, cluster mode. RDB/AOF
are our own simple formats — same idea as real Redis, not byte-compatible with it.

## Language & Concurrency Model

- **Language:** Rust, no external Redis crates.
- **Concurrency:** thread-per-connection. `std::net::TcpListener` accepts connections; each
  connection is handled on its own `std::thread`. This trades the authenticity of Redis's
  single-threaded event loop for simplicity — real Redis's model is explained conceptually
  when we hit milestone 1, but not reproduced.

## Architecture

Single Rust binary crate:

```
src/
  main.rs           – TcpListener, accepts + spawns a thread per connection
  resp.rs           – RESP2 parser/serializer
  db.rs             – shared store: Value enum (String/List/Hash/Set/ZSet) + expiry map
  skiplist.rs       – hand-rolled skip list backing the sorted set
  commands/
    mod.rs          – dispatcher: command name -> handler
    string.rs       – SET/GET/INCR etc
    list.rs         – LPUSH/RPUSH/LPOP/RPOP/LRANGE
    hash.rs         – HSET/HGET/HGETALL/HDEL
    set.rs          – SADD/SREM/SMEMBERS/SISMEMBER
    zset.rs         – ZADD/ZRANGE/ZSCORE
    generic.rs      – DEL/EXISTS/EXPIRE/TTL/PEXPIRE
  persistence/
    rdb.rs          – custom binary snapshot format, SAVE/BGSAVE + load-on-boot
    aof.rs          – append writes as RESP, replay on restart
  pubsub.rs         – channel -> subscriber registry
  transactions.rs   – per-connection MULTI/EXEC command queue
```

## State & Data Flow

All connection threads share one `Arc<Mutex<Db>>`. `Db` holds:
- `HashMap<String, Value>` — the keyspace
- a parallel expiry map (key -> expiration instant)

Per-connection loop: read bytes from the socket → parse a RESP frame → dispatch to the
matching command handler against the shared `Db` → serialize the result → write the response
back to the socket. A dedicated background thread wakes periodically to sweep expired keys
(active expiry); every read/write access to a key also checks its expiry first (passive
expiry).

`SUBSCRIBE`ed connections diverge from the normal request/response loop: once subscribed, the
connection's read loop is replaced by a wait on a per-connection receiver fed by `PUBLISH`
against the subscriber registry.

`MULTI` puts a connection into a queuing mode where subsequent commands are buffered
(not executed) until `EXEC` (run them all while holding the `Db` lock) or `DISCARD` (drop the
queue).

## Error Handling

- Malformed RESP input → `-ERR protocol error`, connection may be closed depending on how
  unrecoverable the parse failure is.
- Unknown command → `-ERR unknown command '<name>'`.
- Wrong number of arguments → `-ERR wrong number of arguments for '<name>' command`.
- Type mismatch (e.g. `LPUSH` on a key holding a string) → `-WRONGTYPE Operation against a key
  holding the wrong kind of value`.

## Testing

No automated test suite. After each milestone, the assistant provides `redis-cli` and/or
`netcat` commands to manually verify behavior against the running server before moving on to
the next milestone. The user confirms each milestone works before implementation continues.

## Explanation-First Workflow

For each milestone, before writing code: explain the relevant real-Redis behavior and edge
cases that shape the implementation. Code is written with no comments. After implementation,
provide manual test commands.
