# ai-context-os: The AI-Native Operating System for Software Projects

> **"Don't generate code. Orchestrate it."**

## 🎯 Purpose

`ai-context-os` is an **installable LLM Orchestrator Framework** designed to manage the intelligence, constraints, and context of AI Agents (Cursor, Claude, Antigravity) within any repository. It treats "AI Context" as **Infrastructure-as-Code**.

This OS uses a **Fallback Architecture (Inheritance)**. Users can define their own project-specific skills and protocols, but if none exist, the AI automatically falls back to the robust, standardized `ai-context-os` core.

### Problem Solved
1.  **Eliminates Overlap**: Prevents conflicting AI skills.
2.  **Enforces SSOT**: Ensures all agents follow `PROJECT_OS.md`.
3.  **Standardizes Interaction**: Creates a unified protocol for AI-to-System communication.
4.  **Graceful Fallback**: Custom rules always win, but a safe standard is always there.

---

## 📂 Architecture

```text
├── PROJECT_OS.md       # L0: The Kernel (Single Source of Truth)
├── CLAUDE.md           # L1: Adapter for Claude/Antigravity
├── .cursorrules        # L1: Adapter for Cursor AI
├── .agent/             # L1: Adapter for Generic Agents
├── skills/             # L2: Modular Capabilities (React, Rust, etc.)
└── docs/               # L3: Human-Readable Context
```

## 🚀 Getting Started

Most projects you work on will already exist. Therefore, the best way to leverage `ai-context-os` is to integrate its core files into your existing repository rather than forking.

### Option 1: Automated Install (Pointer Pattern - Recommended)
You can use the provided install script to automatically copy the core files into a hidden `.ai-context-os/` folder in your project and create pointer files in the root:

```bash
# From the project where you want to install
npx ai-context-os .
# Or to point to a specific directory
npx ai-context-os /path/to/your/existing/project
```
*Why this is better:* Keeps your project root clean. You will only see `.cursorrules` and `CLAUDE.md` in your root which act as pointers to the true OS rules inside `.ai-context-os/`.

### Option 2: Manual Install
1.  **Create OS Folder**: `mkdir .ai-context-os` in your project root.
2.  **Integrate Core Files**: Copy `PROJECT_OS.md`, `.cursorrules`, `CLAUDE.md`, and the `skills` folder into `.ai-context-os/`.
3.  **Boot the AI via Pointers**: Create a `.cursorrules` or `CLAUDE.md` file in the root of your existing project mapping to the files in `.ai-context-os/`.

## 📜 Core Protocols

- **Protocol-First**: Rules in `PROJECT_OS.md` override any AI training.
- **Context Hygiene**: Always scout before implementing.
- **Modularity**: Files < 200 lines. Docker for execution.

---

## 🤝 Contributing

We welcome PRs that improve the OS kernel or add new L1 adapters.
