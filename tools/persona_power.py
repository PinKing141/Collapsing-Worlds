import argparse
import json
import sqlite3
from pathlib import Path
from typing import Dict, List, Tuple

DB_PATH = Path(__file__).resolve().parent.parent / "Superpower_list.db"


def table_exists(conn: sqlite3.Connection, name: str) -> bool:
    row = conn.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name=?",
        (name,)
    ).fetchone()
    return row is not None


def ensure_persona_power_schema(conn: sqlite3.Connection) -> None:
    """Create or upgrade persona_power to the expected shape."""
    if not table_exists(conn, "persona_power"):
        conn.execute("""
            CREATE TABLE persona_power (
              persona_id TEXT NOT NULL,
              power_id INTEGER NOT NULL,
              expression_id TEXT NOT NULL,
              mastery_level INTEGER NOT NULL DEFAULT 1 CHECK (mastery_level BETWEEN 1 AND 100),
              modifiers TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(modifiers)),
              is_unlocked INTEGER NOT NULL DEFAULT 1 CHECK (is_unlocked IN (0,1)),
              PRIMARY KEY (persona_id, expression_id),
              FOREIGN KEY (expression_id) REFERENCES power_expression(expression_id) ON DELETE CASCADE
            )
        """)
    else:
        cols = {c[1] for c in conn.execute("PRAGMA table_info(persona_power)")}
        needs_upgrade = not {"power_id", "mastery_level", "is_unlocked"} <= cols
        if needs_upgrade:
            conn.execute("""
                CREATE TABLE IF NOT EXISTS persona_power_new (
                  persona_id TEXT NOT NULL,
                  power_id INTEGER NOT NULL,
                  expression_id TEXT NOT NULL,
                  mastery_level INTEGER NOT NULL DEFAULT 1 CHECK (mastery_level BETWEEN 1 AND 100),
                  modifiers TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(modifiers)),
                  is_unlocked INTEGER NOT NULL DEFAULT 1 CHECK (is_unlocked IN (0,1)),
                  PRIMARY KEY (persona_id, expression_id),
                  FOREIGN KEY (expression_id) REFERENCES power_expression(expression_id) ON DELETE CASCADE
                )
            """)
            if "mastery" in cols:
                conn.execute("""
                    INSERT INTO persona_power_new (persona_id, power_id, expression_id, mastery_level, modifiers, is_unlocked)
                    SELECT persona_id,
                           COALESCE(power_id, (SELECT power_id FROM power_expression pe WHERE pe.expression_id = pp.expression_id)),
                           expression_id,
                           COALESCE(mastery, 1),
                           modifiers,
                           1
                    FROM persona_power pp
                """)
            else:
                conn.execute("""
                    INSERT INTO persona_power_new (persona_id, power_id, expression_id, mastery_level, modifiers, is_unlocked)
                    SELECT persona_id,
                           COALESCE(power_id, (SELECT power_id FROM power_expression pe WHERE pe.expression_id = pp.expression_id)),
                           expression_id,
                           1,
                           modifiers,
                           1
                    FROM persona_power pp
                """)
            conn.execute("DROP TABLE persona_power")
            conn.execute("ALTER TABLE persona_power_new RENAME TO persona_power")

    # Indexes
    conn.execute("CREATE INDEX IF NOT EXISTS idx_persona_power_persona ON persona_power (persona_id)")
    conn.execute("CREATE INDEX IF NOT EXISTS idx_persona_power_power ON persona_power (power_id)")

    # Triggers to enforce expression->power consistency
    conn.execute("""
        CREATE TRIGGER IF NOT EXISTS persona_power_check_insert
        BEFORE INSERT ON persona_power
        BEGIN
            SELECT CASE
                WHEN NOT EXISTS (
                    SELECT 1 FROM power_expression pe
                    WHERE pe.expression_id = NEW.expression_id
                      AND pe.power_id = NEW.power_id
                ) THEN RAISE(ABORT, 'expression does not belong to power')
            END;
        END;
    """)
    conn.execute("""
        CREATE TRIGGER IF NOT EXISTS persona_power_check_update
        BEFORE UPDATE ON persona_power
        BEGIN
            SELECT CASE
                WHEN NOT EXISTS (
                    SELECT 1 FROM power_expression pe
                    WHERE pe.expression_id = NEW.expression_id
                      AND pe.power_id = NEW.power_id
                ) THEN RAISE(ABORT, 'expression does not belong to power')
            END;
        END;
    """)


