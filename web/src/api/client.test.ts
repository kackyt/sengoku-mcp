import { describe, expect, it, vi } from "vitest";
import { jsonResponse, requestKey, sampleStatus } from "../test/fixtures";
import { createApiClient, createGame, fetchStatus } from "./client";

const TOKEN = "0123456789abcdef0123456789abcdef";

/** 指定のレスポンスを順に返す fetch モックで、再試行を待たないクライアントを作る */
function clientWith(...responses: Array<() => Response | Promise<Response>>) {
  const fetchMock = vi.fn<typeof fetch>();
  for (const response of responses) fetchMock.mockImplementationOnce(async () => response());
  const client = createApiClient({ fetch: fetchMock, retryDelay: () => 0 });
  return { client, fetchMock };
}

/** モックが受け取ったリクエストを「メソッド パス」の形で返す */
function calledKeys(fetchMock: ReturnType<typeof vi.fn<typeof fetch>>): string[] {
  return fetchMock.mock.calls.map(([input, init]) => requestKey(input, init));
}

describe("fetchStatus", () => {
  it("200 なら状況を返す", async () => {
    const { client, fetchMock } = clientWith(() => jsonResponse(200, sampleStatus));
    const result = await fetchStatus(TOKEN, client);
    expect(result).toEqual({ kind: "ok", status: sampleStatus });
    expect(calledKeys(fetchMock)).toEqual([`GET /api/views/${TOKEN}/status`]);
  });

  it.each([
    [409, "waiting_for_join", "waiting_for_join"],
    [409, "daimyo_not_selected", "daimyo_not_selected"],
    [404, "view_not_found", "not_found"],
  ])("HTTP %i（%s）を画面の状態 %s に変換し、再試行しない", async (status, code, kind) => {
    const { client, fetchMock } = clientWith(() => jsonResponse(status, { code, message: "" }));
    expect((await fetchStatus(TOKEN, client)).kind).toBe(kind);
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("一時的なサーバーエラーや通信失敗は再試行し、回復すれば状況を返す", async () => {
    const { client, fetchMock } = clientWith(
      () => new Response("oops", { status: 503 }),
      () => Promise.reject(new TypeError("Failed to fetch")),
      () => jsonResponse(200, sampleStatus),
    );
    expect(await fetchStatus(TOKEN, client)).toEqual({ kind: "ok", status: sampleStatus });
    expect(fetchMock).toHaveBeenCalledTimes(3);
  });

  it("再試行しても失敗したサーバーエラーは HTTP ステータス付きの error を返す", async () => {
    const serverError = () => new Response("oops", { status: 502 });
    const { client, fetchMock } = clientWith(serverError, serverError, serverError);
    expect(await fetchStatus(TOKEN, client)).toEqual({
      kind: "error",
      message: "サーバーエラー（HTTP 502）",
    });
    // 初回 + 再試行2回
    expect(fetchMock).toHaveBeenCalledTimes(3);
  });

  it("再試行しても通信できなければ接続できない旨の error を返す", async () => {
    const networkError = () => Promise.reject(new TypeError("Failed to fetch"));
    const { client, fetchMock } = clientWith(networkError, networkError, networkError);
    expect(await fetchStatus(TOKEN, client)).toEqual({
      kind: "error",
      message: "サーバーに接続できません",
    });
    expect(fetchMock).toHaveBeenCalledTimes(3);
  });
});

describe("createGame", () => {
  it("201 なら作成したゲームを返す", async () => {
    const created = {
      view_token: TOKEN,
      status_url: `/api/views/${TOKEN}/status`,
      join_code: "VQ4X7K",
      join_code_expires_at: "2026-10-08T05:00:00Z",
      join_message: "",
    };
    const { client, fetchMock } = clientWith(() => jsonResponse(201, created));
    expect(await createGame(client)).toEqual(created);
    expect(calledKeys(fetchMock)).toEqual(["POST /api/games"]);
  });

  it("失敗したら二重作成を避けるため再試行せずエラーにする", async () => {
    const { client, fetchMock } = clientWith(() =>
      jsonResponse(500, { code: "internal_error", message: "" }),
    );
    await expect(createGame(client)).rejects.toThrow("HTTP 500");
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
});
