import { afterEach, describe, expect, it } from "vitest";
import { isValidToken, loadSession, saveSession } from "./session";

const TOKEN_A = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const TOKEN_B = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

afterEach(() => {
  window.localStorage.clear();
  window.history.replaceState(null, "", "/");
});

describe("isValidToken", () => {
  it("32桁の小文字16進数のみ受け付ける", () => {
    expect(isValidToken(TOKEN_A)).toBe(true);
    expect(isValidToken("../../etc")).toBe(false);
    expect(isValidToken(TOKEN_A.toUpperCase())).toBe(false);
    expect(isValidToken(null)).toBe(false);
  });
});

describe("loadSession / saveSession", () => {
  it("保存したゲームを読み込み、URL にトークンを反映する", () => {
    saveSession({ token: TOKEN_A, joinCode: "VQ4X7K" });
    expect(new URL(window.location.href).searchParams.get("token")).toBe(TOKEN_A);

    window.history.replaceState(null, "", "/");
    expect(loadSession()).toEqual({ token: TOKEN_A, joinCode: "VQ4X7K" });
    expect(new URL(window.location.href).searchParams.get("token")).toBe(TOKEN_A);
  });

  it("URL のトークンを保存済みより優先する（別ゲームなら参加コードは引き継がない）", () => {
    saveSession({ token: TOKEN_A, joinCode: "VQ4X7K" });
    window.history.replaceState(null, "", `/?token=${TOKEN_B}`);
    expect(loadSession()).toEqual({ token: TOKEN_B });
  });

  it("不正なトークンは無視する", () => {
    window.history.replaceState(null, "", "/?token=invalid");
    expect(loadSession()).toBeNull();
  });

  it("null を保存すると破棄する", () => {
    saveSession({ token: TOKEN_A });
    saveSession(null);
    expect(loadSession()).toBeNull();
    expect(window.location.search).toBe("");
  });
});
