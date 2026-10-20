/// <reference types="vite/client" />

// Build-time settings (README.md §"Settings"); every one has a default.
interface ImportMetaEnv {
  readonly VITE_FEEDBACK_ORIGIN?: string;
  readonly VITE_CENTRAL_API?: string;
}
