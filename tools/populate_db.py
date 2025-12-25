import argparse
import hashlib
import json
import sqlite3
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, Set, List, Tuple, Optional

DB_PATH = Path(__file__).resolve().parent.parent / "Superpower_list.db"
LOCALE = "en-GB"


# -------------------------
# Helpers
# -------------------------

def stable_id(*parts: str) -> str:
    raw = "|".join(parts).encode("utf-8")
    return hashlib.sha1(raw).hexdigest()


def table_exists(conn: sqlite3.Connection, name: str) -> bool:
    row = conn.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name=?",
        (name,)
    ).fetchone()
    return row is not None


def require_tables(conn: sqlite3.Connection, names: List[str]) -> None:
    missing = [n for n in names if not table_exists(conn, n)]
    if missing:
        raise RuntimeError(f"Missing required tables: {missing}")


def load_power_tags(conn: sqlite3.Connection) -> Dict[str, Set[str]]:
    tags: Dict[str, Set[str]] = {}
    for power_id, tag in conn.execute("SELECT power_id, tag FROM power_tag"):
        tags.setdefault(str(power_id), set()).add(str(tag).strip().lower())
    return tags


def _extract_kind(raw_tags: Set[str]) -> str:
    for tag in raw_tags:
        if tag.lower().startswith("kind:"):
            return tag.split(":", 1)[1].strip()
    return ""


def load_powers(conn: sqlite3.Connection) -> List[Tuple[str, str, str]]:
    # Superpower4(rowid, name, tags)
    rows = conn.execute("SELECT rowid, name, tags FROM Superpower4").fetchall()
    out: List[Tuple[str, str, str]] = []
    for pid, name, tags in rows:
        # Prefer explicit kind: tag, otherwise empty
        tag_set = set()
        if tags:
            tag_set = {t.strip().lower() for t in str(tags).split(",") if t.strip()}
        kind = _extract_kind(tag_set)
        out.append((str(pid), str(name), kind))
    return out


def load_kinds(conn: sqlite3.Connection) -> Set[str]:
    tags = load_power_tags(conn)
    kinds: Set[str] = set()
    for tset in tags.values():
        k = _extract_kind(tset)
        if k:
            kinds.add(k)
    return kinds


# -------------------------
# 1) Seed expression templates
# -------------------------

