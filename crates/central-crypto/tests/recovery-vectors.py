# Independent recovery-code vectors (docs/crypto-protocol.md §"Recovery code"):
# Python's RFC 4648 base32 translated to Crockford's alphabet, pyca/cryptography's
# HKDF-SHA512 and libsodium's XChaCha20-Poly1305 (PyNaCl). The unit tests in
# src/vault/recovery.rs hold the crate to this output. To print it again:
#   uv run --no-project --with pynacl==1.6.2 --with cryptography==46.0.3 python -I recovery-vectors.py
import base64, hashlib
from cryptography.hazmat.primitives.hashes import SHA512
from cryptography.hazmat.primitives.kdf.hkdf import HKDF
from nacl.bindings import crypto_aead_xchacha20poly1305_ietf_encrypt as xchacha

RFC = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"
CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"

def text(code):
    symbols = base64.b32encode(code).decode().translate(str.maketrans(RFC, CROCKFORD))
    assert len(symbols) == 32 and "=" not in symbols
    return "-".join(symbols[i:i + 4] for i in range(0, 32, 4))

def vector(code, issuer, account, epoch, master, nonce):
    fields = issuer + account + epoch.to_bytes(8, "big")
    info = b"baylee-central/master-wrap/v1\0" + fields
    aad = b"baylee-central/recovery-wrap/v1\0" + fields
    key = HKDF(algorithm=SHA512(), length=32, salt=b"baylee-central/recovery/v1", info=info).derive(code)
    wire = b"\x01" + nonce + xchacha(master, aad, nonce, key)
    assert len(wire) == 73
    print("text  ", text(code))
    print("key   ", key.hex())
    print("wire  ", wire.hex())

live = hashlib.sha256(b"baylee-central/issuer/v1\0https://central.baylee.app").digest()
print("issuer", live.hex())
account = bytes.fromhex("0192d0c57b3e7a1f8c2d3e4f5a6b7c8d")
vector(bytes.fromhex("f00dfacecafebeef0123456789abcdeffedcba98"), live, account, 1,
       bytes(range(32)), bytes(range(0xa0, 0xb8)))
vector(bytes(20), bytes([1] * 32), account, 2**64 - 1, bytes([0xff] * 32), bytes(24))
print("ff    ", text(bytes([0xff] * 20)))
