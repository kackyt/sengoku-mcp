// REST API（api-server）のクライアント
//
// 型は api-server/openapi.json から生成した schema.d.ts（`pnpm gen:api`）を使用します。
import type { components } from "./schema";

export type MyStatus = components["schemas"]["MyStatusDto"];
export type KuniStatus = components["schemas"]["KuniStatusDto"];
export type OtherKuni = components["schemas"]["OtherKuniDto"];
export type Daimyo = components["schemas"]["DaimyoDto"];
export type CreatedGame = components["schemas"]["CreatedGameDto"];
export type ErrorCode = components["schemas"]["ErrorCode"];

/** API のベースURL（未指定なら同一オリジン。開発時は Vite のプロキシ経由） */
const API_BASE_URL: string = import.meta.env.VITE_API_BASE_URL ?? "";

/**
 * 状況取得の結果
 *
 * HTTP ステータスとエラーコードを、画面の状態として扱いやすい形に変換したものです。
 */
export type StatusResult =
  | { kind: "ok"; status: MyStatus }
  | { kind: "waiting_for_join" }
  | { kind: "daimyo_not_selected" }
  | { kind: "not_found" }
  | { kind: "error"; message: string };

/** エラーレスポンスからエラーコードを取り出す（JSON でない場合は undefined） */
async function readErrorCode(response: Response): Promise<ErrorCode | undefined> {
  try {
    const body = (await response.json()) as Partial<components["schemas"]["ErrorResponse"]>;
    return body.code;
  } catch {
    return undefined;
  }
}

/** 閲覧トークンで自国の状況を取得する */
export async function fetchStatus(
  token: string,
  fetchImpl: typeof fetch = fetch,
): Promise<StatusResult> {
  let response: Response;
  try {
    response = await fetchImpl(
      `${API_BASE_URL}/api/views/${encodeURIComponent(token)}/status`,
      { headers: { Accept: "application/json" } },
    );
  } catch {
    return { kind: "error", message: "サーバーに接続できません" };
  }

  if (response.ok) {
    return { kind: "ok", status: (await response.json()) as MyStatus };
  }
  const code = await readErrorCode(response);
  switch (code) {
    case "waiting_for_join":
      return { kind: "waiting_for_join" };
    case "daimyo_not_selected":
      return { kind: "daimyo_not_selected" };
    case "view_not_found":
    case "session_not_found":
      return { kind: "not_found" };
    default:
      return { kind: "error", message: `サーバーエラー（HTTP ${response.status}）` };
  }
}

/** 新規ゲームを作成し、閲覧トークンと参加コードを受け取る */
export async function createGame(fetchImpl: typeof fetch = fetch): Promise<CreatedGame> {
  const response = await fetchImpl(`${API_BASE_URL}/api/games`, {
    method: "POST",
    headers: { Accept: "application/json" },
  });
  if (response.status !== 201) {
    throw new Error(`ゲームを作成できませんでした（HTTP ${response.status}）`);
  }
  return (await response.json()) as CreatedGame;
}
