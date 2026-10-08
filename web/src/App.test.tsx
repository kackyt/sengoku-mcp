import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { jsonResponse, sampleStatus } from "./test/fixtures";

const TOKEN = "0123456789abcdef0123456789abcdef";

/** URL・メソッドごとにレスポンスを返す fetch モックを登録する */
function mockFetch(routes: Record<string, () => Response>) {
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const key = `${init?.method ?? "GET"} ${String(input)}`;
    const route = routes[key];
    if (!route) throw new Error(`想定外のリクエスト: ${key}`);
    return route();
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

afterEach(() => {
  vi.unstubAllGlobals();
  window.localStorage.clear();
  window.history.replaceState(null, "", "/");
});

describe("App", () => {
  it("新しいゲームを作成すると参加コードを表示する", async () => {
    mockFetch({
      "POST /api/games": () =>
        jsonResponse(201, {
          view_token: TOKEN,
          status_url: `/api/views/${TOKEN}/status`,
          join_code: "VQ4X7K",
          join_code_expires_at: new Date(Date.now() + 30 * 60_000).toISOString(),
          join_message: "",
        }),
      [`GET /api/views/${TOKEN}/status`]: () =>
        jsonResponse(409, { code: "waiting_for_join", message: "" }),
    });
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "新しいゲームを始める" }));

    expect(await screen.findByText("VQ4X7K")).toBeInTheDocument();
    expect(screen.getByText("参加コード VQ4X7K でゲームに参加して")).toBeInTheDocument();
    expect(screen.getByText(/残り \d+分\d{2}秒/)).toBeInTheDocument();
    // 再読み込みしても続きから表示できるよう、URL にトークンを残す
    expect(window.location.search).toBe(`?token=${TOKEN}`);
  });

  it("ターン数・自領の状況・勢力図を表示する", async () => {
    window.history.replaceState(null, "", `/?token=${TOKEN}`);
    mockFetch({ [`GET /api/views/${TOKEN}/status`]: () => jsonResponse(200, sampleStatus) });
    const { container } = render(<App />);

    expect(await screen.findByRole("heading", { name: "第3ターン" })).toBeInTheDocument();
    expect(screen.getByText("織田家")).toBeInTheDocument();

    // 地図: 12国すべてを描画し、自領（三河・尾張）だけアクセント色のクラスを付ける
    const paths = container.querySelectorAll("path.kuni");
    expect(paths).toHaveLength(12);
    const mine = [...container.querySelectorAll("path.kuni.mine")].map((p) =>
      p.getAttribute("data-kuni-id"),
    );
    expect(mine.sort()).toEqual(["6", "7"]);

    // 他国の支配大名の一覧（上杉は2国）
    const row = screen.getByRole("row", { name: /甲信 上杉/ });
    expect(within(row).getByText("（2国）")).toBeInTheDocument();

    // 地図上で他国にホバーすると、支配大名と領地数を表示し、同じ大名の領地を強調する
    const koshin = container.querySelector('path[data-kuni-id="4"]')!;
    fireEvent.pointerMove(koshin, { clientX: 10, clientY: 10 });
    expect(screen.getByRole("status")).toHaveTextContent("上杉家の領地数：2国");
    const highlighted = [...container.querySelectorAll("path.kuni.highlighted")].map((p) =>
      p.getAttribute("data-kuni-id"),
    );
    expect(highlighted.sort()).toEqual(["3", "4"]);
  });

  it("大名未選択・無効なURLをそれぞれ案内する", async () => {
    window.history.replaceState(null, "", `/?token=${TOKEN}`);
    mockFetch({
      [`GET /api/views/${TOKEN}/status`]: () =>
        jsonResponse(409, { code: "daimyo_not_selected", message: "" }),
    });
    const { unmount } = render(<App />);
    expect(await screen.findByText("大名を選んでください")).toBeInTheDocument();
    unmount();

    mockFetch({
      [`GET /api/views/${TOKEN}/status`]: () =>
        jsonResponse(404, { code: "view_not_found", message: "" }),
    });
    render(<App />);
    expect(await screen.findByText("このURLは無効です")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "新しいゲームを始める" })).toBeInTheDocument();
  });
});
