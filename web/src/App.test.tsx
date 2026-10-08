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
    expect(screen.getByRole("button", { name: "チャット用の文をコピー" })).toBeInTheDocument();
    expect(screen.getByRole("timer")).toHaveTextContent(/^\d+:\d{2}$/);
    // 再読み込みしても続きから表示できるよう、URL にトークンを残す
    expect(window.location.search).toBe(`?token=${TOKEN}`);
  });

  it("年と季節・自領の状況・大名ごとに色分けした勢力図を表示する", async () => {
    window.history.replaceState(null, "", `/?token=${TOKEN}`);
    mockFetch({ [`GET /api/views/${TOKEN}/status`]: () => jsonResponse(200, sampleStatus) });
    const { container } = render(<App />);

    // 第3ターン＝1560年秋
    expect(await screen.findByRole("heading", { name: "1560年秋" })).toBeInTheDocument();
    // 自領は国ごとの小さなカードで表示する
    const cards = within(screen.getByRole("list", { name: "自領" }));
    expect(cards.getAllByRole("listitem").map((li) => li.querySelector("strong")?.textContent)).toEqual([
      "三河",
      "尾張",
    ]);

    // 勢力図（接続図）: 12国を支配大名の色で塗る（上杉の越州・甲信は同じ色、織田の三河・尾張は織田の色）
    const fillOf = (kuniId: number) =>
      (container.querySelector(`g.box[data-kuni-id="${kuniId}"] rect`) as SVGRectElement).style
        .fill;
    expect(container.querySelectorAll("g.box")).toHaveLength(12);
    expect(fillOf(3)).toBe(fillOf(4));
    expect(fillOf(6)).toBe(fillOf(7));
    expect(fillOf(7)).not.toBe(fillOf(4));
    // 自領は太い輪郭のクラスを付ける
    expect(container.querySelectorAll("g.box.mine")).toHaveLength(2);
    // 日本地図は表示しない
    expect(container.querySelector("path.kuni")).toBeNull();

    // 凡例: 自国（織田）を先頭に、領地数の多い順
    const legend = within(screen.getByRole("list", { name: "大名" }));
    const items = legend.getAllByRole("listitem").map((li) => li.textContent);
    expect(items.slice(0, 2)).toEqual(["織田2", "上杉2"]);
    expect(items).toHaveLength(10);

    // 自領カードは各国の全項目（兵・金・米・人口・石高・町・忠誠）を表示する
    const owari = cards.getAllByRole("listitem")[1]!;
    for (const value of ["80", "200", "120", "350", "60", "100"]) {
      expect(within(owari).getAllByText(value).length).toBeGreaterThan(0);
    }

    // 国を選ぶと、支配大名を表示し、同じ大名の領地と隣接国・接続線を強調する
    fireEvent.click(screen.getByRole("button", { name: "甲信：上杉" }));
    expect(screen.getByRole("status")).toHaveTextContent("甲信上杉");
    const highlighted = [...container.querySelectorAll("g.box.highlighted")].map((g) =>
      g.getAttribute("data-kuni-id"),
    );
    expect(highlighted.sort()).toEqual(["3", "4"]);
    const activeEdges = [...container.querySelectorAll("line.edge.active")].map((l) =>
      l.getAttribute("data-edge"),
    );
    expect(activeEdges.sort()).toEqual(["3-4", "4-5", "4-6", "4-7", "4-8"]);
    const neighbors = [...container.querySelectorAll("g.box.neighbor")].map((g) =>
      g.getAttribute("data-kuni-id"),
    );
    expect(neighbors.sort((x, y) => Number(x) - Number(y))).toEqual(["3", "5", "6", "7", "8"]);

    // 接続線は隣接情報（17組、うち海路3本は破線）をすべて描画する
    expect(container.querySelectorAll("line.edge")).toHaveLength(17);
    expect(container.querySelectorAll("line.edge.sea")).toHaveLength(3);
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
    expect(await screen.findByText("このゲームは終了しています")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "新しいゲームを始める" })).toBeInTheDocument();
  });
});
