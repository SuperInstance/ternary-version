# ternary-version

Version vectors with ternary comparison. Know whether your state is newer, older, or in conflict—instantly.

Distributed systems need to compare state across nodes. With version vectors, the answer is always one of three things: **I'm newer** (+1), **you're newer** (-1), or **we're concurrent** (0). That ternary comparison isn't just convenient—it's the mathematically complete classification. Two version vectors can relate in exactly three ways.

This crate gives you version vectors, ternary comparison, merge (component-wise max), and a `ConflictResolver` that tracks merge history. All in ~170 lines with no dependencies beyond `HashMap`.

## Why this exists

Every distributed GPU node needs to know if its state is current. Timestamps don't work (clock skew). Vector clocks work but give you more information than you need. Version vectors are the minimal abstraction: per-node counters that you compare and merge.

The ternary insight is that `compare()` returns a `VersionOrder` enum: `Newer`, `Older`, or `Concurrent`. This maps directly to {-1, 0, +1} and directly to the action you should take:

| Comparison | Value | Action |
|------------|-------|--------|
| `Newer` | +1 | Your state wins, push it |
| `Older` | -1 | Their state wins, pull it |
| `Concurrent` | 0 | Conflict—merge required |

## The key insight

Version vectors give you *causality*, not just ordering. When `A.compare(&B)` returns `Newer`, it means A causally precedes B—A's updates are a subset of B's. When it returns `Concurrent`, it means neither causally precedes the other. Both have updates the other doesn't have. You *must* merge.

This is why the ternary comparison is complete: there is no fourth possibility. Either A dominates, B dominates, or they're concurrent. The code reflects the math.

## Quick start

```rust
use ternary_version::*;

// Two nodes track their own versions
let mut node_a = VersionVector::new();
let mut node_b = VersionVector::new();

// Each node increments its own counter on updates
node_a.increment("gpu-0");  // gpu-0: 1
node_a.increment("gpu-0");  // gpu-0: 2
node_b.increment("gpu-1");  // gpu-1: 1

// Compare: both have different nodes incremented → concurrent
assert_eq!(node_a.compare(&node_b), VersionOrder::Concurrent);

// Node B receives A's updates and merges
node_b.merge(&node_a);
node_b.increment("gpu-1");  // mark the merge

// Now B dominates A
assert_eq!(node_a.compare(&node_b), VersionOrder::Older);
```

## API reference

### VersionVector

```rust
let mut vv = VersionVector::new();

// Per-node counter operations
vv.increment("node-1");                    // increment and return new value
vv.set("node-2", 5);                       // set explicit version
vv.get("node-1");                          // → u64 (0 if absent)

// Ternary comparison
vv.compare(&other);   // → VersionOrder::Newer | Older | Concurrent

// Merge (component-wise max, modifies self)
vv.merge(&other);

// Inspection
vv.is_empty();
vv.node_count();
```

### VersionOrder

```rust
pub enum VersionOrder {
    Newer = 1,       // self dominates other
    Concurrent = 0,  // neither dominates
    Older = -1,      // other dominates self
}
```

### ConflictResolver

A higher-level abstraction that tracks merge history:

```rust
let mut resolver = ConflictResolver::new("merge-node");

let merged = resolver.resolve(&version_a, &version_b);
// Merges A and B, then increments the resolver's own counter
// Returns the merged vector

resolver.resolved_count();  // how many conflicts resolved
```

## How comparison works

The algorithm is straightforward:

1. Collect all node IDs from both vectors
2. For each node, compare counters: `a > b` sets `self_greater`, `b > a` sets `other_greater`
3. If only one side has greater values → that side wins
4. If both sides have some greater values → concurrent
5. If all equal → concurrent (equal versions are treated as concurrent, which is semantically correct—no one dominates)

```
A: {x:2, y:1}  vs  B: {x:1}
  x: A > B → self_greater
  y: A > 0 → self_greater
  Result: Newer (A dominates)

A: {x:2}  vs  B: {y:2}
  x: A > B → self_greater
  y: B > A → other_greater
  Result: Concurrent (neither dominates)
```

## Real-world example: GPU state synchronization

```rust
use ternary_version::*;

struct GpuNode {
    id: String,
    state_version: VersionVector,
    model_weights: Vec<i8>,  // ternary weights
}

impl GpuNode {
    fn new(id: &str) -> Self {
        Self {
            id: id.into(),
            state_version: VersionVector::new(),
            model_weights: vec![],
        }
    }

    fn train_step(&mut self) {
        // ... update model weights ...
        self.state_version.increment(&self.id);
    }

    fn sync_with(&mut self, other: &GpuNode) -> SyncResult {
        match self.state_version.compare(&other.state_version) {
            VersionOrder::Newer => SyncResult::Push,
            VersionOrder::Older => {
                self.state_version.merge(&other.state_version);
                SyncResult::Pull
            }
            VersionOrder::Concurrent => {
                self.state_version.merge(&other.state_version);
                self.state_version.increment(&self.id);
                SyncResult::Merge
            }
        }
    }
}

enum SyncResult { Push, Pull, Merge }

// Two nodes train independently, then sync
let mut node_0 = GpuNode::new("gpu-0");
let mut node_1 = GpuNode::new("gpu-1");

node_0.train_step();  // gpu-0: 1
node_0.train_step();  // gpu-0: 2
node_1.train_step();  // gpu-1: 1

// node_0 has updates node_1 doesn't have, and vice versa
let result = node_1.sync_with(&node_0);
assert!(matches!(result, SyncResult::Merge));
```

## Architecture

```
VersionVector ──compare──→ VersionOrder {Newer, Older, Concurrent}
      │                         │
      └──merge──→ merged VersionVector
                       │
              ConflictResolver
              (tracks merge history, auto-increments)
```

The `VersionVector` is a `HashMap<String, u64>` under the hood. Comparison is O(n) where n = total unique node IDs. Merge is O(n) where n = entries in the other vector.

## Ecosystem connections

- **ternary-paxos** — consensus decisions produce versioned values; use version vectors to track which round produced which decision
- **ternary-rate-limiter** — rate limiter state across distributed nodes needs version tracking
- **ternary-resilience** — network health state is versioned; cascading failures propagate versioned updates

## Open questions

- **Garbage collection**: Version vectors grow unboundedly as new nodes join. Dotted version vectors (VVV) or interval tree clocks could bound the size.
- **Partial merge semantics**: Current merge takes component-wise max. Some applications need CRDT-style merge (OR-Set, LWW-Register) layered on top.
- **Byzantine comparison**: What if a node lies about its version? Signed version vectors with merkle proofs would help.

## Stats

| Metric | Value |
|--------|-------|
| Tests | 8 |
| Lines of code | 166 |
| Public API surface | 14 items |
| License | Apache-2.0 |
| Unsafe | 0 |

## Installation

```toml
[dependencies]
ternary-version = "0.1.0"
```

## License

Apache-2.0
