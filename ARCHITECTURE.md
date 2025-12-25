# Superhero Sim Architecture Rules

## Hard Boundaries
- Content is read-only at runtime: `powers.db` (powers/expressions/acquisition/costs/signatures/text).
- World state is mutable and save-scoped: `world.db` (heat, factions, cases, time, evidence, personas, story state).
- No cross-writes: never write to `powers.db`, never store world state inside content tables.

## Dependency Rule (Inward Only)
`ui → game → simulation → rules → domain → util`

Forbidden:
- rules importing UI or DB
- simulation importing UI
- DB layer calling rules logic

## Ports and Adapters
All IO behind interfaces (ports), with concrete adapters:
- Ports (traits): `PowerRepository`, `WorldRepository`, `Clock`, `Rng`, `EventSink`
- Adapters: `SqlitePowerRepository`, `SqliteWorldRepository`, `StdRngAdapter`, `FileSaveAdapter`

Core logic never depends on SQLite directly.

## Determinism
- Reproducible from `world_seed`, `tick`, and player inputs.
- One RNG stream per system (`rng_factions`, `rng_incidents`, `rng_names`, etc.).
- Never call global randomness from simulation/rules.
- Log every player-visible random outcome.

## Event Sourcing
Systems emit events; a resolver applies mutations:
- `FactionEvent`, `IncidentEvent`, `EvidenceEvent`, `PersonaEvent`, `NemesisEvent`

No system directly mutates another system’s state.

## Single Source of Truth (Rules)
All gameplay enforcement flows through:
- `can_use(...)`
- `use_power(...)`
- `switch_persona(...)`

UI/AI only call “can I?” then “do it.”

## State Normalisation
Store canonical IDs only (`power_id`, `expression_id`, `persona_id`, `faction_id`, `location_id`).
Caches must be rebuildable and disposable.

## Schema Versioning
Both databases must carry:
- `schema_version`
- `content_version` (content DB)
- `save_version` (world DB)

On load: migrate or refuse with a clear error.

## Save Safety
- Atomic save (temp → fsync → rename).
- Validate loads in debug builds.
- Forward-compatible (additive changes, defaults).

## Performance
- Avoid hidden O(N²).
- Maintain indices (evidence by location, cases by faction, actors by location, hotspot lists).
- Process only active areas.

## Tick Order Contract
1) Decay phase (cooldowns, evidence persistence, heat decay)
2) World generation (incidents, crime pressure)
3) Faction phase (detect → decide → emit events)
4) Nemesis phase (learn → adapt → act)
5) Narrative pressure phase (pressure modifiers)
6) Resolve phase (apply events to world state)
7) Commit phase (checkpoint if needed)

## Content Contract
All content is validated at load:
- Required fields exist
- Referential integrity holds
- No “fixed in code” power special-cases

## Debugability
Every system must provide:
- `stats()` dump
- `trace` mode for decisions
- REPL hooks to force events (dev-only)

## Tests (Minimum)
- Rules: `use_power` applies costs and emits signatures
- Determinism: same seed + inputs → same outcomes
- Schema: content/world DB versions load correctly
- Integration: tick loop produces incidents → factions respond → cases progress

## Ownership Rule
Only the owner module mutates its state:
- Evidence: `WorldEvidence` + resolver
- Heat: `HeatSystem` + resolver
- Persona: `PersonaSystem`
- Nemesis: `NemesisSystem`

## Recommended Module Layout
- `domain/` (IDs, enums, structs only)
- `content/` (PowerRepository + loaders)
- `world/` (mutable state + WorldRepository)
- `rules/` (pure enforcement)
- `simulation/` (systems emitting events)
- `narrative/` (pressure engine + storylets)
- `ui/` (REPL/TUI)
- `app/` (main wiring)

