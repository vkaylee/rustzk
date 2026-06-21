---
trigger: always_on
---

# ✨ GEMINI.md - RustZK Context OS

> 🔴 **MANDATORY:** You MUST read and understand this document in its entirety before taking any action. This is the supreme law of this workspace.

<project_soul>
**rustzk** is an Enterprise-Grade, Pure Rust implementation of the ZK protocol for attendance devices.
- **Architecture:** Zero-dependency low-level network protocol parsing (TCP/UDP socket connection wrapper, binary payload decoding, checksum calculation).
- **Quality:** Zero panic mindset. Every input must be validated, and packets must have strict size checks (`validate_packet_size`) to prevent denial-of-service/memory-exhaustion attacks.
- **Security:** Safe communication passwords handling, zero hardcoded credentials, total security auditing for socket connection handling.
- **Mindset:** Maintain premium engineering standards. Standard Rust idiomatic patterns. Leave the library cleaner and more robust than you found it (Boy Scout).
</project_soul>

## 💎 Layer 0: CORE PRINCIPLES (The "Big Five")
> These universal rules CANNOT be overridden by any external skill or lazy-loaded rule.
1. **Premium Library Standard:** EVERY change must compile, pass tests, and conform to strict network safety guidelines. No untested buffer operations.
2. **Robust Error Handling:** Always return `ZKResult` with domain-specific errors (`ZKError`). NEVER throw generic `Error` or use `unwrap()` / `expect()` blindly.
3. **Boy Scout Rule:** If you see a typo, bad pattern, or visible bug nearby, FIX IT IMMEDIATELY. Never say "it's out of scope".
4. **PTY Safety & File Creation (CRITICAL):** 
   - **Execution Pattern:** For ALL terminal commands, you **MUST** append ` > logs/run_{command_slug}.log 2>&1 </dev/null` directly inside the `CommandLine` argument. Always prefix with `mkdir -p logs &&`. Example: `CommandLine: "mkdir -p logs && ls -la > logs/run_ls.log 2>&1 </dev/null"`.
   - **Anti-pattern:** Leaving stdout/stderr un-redirected, which hangs the system.
   - **Script Execution Pattern (2-Step Workflow):** Any code execution (Python, Bash, JS) MUST follow a strict 2-step workflow:
     1. Use the native `write_to_file` tool to create a temporary script (e.g., `scratch/temp_script.py`).
     2. Execute the file via `run_command` (e.g., `python scratch/temp_script.py > logs/run_script.log 2>&1`).
   - **Pre-Flight Check for Multiline:** ALWAYS check if your command requires multiline inputs, complex formatting, or inline execution (`python -c`, `bash -c`, multiline `echo`/`cat`). If it does, YOU MUST USE the `write_to_file` tool to save it as a physical file first. Multiline strings in the terminal break the PTY parser and cause system hangs. Use `write_to_file` as your best friend!
   - **Log Monitoring Pattern:** Use **periodic sampling** (e.g., one-shot reads via `tail -n 100`, `cat`, or native `view_file` tool) to inspect states safely.
   - **Anti-pattern:** Using infinite-blocking streams (`tail -f`, `watch`, `top`, `ping`) via `run_command` in background mode. These commands hang the PTY and pollute the task list forever.
5. **Hermetic Wrappers (CRITICAL):** DO NOT call `cargo` directly because the host does not have cargo installed. You MUST run all cargo commands via the wrapper script `./cargo.sh` in the root of the workspace (e.g., `./cargo.sh check`, `./cargo.sh test`).
6. **AI MUST BE CALM & PATIENT (CRITICAL):** This is a fundamental mindset for all situations. DO NOT RUSH.
    - Read requirements carefully before starting.
    - Cross-check risks before modifying source code.
    - Never jump to conclusions based on assumptions or subjective guesses.
    - Always verify with concrete evidence before responding.
    - Always wait for processes to finish, carefully read error logs (if any), and only then report back to the user. Never say "it's done" without actually verifying the outcome.
7. **AI MUST EXPLORE BEFORE CREATING (ANTI-REINVENTION):**
    - You MUST use `list_dir`, `grep_search`, or read root-level scripts to verify that a tool doesn't already exist BEFORE creating any new scripts (bash/python).
    - AI tends to ignore existing large codebase tools and writes new ones, causing context bloat and technical debt. DO NOT REINVENT THE WHEEL.

## 🥇 Layer 1: CRITICAL PROTOCOL
**Before writing ANY code or proposing solutions, you MUST:**
1. **MANDATORY Lazy-Load:** You MUST run `view_file` on related rulebooks and explicitly output 'Read: [Rule Name]' in your Socratic response BEFORE generating any code.
2. **Selective Loading:** Only load rule files when needed for the task.
3. **MANDATORY Rule Compliance Check:** For COMPLEX CODE or DESIGN tasks, create `{task-slug}.md` and list at least 3 Rule IDs.

## 📥 Layer 2: REQUEST CLASSIFIER & ROUTING
- Classify request: QUESTION, SURVEY, SIMPLE CODE, COMPLEX CODE, DESIGN/UI.
- Detect domain and announce: `🤖 **Applying knowledge of @[agent-name]...**`
- Cross-Agent Handoff: Contract First (API before UI), Zero Assumptions.

## 🛑 Layer 3: SOCRATIC GATE
- For New Features / Bug Fixes: 🔴 STOP and ASK minimum 3 strategic questions. Confirm understanding. Do not implement based on assumptions.

## 📚 Layer 4: DOMAIN RULEBOOKS (Lazy Load Index)
> 🔴 **MANDATORY:** Before executing a task, you MUST read the relevant dictionary files using the `view_file` tool. Do NOT guess the rules.

### 💻 Code & Architecture
- **Coding Standards:** `@[.agents/rules/coding-standards.md]` (Clean code, errors, security, enterprise grade)
- **Testing Standards:** `@[.agents/rules/testing-standards.md]`
- **Execution Wrappers:** `@[.agents/rules/execution-wrappers.md]` (cargo wrapper, docker-compose)
- **Change Management:** `@[.agents/rules/change-management.md]` (Code review, branch protection, deployment gates, feature flags)
- **Configuration Management:** `@[.agents/rules/configuration-management.md]` (Env vars, fail-fast validation, environment parity)

## 🔌 External Skill Integration (Plugin Protocol)
To prevent capability limitations, this workspace supports external AI skills:
1. **Persona-Bound:** External skills must be registered directly inside the specific `.agents/agent/*.md` file. Do NOT load global skills randomly.
2. **Conflict Resolution:** Project internal rules (`.agents/rules/`) **ALWAYS OVERRIDE** external skill instructions. Internal context is king.

## 🤖 Layer 5: AGENTS & SKILLS (Concurrent Pre-fetching)
To activate an agent, use the `view_file` tool to load their profile AND their dependencies IN PARALLEL:
- **Backend / Rust / Network:** `@[.agents/agent/backend-specialist.md]` (Dependencies: `coding-standards.md`, `access-control.md`)
- **DevOps / Container:** `@[.agents/agent/devops-engineer.md]` (Dependencies: `execution-pty-safety.md`, `testing-standards.md`)
