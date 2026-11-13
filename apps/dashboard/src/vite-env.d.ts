/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Override the Kira API origin baked into the build. */
  readonly VITE_API_BASE_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
