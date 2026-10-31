# Independent vectors for what is derived from the master key
# (docs/crypto-protocol.md §"Derived from the master key"): HKDF-SHA512 as
# RFC 5869 defines it, written here with the standard library's HMAC so that
# nothing of the crate's own stack checks itself. The unit tests in
# src/vault/derived.rs hold the crate to this output. To print it again:
#   python3 -I derived-vectors.py
import hashlib, hmac


def hkdf_sha512(salt, ikm, info, length):
    prk = hmac.new(salt, ikm, hashlib.sha512).digest()
    out, block, counter = b"", b"", 1
    while len(out) < length:
        block = hmac.new(prk, block + info + bytes([counter]), hashlib.sha512).digest()
        out += block
        counter += 1
    return out[:length]


# RFC 5869 is defined for SHA-256 in its appendix; this checks the HMAC
# construction itself against test case 1 there, with SHA-256 swapped in.
def hkdf_sha256(salt, ikm, info, length):
    prk = hmac.new(salt, ikm, hashlib.sha256).digest()
    out, block, counter = b"", b"", 1
    while len(out) < length:
        block = hmac.new(prk, block + info + bytes([counter]), hashlib.sha256).digest()
        out += block
        counter += 1
    return out[:length]


assert hkdf_sha256(bytes(range(13)), bytes([0x0b] * 22), bytes(range(0xf0, 0xfa)), 42).hex() == (
    "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
)


def fields(issuer, account, epoch):
    return issuer + account + epoch.to_bytes(8, "big")


def vector(master, issuer, account, epoch, scope):
    f = fields(issuer, account, epoch)
    check = hkdf_sha512(b"baylee-central/key-check/v1", master, b"baylee-central/key-check/v1\0" + f, 32)
    vault = hkdf_sha512(b"baylee-central/local-vault/v1", master, b"baylee-central/local-vault/v1\0" + f + scope, 32)
    print("check ", check.hex())
    print("vault ", vault.hex())


live = hashlib.sha256(b"baylee-central/issuer/v1\0https://central.baylee.app").digest()
print("issuer", live.hex())
account = bytes.fromhex("0192d0c57b3e7a1f8c2d3e4f5a6b7c8d")
vector(bytes(range(32)), live, account, 1, bytes(range(0x40, 0x60)))
vector(bytes([0xff] * 32), bytes([1] * 32), account, 2**64 - 1, bytes(32))
