# 📖 Documentation Engine (v1.0.0)

> This skill ensures that documentation is never an afterthought and stays perfectly synced with implementation.

## 1. Documentation-as-Code
- **Sync Point**: Every significant feature change MUST be accompanied by an update to the relevant `README.md` or `docs/*.md` in the same task.
- **Self-Documenting Code**: Prefer clear variable naming and small functions over heavy inline comments.

## 2. Machine-Readable Context
- Maintain indices in `docs/` for AI agents to perform rapid lookup.
- Ensure all technical decisions are logged in the `docs/adr/` (Architecture Decision Records) directory.

## 3. Language Governance
- Strictly enforce the **English-only** rule in all `.md` files.
- Use clear, concise language. Avoid jargon where simple words suffice.