def seed_expression_templates(conn: sqlite3.Connection) -> None:
    require_tables(conn, ["expression_template"])

    # Templates aligned to observed top tags; no reliance on sparse kind:* categories.
    templates = [
        # Touch (interaction-driven)
        ("tmpl_touch_direct", None, ["touch", "absorb", "copy", "change", "pain", "control", "manipulation"], "TOUCH", "INSTANT", "STREET",
         {"requires_contact": True, "cooldown": 1}, 85),
        ("tmpl_touch_psychic", None, ["mind", "mental", "psychic"], "TOUCH", "INSTANT", "STREET",
         {"requires_contact": True, "cooldown": 2, "cost": {"focus": 2}}, 70),
        ("tmpl_touch_soul", None, ["death", "life", "soul", "ghost"], "TOUCH", "INSTANT", "STREET",
         {"requires_contact": True, "cooldown": 2, "cost": {"focus": 2}}, 60),

        # Energy / elemental projection
        ("tmpl_beam_energy", None, ["energy", "light", "heat", "electricity", "fire"], "BEAM", "INSTANT", "STREET",
         {"range_m": 25, "requires_los": True, "cooldown": 2, "cost": {"stamina": 2}}, 85),
        ("tmpl_proj_element", None, ["ice", "water", "earth", "air", "shadow", "dark", "darkness"], "PROJECTILE", "INSTANT", "STREET",
         {"range_m": 20, "cooldown": 2, "cost": {"stamina": 2}}, 70),
        ("tmpl_zone_element", None, ["fire", "ice", "water", "shadow", "darkness", "electricity"], "ZONE", "TOGGLED", "STREET",
         {"radius_m": 6, "cost_per_tick": {"stamina": 2}}, 45),

        # Control / manipulation
        ("tmpl_zone_control", None, ["control", "manipulation", "change", "pain"], "ZONE", "TOGGLED", "STREET",
         {"radius_m": 7, "cost_per_tick": {"focus": 2}}, 80),
        ("tmpl_aura_control", None, ["control", "manipulation", "absorb"], "AURA", "TOGGLED", "STREET",
         {"cost_per_tick": {"focus": 1}}, 55),

        # Movement / speed / space
        ("tmpl_move_speed", None, ["speed", "air"], "MOVEMENT", "TOGGLED", "STREET",
         {"cost_per_tick": {"stamina": 1}}, 55),
        ("tmpl_move_space", None, ["space"], "MOVEMENT", "TRIGGERED", "STREET",
         {"range_m": 15, "cooldown": 3}, 45),

        # Summoning / ghost / soul / death
        ("tmpl_summon", None, ["summoning", "ghost", "death", "soul"], "SUMMON", "INSTANT", "STREET",
         {"cooldown": 4, "cost": {"focus": 3}}, 65),

        # Magic / god / reality / time
        ("tmpl_zone_magic", None, ["magic", "reality", "time"], "ZONE", "CHANNELED", "STREET",
         {"radius_m": 5, "cost_per_tick": {"focus": 3}, "cooldown": 3}, 55),
        ("tmpl_aura_magic", None, ["magic", "god", "reality"], "AURA", "TOGGLED", "STREET",
         {"cost_per_tick": {"focus": 2}}, 45),

        # Sense
        ("tmpl_sense_visual", None, ["vision", "sight", "eyes"], "SENSE", "CHANNELED", "STREET",
         {"always_on": True}, 85),
        ("tmpl_sense_knowledge", None, ["knowledge"], "SENSE", "CHANNELED", "STREET",
         {"always_on": True}, 70),
        ("tmpl_sense_psychic", None, ["psychic", "mind"], "SENSE", "CHANNELED", "STREET",
         {"always_on": True}, 35),

        # Passive / Trait (gated)
        ("tmpl_trait_body", None, ["body", "strength", "speed", "durability", "regeneration", "animal", "ghost", "god", "life", "death"], "PASSIVE", "CHANNELED", "STREET",
         {"always_on": True}, 80),
    ]

    # Insert or replace
    conn.execute("BEGIN;")
    conn.execute("DELETE FROM expression_template")
    for template_id, kind_match, tags_any, form, delivery, scale, default_constraints, rarity_weight in templates:
        conn.execute("""
            INSERT OR REPLACE INTO expression_template
            (template_id, kind_match, tags_any, form, delivery, scale, default_constraints, rarity_weight, is_enabled)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1)
        """, (
            template_id,
            kind_match,
            json.dumps(tags_any, ensure_ascii=False),
            form, delivery, scale,
            json.dumps(default_constraints, ensure_ascii=False),
            int(rarity_weight),
        ))
    conn.execute("COMMIT;")


# -------------------------
# 2) Generate expressions from templates
# -------------------------

def load_expression_templates(conn: sqlite3.Connection) -> List[dict]:
    require_tables(conn, ["expression_template"])
    rows = conn.execute("""
        SELECT template_id, kind_match, tags_any, form, delivery, scale, default_constraints, rarity_weight
        FROM expression_template
        WHERE is_enabled = 1
    """).fetchall()

    out = []
    for template_id, kind_match, tags_any, form, delivery, scale, default_constraints, rarity_weight in rows:
        out.append({
            "template_id": str(template_id),
            "kind_match": (str(kind_match).strip() if kind_match is not None else None),
            "tags_any": set(json.loads(tags_any)) if tags_any else set(),
            "form": str(form),
            "delivery": str(delivery),
            "scale": str(scale),
            "constraints": json.loads(default_constraints) if default_constraints else {},
            "rarity_weight": int(rarity_weight),
        })
    return out


def template_matches(power_kind: str, power_tags: Set[str], tmpl: dict) -> bool:
    # Tags drive matching; kind_match is rarely used with this dataset.
    if tmpl["kind_match"] and tmpl["kind_match"] != power_kind:
        return False
    if tmpl["tags_any"]:
        return len(power_tags.intersection({t.lower() for t in tmpl["tags_any"]})) > 0
    return True


def make_ui_name(power_name: str, form: str) -> str:
    suffix = {
        "BEAM": "Lance",
        "PROJECTILE": "Bolt",
        "TOUCH": "Touch",
        "AURA": "Aegis",
        "ZONE": "Field",
        "CONSTRUCT": "Construct",
        "SUMMON": "Summon",
        "PASSIVE": "Trait",
        "MOVEMENT": "Step",
        "SENSE": "Sense",
    }.get(form, form.title())
    return f"{power_name} {suffix}"


