// 閲覧トークンと参加コードの保持
//
// - URL の `?token=` を最優先する（MCPツールの結果に付く閲覧URLから開いた場合など）
// - 再読み込みしても続きから表示できるよう、localStorage にも保存する
//   （プライベートブラウズ等で使えない場合は URL のみで動作する）

const STORAGE_KEY = "sengoku-web:session";
const TOKEN_PATTERN = /^[0-9a-f]{32}$/;

/** ブラウザで保持するゲームの情報 */
export interface StoredSession {
  /** 閲覧トークン */
  token: string;
  /** 参加コード（ブラウザで作成したゲームの場合のみ） */
  joinCode?: string;
  /** 参加コードの有効期限（ISO 8601） */
  joinCodeExpiresAt?: string;
}

/** 閲覧トークンの形式（32桁の16進数）かどうか */
export function isValidToken(value: string | null | undefined): value is string {
  return typeof value === "string" && TOKEN_PATTERN.test(value);
}

function readStorage(): StoredSession | null {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as StoredSession;
    return isValidToken(parsed.token) ? parsed : null;
  } catch {
    return null;
  }
}

function writeStorage(session: StoredSession | null): void {
  try {
    if (session) {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
    } else {
      window.localStorage.removeItem(STORAGE_KEY);
    }
  } catch {
    // 保存できない環境では URL のみで動作する
  }
}

/** URL に閲覧トークンを反映する（履歴は増やさない） */
function writeUrlToken(token: string | null): void {
  const url = new URL(window.location.href);
  if (token) {
    url.searchParams.set("token", token);
  } else {
    url.searchParams.delete("token");
  }
  window.history.replaceState(null, "", url);
}

/** 現在のゲーム情報を読み込む（URL のトークンを優先） */
export function loadSession(): StoredSession | null {
  const urlToken = new URL(window.location.href).searchParams.get("token");
  const stored = readStorage();
  if (isValidToken(urlToken)) {
    // 保存済みと同じゲームなら参加コード情報も引き継ぐ
    const session = stored?.token === urlToken ? stored : { token: urlToken };
    writeStorage(session);
    return session;
  }
  if (stored) writeUrlToken(stored.token);
  return stored;
}

/** ゲーム情報を保存する（null で破棄） */
export function saveSession(session: StoredSession | null): void {
  writeStorage(session);
  writeUrlToken(session?.token ?? null);
}
