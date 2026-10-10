/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** REST API のベースURL（未指定なら同一オリジン） */
  readonly VITE_API_BASE_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
