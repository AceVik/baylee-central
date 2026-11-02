import { answer, exact, malformed, poster, safeInteger, type Json } from "../account/api";
export type Category = "chat" | "contacts" | "updates" | "feedback" | "gateways";
export const BITS: Record<Category, number> = { chat: 1, contacts: 2, updates: 4, feedback: 8, gateways: 16 };
export interface PushState { enabled: number; publicKey: string | null; subscribed: boolean }
const post = poster({});
export function notifications(request: Json): Promise<PushState> {
  return post({ operation: "notifications", request }, (raw) => {
    const state = exact(answer(raw, "notifications", ["state"])["state"], ["enabled", "public_key", "subscribed"]);
    const enabled = safeInteger(state["enabled"], 0);
    const key = state["public_key"];
    if (enabled > 31 || typeof state["subscribed"] !== "boolean" || (key !== null && (typeof key !== "string" || !/^B[\w-]{86}$/.test(key)))) throw malformed();
    return { enabled, publicKey: key, subscribed: state["subscribed"] };
  });
}
