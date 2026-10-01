#!/usr/bin/env python3
"""Read-only inventory of the LIVE Stratum vault used for the journal drift
reproduction (t_12ffd5ff). Compares journal .md files on disk vs journal pages
registered in SQLite blocks.db. Does NOT modify anything.

Usage: python3 inspect_vault.py [VAULT_PATH]   (default: ~/StratumVault)
"""
import sqlite3
import os
import sys

def main() -> int:
    vault = sys.argv[1] if len(sys.argv) > 1 else os.path.expanduser("~/StratumVault")
    db = os.path.join(vault, ".pkm", "blocks.db")

    if not os.path.exists(db):
        print(f"no blocks.db at {db}")
        return 1

    con = sqlite3.connect(db)
    try:
        tables = [r[0] for r in con.execute("SELECT name FROM sqlite_master WHERE type='table'")]
        print("TABLES:", tables)

        pages = con.execute("SELECT path FROM pages").fetchall()
        db_paths = [r[0] for r in pages]
        print("PAGES from table pages:", len(db_paths))

        journals_dir = os.path.join(vault, "journals")
        disk_journals = []
        if os.path.isdir(journals_dir):
            disk_journals = [f"journals/{f}" for f in sorted(os.listdir(journals_dir))
                             if f.endswith(".md")]

        disk_set = set(disk_journals)
        db_set = set(db_paths)

        absent = sorted(disk_set - db_set)
        orphans = sorted(db_set - disk_set)
        print("\nDisk journal files:   ", len(disk_set))
        print("DB journal pages:     ", len(db_set))
        print("On disk but NOT in DB (absent): ", len(absent))
        print("In DB but NOT on disk (orphans):", len(orphans))
        if absent:
            print("  absent:", absent[:20])
        if orphans:
            print("  orphans:", orphans[:20])

        print("\nBlock counts for shared journal pages (sample):")
        for p in sorted(shared := disk_set & db_set)[:15]:
            n = con.execute("SELECT COUNT(*) FROM blocks WHERE page_path=?", (p,)).fetchone()[0]
            print(f"  {p}  blocks={n}")
    finally:
        con.close()
    return 0

if __name__ == "__main__":
    sys.exit(main())
