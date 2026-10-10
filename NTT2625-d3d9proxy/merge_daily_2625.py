"""Merge only the reserved daily quest rows into a 2625 client; preserve its other rows."""
from pathlib import Path
import shutil
import sys
import kotbl_rows as tbl

source = Path(sys.argv[1])
client = Path(sys.argv[2])
backup = Path(sys.argv[3])
backup.mkdir(parents=True, exist_ok=True)
files = {
    "Quest_Menu_us.tbl": lambda row: row[0] in (60010, 60021, 60022) or 61000 < row[0] <= 61200,
    "Quest_Talk_us.tbl": lambda row: row[0] in (60010, 60021, 60022) or 61000 < row[0] <= 61200,
    "Quest_Guide_us.tbl": lambda row: 3600 < row[0] <= 3800,
    "Quest_Helper.tbl": lambda row: 21000 < row[0] <= 21200,
    "Quest_Monster_Exchange.tbl": lambda row: 3600 < row[0] <= 3800,
}
prepared = []
for name, selected in files.items():
    current = tbl.load(str(client / name))
    patch = tbl.load(str(source / name))
    integer_types = {1, 2, 3, 4, 5, 6, 10, 11}
    if len(current.types) != len(patch.types) or any(
        a != b and not (a in integer_types and b in integer_types)
        for a, b in zip(current.types, patch.types)
    ):
        raise ValueError(f"Schema mismatch: {name}; no client files modified")
    rows = [row for row in patch.rows if selected(row)]
    if not rows:
        raise ValueError(f"No daily rows found: {name}")
    existing = {row[0]: row for row in current.rows}
    for row in rows:
        old = existing.get(row[0])
        if old and old != row and not any(isinstance(v, bytes) and b'daily' in v.lower() for v in old):
            raise ValueError(f"Reserved key collision: {name}:{row[0]}; no files modified")
        existing[row[0]] = row
    current.rows = list(existing.values())
    tbl.build(current)  # validate integer widths before any file is modified
    prepared.append((name, current, len(rows)))

for name, table, count in prepared:
    destination = client / name
    saved = backup / name
    if not saved.exists():
        shutil.copy2(destination, saved)
    tbl.save(str(destination), table)
    verified = tbl.load(str(destination))
    assert verified.rows == table.rows
    print(f"{name}: merged {count} daily rows, total {len(table.rows)}; verified")
