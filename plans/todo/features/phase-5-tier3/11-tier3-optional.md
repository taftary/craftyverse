# 11 — Tier 3 extras (optional)

- **Phase:** 5 — Optional Tier 3
- **Status:** Optional — only after Tier 2 (Phases 1-3, default target) is
  complete
- **Depends on:** Phases 1-4
- **Plan reference:** notion.md Sections 12, 13 (Tier 3), 16 (Appendix)

## Goal

Optional advanced layers. None of these are on the default roadmap; each
item is its own mini-feature and must be re-approved before starting.

## Candidate items

- **Epoch snapshots** (notion.md Section 12) — precompute a handful of
  discrete snapshots (plate positions, elevation) at world-gen time; blend
  between them at runtime. Plate motion per epoch:
  `X_p(epoch) = X0 + v_p * epoch`. Never integrate live per frame.
- **Seasonal climate** — vary the ITCZ latitude and temperature bands over
  a season parameter; re-run the climate lookup cheaply.
- **Cheap isostasy** — Airy variant only (`h_iso = k_iso * E_cumulative`),
  a single O(n) pass. FFT / flexural isostasy stays deferred (Appendix).
- **Plate age / fragmentation** — extra plate attributes and boundary
  evolution over epochs.
- **Glacial effects** — elevation/climate-driven ice caps and simple
  glacial shaping.

## Explicitly NOT included (stay deferred, notion.md Appendix)

- 2D toroidal grid alternative.
- Recompute-from-`t` temporal model (continuous integrals).
- Iterative stream-power erosion simulation loops.
- FFT / flexural isostasy.
- Rust core / C++ renderer split.
- Full fluid simulation, per-frame erosion solving.

## Debug visualization

No screen up front — this phase is optional. When an item is scheduled,
its mini-feature defines a debug screen meeting the standard in
`plans/features/README.md`, for example:

- Epoch snapshots: timeline slider blending snapshots, with a delta
  overlay showing what moved between epochs.
- Seasonal climate: season slider re-running the climate lookup live.
- Isostasy: before/after toggle over the elevation field.

## Acceptance criteria (per item, when scheduled)

- Debug screen defined and shipped per the standard in
  `plans/features/README.md`; new keys/toggles documented in `README.md`.

- Item re-approved and scoped before work starts.
- Deterministic from the world seed; O(n) passes only.
- Standard definition of done passes; spec written under
  `docs/book/specs/`.
