<?php
declare(strict_types=1);

// Replicates KnightOnline.exe's own password encoding exactly (reverse-engineered from the
// decompiled client at docs/reverse-engineering/2625/client_src/KnightOnline.exe.c,
// sub_6FD530 @ 0x6FD530, verified against 8 known (plaintext, encoded) pairs produced by the
// real client). tb_user.str_passwd is never plaintext -- it's this encoding's output, produced
// client-side before the password ever reaches the wire. The server/this panel just store and
// compare whatever representation arrives/is computed; neither ever sees the "real" plaintext
// again once encoded. Case-sensitive: do not uppercase/lowercase the password before calling
// this.
//
// Per 4-byte (little-endian) chunk of the password (last chunk zero-padded to 4 bytes; a
// password whose length isn't a multiple of 4 reads past its own end into guaranteed-zero
// padding, exactly as Knight Online 2625 client's Dst[256] buffer does):
//   x = (chunk_as_u32_le + 1000) mod 2^32
//   v8 = bit-permute(x) using the fixed 32-entry table below (bit i of x -> bit PERM[i] of v8)
//   emit 7 base-36 digits of v8, least-significant digit first, alphabet "0-9A-Z"

const KO_PASSWORD_PERM = [
    26, 31, 17, 10, 30, 16, 24, 2, 29, 8, 20, 15, 28, 11, 13, 4,
    19, 23, 0, 12, 14, 27, 6, 18, 21, 3, 9, 7, 22, 1, 25, 5,
];
const KO_PASSWORD_ALPHABET = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';

function jstko_ko_password_encode(string $password): string
{
    // Match the client's own input limit (str_passwd is VARCHAR(28), 4 output chars per input
    // byte on average... in practice this is already far more than any real password needs).
    $data = substr($password, 0, 64);
    $len = strlen($data);
    $data = $data . str_repeat("\x00", 4); // guaranteed zero padding for the last partial chunk
    $chunks = $len > 0 ? (int)ceil($len / 4) : 0;

    $out = '';
    for ($c = 0; $c < $chunks; $c++) {
        $bytes = substr($data, $c * 4, 4);
        $dword = ord($bytes[0]) | (ord($bytes[1]) << 8) | (ord($bytes[2]) << 16) | (ord($bytes[3]) << 24);
        $x = ($dword + 1000) & 0xFFFFFFFF;

        $v8 = 0;
        $n = $x;
        $i = 0;
        while ($n !== 0 && $i < 32) {
            if ($n & 1) {
                $v8 += 1 << KO_PASSWORD_PERM[$i];
            }
            $n >>= 1;
            $i++;
        }

        for ($d = 0; $d < 7; $d++) {
            $out .= KO_PASSWORD_ALPHABET[$v8 % 36];
            $v8 = intdiv($v8, 36);
        }
    }
    return $out;
}
