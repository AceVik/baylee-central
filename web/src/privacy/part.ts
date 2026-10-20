import type { ReactNode } from "react";

/** One section of the notice, in both languages. */
export interface Part {
  id: string;
  title: [string, string];
  body: [ReactNode, ReactNode];
  basis?: [string, string];
}
