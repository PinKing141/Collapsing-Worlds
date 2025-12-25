# Dependency Map

## Current Layout (as-is)
- `src/main.rs` — REPL entry point, wiring.
- `src/core/` — ECS wiring, save/load, action queue.
- `src/rules/` — power rules + usage enforcement.
- `src/simulation/` — world state structs (city, evidence, time, cases).
- `src/systems/` — tick systems (heat, factions, persona, cases).
- `src/content/` — content repository + adapters (SQLite).
- `src/world/` — world repository + adapters (SQLite).
- `src/data/` — JSON loaders (factions, storylets).

### Current dependency flow (observed)
`main → content/world → rules → simulation → systems`

Notes:
- `main.rs` still wires adapters directly.
- `db/` and `persistence/` are legacy shims (not exported).

## Target Layout (architecture goal)
- `domain/` — IDs/enums/structs only.
- `content/` — `PowerRepository` port + loaders.
- `world/` — mutable world state + `WorldRepository` port.
- `rules/` — pure enforcement (no IO).
- `simulation/` — tick systems emitting events.
- `narrative/` — pressure + storylets.
- `ui/` — REPL/TUI.
- `app/` — wiring and adapters.

### Target dependency flow
`ui → app → simulation → rules → domain → util`

## Mapping Notes (current → target)
- `src/db` → `content/adapters`
- `src/persistence` → `world/adapters`
- `src/data` → `content` + `narrative` loaders
- `src/main.rs` → `app` + `ui`
- `src/core` → split into `app` wiring + `domain`
- `src/systems` → `simulation` (event emitters)
