# axiom-ir

Parser, structural validator, canonicalizer and hasher for `AXIOM-IR/1`. This is intentionally a separate trust boundary
from the surface DSL.

> **Maturity:** research prototype v0.1. The default verifier proves properties by exhaustive evaluation over an
> explicitly finite input domain. A VALID receipt is therefore a theorem about that bounded model, not a claim of
> unbounded program correctness.

```bash
cargo run -- inspect ../axiom-spec/examples/abs.aix
cargo run -- canonical input.aix --out canonical.aix
```
