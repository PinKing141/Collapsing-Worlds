import argparse
import sqlite3
from pathlib import Path
from typing import Iterable, List

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_SRC = ROOT / "Superpower_list.db"
DEFAULT_DEST = ROOT / "assets" / "db" / "content_v1.db"
CONTENT_SCHEMA_VERSION = 1
CONTENT_VERSION = "v1"

RUNTIME_TABLES = [
    "Superpower4",
    "power_text",
    "power_expression",
    "power_expression_text",
    "power_expression_cost",
    "power_expression_signature",
    "power_acquisition_profile",
    "persona_power",
    "power_tag",
    "origin_event_template",
    "OriginClass",
    "OriginSubtype",
    "TagDictionary",
    "TaxonomyMeta",
]


def export_db(src_path: Path, dest_path: Path, tables: Iterable[str]) -> None:
    if not src_path.exists():
        raise FileNotFoundError(f"Source DB not found: {src_path}")

    dest_path.parent.mkdir(parents=True, exist_ok=True)
    if dest_path.exists():
        dest_path.unlink()

    conn = sqlite3.connect(dest_path)
    conn.execute("PRAGMA foreign_keys = OFF;")
    conn.execute("ATTACH DATABASE ? AS src", (str(src_path),))

    conn.execute("BEGIN;")
    for table in tables:
        row = conn.execute(
            "SELECT sql FROM src.sqlite_master WHERE type='table' AND name=?",
            (table,),
        ).fetchone()
        if not row or not row[0]:
            raise RuntimeError(f"Missing table in source DB: {table}")
        conn.execute(row[0])
        conn.execute(f"INSERT INTO {table} SELECT * FROM src.{table}")

        index_rows = conn.execute(
            "SELECT sql FROM src.sqlite_master WHERE type='index' AND tbl_name=? AND sql IS NOT NULL",
            (table,),
        ).fetchall()
        for (sql,) in index_rows:
            conn.execute(sql)

        trigger_rows = conn.execute(
            "SELECT sql FROM src.sqlite_master WHERE type='trigger' AND tbl_name=? AND sql IS NOT NULL",
            (table,),
        ).fetchall()
        for (sql,) in trigger_rows:
            conn.execute(sql)

    conn.execute("COMMIT;")
    conn.execute(
        "CREATE TABLE IF NOT EXISTS content_meta ("
        "id INTEGER PRIMARY KEY CHECK (id = 1), "
        "schema_version INTEGER NOT NULL, "
        "content_version TEXT NOT NULL"
        ")"
    )
    conn.execute("DELETE FROM content_meta")
    conn.execute(
        "INSERT INTO content_meta (id, schema_version, content_version) VALUES (1, ?, ?)",
        (CONTENT_SCHEMA_VERSION, CONTENT_VERSION),
    )
    conn.execute("DETACH DATABASE src")
    conn.execute("VACUUM;")
    conn.close()


def parse_tables(raw: str) -> List[str]:
    return [t.strip() for t in raw.split(",") if t.strip()]


def main() -> None:
    parser = argparse.ArgumentParser(description="Export runtime tables into content_v1.db.")
    parser.add_argument("--src", default=str(DEFAULT_SRC), help="Source DB path (default: Superpower_list.db)")
    parser.add_argument("--dest", default=str(DEFAULT_DEST), help="Destination DB path (default: assets/db/content_v1.db)")
    parser.add_argument(
        "--tables",
        default="",
        help="Comma-separated table list override (default: runtime table set).",
    )
    args = parser.parse_args()

    tables = RUNTIME_TABLES if not args.tables else parse_tables(args.tables)
    export_db(Path(args.src), Path(args.dest), tables)
    print(f"Exported {len(tables)} tables to {args.dest}")


if __name__ == "__main__":
    main()