def tooltip_short(power_name: str, form: str) -> str:
    return f"{form.title().replace('_',' ')} expression of {power_name}."


def tooltip_rules(constraints: dict) -> str:
    bits = []
    if "range_m" in constraints: bits.append(f"Range {constraints['range_m']}m")
    if "radius_m" in constraints and constraints["radius_m"]: bits.append(f"Radius {constraints['radius_m']}m")
    if constraints.get("requires_los"): bits.append("Requires line of sight")
    if constraints.get("requires_contact"): bits.append("Requires contact")
    if "cooldown" in constraints: bits.append(f"Cooldown {constraints['cooldown']}")
    if "duration_turns" in constraints: bits.append(f"Duration {constraints['duration_turns']}")
    if "cost_per_tick" in constraints: bits.append(f"Upkeep {constraints['cost_per_tick']}")
    if "cost" in constraints: bits.append(f"Cost {constraints['cost']}")
    return ". ".join(bits) + "." if bits else "Standard rules."


def generate_power_expressions(conn: sqlite3.Connection, max_per_power: int = 3) -> None:
    require_tables(conn, ["power_expression", "power_expression_text", "Superpower4", "power_tag", "expression_template"])

    tags_by_power = load_power_tags(conn)
    templates = load_expression_templates(conn)

    powers = load_powers(conn)

    created = 0
    conn.execute("BEGIN;")
    conn.execute("DELETE FROM power_expression_text")
    conn.execute("DELETE FROM power_expression")

    for power_id, power_name, kind in powers:
        power_tags = tags_by_power.get(power_id, set())
        if not kind:
            kind = _extract_kind(power_tags)

        matches = [t for t in templates if template_matches(kind, power_tags, t)]

        def fallback_templates() -> List[dict]:
            # Universal safety net: always a neutral TOUCH, optionally a PROJECTILE if elemental/energy tags exist.
            def has_any(options: Set[str]) -> bool:
                return bool(power_tags.intersection(options))

            items = [
                {"template_id": "fallback_touch_neutral", "form": "TOUCH", "delivery": "INSTANT",
                 "scale": "STREET", "constraints": {"requires_contact": True, "cooldown": 2}, "rarity_weight": 12},
            ]
            if has_any({"energy", "light", "heat", "electricity", "fire", "ice", "water", "earth", "air", "shadow", "dark", "darkness"}):
                items.append({
                    "template_id": "fallback_projectile_element", "form": "PROJECTILE", "delivery": "INSTANT",
                    "scale": "STREET", "constraints": {"range_m": 15, "cooldown": 2}, "rarity_weight": 10
                })
            elif has_any({"control", "manipulation", "change", "absorb", "magic", "reality", "time"}):
                items.append({
                    "template_id": "fallback_aura_control", "form": "AURA", "delivery": "TOGGLED",
                    "scale": "STREET", "constraints": {"cost_per_tick": {"focus": 1}}, "rarity_weight": 9
                })
            else:
                items.append({
                    "template_id": "fallback_touch_followup", "form": "TOUCH", "delivery": "TRIGGERED",
                    "scale": "STREET", "constraints": {"requires_contact": True, "cooldown": 3}, "rarity_weight": 8
                })
            return items

        if not matches:
            matches = fallback_templates()

        if len(matches) < 2:
            # Top up with sensible fallbacks so powers land with 2+ expressions.
            supplemental = fallback_templates()
            existing_ids = {m["template_id"] for m in matches}
            for m in supplemental:
                if m["template_id"] not in existing_ids:
                    matches.append(m)
                    existing_ids.add(m["template_id"])

        matches = sorted(matches, key=lambda x: -x["rarity_weight"])[:max_per_power]

        for t in matches:
            expr_id = stable_id(power_id, t["template_id"], t["form"], t["delivery"])
            ui = make_ui_name(power_name, t["form"])
            rules = tooltip_rules(t["constraints"])

            conn.execute("""
                INSERT OR REPLACE INTO power_expression
                (expression_id, power_id, expression_name, form, delivery, scale, constraints, tags_override, is_enabled)
                VALUES (?, ?, ?, ?, ?, ?, ?, NULL, 1)
            """, (
                expr_id, power_id, ui, t["form"], t["delivery"], t["scale"],
                json.dumps(t["constraints"], ensure_ascii=False),
            ))

            conn.execute("""
                INSERT OR REPLACE INTO power_expression_text
                (expression_id, locale, ui_name, tooltip_short, tooltip_rules, text_source, text_version, updated_at)
                VALUES (?, ?, ?, ?, ?, 'GENERATED', 1, ?)
            """, (
                expr_id, LOCALE, ui,
                tooltip_short(power_name, t["form"]),
                rules,
                datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S"),
            ))

            created += 1

    conn.execute("COMMIT;")
    print(f"Expressions created/updated: {created}")


