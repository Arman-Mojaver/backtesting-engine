---
description: Create well-formatted commits with conventional commit messages
---

# Commit Command

You are an AI agent that helps create well-formatted git commits using plain conventional commit messages (no emojis, ever). Follow these instructions exactly. Create the commit, but NEVER push — pushing is the user's responsibility.

## Instructions for Agent

When the user runs this command, execute the following workflow:

1. **Check command mode**:
   - If user provides $ARGUMENTS (a simple message), skip to step 3

2. **Run pre-commit validation** (only if the project defines them):
   - Detect package manager / tooling (e.g. `package.json`, `Cargo.toml`)
   - Run the project's lint and build/test commands if they exist
   - If either fails, ask the user whether to proceed or fix first
   - Skip silently if no such tooling is defined

3. **Analyze git status**:
   - Run `git status --porcelain` to check for changes
   - If no files are staged, run `git add .` to stage all modified files
   - If files are already staged, proceed with only those files

4. **Analyze the changes**:
   - Run `git diff --cached` to see what will be committed
   - Determine the primary change type (feat, fix, docs, etc.)
   - Identify the main scope and purpose of the changes

5. **Generate commit message**:
   - Check staged file paths:
     - If **ALL** staged files are inside `.opencode/` → use prefix `ai: <description>` (replaces the conventional type entirely)
     - Otherwise → use standard `<type>: <description>`
   - Format rules: NO emoji, NO prefix glyphs, plain text only
   - Keep description concise, clear, and in imperative mood
   - Show the proposed message to the user for confirmation

6. **Execute the commit**:
   - Run `git commit -m "<generated message>"`
   - Display the commit hash and confirm success
   - DO NOT push. Never run `git push`. The user pushes manually.

## Commit Message Guidelines

- **No emojis. Ever.** Plain text only.
- **`.opencode/`-only commits use `ai:` prefix** instead of a conventional type. Applies only when **every** staged file lives under `.opencode/`. Mixed commits use the normal conventional type.
- **Atomic commits**: each commit serves a single purpose
- **Imperative mood**: "add feature", not "added feature"
- **Concise first line**: under 72 characters
- **Format**: `<type>: <description>` where type is one of:
  - `feat`: A new feature
  - `fix`: A bug fix
  - `docs`: Documentation changes
  - `style`: Code style changes (formatting, etc.)
  - `refactor`: Code changes that neither fix bugs nor add features
  - `perf`: Performance improvements
  - `test`: Adding or fixing tests
  - `chore`: Build process, tooling, configuration
  - `ci`: CI/CD changes
  - `revert`: Reverting changes

## Reference: Good Commit Examples

- feat: add user authentication system
- fix: resolve memory leak in rendering process
- docs: update API documentation with new endpoints
- refactor: simplify error handling logic in parser
- fix: resolve linter warnings in component files
- chore: improve developer tooling setup process
- feat: implement business logic for transaction validation
- fix: address minor styling inconsistency in header
- fix: patch critical security vulnerability in auth flow
- style: reorganize component structure for better readability
- fix: remove deprecated legacy code
- feat: add input validation for user registration form
- fix: resolve failing CI pipeline tests
- test: add unit tests for authentication flow

### `.opencode/`-only commit examples (use `ai:` prefix)

- ai: add session takeaway entry for permissions config
- ai: tighten bash permission allowlist in opencode.json
- ai: update commit command to forbid push

## Agent Behavior Notes

- **No emojis under any circumstance** — neither in the message subject nor body.
- **Error handling**: If validation fails, give the user the option to proceed or fix first.
- **Auto-staging**: If no files are staged, stage all changes with `git add .`.
- **File priority**: If files are already staged, only commit those.
- **Always run the commit, never push**: never run `git push`. Pushing is the user's responsibility.
- **Message quality**: clear, concise, conventional format.
- **Success feedback**: after commit, show the commit hash and a brief summary.
