# ternary-version

Version vectors with ternary comparison for distributed GPU state. {+1=newer, 0=concurrent, -1=older}. Conflict detection, merge, dominance.

## Stats

- **Tests**: 8
- **LOC**: 165
- **License**: Apache-2.0

## Part of the Oxide Stack

This crate is part of the [Flux→PTX](https://github.com/SuperInstance/cuda-oxide/blob/main/FLUX_TO_PTX.md) experimental suite — a distributed GPU runtime built on five layers:

1. **open-parallel** — async runtime (tokio fork)
2. **pincher** — "Vector DB as runtime, LLM as compiler"
3. **flux-core** — bytecode VM + A2A agent protocol
4. **cuda-oxide** — Flux→MIR→Pliron→NVVM→PTX compiler
5. **cudaclaw** — persistent GPU kernels, warp-level consensus, SmartCRDT

## Usage

```rust
use ternary_version::*;
// See tests in src/lib.rs for complete examples
```

## License

Apache-2.0