# -------------------------
# 3) Generate expression costs and signatures
# -------------------------

def _base_cooldown(constraints: dict, default: int) -> int:
    try:
        return int(constraints.get("cooldown", default))
    except Exception:
        return default


def generate_power_expression_costs(conn: sqlite3.Connection) -> None:
    require_tables(conn, ["power_expression_cost", "power_expression", "power_tag"])
    tags_by_power = load_power_tags(conn)

    conn.execute("BEGIN;")
    conn.execute("DELETE FROM power_expression_cost")

    rows = conn.execute("""
        SELECT e.expression_id, e.power_id, e.form, e.delivery, e.constraints
        FROM power_expression e
        WHERE e.is_enabled = 1
    """).fetchall()

    created = 0
    for expr_id, power_id, form, delivery, constraints_json in rows:
        constraints = json.loads(constraints_json or "{}")
        tags = tags_by_power.get(str(power_id), set())
        tags_lower = {t.lower() for t in tags}

        costs = []
        if form == "TOUCH":
            if {"mind", "psychic", "mental"} & tags_lower:
                costs.append(("FOCUS", 1))
            else:
                costs.append(("STAMINA", 1))
            costs.append(("COOLDOWN", _base_cooldown(constraints, 1)))
        elif form in ("PROJECTILE", "BEAM"):
            costs.append(("STAMINA", 2))
            costs.append(("COOLDOWN", _base_cooldown(constraints, 2)))
        elif form in ("ZONE", "AURA"):
            costs.append(("FOCUS", 2))
            costs.append(("STAMINA", 1))
            costs.append(("COOLDOWN", _base_cooldown(constraints, 2)))
        elif form == "MOVEMENT":
            costs.append(("STAMINA", 1))
            costs.append(("COOLDOWN", _base_cooldown(constraints, 2)))
        elif form == "SUMMON":
            costs.append(("FOCUS", 3))
            costs.append(("COOLDOWN", _base_cooldown(constraints, 4)))
        elif form == "SENSE":
            costs.append(("FOCUS", 1))
        elif form == "PASSIVE":
            # No direct cost; leave empty for now.
            costs = []

        for ctype, value in costs:
            conn.execute("""
                INSERT INTO power_expression_cost (expression_id, cost_type, value)
                VALUES (?, ?, ?)
            """, (expr_id, ctype, int(value)))
            created += 1

    conn.execute("COMMIT;")
    print(f"Expression costs created/updated: {created}")


