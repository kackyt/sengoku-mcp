import ky, { isHTTPError, type KyInstance } from "ky";
import { type Client, createClient, createConfig } from "./generated/client";
import { createGame as postGame, viewStatus } from "./generated/sdk.gen";
import type {
  CreatedGameDto,
  DaimyoDto,
  ErrorCode,
  ErrorResponse,
  KuniStatusDto,
  MyStatusDto,
  OtherKuniDto,
} from "./generated/types.gen";

// api-server/openapi.json から @hey-api/openapi-ts で生成した型（pnpm gen:api）に、画面側の名前を付ける
export type MyStatus = MyStatusDto;
export type KuniStatus = KuniStatusDto;
export type OtherKuni = OtherKuniDto;
export type Daimyo = DaimyoDto;
export type CreatedGame = CreatedGameDto;
export type { ErrorCode };

/**
 * API のベースURL（未指定なら同一オリジン。開発時は Vite のプロキシ経由）
 *
 * 生成クライアントは Request オブジェクトを作るため、相対パスではなく絶対URLにしておく。
 */
const API_BASE_URL: string = import.meta.env.VITE_API_BASE_URL || globalThis.location.origin;

/** 一時的な障害とみなして再試行する HTTP ステータス（タイムアウト・混雑・サーバーエラー） */
const RETRY_STATUS_CODES = [408, 429, 500, 502, 503, 504];

/** 再試行の回数（初回を除く） */
const RETRY_LIMIT = 2;

export interface ApiClientOptions {
  /** API のベースURL */
  baseUrl?: string;
  /** 実際に通信する fetch（テストでモックに差し替える） */
  fetch?: typeof fetch;
  /** 再試行までの待ち時間（ミリ秒）。未指定なら ky の指数バックオフ */
  retryDelay?: (attemptCount: number) => number;
}

/**
 * ky で送信し、一時的なエラーは再試行したうえで Response を返す
 *
 * 生成クライアント（fetch）は Response のステータスと本文でエラーを判定するため、
 * 再試行しても失敗した HTTP エラー（ky の HTTPError）は、読み取り済みの本文から Response に戻して返す。
 * 409（参加待ち）・404（ゲームなし）などは再試行の対象外なので、そのまま返る。
 */
async function sendWithRetry(api: KyInstance, request: Request): Promise<Response> {
  try {
    return await api(request);
  } catch (error) {
    if (!isHTTPError(error)) throw error;
    const { data, response } = error;
    const body = data === undefined ? null : typeof data === "string" ? data : JSON.stringify(data);
    return new Response(body, {
      status: response.status,
      statusText: response.statusText,
      headers: response.headers,
    });
  }
}

/**
 * REST API クライアント（生成した fetch クライアントの通信部分を ky に差し替えたもの）を作る
 *
 * GET は通信失敗・一時的なエラー（RETRY_STATUS_CODES）を最大 RETRY_LIMIT 回まで再試行する。
 * POST（ゲーム作成）は二重作成を避けるため再試行しない。
 */
export function createApiClient({
  baseUrl = API_BASE_URL,
  // 呼び出し時点の fetch を使う（テストでの差し替えに追従させる）
  fetch: fetchImpl = (input, init) => globalThis.fetch(input, init),
  retryDelay,
}: ApiClientOptions = {}): Client {
  const api = ky.create({
    fetch: fetchImpl,
    retry: {
      limit: RETRY_LIMIT,
      methods: ["get"],
      statusCodes: RETRY_STATUS_CODES,
      ...(retryDelay && { delay: retryDelay }),
    },
    // 再試行の対象となるステータスだけを ky のエラーにする（それ以外は Response のまま返す）
    throwHttpErrors: (status) => RETRY_STATUS_CODES.includes(status),
  });
  return createClient(
    createConfig({ baseUrl, fetch: (input) => sendWithRetry(api, input as Request) }),
  );
}

/** 画面で使う既定のクライアント */
const defaultClient = createApiClient();

/** 状況の取得結果（画面の状態に対応） */
export type StatusResult =
  | { kind: "ok"; status: MyStatus }
  | { kind: "waiting_for_join" }
  | { kind: "daimyo_not_selected" }
  | { kind: "not_found" }
  | { kind: "error"; message: string };

/** エラーレスポンス（ErrorResponse）ならエラーコードを取り出す */
function errorCodeOf(error: unknown): ErrorCode | undefined {
  if (typeof error === "object" && error !== null && "code" in error) {
    return (error as ErrorResponse).code;
  }
  return undefined;
}

/**
 * 閲覧トークンで自国の状況を取得し、画面の状態に変換する
 *
 * - 200: 状況を表示
 * - 409 waiting_for_join: 参加コードの入力待ち
 * - 409 daimyo_not_selected: 大名の選択待ち
 * - 404: ゲームが存在しない（終了済み・トークン無効）
 */
export async function fetchStatus(token: string, client: Client = defaultClient): Promise<StatusResult> {
  const { data, error, response } = await viewStatus({ client, path: { token } });
  if (data) return { kind: "ok", status: data };
  // 再試行しても通信できなかった（レスポンスなし）
  if (!response) return { kind: "error", message: "サーバーに接続できません" };

  switch (errorCodeOf(error)) {
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

/** 新しいゲームを作成する（参加コードと閲覧トークンを受け取る） */
export async function createGame(client: Client = defaultClient): Promise<CreatedGame> {
  const { data, response } = await postGame({ client });
  if (!data) {
    throw new Error(
      response
        ? `ゲームを作成できませんでした（HTTP ${response.status}）`
        : "サーバーに接続できません",
    );
  }
  return data;
}
