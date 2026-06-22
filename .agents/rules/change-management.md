# 🔄 Change Management Rules (SOC 2 CC8.1 / ISO 27001 A.8.32)

## 1. Code Review Policy (MANDATORY)
> 🔴 **No code reaches `main` without review. No exceptions.**

- **Minimum Reviewers:** Every pull request MUST receive at least **1 approved review** before merge.
- **Security-Sensitive Changes:** PRs touching encryption, network socket protocol, ZK checksum modifications require **2 approved reviews**, at least one from a senior engineer.
- **Self-Merge Ban:** The PR author MUST NOT approve their own PR.
- **Review Scope:** Reviewers MUST verify: correctness, security implications, test coverage, backward compatibility, and adherence to coding standards.

## 2. Branch Protection Rules (MANDATORY)
| Branch | Direct Push | Required Reviews | Status Checks | Force Push |
|--------|-------------|-----------------|---------------|------------|
| `main` | ❌ Blocked | 1 (2 for security) | All CI checks pass | ❌ Blocked |
| Feature branches | ✅ Allowed | N/A | N/A | ✅ Allowed |

- **Branch Naming:** Feature branches MUST follow `<type>/[task-slug]` (e.g., `feature/[task-slug]` or `fix/[bug-slug]`).
- **Branch Lifecycle:** Feature branches MUST be deleted after merge.

## 3. Emergency Hotfix Process
> For P1/P2 security incidents or critical bugs ONLY.

- **Review:** 1 reviewer (any senior engineer).
- **Deploy:** Immediate.
- **Retroactive Review:** Emergency PRs MUST receive a full review within 24 hours of deployment.

## 4. Breaking Change Policy
> 🔴 **Breaking changes require version bumping.**

- **Definition:** Any change that changes public API signatures (rustzk module traits, structs, public functions/types).
- **SemVer Compliance:** Standard Rust Semantic Versioning (SemVer) rules apply:
  - Bump Major for breaking changes.
  - Bump Minor for backwards-compatible new features.
  - Bump Patch for backwards-compatible bug fixes.

## 5. Rulebook and Standard Changes (AI Proposals)
- AI agents and developers MUST NOT modify rulebooks or core standard files in `.agents/rules/` and `GEMINI.md` autonomously without direct, explicit user approval.
- If a technical debt resolution or refactoring reveals a gap or an outdated policy in the core rules:
  1. The AI agent MUST outline a recommended addition or modification in the final walkthrough report or pull request comments.
  2. The changes to `.agents/rules/` or `GEMINI.md` may only be merged and applied after Lead Engineer reviews and approves the proposal.
