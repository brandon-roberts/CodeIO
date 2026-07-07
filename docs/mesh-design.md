# P7 Mesh Design — Local-First Resource Delegation

Status: contract defined (`proto/p2p/mesh.proto`), implementation planned. This doc is honest
about what is standard engineering vs. research frontier, so we build in the right order.

## The model
Every device (Android/iOS/Windows/macOS/Linux) is a **Node** advertising measured **Capacity**.
A local network's total potential is the **aggregate** of its nodes — surfaced as one big node
("your mesh can do ~N tokens/sec across M devices"). Work is dispatched as **streamed blocks**
to whichever node(s) have capacity, over the **most local transport available**.

## Transport preference (explicit, by design)
LAN → Bluetooth → Direct (WiFi-Direct/USB) → Internet (fallback only). The scheduler always
prefers local links to avoid unnecessary internet/data usage. A phone on the same WiFi as a
desktop talks to it directly; the internet is used only when no local path exists.

## What each capability realistically is

### ✅ Standard engineering (build with confidence)
- **Discovery on LAN:** mDNS/DNS-SD advertises nodes and capacities. Proven, cross-platform.
- **Capacity measurement:** RAM/VRAM/cores/battery/link are OS-queryable; tokens/sec is *measured*
  by a short local benchmark (never claimed). `codeio doctor --bench` produces it.
- **Remote-node dispatch (the primary win):** a thin client streams a job to the strongest node
  (e.g. desktop running Ollama) and streams results back. This is what makes a cheap phone feel
  like a workstation. Latency is one network hop — excellent on LAN.
- **Aggregate view + scheduler:** summing capacities and routing whole jobs to the best node is
  ordinary distributed-systems work.
- **App-server packaging:** `codeio serve` hosts the engine + Ollama + services; other devices
  connect as clients. This is the near-term deliverable.

### ⚠️ Real but constrained (build carefully, measure honestly)
- **Sharded inference (one model split across peers):** real (exo/llama.cpp RPC/Petals), but over
  LAN it adds per-token latency because layers exchange activations each token. It buys *capacity*
  (run a model too big for any single node), not speed. Use only when a model fits nowhere.
- **Bluetooth as a compute transport:** fine for discovery/control and small blocks; its bandwidth
  (~1–3 MB/s BT, more for BLE-lite) is too low for tensor streaming. Treat BT as control-plane and
  small-payload path, LAN as the data-plane.

### 🔬 Research frontier (do not overpromise)
- **iOS as a serving node:** iOS sandboxing forbids long-lived background compute/servers for
  third-party apps. iOS is a strong *client*, a weak *server*. Plan iOS as client-first; on-device
  inference only in-foreground.
- **General "seed any resource from any device":** heterogeneous GPU/CPU/RAM pooling across OSes
  for arbitrary workloads is unsolved in general. We scope it to specific block kinds
  (inference, indexing, batch compute) with explicit protocols — not a magic universal pool.

## Build order
1. `codeio serve` app-server (host engine + Ollama + services; clients connect). ← next after M3
2. `codeio doctor --bench` measured tokens/sec into Capacity.
3. LAN mDNS discovery + aggregate MeshSummary view.
4. Remote-node inference dispatch (phone → desktop Ollama) — the primary P7 payoff.
5. Battery/thermal policy, leases, scheduler.
6. Sharded inference experiments — measured, honestly, capacity-only.
7. Bluetooth control-plane; iOS client.

## Discipline
Every capability advertised in the UI must be *measured* on the actual mesh before it is claimed.
Aggregate numbers are computed from benchmarks, never from spec sheets. (Same receipts-over-vibes
rule as the Trust Ledger and Architecture Authority.)

## Coordinator + worker inversion ("FrostWire for compute")
Two dispatch directions coexist on the same mesh:
1. **Server-as-coordinator, devices-as-workers:** AWS/Linux server holds the job queue and pushes
   work out to the device fleet; devices execute and stream results back. The server is also a
   *participating node* (it can run Ollama itself), so it's coordinator + worker, not just a router.
2. **Device-as-coordinator, peers-as-workers:** the earlier remote-node model — a phone dispatches
   to the strongest LAN peer. Same protocol, roles swapped.

Both use `blockstream.proto`. The design principle is P2P resource sharing for compute: every
online device is a building block; the coordinator matches work to capacity and prefers local
transport, reaching over the internet only when needed.

### The carrier-NAT rule (baked into the protocol)
A device cannot be freely dialed into over the internet (carrier NAT/firewalls). So the **device
opens one long-lived outbound stream** to the coordinator (`BlockStream.Connect`); the coordinator
pushes DISPATCH frames down that existing pipe and receives RESULT frames up it. This is why the
protocol is a single bidirectional stream multiplexed by `stream_id`, not per-job inbound calls.

### Efficiency: context-once, nested multiplexing
- HEADER declares context (model, params, encoding) exactly once per logical stream.
- BODY frames carry only `stream_id + seq + payload` — no re-sent context.
- CLOSER carries integrity (checksum) + outcome.
- A BODY may tunnel child Frames (`nested`) to multiplex many sub-streams over one pipe without
  new connections — e.g. one job fanning to several models, or a command emitting stdout+stderr
  as nested sub-streams.

### iOS/Android reality (unchanged, restated for this model)
- **Android:** can hold the outbound worker stream in background within OS policy — a capable worker.
- **iOS:** holds the stream reliably only in-foreground (or brief push-woken windows); strong
  worker while active, not a persistent headless one. Coordinator treats iOS capacity as
  intermittent and schedules accordingly.
