# Performance and Observability

## Summary

Performance work starts with a workload and a measurement. Do not trade API
clarity for speculative allocation or synchronization improvements.

## Key points

- Measure CPU, GPU, memory, loading, and mobile energy/thermal behavior separately.
- Use structured logs and tracing at lifecycle and resource boundaries.
- Avoid logging hot-path data in production configurations.
- Every performance claim links to a benchmark, profile, or target-device result.

Measure separately:

- CPU frame and system time;
- allocations and memory footprint;
- GPU submission and pass timing;
- synchronization and queue stalls;
- asset loading and startup time;
- mobile energy and thermal behavior.

Use structured logs and tracing at lifecycle and resource boundaries. Avoid
logging per-vertex or per-frame hot-path data in production configurations.

A performance claim must link to a benchmark, profile, or target-device result.
