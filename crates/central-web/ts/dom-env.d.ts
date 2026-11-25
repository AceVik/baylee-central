// Global ambient augmentations shared across DOM modules and classic scripts.
// Keep this file free of top-level `import` / `export` so classic configs work.

interface Navigator {
  readonly standalone?: boolean;
  readonly connection?: {
    readonly saveData?: boolean;
  };
}

interface BeforeInstallPromptEvent extends Event {
  prompt(): Promise<void>;
  readonly userChoice: Promise<{ readonly outcome: "accepted" | "dismissed" }>;
}

interface WindowEventMap {
  beforeinstallprompt: BeforeInstallPromptEvent;
}

interface BayleeTrustedHTML {}
interface BayleeTrustedScriptURL {}

interface BayleeTrustedTypePolicy {
  createHTML(input: string): BayleeTrustedHTML;
  createScriptURL(input: string): BayleeTrustedScriptURL;
}

interface BayleeTrustedTypePolicyFactory {
  createPolicy(
    policyName: string,
    policyOptions: {
      createHTML?: (input: string) => string;
      createScriptURL?: (input: string) => string;
    },
  ): BayleeTrustedTypePolicy;
}

interface Window {
  readonly trustedTypes?: BayleeTrustedTypePolicyFactory;
}

interface CSSStyleDeclaration {
  viewTransitionClass: string;
}
