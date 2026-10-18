r"""Akara Altar: writes akara_altar.sql from the client's Data\Special_Auction_us.tbl (the table CUISpecialAuction
reads), so the server item list is always the same as the client's.
  AKARA_ITEM (nList, bSlot, nItemID, sCount, nStartPrice, nStep)  list = round block * 14 + day (0..13)
  AKARA_BID  one row per character bid / claim (see AkaraAltar.cpp)
Usage: python build_akara_sql.py <gameDir> <out.sql>"""
import sys, os, struct
import kotbl

def parse(path):
    d = kotbl.decrypt(open(path, 'rb').read(), 'rev')
    at = 5
    nc = struct.unpack_from('<I', d, at)[0]; types = struct.unpack_from('<%dI' % nc, d, at + 4); at += 4 + 4 * nc
    nr = struct.unpack_from('<I', d, at)[0]; at += 4
    size = {5: 4, 6: 4, 10: 8}
    fmt = {5: '<I', 6: '<i', 10: '<Q'}
    rows = []
    for _ in range(nr):
        vals = []
        for t in types:
            vals.append(struct.unpack_from(fmt[t], d, at)[0]); at += size[t]
        rows.append(vals)
    return rows

game, out = sys.argv[1], sys.argv[2]
rows = parse(os.path.join(game, 'Data', 'Special_Auction_us.tbl'))
items = []
for r in rows:
    for k in range(8):
        item, count, price, step, extra = r[1 + 5 * k:6 + 5 * k]
        if item:
            items.append((r[0], k, item, count or 1, price, step))
with open(out, 'w', encoding='utf-8') as f:
    f.write('USE [KO_DATABASE_SERVER_00125]\nGO\n')
    f.write('''IF OBJECT_ID('dbo.AKARA_ITEM') IS NULL
CREATE TABLE dbo.AKARA_ITEM (
  nList int NOT NULL, bSlot tinyint NOT NULL, nItemID int NOT NULL, sCount smallint NOT NULL,
  nStartPrice bigint NOT NULL, nStep int NOT NULL,
  CONSTRAINT PK_AKARA_ITEM PRIMARY KEY (nList, bSlot));
IF OBJECT_ID('dbo.AKARA_BID') IS NULL
CREATE TABLE dbo.AKARA_BID (
  nID int IDENTITY(1,1) NOT NULL PRIMARY KEY,
  nDate int NOT NULL,               -- auction day yyyymmdd
  nList int NOT NULL, bSlot tinyint NOT NULL, nItemID int NOT NULL, sCount smallint NOT NULL,
  strUserID varchar(21) NOT NULL,
  nPrice bigint NOT NULL,           -- noah paid (kept by the altar until refunded)
  bState tinyint NOT NULL,          -- 1 highest bidder, 2 outbid (noah to collect), 3 won (item to claim), 4 cancelled
  bClaimed tinyint NOT NULL DEFAULT 0,
  dtTime datetime NOT NULL DEFAULT GETDATE());
IF NOT EXISTS (SELECT 1 FROM sys.indexes WHERE name = 'IX_AKARA_BID_USER')
CREATE INDEX IX_AKARA_BID_USER ON dbo.AKARA_BID (strUserID, bClaimed);
GO
DELETE FROM dbo.AKARA_ITEM;
''')
    for i in range(0, len(items), 200):
        part = items[i:i + 200]
        f.write('INSERT INTO dbo.AKARA_ITEM (nList, bSlot, nItemID, sCount, nStartPrice, nStep) VALUES\n')
        f.write(',\n'.join('(%d, %d, %d, %d, %d, %d)' % it for it in part) + ';\n')
    f.write('GO\n')
print('items', len(items), '->', out)