def generate_power_expression_signatures(conn: sqlite3.Connection) -> None:
    require_tables(conn, ["power_expression_signature", "power_expression", "power_tag"])
    tags_by_power = load_power_tags(conn)

    conn.execute("BEGIN;")
    conn.execute("DELETE FROM power_expression_signature")

    rows = conn.execute("""
        SELECT e.expression_id, e.power_id, e.form, e.delivery, e.constraints
        FROM power_expression e
        WHERE e.is_enabled = 1
    """).fetchall()

    form_strength = {
        "TOUCH": 15,
        "PROJECTILE": 35,
        "BEAM": 40,
        "ZONE": 50,
        "AURA": 45,
        "SUMMON": 70,
        "SENSE": 10,
        "MOVEMENT": 25,
    }
    form_persistence = {
        "TOUCH": 2,
        "PROJECTILE": 3,
        "BEAM": 3,
        "ZONE": 6,
        "AURA": 6,
        "SUMMON": 10,
        "SENSE": 8,
        "MOVEMENT": 4,
    }

    visual_tags = {
        "light", "dark", "darkness", "shadow", "invisibility", "illusion",
        "vision", "sight", "eyes",
    }
    em_tags = {"electricity", "electric", "energy", "technology", "tech", "nanotech"}
    thermal_tags = {"fire", "heat", "ice", "cool", "water"}
    acoustic_tags = {"sound", "sonic"}
    chemical_tags = {"acid", "poison", "toxin", "chemical"}
    bio_tags = {"blood", "biological", "bio", "body", "healing", "regeneration", "animal"}
    psychic_tags = {"mind", "mental", "psychic", "emotion", "memory", "telepathy", "dream", "dreams"}
    dimensional_tags = {"teleport", "teleportation", "portal", "space", "summoning"}
    grav_tags = {"gravity"}
    arcane_tags = {"magic", "demon", "demonic", "curse", "arcane"}
    causal_tags = {"time", "reality"}
    kinetic_tags = {"strength", "powerful", "earth", "pain"}
    radiation_tags = {"radiation"}

    created = 0
    for expr_id, power_id, form, delivery, constraints_json in rows:
        tags = tags_by_power.get(str(power_id), set())
        tags_lower = {t.lower() for t in tags}

        signatures: Dict[str, Tuple[int, int]] = {}

        def add(sig_type: str, strength: Optional[int] = None, persistence: Optional[int] = None) -> None:
            base_strength = form_strength.get(form, 20)
            base_persist = form_persistence.get(form, 2)
            s = strength if strength is not None else base_strength
            p = persistence if persistence is not None else base_persist
            prev = signatures.get(sig_type)
            if prev:
                signatures[sig_type] = (max(prev[0], s), max(prev[1], p))
            else:
                signatures[sig_type] = (s, p)

        if form == "PASSIVE":
            signatures = {}
        else:
            # Step A: form-driven base
            if form in ("PROJECTILE", "BEAM"):
                add("VISUAL_ANOMALY")
                if tags_lower & em_tags:
                    add("EM_SPIKE")
                if tags_lower & thermal_tags:
                    add("THERMAL_BLOOM")
            elif form in ("ZONE", "AURA"):
                add("VISUAL_ANOMALY")
            elif form == "TOUCH":
                if tags_lower & bio_tags:
                    add("BIO_MARKER", strength=12, persistence=2)
                else:
                    add("VISUAL_ANOMALY", strength=10, persistence=1)
            elif form == "MOVEMENT":
                if tags_lower & dimensional_tags:
                    add("DIMENSIONAL_RESIDUE", strength=25, persistence=4)
                else:
                    add("VISUAL_ANOMALY", strength=20, persistence=3)
            elif form == "SENSE":
                add("PSYCHIC_ECHO", strength=10, persistence=8)
            elif form == "SUMMON":
                add("DIMENSIONAL_RESIDUE", strength=70, persistence=10)
                if tags_lower & arcane_tags:
                    add("ARCANE_RESONANCE", strength=65, persistence=8)

            # Step B: tag-driven extras (0-2)
            extras = [
                (visual_tags, "VISUAL_ANOMALY"),
                (em_tags, "EM_SPIKE"),
                (thermal_tags, "THERMAL_BLOOM"),
                (acoustic_tags, "ACOUSTIC_SHOCK"),
                (chemical_tags, "CHEMICAL_RESIDUE"),
                (bio_tags, "BIO_MARKER"),
                (psychic_tags, "PSYCHIC_ECHO"),
                (dimensional_tags, "DIMENSIONAL_RESIDUE"),
                (grav_tags, "GRAVITIC_DISTURBANCE"),
                (arcane_tags, "ARCANE_RESONANCE"),
                (causal_tags, "CAUSAL_IMPRINT"),
                (kinetic_tags, "KINETIC_STRESS"),
                (radiation_tags, "RADIATION_TRACE"),
            ]
            added = 0
            for tag_set, sig_type in extras:
                if added >= 2:
                    break
                if tags_lower & tag_set and sig_type not in signatures:
                    add(sig_type)
                    added += 1

            # Step C: hard fallback
            if not signatures:
                add("VISUAL_ANOMALY", strength=5, persistence=1)

        for sig_type, (strength, persistence) in signatures.items():
            conn.execute("""
                INSERT INTO power_expression_signature (expression_id, signature_type, strength, persistence_turns)
                VALUES (?, ?, ?, ?)
            """, (expr_id, sig_type, int(strength), int(persistence)))
            created += 1

    conn.execute("COMMIT;")
    print(f"Expression signatures created/updated: {created}")


