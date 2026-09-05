# Content retirement map

This map records where guidance from deleted standalone documents now lives in
the consolidated Rust handbook. It is intended to prevent lost decisions during
the documentation restructuring.

## Deleted documents and their successors

| Deleted document | New home | Notes |
| --- | --- | --- |
| `docs/ARCHITECTURE.md` | `docs/book/architecture/` (`workspace.md`, `crate-boundaries.md`, `technology.md`, `migration-principles.md`) | The target workspace and crate boundaries are now authoritative. Old baseline-specific module breakdowns were dropped because they described the single-package layout, not the migration target. |
| `docs/technology-definition.md` | `docs/book/architecture/technology.md` | Technology decisions were merged into the target/decision format. |
| `docs/project-structure-and-best-practices/01-basic-project-layout.md` | `docs/book/architecture/workspace.md` and `docs/book/practices/project-structure.md` | Basic layout guidance is now part of the target architecture and practices chapters. |
| `docs/project-structure-and-best-practices/02-modules.md` | `docs/book/practices/project-structure.md` and `docs/book/architecture/crate-boundaries.md` | Module organization guidance moved into practices and crate-boundary pages. |
| `docs/project-structure-and-best-practices/03-library-vs-binary.md` | `docs/book/architecture/crate-boundaries.md` | Library/binary separation is now described in crate boundaries. |
| `docs/project-structure-and-best-practices/04-folder-organization.md` | `docs/book/practices/project-structure.md` | Folder conventions merged into the project-structure practice page. |
| `docs/project-structure-and-best-practices/05-clean-code-practices.md` | `docs/book/practices/` (multiple pages) and `docs/STYLEGUIDE.md` | Clean-code guidance distributed across practices and the style guide. |
| `docs/project-structure-and-best-practices/06-error-handling-and-logging.md` | `docs/book/practices/error-handling.md` | Error-handling content moved here. |
| `docs/project-structure-and-best-practices/07-dependency-management.md` | `docs/book/practices/dependencies.md` | Dependency guidance moved here. |
| `docs/project-structure-and-best-practices/08-example-scalable-project.md` | `docs/book/architecture/workspace.md` | Scalable-project example replaced by the target workspace. |
| `docs/project-structure-and-best-practices/README.md` | `docs/book/index.md` and `docs/CONTRIBUTING.md` | Entry-level guidance moved to the book introduction and contribution guide. |
| `docs/classes-definitions/node.md` | `docs/book/specs/node.md` | Moved with minor updates for the new module layout; internal links updated to mdBook-relative paths. |
| `docs/classes-definitions/scene.md` | `docs/book/specs/scene.md` | Moved with minor updates for the new module layout; internal links updated to mdBook-relative paths. |
| `docs/classes-definitions/text.md` | `docs/book/specs/text.md` | Moved with minor updates for the new module layout; internal links updated to mdBook-relative paths. |
| `docs/classes-definitions/render.md` | `docs/book/specs/render.md` | Substantially rewritten for the extracted `crates/engine` renderer and the `Scenario` API. |
| `docs/classes-definitions/ARCHITECTURE.md` | Retired | Its generic workspace/crate breakdown was superseded by `docs/book/architecture/`. Of its future-extension ideas, networking is captured by an open ADR in `docs/book/architecture/decisions/`; scripting, editor GUI, hot-reload, and plugins were intentionally dropped until requirements exist. |

## What was intentionally dropped

- **Old single-package module breakdowns.** Documents that described the
  pre-migration `src/` layout were not preserved because the migration target is
  a workspace with `crates/engine`, `crates/game`, and `crates/tools`.
- **Generic engine subsystem placeholders.** ECS, physics, assets, and audio
  subsystem layouts from the old generic architecture document were replaced by
  open ADRs that will be filled in once requirements and constraints are known.

## Verification

If you cannot find guidance that used to be in one of the deleted documents,
check the corresponding new home above. If it is genuinely missing, open a
follow-up change rather than re-creating the old standalone file.
