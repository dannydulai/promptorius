# Core Beliefs

These principles guide design and implementation decisions in promptorius. When in doubt, refer here.

1. **Scripts are the unit of extensibility.** There are no hardcoded prompt modules. If it renders, it's a `.rhai` script. The binary provides primitives; scripts provide behavior.

2. **The config is layout, not logic.** `config.toml` declares *what* appears and *where*. Scripts decide *how*. Format templates can contain Rhai expressions for glue logic, but heavy logic belongs in scripts.

3. **Fast or invisible.** A prompt that takes >50ms feels broken. Every design choice is evaluated against this budget. If a feature can't be fast, it should degrade gracefully (timeout and omit).

4. **Boring dependencies.** Prefer well-known, stable crates. Avoid large framework dependencies. If a small piece of functionality can be implemented in <100 lines with full test coverage, prefer that over pulling in an opaque upstream.

5. **Color is a palette, not inline markup.** Colors are defined once in `[colors]` as a named map. Scripts reference semantic names. Users change colors in one place.

6. **The CLI is the only interface.** No daemon, no socket, no background process. The shell calls the binary, gets a string back. Statelessness keeps things simple and debuggable.

7. **Agent-legible code.** Code should be straightforward enough that an agent (or a new contributor) can understand module boundaries, data flow, and invariants from the source and these docs alone.