# -------------------------
# 4) Generate acquisition profiles from tags/kind
# -------------------------

ORIGIN_RULES = [
    # (match_tags_any, origin_class, origin_subtype, delivery_channel, event_kind, requires_entity_kind, rarity_weight)
    ({"tech", "technology", "robot", "cyber", "armour", "armor", "ai", "nanotech"}, "FORGED", "TECHNOLOGICAL", "DEVICE", "CRAFTING", "DEVICE_BLUEPRINT", 70),
    ({"magic", "occult", "curse", "demon", "spell", "ritual"}, "BOUND", "COVENANT", "CONTRACT", "RITUAL", "PATRON", 60),
    ({"artifact", "artefact", "relic", "ring", "weapon", "amulet"}, "BOUND", "ARTEFACT", "ITEM", "DISCOVERY", "ARTEFACT", 60),
    ({"mutation", "genetic", "dna"}, "ASCENDANT", "GENETIC", "BIOLOGY", "PUBERTY", None, 60),
    ({"alien", "extraterrestrial", "space"}, "ASCENDANT", "XENOBIOLOGICAL", "BIOLOGY", "BIRTH", "SPECIES", 40),
    ({"radiation", "chemical", "experiment", "serum"}, "ALTERED", "EXPOSURE", "ACCIDENT", "LAB_ACCIDENT", "LAB", 60),
    ({"symbiote", "parasite", "infection"}, "ALTERED", "INFESTATION", "ACCIDENT", "INFECTION", "SYMBIOTE", 55),
]


def generate_acquisition_profiles(conn: sqlite3.Connection, max_profiles_per_power: int = 3) -> None:
    require_tables(conn, ["power_acquisition_profile", "Superpower4", "power_tag"])

    tags_by_power = load_power_tags(conn)
    powers = load_powers(conn)

    created = 0
    conn.execute("BEGIN;")

    for power_id, power_name, kind in powers:
        tags = tags_by_power.get(power_id, set())
        if not kind:
            kind = _extract_kind(tags)

        matches = []
        for match_tags, oclass, osub, channel, event_kind, req_entity, weight in ORIGIN_RULES:
            if tags.intersection(match_tags):
                matches.append((oclass, osub, channel, event_kind, req_entity, weight, match_tags))

        if not matches:
            # Fallback profiles by kind (keeps everything usable)
            # You can tune later.
            fallback = []
            k = kind.lower()
            if "mob" in k or "move" in k:
                fallback.append(("FORGED", "TECHNOLOGICAL", "DEVICE", "CRAFTING", "DEVICE_BLUEPRINT", 30, set()))
                fallback.append(("ALTERED", "EXPOSURE", "ACCIDENT", "COSMIC_CONTACT", "PORTAL", 20, set()))
            elif "mind" in k or "psy" in k or "info" in k:
                fallback.append(("ASCENDANT", "GENETIC", "BIOLOGY", "STRESS_AWAKENING", None, 30, set()))
                fallback.append(("BOUND", "COVENANT", "CONTRACT", "RITUAL", "PATRON", 20, set()))
            else:
                fallback.append(("ALTERED", "EXPOSURE", "ACCIDENT", "LAB_ACCIDENT", "LAB", 30, set()))
                fallback.append(("ASCENDANT", "GENETIC", "BIOLOGY", "PUBERTY", None, 20, set()))
            matches = fallback

        matches = sorted(matches, key=lambda x: -x[5])[:max_profiles_per_power]

        for oclass, osub, channel, event_kind, req_entity, weight, match_tags in matches:
            acq_id = stable_id(power_id, oclass, osub, channel, event_kind)
            requires_tags_any = list(match_tags) if match_tags else None

            # Minimal defaults; you can tune via later passes.
            default_costs = {}
            default_signatures = []

            # Small heuristic: tech -> EM spike, gravity/space -> dimensional/grav traces, radiation -> radiation trace
            if oclass == "FORGED":
                default_signatures.append({"type": "EM_SPIKE", "strength": 35})
            if "gravity" in tags:
                default_signatures.append({"type": "GRAVITIC_DISTURBANCE", "strength": 25})
            if "space" in tags or "teleport" in tags or "portal" in tags:
                default_signatures.append({"type": "DIMENSIONAL_RESIDUE", "strength": 25})
            if "magic" in tags or "demon" in tags or "curse" in tags:
                default_signatures.append({"type": "ARCANE_RESONANCE", "strength": 25})
            if "radiation" in tags:
                default_signatures.append({"type": "RADIATION_TRACE", "strength": 45})

            collateral = "LOW"
            if "explosion" in tags or "earthquake" in tags:
                collateral = "HIGH"

            stability = "STABLE"
            if oclass == "ALTERED":
                stability = "DRIFTING"
            if oclass == "BOUND" and osub == "ARTEFACT":
                stability = "DEPENDENT"
            if oclass == "BOUND" and osub == "COVENANT":
                stability = "CORRUPTING"

            counterplay = []
            if oclass == "FORGED":
                counterplay.append("emp")
            if oclass == "BOUND" and osub == "ARTEFACT":
                counterplay.append("disarm")
            if oclass == "ASCENDANT" and osub == "GENETIC":
                counterplay.append("suppressor")

            conn.execute("""
                INSERT OR REPLACE INTO power_acquisition_profile
                (acq_id, power_id, origin_class, origin_subtype, delivery_channel, acquisition_event_kind,
                 rarity_weight, requires_entity_kind, requires_tags_any, requires_tags_all,
                 counterplay_tags, default_costs, default_limits, default_signatures,
                 collateral_profile, stability_profile, notes, is_enabled)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, '{}', ?, ?, ?, NULL, 1)
            """, (
                acq_id, power_id, oclass, osub, channel, event_kind,
                int(weight),
                req_entity,
                json.dumps(requires_tags_any, ensure_ascii=False) if requires_tags_any else None,
                json.dumps(counterplay, ensure_ascii=False),
                json.dumps(default_costs, ensure_ascii=False),
                json.dumps(default_signatures, ensure_ascii=False),
                collateral,
                stability,
            ))

            created += 1

    conn.execute("COMMIT;")
    print(f"Acquisition profiles created/updated: {created}")


