// The Impressum's proof of work, the same puzzle as Baylee's feedback
// service checks (crates/baylee-feedback/src/impressum.rs) and as the old
// site's impressum-worker.js solved:
//
//   for part = 0 .. parts-1, find the smallest counter c (0, 1, 2, …) with
//   SHA-256( UTF-8( nonce + ":" + part + ":" + c ) )
//   starting with at least `difficulty` zero bits;
//   `nonce` is the challenge's nonce as sent (base64url text), `part` and `c`
//   in decimal. The answer is the list of counters.
//
// SHA-256 is written out because crypto.subtle answers each hash
// asynchronously, far too slowly for a million of them. This is no
// cryptography of ours: the puzzle only costs time, and the service checks
// every answer with a reviewed SHA-256.

const K = new Int32Array([
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98,
  0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
  0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8,
  0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
  0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819,
  0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
  0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
  0xc67178f2,
]);
const W = new Int32Array(64);
const H = new Int32Array(8);
const BUF = new Uint8Array(256);
const DV = new DataView(BUF.buffer);

function at(a: Int32Array, i: number): number {
  return a[i] ?? 0;
}

/** SHA-256 of the first `len` bytes of BUF (len ≤ 247) into H. */
function digest(len: number): Int32Array {
  BUF[len] = 0x80;
  const blocks = (len + 9 + 63) >> 6;
  const end = blocks << 6;
  for (let z = len + 1; z < end - 4; z++) BUF[z] = 0;
  DV.setUint32(end - 4, len * 8, false);
  H.set([0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19]);
  for (let b = 0; b < blocks; b++) {
    for (let t = 0; t < 16; t++) W[t] = DV.getInt32((b << 6) + (t << 2), false);
    for (let t = 16; t < 64; t++) {
      const x = at(W, t - 15);
      const y = at(W, t - 2);
      const s0 = ((x >>> 7) | (x << 25)) ^ ((x >>> 18) | (x << 14)) ^ (x >>> 3);
      const s1 = ((y >>> 17) | (y << 15)) ^ ((y >>> 19) | (y << 13)) ^ (y >>> 10);
      W[t] = (at(W, t - 16) + s0 + at(W, t - 7) + s1) | 0;
    }
    let a = at(H, 0);
    let bb = at(H, 1);
    let c = at(H, 2);
    let d = at(H, 3);
    let e = at(H, 4);
    let f = at(H, 5);
    let g = at(H, 6);
    let h = at(H, 7);
    for (let t = 0; t < 64; t++) {
      const S1 = ((e >>> 6) | (e << 26)) ^ ((e >>> 11) | (e << 21)) ^ ((e >>> 25) | (e << 7));
      const t1 = (h + S1 + ((e & f) ^ (~e & g)) + at(K, t) + at(W, t)) | 0;
      const S0 = ((a >>> 2) | (a << 30)) ^ ((a >>> 13) | (a << 19)) ^ ((a >>> 22) | (a << 10));
      const t2 = (S0 + ((a & bb) ^ (a & c) ^ (bb & c))) | 0;
      h = g;
      g = f;
      f = e;
      e = (d + t1) | 0;
      d = c;
      c = bb;
      bb = a;
      a = (t1 + t2) | 0;
    }
    H[0] = (at(H, 0) + a) | 0;
    H[1] = (at(H, 1) + bb) | 0;
    H[2] = (at(H, 2) + c) | 0;
    H[3] = (at(H, 3) + d) | 0;
    H[4] = (at(H, 4) + e) | 0;
    H[5] = (at(H, 5) + f) | 0;
    H[6] = (at(H, 6) + g) | 0;
    H[7] = (at(H, 7) + h) | 0;
  }
  return H;
}

/** Writes `text` (ASCII) into BUF at 0; its length. */
function put(text: string): number {
  if (text.length > 200) throw new Error("too long");
  for (let i = 0; i < text.length; i++) BUF[i] = text.charCodeAt(i) & 0x7f;
  return text.length;
}

/** Writes `n` in decimal into BUF at `from`; the new length. */
function putNumber(from: number, n: number): number {
  let digits = 1;
  for (let m = n; m >= 10; m = Math.floor(m / 10)) digits++;
  for (let i = digits - 1, m = n; i >= 0; i--, m = Math.floor(m / 10)) BUF[from + i] = 48 + (m % 10);
  return from + digits;
}

/** Whether the first `bits` bits of `h` are zero. */
function leadingZeros(h: Int32Array, bits: number): boolean {
  for (let i = 0; bits > 0; i++, bits -= 32) {
    const word = at(h, i) >>> 0;
    if (bits >= 32) {
      if (word !== 0) return false;
    } else if (word >>> (32 - bits) !== 0) return false;
  }
  return true;
}

/** The counters for every part; `progress(done, parts)` after each part. */
export function solve(nonce: string, parts: number, difficulty: number, progress?: (done: number, all: number) => void): number[] {
  const counters: number[] = [];
  for (let p = 0; p < parts; p++) {
    const start = put(`${nonce}:${p}:`);
    for (let c = 0; ; c++) {
      if (leadingZeros(digest(putNumber(start, c)), difficulty)) {
        counters.push(c);
        break;
      }
    }
    progress?.(p + 1, parts);
  }
  return counters;
}

/** SHA-256 of ASCII `text` in hex, for tests against a reference. */
export function hex(text: string): string {
  const h = digest(put(text));
  let s = "";
  for (let i = 0; i < 8; i++) s += (at(h, i) >>> 0).toString(16).padStart(8, "0");
  return s;
}
