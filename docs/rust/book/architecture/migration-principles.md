# Migration Principles

The migration changes ownership boundaries before it changes behavior.

- Establish a buildable workspace before moving implementation files.
- Extract stable, renderer-neutral data before renderer-specific code.
- Preserve behavior and tests at every stage.
- Use adapters temporarily; do not turn compatibility shims into permanent APIs.
- Move one responsibility at a time and record public API changes.
- Keep headless tests independent from display and GPU requirements.
- Remove old entry points only after the target boundary is exercised by its consumers.

The detailed stage gates and rollback points live in
[`MIGRATION.md`](../../MIGRATION.md).
