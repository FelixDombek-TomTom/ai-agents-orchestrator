# Untracked resumable sessions in the Closed tab

**Date:** 2026-06-21
**Status:** Part 1 approved (pending spec review). Part 2 (Adopt) deferred — see end.

## Problem

The Closed/Stale/Archived tabs only show sessions created through the app's own
`/start`→`/close` workflow (they scan `<root>/<CATEGORY>/*/notes.md`). Sessions
you ran in Claude Code *without* the app — the common case — never appear, so
there is no way to find and resume them from the dashboard, despite the README
promising "a unified view of every session, across every project." This adds
those untracked sessions to the Closed tab as resumable cards.

## What the data actually looks like (measured 2026-06-21)

- **`~/.claude/sessions/`** — a *live-process registry*. Files named by PID, one
  per running `claude` process; removed on exit. **No history** (3 files now, all
  alive). Unusable as a catalog of past sessions.
- **`~/.claude/projects/`** — the durable transcript archive: 556 `.jsonl` (one
  per session, filename = session UUID), in 14 directories whose **names encode
  the launch cwd** (`-home-felix-dombek-code`, …). ~1 month of history.
  - **487 of 556 (88%) are subagent runs**, all in a single `-tmp` directory.
  - **69 are real user sessions** in the `-home-*` dirs.
  - Of the 69: 60 have an `ai-title`, 6 have a `last-prompt`, 3 have neither.
- **Key simplification:** filtering by **directory name** (`-tmp*` / `-var-tmp*`)
  matches the cwd-based `/tmp` filter with **0 mismatches across all 556 files**.
  So subagents are excluded with no file reads, leaving ~69 files to parse.

## Catalog source decision

`projects/*.jsonl` is the catalog (the only source with history). `sessions/*.json`
is used only to identify currently-live session ids to exclude.

## Architecture

New backend scan produces "untracked" session objects folded into the existing
Closed bucket; the renderer shows them with a transcript-derived title, an
"untracked" badge, and a Resume action. Pure read + the existing launcher resume
— ADR-001 (read-only `~/.claude`) and ADR-012 (launcher, not writer) intact. No
new IPC channel (ADR-005): results flow through the existing
`get_historical_sessions` / `get_historical_sessions_all` commands.

### Backend (`src-tauri/src/reader.rs`)

**1. Extend the transcript fold.** `read_transcript()` already folds each JSONL
once for `gitBranch`/`cwd`/last activity. Add two fields to `Transcript`:
- `ai_title: Option<String>` ← record `type == "ai-title"`, field `aiTitle`
- `last_prompt: Option<String>` ← record `type == "last-prompt"`, field `lastPrompt`

**2. New `scan_untracked() -> Vec<Value>`:**
- List `~/.claude/projects/` directories; **skip** any whose name starts with
  `-tmp` or `-var-tmp` (cheap subagent prefilter, no file reads).
- For each `*.jsonl` in the remaining dirs: `sessionId` = filename stem (validate
  it's a UUID-shaped id). Read the (cached) transcript for title/cwd/branch/
  last-activity/launch_cwd.
- **Exclude** a session when:
  - its `sessionId` is currently live (in `running_session_ids()`), or
  - its `sessionId` is already surfaced by `scan_historical()` (app-managed —
    dedup), or
  - it has no resolvable launch cwd (nothing to resume into).
- **Defense in depth:** also drop any survivor whose resolved cwd is under
  `/tmp`/`/var/tmp` (in case the dir-name prefilter ever diverges).
- Emit a session object shaped like the historical ones, with:
  - `state: "closed"`, `untracked: true`
  - `name`: `aiTitle` → else `lastPrompt` truncated to ~80 chars → else the
    8-char short UUID
  - `sessionId`, `cwd` (launch cwd, the resume key), `gitBranch`,
    `lastActivity`/`lastActivityAt`, `updatedAt` = transcript file mtime (ISO)
  - `category`/`goal`/`nextSteps`/`notesPath`/`ticket`/`prLink` = null
- Sort by `updatedAt` desc. Cap to the 50 most recent (with ~69 total this is a
  safety bound, not a routine truncation); `log`/comment notes the cap.

**3. Caching.** Reuse the existing `(len, mtime)` `TRANSCRIPT_CACHE` so unchanged
transcripts are not re-parsed each 5s poll. With ~69 parsed files the cost is
small; the cache keeps it flat.

**4. Wire in.** `get_historical_sessions("closed")` and
`get_historical_sessions_all()` append `scan_untracked()` to the `closed` bucket.
`stale`/`archived` are unaffected.

### Frontend (Closed tab — `renderer/`)

- Render untracked cards from the same closed list, distinguished by
  `untracked === true`:
  - title as the card name; an **"untracked"** badge in place of the category
    chip; short UUID, last-activity, and branch/cwd shown; no goal/next-steps row.
  - Merge-sorted by recency with app-closed cards.
  - Flow through the existing search box; with the category filter set to a
    specific category, untracked cards (which have none) are hidden — they show
    under "all"/no-category.
- **Actions:** Resume only — the existing embedded-terminal resume and
  "open in your own terminal" path (`claude --resume <sid>` in the card's cwd),
  reachable from the card / detail slide-over. **No Restart, no Archive** for
  untracked cards (both require a `notes.md`).

## Testing

- **Backend unit tests** (pure classifier, fixture `projects/` tree):
  - `-tmp`/`-var-tmp` dirs excluded without reads; real dirs included.
  - exclusions: live sid, already-tracked sid, no-cwd, post-filter `/tmp` cwd.
  - label fallback chain: ai-title → last-prompt(trunc) → short uuid.
  - dedup against a scan_historical sid; sort-by-recency; cap at 50.
- **Frontend:** untracked card renders the badge, omits category/goal, exposes
  Resume but not Restart/Archive.

## Out of scope (Part 1)

- **Adopt** (→ Part 2 below).
- Live PR / CI / review status enrichment (gwt-ls does this via `gh`).
- Content-search *inside* transcripts (gwt-ls `--find`); Closed search stays
  title/name-based.
- Pagination / "show more" beyond the 50 cap.

## Reference: gwt-ls (`~/code/my/gwt-ls`)

The companion CLI already lists worktrees + Claude sessions and informed this
design: transcript-derived titles (`ai-title`/`last-prompt`), `/tmp` exclusion
(`_is_tmp_cwd`), resume by short-UUID prefix, live detection via SessionStart/
SessionEnd hooks, and `--find` content search. Part 1 borrows the title
extraction and `/tmp` handling; the live-hooks and content-search are noted as
future ideas.

---

## Part 2 — Adopt (DEFERRED to a follow-up spec)

**Goal:** from an untracked card, "Adopt" the session into app management so it
gains a category + `notes.md` and appears as a normal tracked session.

**Agreed scope (from brainstorming, not yet designed in detail):**
- An **Adopt** action on untracked cards, alongside Resume.
- A **category picker** in the untracked card's detail slide-over.
- Implemented as a **launcher action** (ADR-012 compliant): the app does not
  write state itself — it launches `claude` with a skill that does the writing.
  Needs a new or extended skill (e.g. `/adopt <sid> <CATEGORY>`) that creates the
  category workspace + `notes.md` seeded from the session's transcript (title,
  cwd, branch, session id) and registers it in `active-sessions.json`, then
  resumes.

**Why deferred:** Part 1 is read-only and ships value alone; Adopt adds a write
path + a skill + UI, and is cleanly separable. It gets its own spec → plan →
implementation cycle.
