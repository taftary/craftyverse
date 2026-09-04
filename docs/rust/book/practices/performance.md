# Performance and Observability

Performance work starts with a workload and a measurement. Do not trade API
clarity for speculative allocation or synchronization improvements.

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
