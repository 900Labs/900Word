# Performance Budget

900Word targets low-resource environments, but the project will publish measured budgets rather than claiming historical AbiWord-class memory use.

Initial budgets to measure before public release:

- Cold startup time.
- Idle memory.
- Typing latency in representative documents.
- ODT open/save time.
- PDF export time.
- Installer size.

Sprint 008 adds automated bootstrap budgets:

- Desktop frontend build output must stay below `MAX_DESKTOP_DIST_BYTES`, default `2500000` bytes.
- Any single desktop frontend asset must stay below `MAX_DESKTOP_ASSET_BYTES`, default `750000` bytes.
- `scripts/performance-smoke.sh` records desktop build-output bytes and smoke timing for `word-export` and the generated ODT round-trip test.

These are early guardrails, not full product performance claims. The first public release may publish only measured values. Hard pass/fail thresholds for startup time, idle memory, typing latency, ODT open/save time, PDF export time, and installer size must be introduced after baseline measurements exist on supported hardware.
