# CodeGraph — Codebase Exploration

CodeGraph is available as an MCP server with a pre-indexed knowledge graph of the project. For anything involving exploration across multiple files or understanding how pieces connect, prefer CodeGraph over manual file reads — it's faster and gives you the full picture in one call.

For small, targeted tasks (reading one specific file, checking a known line, editing a config) just use `read_file` directly. CodeGraph is a tool for exploration, not a bureaucratic gate.

## Tool Reference

| Tool | Use for |
|---|---|
| `codegraph_explore` | Understanding how something works, tracing a flow, getting an overview of an area |
| `codegraph_search` | Finding where a symbol is defined by name |
| `codegraph_callers` | Who calls a function or method |
| `codegraph_callees` | What does a function call internally |
| `codegraph_impact` | What would break if you changed this — run before refactoring shared symbols |
| `codegraph_node` | Full source + details of a single symbol |
| `codegraph_files` | Project file structure |
| `codegraph_status` | Check index health |

`codegraph_explore` answers most "how does X work?" questions in a single call. The result includes verbatim source of relevant symbols — no need to re-read with `read_file` afterwards.

## When to Use CodeGraph

**Use CodeGraph when:**
- You need to understand how something works across multiple files
- You're tracing a call chain or data flow
- You want to find all callers/callees of a symbol
- You're about to refactor something that other files might depend on → run `codegraph_impact` first

**Skip CodeGraph when:**
- You already know exactly which file and line you need
- You're editing a config, JSON, `.env`, or lock file
- The task touches one file and the change is obvious
- You just edited a file and need to re-read it (the staleness banner `⚠️` is active)

## If `.codegraph/` Doesn't Exist

```bash
codegraph status
```

If not initialized yet:

```bash
codegraph init -i
```

## Examples

**"How does authentication work?"**
```
codegraph_explore("auth flow")
```

**"Where is `parseToken` defined?"**
```
codegraph_search("parseToken")
```

**"What does `UserService.login` call?"**
```
codegraph_callees("UserService.login")
```

**"I want to refactor `DatabasePool`"**
```
codegraph_impact("DatabasePool")  → check blast radius first, then plan the change
```
