import argparse
import csv
import re
import sqlite3
import unicodedata
from pathlib import Path
from typing import Optional, Tuple

NAME_RE = re.compile(r"^[A-Za-z][A-Za-z' -]*$")
BATCH_SIZE = 50_000


def _clean_optional(value: Optional[str]) -> Optional[str]:
    if value is None:
        return None
    cleaned = value.strip()
    return cleaned if cleaned else None


def _is_english_name(value: str) -> bool:
    return bool(NAME_RE.match(value))


def _normalize_name(value: str) -> str:
    cleaned = value.strip()
    if not cleaned:
        return ""
    normalized = unicodedata.normalize("NFKD", cleaned)
    ascii_only = normalized.encode("ascii", "ignore").decode("ascii")
    ascii_only = re.sub(r"\s+", " ", ascii_only)
    return ascii_only.strip()


def _import_table(
    conn: sqlite3.Connection,
    csv_path: Path,
    table_name: str,
    name_field: str,
) -> Tuple[int, int]:
    total_rows = 0
    inserted_rows = 0
    batch = []

    with csv_path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        conn.execute("BEGIN;")
        for row in reader:
            total_rows += 1
            raw_name = row.get(name_field) or ""
            normalized_name = _normalize_name(raw_name)
            if not normalized_name:
                continue
            if not _is_english_name(normalized_name):
                continue

            gender = _clean_optional(row.get("gender"))
            country = _clean_optional(row.get("country"))
            raw_count = (row.get("count") or "").strip()
            try:
                count = int(raw_count)
            except ValueError:
                continue

            batch.append((normalized_name, gender, country, count))
            if len(batch) >= BATCH_SIZE:
                conn.executemany(
                    f"INSERT INTO {table_name} (name, gender, country, count) VALUES (?, ?, ?, ?)",
                    batch,
                )
                inserted_rows += len(batch)
                batch.clear()

    if batch:
        conn.executemany(
            f"INSERT INTO {table_name} (name, gender, country, count) VALUES (?, ?, ?, ?)",
            batch,
        )
        inserted_rows += len(batch)
        batch.clear()

    conn.execute("COMMIT;")
    return total_rows, inserted_rows


def _init_db(conn: sqlite3.Connection) -> None:
    conn.execute("PRAGMA journal_mode = WAL;")
    conn.execute("PRAGMA synchronous = OFF;")
    conn.execute("PRAGMA temp_store = MEMORY;")

    conn.execute("DROP TABLE IF EXISTS forenames;")
    conn.execute("DROP TABLE IF EXISTS surnames;")
    conn.execute("DROP TABLE IF EXISTS name_db_meta;")

    conn.execute(
        """
        CREATE TABLE forenames (
          name TEXT NOT NULL,
          gender TEXT,
          country TEXT,
          count INTEGER NOT NULL
        );
        """
    )
    conn.execute(
        """
        CREATE TABLE surnames (
          name TEXT NOT NULL,
          gender TEXT,
          country TEXT,
          count INTEGER NOT NULL
        );
        """
    )
    conn.execute(
        """
        CREATE TABLE name_db_meta (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL
        );
        """
    )


def _finalize_db(conn: sqlite3.Connection) -> None:
    conn.execute("CREATE INDEX idx_forenames_name ON forenames(name);")
    conn.execute("CREATE INDEX idx_surnames_name ON surnames(name);")
    conn.execute("CREATE INDEX idx_forenames_gender ON forenames(gender);")
    conn.execute("CREATE INDEX idx_surnames_gender ON surnames(gender);")
    conn.execute("CREATE INDEX idx_forenames_country ON forenames(country);")
    conn.execute("CREATE INDEX idx_surnames_country ON surnames(country);")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Build a SQLite name database from forename/surname CSVs."
    )
    parser.add_argument(
        "--forenames",
        default="forenames.csv",
        help="Path to forenames.csv",
    )
    parser.add_argument(
        "--surnames",
        default="surnames.csv",
        help="Path to surnames.csv",
    )
    parser.add_argument(
        "--out",
        default=str(Path("assets") / "names" / "names.db"),
        help="Output SQLite DB path",
    )
    args = parser.parse_args()

    forenames_path = Path(args.forenames)
    surnames_path = Path(args.surnames)
    out_path = Path(args.out)

    out_path.parent.mkdir(parents=True, exist_ok=True)

    conn = sqlite3.connect(out_path)
    _init_db(conn)

    total_forenames, inserted_forenames = _import_table(
        conn, forenames_path, "forenames", "forename"
    )
    total_surnames, inserted_surnames = _import_table(
        conn, surnames_path, "surnames", "surname"
    )

    conn.execute(
        "INSERT OR REPLACE INTO name_db_meta (key, value) VALUES (?, ?)",
        ("forenames_source", str(forenames_path)),
    )
    conn.execute(
        "INSERT OR REPLACE INTO name_db_meta (key, value) VALUES (?, ?)",
        ("surnames_source", str(surnames_path)),
    )
    conn.execute(
        "INSERT OR REPLACE INTO name_db_meta (key, value) VALUES (?, ?)",
        ("filter_regex", NAME_RE.pattern),
    )
    conn.execute(
        "INSERT OR REPLACE INTO name_db_meta (key, value) VALUES (?, ?)",
        ("normalization", "NFKD_ASCII"),
    )
    conn.execute(
        "INSERT OR REPLACE INTO name_db_meta (key, value) VALUES (?, ?)",
        ("forenames_rows_written", str(inserted_forenames)),
    )
    conn.execute(
        "INSERT OR REPLACE INTO name_db_meta (key, value) VALUES (?, ?)",
        ("surnames_rows_written", str(inserted_surnames)),
    )

    _finalize_db(conn)
    conn.close()

    print(
        "Name database created:",
        out_path,
        f"(forenames kept {inserted_forenames}/{total_forenames}, "
        f"surnames kept {inserted_surnames}/{total_surnames})",
    )


if __name__ == "__main__":
    main()