# -------------------------
# 5) Validate coverage
# -------------------------

def validate(conn: sqlite3.Connection) -> None:
    require_tables(conn, ["Superpower4", "power_expression", "power_acquisition_profile"])

    total = conn.execute("SELECT COUNT(*) FROM Superpower4").fetchone()[0]
    no_expr = conn.execute("""
        SELECT COUNT(*) FROM Superpower4 p
        LEFT JOIN power_expression e ON e.power_id = p.rowid AND e.is_enabled = 1
        WHERE e.power_id IS NULL
    """).fetchone()[0]

    no_acq = conn.execute("""
        SELECT COUNT(*) FROM Superpower4 p
        LEFT JOIN power_acquisition_profile a ON a.power_id = p.rowid AND a.is_enabled = 1
        WHERE a.power_id IS NULL
    """).fetchone()[0]

    print(f"Total powers: {total}")
    print(f"Powers with 0 expressions: {no_expr}")
    print(f"Powers with 0 acquisition profiles: {no_acq}")


# -------------------------
# Runner
# -------------------------

def main(db_path: Optional[str] = None) -> None:
    db_file = Path(db_path).expanduser().resolve() if db_path else DB_PATH
    if not db_file.exists():
        raise FileNotFoundError(f"DB not found at {db_file}")

    conn = sqlite3.connect(db_file)
    conn.execute("PRAGMA foreign_keys = ON;")

    # Check your DB has the expected tables
    require_tables(conn, [
        "Superpower4", "power_tag", "power_text",
        "expression_template",
        "power_expression", "power_expression_text",
        "power_expression_cost", "power_expression_signature",
        "power_acquisition_profile"
    ])

    # 1) Seed templates (safe to re-run)
    seed_expression_templates(conn)

    # 2) Generate expressions (safe to re-run)
    generate_power_expressions(conn, max_per_power=3)

    # 3) Generate expression costs/signatures (safe to re-run)
    generate_power_expression_costs(conn)
    generate_power_expression_signatures(conn)

    # 4) Generate acquisition profiles (safe to re-run)
    generate_acquisition_profiles(conn, max_profiles_per_power=3)

    # 5) Validate coverage
    validate(conn)

    conn.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Populate power expression/acquisition tables deterministically.")
    parser.add_argument("--db", dest="db_path", default=None, help="Path to the SQLite DB (defaults to Superpower_list.db in repo root).")
    args = parser.parse_args()
    main(args.db_path)
