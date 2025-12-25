import argparse
import sqlite3
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.append(str(ROOT))

from tools.populate_db import generate_power_expression_signatures

DEFAULT_DB = ROOT / "Superpower_list.db"

SIGNATURE_TYPES = [
    "VISUAL_ANOMALY",
    "EM_SPIKE",
    "THERMAL_BLOOM",
    "ACOUSTIC_SHOCK",
    "CHEMICAL_RESIDUE",
    "BIO_MARKER",
    "PSYCHIC_ECHO",
    "DIMENSIONAL_RESIDUE",
    "GRAVITIC_DISTURBANCE",
    "ARCANE_RESONANCE",
    "CAUSAL_IMPRINT",
    "KINETIC_STRESS",
    "RADIATION_TRACE",
]


def recreate_signature_table(conn: sqlite3.Connection) -> None:
    allowed = ",".join(f"'{t}'" for t in SIGNATURE_TYPES)
    conn.execute("PRAGMA foreign_keys = OFF;")
    conn.execute("BEGIN;")
    conn.execute("DROP TABLE IF EXISTS power_expression_signature")
    conn.execute(f"""
        CREATE TABLE power_expression_signature (
          signature_id INTEGER PRIMARY KEY AUTOINCREMENT,
          expression_id TEXT NOT NULL,
          signature_type TEXT NOT NULL CHECK (signature_type IN ({allowed})),
          strength INTEGER NOT NULL CHECK (strength BETWEEN 1 AND 100),
          persistence_turns INTEGER NOT NULL DEFAULT 0 CHECK (persistence_turns >= 0),
          FOREIGN KEY (expression_id) REFERENCES power_expression(expression_id) ON DELETE CASCADE
        )
    """)
    conn.execute("COMMIT;")
    conn.execute("PRAGMA foreign_keys = ON;")


def main() -> None:
    parser = argparse.ArgumentParser(description="Recreate signature table and regenerate expression signatures.")
    parser.add_argument("--db", default=str(DEFAULT_DB), help="Path to the source DB (default: Superpower_list.db).")
    args = parser.parse_args()

    db_path = Path(args.db).expanduser().resolve()
    if not db_path.exists():
        raise FileNotFoundError(f"DB not found at {db_path}")

    conn = sqlite3.connect(db_path)
    recreate_signature_table(conn)
    generate_power_expression_signatures(conn)
    conn.close()
    print("Signatures refreshed.")


if __name__ == "__main__":
    main()