def load_expressions(conn: sqlite3.Connection) -> Dict[int, List[str]]:
    """Return expressions grouped by power_id, ordered deterministically."""
    rows = conn.execute("""
        SELECT power_id, expression_id
        FROM power_expression
        WHERE is_enabled = 1
        ORDER BY power_id, expression_id
    """).fetchall()
    grouped: Dict[int, List[str]] = {}
    for power_id, expr_id in rows:
        grouped.setdefault(int(power_id), []).append(str(expr_id))
    return grouped


def seed_persona(
    conn: sqlite3.Connection,
    persona_id: str,
    expressions_per_power: int,
    mastery_level: int,
    modifiers: Dict,
    replace: bool,
) -> int:
    grouped = load_expressions(conn)

    conn.execute("BEGIN;")
    if replace:
        conn.execute("DELETE FROM persona_power WHERE persona_id = ?", (persona_id,))

    inserted = 0
    for power_id, expr_ids in grouped.items():
        for expr_id in expr_ids[:expressions_per_power]:
            conn.execute("""
                INSERT INTO persona_power (persona_id, power_id, expression_id, mastery_level, modifiers, is_unlocked)
                VALUES (?, ?, ?, ?, ?, 1)
                ON CONFLICT(persona_id, expression_id) DO UPDATE SET
                    mastery_level=excluded.mastery_level,
                    modifiers=excluded.modifiers,
                    is_unlocked=1
            """, (
                persona_id,
                int(power_id),
                expr_id,
                int(mastery_level),
                json.dumps(modifiers, ensure_ascii=False),
            ))
            inserted += 1

    conn.execute("COMMIT;")
    return inserted


def main() -> None:
    parser = argparse.ArgumentParser(description="Ensure persona_power schema and optionally bind a persona to expressions.")
    parser.add_argument("--db", dest="db_path", default=str(DB_PATH), help="Path to the SQLite DB (defaults to Superpower_list.db in repo root).")
    parser.add_argument("--persona", dest="persona_id", default=None, help="Persona id to bind. If omitted, only schema is ensured.")
    parser.add_argument("--take-per-power", dest="take_per_power", type=int, default=1, help="How many expressions per power to assign (default: 1).")
    parser.add_argument("--mastery", dest="mastery", type=int, default=1, help="Mastery level to assign (default: 1).")
    parser.add_argument("--modifiers", dest="modifiers", default="{}", help="JSON for modifiers column (default: {}).")
    parser.add_argument("--replace", dest="replace", action="store_true", help="Replace existing entries for the persona.")
    args = parser.parse_args()

    db_file = Path(args.db_path).expanduser().resolve()
    if not db_file.exists():
        raise FileNotFoundError(f"DB not found at {db_file}")

    conn = sqlite3.connect(db_file)
    conn.execute("PRAGMA foreign_keys = ON;")

    ensure_persona_power_schema(conn)
    print("persona_power schema ensured.")
    conn.commit()

    if args.persona_id:
        try:
            modifiers = json.loads(args.modifiers)
        except json.JSONDecodeError as exc:
            raise SystemExit(f"Invalid JSON for modifiers: {exc}") from exc

        inserted = seed_persona(
            conn=conn,
            persona_id=args.persona_id,
            expressions_per_power=max(1, args.take_per_power),
            mastery_level=max(1, args.mastery),
            modifiers=modifiers,
            replace=args.replace,
        )
        print(f"Persona '{args.persona_id}' bindings created/updated: {inserted}")
    else:
        print("No persona specified; schema only.")

    conn.close()


if __name__ == "__main__":
    main()
