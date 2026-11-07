// src/utils/nodeId.ts
// Utility for normalizing and comparing Ed25519 EndpointIds / NodeIds across
// RFC4648 lowercase base32 (52 chars) and canonical hex (64 chars) encodings.

const RFC4648_B32_ALPHABET = 'abcdefghijklmnopqrstuvwxyz234567';

/**
 * Convert any 52-character base32 or 64-character hex NodeId into its canonical 64-character hex string.
 * Returns null if the string is invalid.
 */
export function toCanonicalHexNodeId(input?: string | null): string | null {
  if (!input) return null;
  const clean = input.trim().toLowerCase();
  if (/^[0-9a-f]{64}$/.test(clean)) {
    return clean;
  }
  if (/^[a-z2-7]{52}$/.test(clean)) {
    try {
      const bytes = new Uint8Array(32);
      let buffer = 0;
      let bitsLeft = 0;
      let byteIndex = 0;

      for (let i = 0; i < clean.length; i++) {
        const val = RFC4648_B32_ALPHABET.indexOf(clean[i]);
        if (val === -1) return null;
        buffer = (buffer << 5) | val;
        bitsLeft += 5;
        if (bitsLeft >= 8) {
          bitsLeft -= 8;
          if (byteIndex < 32) {
            bytes[byteIndex++] = (buffer >> bitsLeft) & 0xff;
          }
        }
      }

      if (byteIndex === 32) {
        return Array.from(bytes)
          .map((b) => b.toString(16).padStart(2, '0'))
          .join('');
      }
    } catch {
      return null;
    }
  }
  return null;
}

/**
 * Compare two node IDs for equivalence regardless of whether they are formatted
 * as base32 or canonical hex.
 */
export function isSameNodeId(a?: string | null, b?: string | null): boolean {
  if (!a || !b) return false;
  const t1 = a.trim();
  const t2 = b.trim();
  if (t1.toLowerCase() === t2.toLowerCase()) return true;

  const hex1 = toCanonicalHexNodeId(t1);
  const hex2 = toCanonicalHexNodeId(t2);
  return hex1 !== null && hex2 !== null && hex1 === hex2;
}
