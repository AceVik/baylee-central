// Only authenticated ciphertext is persisted. No password, private key or message.
export interface TrustStore { read(): Uint8Array | null; write(bytes: Uint8Array): void }
export function trustStore(account: Uint8Array): TrustStore {
  const key = `baylee-chat-trust:v1:${Array.from(account, (b) => b.toString(16).padStart(2, "0")).join("")}`;
  return {
    read: () => {
      const raw = localStorage.getItem(key);
      if (raw === null) return null;
      if (raw.length > 1_400_000 || !/^[A-Za-z0-9+/]*={0,2}$/.test(raw)) throw new Error("trust");
      try { return Uint8Array.from(atob(raw), (c) => c.charCodeAt(0)); } catch { throw new Error("trust"); }
    },
    write: (bytes) => {
      let binary = "";
      for (let i = 0; i < bytes.length; i += 4096) binary += String.fromCharCode(...bytes.subarray(i, i + 4096));
      localStorage.setItem(key, btoa(binary));
    },
  };
}
