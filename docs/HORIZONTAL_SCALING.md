# Horizontal Scaling

This page covers how the backend runs as multiple replicas behind a load balancer, what each replica
shares, and how to scale and verify it.

## Architecture

```
            ┌──────────── ingress-nginx (cookie affinity: SI_BACKEND) ────────────┐
            │                                                                       │
      ┌─────▼─────┐              ┌───────────┐              ┌───────────┐
      │ backend-0 │              │ backend-1 │              │ backend-2 │   HPA: 3–10 replicas
      └─────┬─────┘              └─────┬─────┘              └─────┬─────┘
            │  pub/sub ws:broadcast   │   locks / leases         │
            └───────────────┬─────────┴──────────────┬───────────┘
                         ┌──▼──┐                  ┌──▼──┐
                         │Redis│                  │ DB  │
                         └─────┘                  └─────┘
```

| State | Where it lives | Multi-instance behaviour |
|-------|----------------|---------------------------|
| WebSocket broadcasts | Redis pub/sub `ws:broadcast` | Published once, delivered by every replica to its own sockets (`WsState::fanout`) |
| WebSocket channel messages | Redis pub/sub (with `channel`) | Every replica delivers to its local subscribers of that channel |
| WebSocket sockets and subscriptions | In-memory, per replica | A socket can only live on one process. Clients reconnect to any replica and re-subscribe |
| Scheduled jobs (`JobScheduler`, backups) | Redis lock `job-lock:<name>` | Runs on one replica per interval. The lock is renewed while the job runs |
| Long-running singletons (webhook dispatcher) | Redis lease `leader:<role>` | Runs on the elected leader only. Failover within one lease (30 s) |
| Response cache | Redis (`CacheManager`) | Shared |
| Application data | Database | Shared (see limitation below) |

### Distributed locks (`src/distributed_lock.rs`)

- `SET key <token> NX PX ttl`. The token is `<instance_id>:<uuid>`.
- Release and extend use compare-and-set Lua scripts, so a replica never touches a lock that expired and was taken by another replica.
- One auto-reconnecting Redis connection per process (`DistributedLock::shared()`).
- **Fallback**: with `REDIS_URL` unset, the process assumes it's the only instance and every lock succeeds. With `REDIS_URL` set but Redis down, locks **fail closed** and singleton work pauses instead of running on every replica.

### Leader election (`src/leader_election.rs`)

- `spawn_leader_task(lock, role, ttl, make_task)` campaigns for `leader:<role>`, renews every `ttl/3`, starts the task when elected, and aborts it when the lease is lost.
- If the task exits on its own (for example, the webhook dispatcher gave up after repeated failures), the replica resigns so another replica can take over.
- Tasks must still be idempotent. During a network partition, two replicas can overlap for up to one TTL.

### WebSocket fan-out (`src/websocket.rs`)

- `broadcast(msg)` and `broadcast_to_channel(ch, msg)` publish to Redis when this replica's subscriber is connected. The replica receives its own message back like everyone else.
- If Redis is not configured, the subscriber is down, or the publish fails, the message is delivered locally so it's never silently dropped.
- `broadcast_local(msg)` reaches only this replica's sockets. Use it for instance-specific events such as `ServerShutdown`.
- The subscriber also accepts the pre-envelope payload format, so rolling deploys don't lose updates.

### Session affinity

Ingress uses cookie affinity (`SI_BACKEND`), so a browser keeps hitting the same replica. With Redis fan-out,
affinity is **not** required for correctness. It only avoids re-subscribing when a client reconnects. When
a replica goes away, clients reconnect through the ingress and land on a healthy replica.

### Client IP behind the ingress

`TRUST_PROXY_HEADERS=true` and `TRUSTED_PROXY_HOPS=1` (set in `k8s/backend/deployment.yaml`) make the
backend read the client IP from `X-Forwarded-For`. It counts trusted hops from the right, since entries
further left are client-supplied. Per-IP WebSocket limits and access logs use this IP. Without it, every
client appears as the ingress pod, and per-IP limits apply to the whole ingress.

Per-IP WebSocket limits (10 concurrent, 20 attempts/min) are enforced **per replica**. A client could
open up to `10 × replicas` sockets. The ingress `limit-connections` annotation is the global cap.

## Configuration

| Variable | Where | Purpose |
|----------|-------|---------|
| `REDIS_URL` | secret `redis-url` | Enables distributed locks, leader election and WebSocket fan-out |
| `POD_NAME` | Downward API | Instance ID in lock tokens and logs (falls back to `HOSTNAME`, then a UUID) |
| `TRUST_PROXY_HEADERS`, `TRUSTED_PROXY_HOPS` | deployment env | Real client IP behind the ingress |

Graceful shutdown: `preStop: sleep 10` lets endpoints stop routing to the pod first. Then the app notifies
its own WebSocket clients, flushes the cache and closes the DB, within `terminationGracePeriodSeconds: 45`.

## Scaling procedures

```bash
# Manual scale (the HPA reconciles back into its 3–10 range)
kubectl -n stellar-insights scale deployment stellar-insights-backend --replicas=5

# Adjust autoscaling bounds
kubectl -n stellar-insights patch hpa stellar-insights-backend \
  -p '{"spec":{"minReplicas":3,"maxReplicas":15}}'

# Watch the rollout / autoscaler
kubectl -n stellar-insights rollout status deployment/stellar-insights-backend
kubectl -n stellar-insights get hpa stellar-insights-backend -w
```

Before raising `maxReplicas`, check that the database connection limit covers
`replicas × DB_POOL_MAX_CONNECTIONS` (20 per pod by default) plus headroom.

## Verifying a multi-replica deployment

```bash
kubectl -n stellar-insights scale deployment stellar-insights-backend --replicas=3

# 1. Leader election: exactly one replica should log "Acquired leadership" for webhook-dispatcher
kubectl -n stellar-insights logs -l component=backend --prefix | grep -E 'leadership|Leader-only'

# 2. Scheduled jobs/backups run once per interval: the other replicas log that they skipped
kubectl -n stellar-insights logs -l component=backend --prefix | grep -E "skipped — another instance"

# 3. Locks and leases in Redis
kubectl -n stellar-insights exec statefulset/redis -- redis-cli --scan --pattern 'leader:*'
kubectl -n stellar-insights exec statefulset/redis -- redis-cli GET leader:webhook-dispatcher

# 4. WebSocket fan-out: every replica is subscribed
kubectl -n stellar-insights exec statefulset/redis -- redis-cli PUBSUB NUMSUB ws:broadcast   # → replica count

# 5. Failover: delete the leader and watch another replica take over within ~30 s
kubectl -n stellar-insights delete pod <leader-pod>
```

To test WebSocket delivery across replicas, open two clients without the affinity cookie so they land on
different pods (check the `instance_id` in the access log). Then trigger a corridor or anchor update
through either pod and confirm both clients receive it.

For load testing across replicas, see `backend/load-tests/`. Point them at the ingress host, not a single pod.

## Known limitation: database

The backend's sqlx build currently enables only the **SQLite** driver (`backend/Cargo.toml`). A SQLite
file is local to each pod, so replicas would each have their own data. Truly stateless replicas require
the Postgres driver and migrating the SQLite-specific queries. That work is tracked separately. Until
then, every coordination mechanism above is in place and ready, but data isn't shared between pods.
