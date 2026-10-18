#!/usr/bin/env python3
"""Interactively configure the JstKO admin password on the VPS."""

from __future__ import annotations

import getpass
import hashlib
import os
import secrets
import subprocess
import tempfile
from pathlib import Path


ENV_FILE = Path("/etc/ko-server.env")
SERVICE = "ko-server.service"
ITERATIONS = 310_000


def main() -> None:
    if os.geteuid() != 0:
        raise SystemExit("Run this utility with sudo: sudo python3 tools/set-admin-password.py")

    password = getpass.getpass("Yeni panel parolasi (en az 12 karakter): ")
    confirmation = getpass.getpass("Parolayi tekrar girin: ")
    if len(password) < 12:
        raise SystemExit("Parola en az 12 karakter olmali.")
    if not secrets.compare_digest(password, confirmation):
        raise SystemExit("Parolalar eslesmiyor; hicbir degisiklik yapilmadi.")

    salt = secrets.token_bytes(16)
    derived = hashlib.pbkdf2_hmac("sha256", password.encode("utf-8"), salt, ITERATIONS, 32)
    encoded = f"pbkdf2_sha256${ITERATIONS}${salt.hex()}${derived.hex()}"
    del password, confirmation, derived

    lines = ENV_FILE.read_text(encoding="utf-8").splitlines()
    lines = [line for line in lines if not line.startswith("ADMIN_PASSWORD_HASH=")]
    lines.append(f"ADMIN_PASSWORD_HASH={encoded}")
    payload = ("\n".join(lines) + "\n").encode("utf-8")

    fd, temp_name = tempfile.mkstemp(prefix=".ko-server.env.", dir=ENV_FILE.parent)
    try:
        os.fchmod(fd, 0o600)
        with os.fdopen(fd, "wb") as temp_file:
            temp_file.write(payload)
            temp_file.flush()
            os.fsync(temp_file.fileno())
        os.replace(temp_name, ENV_FILE)
        os.chmod(ENV_FILE, 0o600)
    finally:
        if os.path.exists(temp_name):
            os.unlink(temp_name)

    subprocess.run(["systemctl", "restart", SERVICE], check=True)
    subprocess.run(["systemctl", "is-active", "--quiet", SERVICE], check=True)
    print("Panel parolasi kaydedildi; oyun sunucusu yeniden baslatildi.")


if __name__ == "__main__":
    main()
