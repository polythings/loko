# Specs

loko's design. Start here, people and agents alike.

- **Specs** (`NNNN-name.md`): one feature each, with the same template ([TEMPLATE.md](TEMPLATE.md)). A spec says what
  the feature does and why, what is still open (as GitHub issues), how far it is built, and the releases that changed
  it.
- **Milestones** ([`milestones/`](milestones/README.md)): goals made of specs. The roadmap is the list of milestones.

## Statuses

| Status | Meaning |
|---|---|
| Done | Built and in effect; the spec describes how it works now. |
| In progress | Partly built: the spec says which parts are, and the open parts have issues. |
| Planned | Agreed or proposed, not built yet. |
| Dropped | Discussed and not done; kept so the reasons stay on record. |

## Writing and changing a spec

- **When:** anything beyond a small fix gets a spec, or a change to one, agreed before it is built. A new resource is
  a new spec.
- **New spec:** copy [TEMPLATE.md](TEMPLATE.md) to `NNNN-short-name.md` with the next free number (numbers are never
  reused), and add it to a milestone and to the index below.
- **Front matter:** `number`, `title`, `status` (`done`, `in-progress`, `planned`, `dropped`), `updated`, `versions`
  (the releases that changed it; `unreleased` until a release names it), and optionally `related` and `issues`.
- **Open questions** are GitHub issues (label `question`), listed under `issues`, not paragraphs in the spec.
- **Same commit:** when work lands, its spec's status, versions, and history change with the code and the changelog.

## Index

7 specs: 7 done. Decisions D1–D18.

| # | Spec | Status | Updated |
|---:|---|---|---|
| 1 | [Resources, Provides, and wrappers](0001-resources.md) | Done | 2026-10-21 |
| 2 | [Clock](0002-clock.md) | Done | 2026-10-21 |
| 3 | [Random](0003-random.md) | Done | 2026-10-21 |
| 4 | [Env](0004-env.md) | Done | 2026-10-20 |
| 5 | [Console](0005-console.md) | Done | 2026-10-20 |
| 6 | [Store](0006-store.md) | Done | 2026-10-21 |
| 7 | [Platforms](0007-platforms.md) | Done | 2026-10-20 |

## Milestones

| Milestone | Status |
|---|---|
| [0.2: First release](milestones/v0-2.md) | In progress |
