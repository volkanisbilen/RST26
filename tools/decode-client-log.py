"""Read the 2615 client's framed log.klg without modifying it."""
import argparse
import struct
from pathlib import Path


def decode_records(data):
    offset = 0
    while offset < len(data):
        if len(data) - offset < 4:
            raise ValueError(f"Incomplete length at {offset}")
        size = struct.unpack_from('<I', data, offset)[0]
        offset += 4
        if size > len(data) - offset:
            raise ValueError(f"Incomplete record at {offset}: {size} bytes")
        key = 0x816
        plain = bytearray()
        for cipher in data[offset:offset + size]:
            plain.append(cipher ^ (key >> 8))
            key = ((cipher + key) * 0x6081 + 0x1608) & 0xffff
        offset += size
        yield plain.decode('cp1254', errors='replace')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('path', type=Path)
    parser.add_argument('--contains', default='')
    args = parser.parse_args()
    for record in decode_records(args.path.read_bytes()):
        if args.contains.casefold() in record.casefold():
            print(record.rstrip())
