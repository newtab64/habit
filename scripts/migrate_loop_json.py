#!/usr/bin/env python3
"""
Migrates a JSON habit export (e.g. from Loop Habit Tracker) into a Habit SQLite database (.db).

Usage:
    python3 scripts/migrate_loop_json.py [input.json] [output.db]
"""

import argparse
import datetime
import json
import os
import sqlite3
import sys

SCHEMA_SQL = """
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS habits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    habit_type TEXT NOT NULL CHECK(habit_type IN ('binary', 'measurable')),
    target_value REAL NOT NULL DEFAULT 1.0,
    unit TEXT,
    accent_color TEXT NOT NULL DEFAULT '#3584E4',
    scheduled_days INTEGER NOT NULL DEFAULT 127,
    archived INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS habit_entries (
    habit_id INTEGER NOT NULL REFERENCES habits(id) ON DELETE CASCADE,
    date TEXT NOT NULL,
    entry_status TEXT NOT NULL CHECK(entry_status IN ('completed', 'skipped', 'missed')),
    value REAL NOT NULL DEFAULT 0.0,
    PRIMARY KEY (habit_id, date)
);

CREATE INDEX IF NOT EXISTS idx_entries_habit_date ON habit_entries(habit_id, date);
"""

def migrate(input_json_path: str, output_db_path: str):
    if not os.path.exists(input_json_path):
        print(f"Error: input file '{input_json_path}' not found.", file=sys.stderr)
        sys.exit(1)

    print(f"Reading JSON export: {input_json_path}")
    with open(input_json_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    raw_habits = data.get("habits", [])
    if not raw_habits:
        print("Error: No habits found in JSON export.", file=sys.stderr)
        sys.exit(1)

    # Filter to active habits (status == 1)
    active_habits = [h for h in raw_habits if h.get("status") == 1]
    if not active_habits:
        print("Warning: No status=1 active habits found, falling back to all non-deleted habits.")
        active_habits = raw_habits

    if os.path.exists(output_db_path):
        os.remove(output_db_path)

    conn = sqlite3.connect(output_db_path)
    cur = conn.cursor()
    cur.executescript(SCHEMA_SQL)

    epoch = datetime.date(1970, 1, 1)
    total_entries = 0

    print(f"\nMigrating {len(active_habits)} active habits into '{output_db_path}':")
    for h in active_habits:
        name = h.get("name", "Unnamed Habit")
        daily_goal = float(h.get("daily_goal", 1.0))
        unit_str = h.get("daily_goal_unit", "").strip()

        if daily_goal > 1.0 or unit_str != "":
            habit_type = "measurable"
            target_value = daily_goal
            unit = unit_str if unit_str else None
        else:
            habit_type = "binary"
            target_value = 1.0
            unit = None

        custom_color = h.get("custom_color")
        if custom_color:
            accent_color = f"#{custom_color & 0xFFFFFF:06X}"
        else:
            accent_color = "#3584E4"

        scheduled_days = 127  # Every day
        archived = 0
        created_at = int(h.get("create_t", 0))

        cur.execute(
            """
            INSERT INTO habits (name, habit_type, target_value, unit, accent_color, scheduled_days, archived, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (name, habit_type, target_value, unit, accent_color, scheduled_days, archived, created_at),
        )
        habit_id = cur.lastrowid

        rec_count = 0
        for r in h.get("records", []):
            if r.get("is_deleted", False):
                continue

            r_date = (epoch + datetime.timedelta(days=r["record_date"])).isoformat()
            r_type = r.get("record_type", 1)
            r_val = float(r.get("record_value", 0.0))

            if r_type == 2:
                status = "skipped"
                val = 0.0
            elif r_val == 0.0:
                status = "missed"
                val = 0.0
            else:
                status = "completed"
                val = r_val

            cur.execute(
                """
                INSERT OR REPLACE INTO habit_entries (habit_id, date, entry_status, value)
                VALUES (?, ?, ?, ?)
                """,
                (habit_id, r_date, status, val),
            )
            rec_count += 1

        total_entries += rec_count
        unit_display = f" {unit}" if unit else ""
        print(f"  [{habit_id:2d}] {name:<22} | {habit_type:<10} | Target: {target_value}{unit_display} | Color: {accent_color} | {rec_count:3d} entries")

    conn.commit()

    # Validate database integrity
    integrity = cur.execute("PRAGMA integrity_check;").fetchall()
    assert integrity == [("ok",)], f"Integrity check failed: {integrity}"

    orphans = cur.execute(
        "SELECT COUNT(*) FROM habit_entries WHERE habit_id NOT IN (SELECT id FROM habits);"
    ).fetchone()[0]
    assert orphans == 0, f"Foreign key error: {orphans} orphaned entries found."

    conn.close()
    print(f"\nMigration complete!")
    print(f"  Habits:  {len(active_habits)}")
    print(f"  Entries: {total_entries}")
    print(f"  Database: {output_db_path}")

def main():
    parser = argparse.ArgumentParser(description="Convert JSON habit export to Habit SQLite database.")
    parser.add_argument("input_json", nargs="?", default="export-all-2026_10_27_19_1_33.json", help="Path to JSON file")
    parser.add_argument("output_db", nargs="?", default="habits_backup.db", help="Path to output .db file")
    args = parser.parse_args()

    migrate(args.input_json, args.output_db)

if __name__ == "__main__":
    main()
