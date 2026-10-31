# Independent vectors for the pairing's MACs and comparison code
# (docs/crypto-protocol.md §"Pairing"), with the standard library's HMAC and
# SHA-256 only. The unit tests in src/pairing/tests.rs hold the crate to this
# output. The seal itself is pinned by RFC 9180's Appendix A.2.1 there. To
# print it again:
#   python3 -I pairing-vectors.py
import hashlib, hmac

CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"

issuer = hashlib.sha256(b"baylee-central/issuer/v1\0https://central.baylee.app").digest()
secret = bytes(range(0x10, 0x30))
handle = bytes(range(0x30, 0x50))
public = bytes(range(0x50, 0x70))
verifier = bytes(range(0x70, 0x90))
check = bytes(range(0x90, 0xb0))
sealed = bytes(range(0xb0, 0x100))  # 80 bytes

challenge = hashlib.sha256(verifier).digest()
mac = hmac.new(secret, b"baylee-central/pair-claim/v1\0" + issuer + handle + public + challenge, hashlib.sha256).digest()
proof = hmac.new(check, b"baylee-central/pair-approve/v1\0" + handle + public + challenge + hashlib.sha256(sealed).digest(), hashlib.sha256).digest()
digest = hashlib.sha256(b"baylee-central/pair-code/v1\0" + handle + public).digest()
bits = int.from_bytes(digest[:4], "big") >> 2
symbols = "".join(CROCKFORD[(bits >> (25 - 5 * i)) & 0x1f] for i in range(6))
assert len(sealed) == 80

print("challenge", challenge.hex())
print("mac      ", mac.hex())
print("proof    ", proof.hex())
print("code     ", symbols[:3] + "-" + symbols[3:])
