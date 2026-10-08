import { describe, expect, it, vi } from "vitest";
import { jsonResponse, sampleStatus } from "../test/fixtures";
import { createGame, fetchStatus } from "./client";

const TOKEN = "0123456789abcdef0123456789abcdef";

describe("fetchStatus", () => {
  it("200 なら状況を返す", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, sampleStatus));
    const result = await fetchStatus(TOKEN, fetchMock);
    expect(result).toEqual({ kind: "ok", status: sampleStatus });
    expect(fetchMock).toHaveBeenCalledWith(`/api/views/${TOKEN}/status`, expect.anything());
  });

  it.each([
    [409, "waiting_for_join", "waiting_for_join"],
    [409, "daimyo_not_selected", "daimyo_not_selected"],
    [404, "view_not_found", "not_found"],
  ])("HTTP %i（%s）を画面の状態 %s に変換する", async (status, code, kind) => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(status, { code, message: "" }));
    expect((await fetchStatus(TOKEN, fetchMock)).kind).toBe(kind);
  });

  it("想定外のエラーや通信失敗は error を返す", async () => {
    const serverError = vi.fn().mockResolvedValue(new Response("oops", { status: 502 }));
    expect(await fetchStatus(TOKEN, serverError)).toEqual({
      kind: "error",
      message: "サーバーエラー（HTTP 502）",
    });

    const networkError = vi.fn().mockRejectedValue(new TypeError("Failed to fetch"));
    expect((await fetchStatus(TOKEN, networkError)).kind).toBe("error");
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
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(201, created));
    expect(await createGame(fetchMock)).toEqual(created);
    expect(fetchMock).toHaveBeenCalledWith("/api/games", expect.objectContaining({ method: "POST" }));
  });

  it("201 以外はエラーにする", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(500, {}));
    await expect(createGame(fetchMock)).rejects.toThrow("HTTP 500");
  });
});
