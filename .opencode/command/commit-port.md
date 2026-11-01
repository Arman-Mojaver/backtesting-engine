---
description: Port a commit onto the current branch — auto-resolves non-semantic conflicts, prompts on semantic ones, and proves the port matches the commit
---

# Commit Port Command

You are an AI agent that ports a single commit onto the currently checked-out branch. Follow these instructions exactly. The commit almost always belongs on this branch, but the branch has evolved (refactors, renames, line moves), so git often reports textual conflicts that are NOT semantic conflicts. Resolve those yourself. Only involve the user when the commit would genuinely change something the branch deliberately set to a different value.

The commit to port: `$ARGUMENTS`

## Core Rules

- NEVER switch, create, or check out branches — everything happens on the current branch.
- NEVER run `git commit` and NEVER leave anything staged at the end — the user stages and commits manually.
- NEVER push.
- Auto-resolve non-semantic conflicts without prompting. Prompt the user ONLY for semantic conflicts or big issues (dirty tree, missing commit, merge commit, semantic deviation found during verification).
- ALWAYS run the equivalence check (step 4) before reporting — never claim a faithful port without the proof.

## Workflow

### 1. Validate and inspect (read-only)

- If `$ARGUMENTS` is empty, ask the user which commit to port and wait.
- Run `git show <commit>` — full diff. Run `git show -s --format=%B <commit>` — the exact commit message; save it verbatim for the final report.
- If the commit does not exist, report the error and stop.
- Run `git status`:
  - Clean tree → continue.
  - Dirty files that do NOT overlap the commit's files → warn the user once and continue.
  - Dirty files that DO overlap the commit's files → stop and ask (stash/commit first or abort).
- If the commit is a merge commit, ask the user which parent line to port (suggest first parent as default) — cherry-pick needs `-m <n>`.

### 2. Attempt the mechanical port

- Run `git cherry-pick -n <commit>` (applies the changes without committing).
- Applies cleanly → skip to step 4.
- Conflicts → step 3.
- If cherry-pick wedges in a broken state, `git cherry-pick --abort`, then fall back to applying the diff by hand with the edit tool, guided by `git show <commit>`.

### 3. Resolve conflicts

List unmerged files with `git status`, then read every conflict hunk and classify it:

**A. Non-semantic conflict — auto-resolve, do NOT prompt**

The conflict exists only because surrounding code on this branch moved, was renamed, or was refactored. The commit's actual change still applies cleanly in spirit to the same logical location; git just can't line the text up.

→ Resolve it yourself: place the commit's change at the correct location within the branch's refactored code (move things up or down as needed). Preserve the branch's refactor. Include ALL coherent changes from the commit.

**B. Semantic conflict — STOP and prompt the user**

The commit changes a line whose current value on the branch is a different intentional value. Applying it would silently overwrite a deliberate branch decision.

Example: branch has `MY_ENV_VAR = 5`, the commit changes it to `MY_ENV_VAR = 4` → ask the user.

→ Collect ALL semantic conflicts first, then present them together: for each, show the file and line, the branch's version, and the commit's version, and ask which to keep (or how to combine them). Apply the user's decisions.

**How to tell them apart**: is the branch's conflicting content just the same code in a different place or shape (refactor) → non-semantic. Is it a different value or decision on the same logical line → semantic. When genuinely ambiguous, treat it as semantic and ask.

**Modify/delete conflicts**: if the file was renamed/moved/refactored on this branch, port the change to its new location (non-semantic). If the branch deliberately deleted the file, treat it as semantic and ask.

After every conflict is resolved in the working tree (no conflict markers remain anywhere):

- `git add` the resolved files — this only clears git's unmerged state; it gets unstaged in step 5.
- Run `git cherry-pick --quit` if `git status` still shows a cherry-pick in progress.

### 4. Verify — prove the port carries exactly the commit's changes

Run this BEFORE the `git reset` in step 5, while the port is still staged, so `git diff HEAD` captures the complete port.

The ported code will almost never be textually identical to the commit's post-image — the branch's refactor is intentionally preserved — so do NOT diff the working tree against the commit directly; that would show all the branch's refactoring and prove nothing. Instead, compare the CHANGE the port leaves on the branch against the CHANGE the commit made, ignoring position, context, file paths, and whitespace:

- The commit's change: `git diff <commit>^ <commit>` (for a merge commit, diff against the chosen parent `<commit>^<n>`).
- The port's change: `git diff HEAD`.

**Modified or deleted files** — compare the added/removed lines of both diffs as sets (order- and position-independent):

```bash
diff <(git diff <commit>^ <commit> -w --no-color -- <commit's paths> | grep -E '^[+-]' | grep -vE '^(\+{3}|-{3})' | sort) \
     <(git diff HEAD -w --no-color -- <ported paths> | grep -E '^[+-]' | grep -vE '^(\+{3}|-{3})' | sort)
```

Scope each side to its own paths — after a rename, the commit's side uses the old path and the port's side uses the new one. `-w` makes the check ignore whitespace/indentation drift.

- **Empty output** → the port added and removed exactly the same lines as the commit, just possibly at different positions. This is the guarantee: no actual difference.
- **Non-empty output** → the residual lines are exactly what the port did differently from the commit. Classify each residual:
  - *Benign adaptation* — forced by the branch's refactor: an identifier renamed on the branch so the change is re-expressed with the new name, a different path after a file move, reformatting, or lines that already exist on the branch (partially applied earlier). Explain each in the report; no prompt.
  - *Semantic deviation* — the port changed something the commit did not change, or dropped or altered part of the commit's change. Show the user the residual diff and ask what to do before finishing.

**New files added by the commit** — a new file cannot drift, so verify it directly and expect it to be identical:

```bash
diff <(git show <commit>:<path>) <ported path>
```

Empty output → identical. (New files are checked separately because after finalization they are untracked and invisible to `git diff`.)

If the user rejects the port at any point, undo it scoped: `git restore --source=HEAD --staged --worktree -- <ported files that exist on HEAD>` and delete any files the port created (they are not in HEAD). NEVER `git reset --hard`.

### 5. Finalize — leave nothing staged

- Run `git reset` (mixed). This unstages everything while keeping all ported changes in the working tree as unstaged modifications (new files become untracked).
- Verify with `git status` and `git diff`:
  - all ported changes are present in the working tree
  - NOTHING is staged
  - no cherry-pick/merge/rebase state remains
- If the working-tree diff is empty, the commit was likely already applied — report that and stop without leaving a commit line.

### 6. Report

Give the user, in order:

1. **Port summary** — files ported; for every conflict, how it was classified and resolved (including the user's semantic decisions); the verification result (clean, or each residual explained).
2. **The prepared verification command(s)** — the exact commands from step 4 with the real commit hash and paths filled in, so the user can re-run them and see the empty result for themselves. They work both before and after finalization (new-file checks use `git show` and always work).
3. **The reference commit line**, using the original commit message verbatim:

   ```
   git commit -m "<message from the ported commit>"
   ```

   If the original message has a body, add one `-m "<paragraph>"` per paragraph.

4. **Reminder** — stage files manually (`git add ...`), then run the commit line above.

Do NOT stage anything. Do NOT commit. Leave the changes in the working tree, the proof, and the commit line as reference.
