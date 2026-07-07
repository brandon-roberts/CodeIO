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
