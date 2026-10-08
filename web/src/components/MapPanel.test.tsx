import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { sampleStatus } from "../test/fixtures";
import { buildTerritories } from "../territory";
import { effectiveMapWidth, MapPanel } from "./MapPanel";

const territories = buildTerritories(sampleStatus);

function renderPanel() {
  const onHighlightOwner = vi.fn();
  const utils = render(
    <MapPanel
      territories={territories}
      highlightedOwnerId={null}
      onHighlightOwner={onHighlightOwner}
    />,
  );
  return { ...utils, onHighlightOwner };
}

/** 要素の表示幅を指定した値に見せかけ、ResizeObserver が即座に通知するようにする */
function mockResizeObserver(width: number) {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
    width,
    height: width,
  } as DOMRect);
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(private readonly callback: ResizeObserverCallback) {}
      observe() {
        this.callback(
          [{ contentRect: { width, height: width } } as ResizeObserverEntry],
          this as unknown as ResizeObserver,
        );
      }
      disconnect() {}
    },
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("MapPanel", () => {
  it("狭い画面では既定で接続図を表示し、日本地図にも切り替えられる", () => {
    mockResizeObserver(360);
    const { container } = renderPanel();
    expect(screen.getByRole("button", { name: "接続" })).toHaveAttribute("aria-pressed", "true");
    expect(container.querySelector(".schematic")).not.toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "地図" }));
    expect(container.querySelector(".schematic")).toBeNull();
    expect(container.querySelectorAll("path.kuni")).toHaveLength(12);
  });

  it("広い画面では既定で日本地図を表示する", () => {
    window.innerHeight = 1000;
    mockResizeObserver(800);
    renderPanel();
    expect(screen.getByRole("button", { name: "地図" })).toHaveAttribute("aria-pressed", "true");
  });

  it("画面の高さが低いと地図は高さで縮むため、実際の表示幅で判定する", () => {
    // 幅 600px でも高さ 600px の画面では、縦長の地図は約 410px 幅でしか表示できない
    expect(effectiveMapWidth(600, 1000)).toBe(600);
    expect(effectiveMapWidth(600, 600)).toBeLessThan(460);
  });
});

describe("接続図", () => {
  it("国をタップすると隣接国と接続線を強調し、詳細を表示する", () => {
    mockResizeObserver(360);
    const { container, onHighlightOwner } = renderPanel();

    // 12国・17本の接続線（うち海路3本は破線）
    expect(container.querySelectorAll("g.box")).toHaveLength(12);
    expect(container.querySelectorAll("line.edge")).toHaveLength(17);
    expect(container.querySelectorAll("line.edge.sea")).toHaveLength(3);

    // 国の箱は支配大名の色で塗り、自領は太い輪郭のクラスを付ける
    const fillOf = (kuniId: number) =>
      (container.querySelector(`g.box[data-kuni-id="${kuniId}"] rect`) as SVGRectElement).style.fill;
    expect(fillOf(3)).toBe(fillOf(4));
    expect(fillOf(3)).not.toBe(fillOf(7));
    expect(container.querySelectorAll("g.box.mine")).toHaveLength(2);

    // 未選択の間は説明文を出さない
    expect(screen.queryByRole("status")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "甲信：上杉" }));
    expect(onHighlightOwner).toHaveBeenLastCalledWith(3);
    expect(screen.getByRole("status")).toHaveTextContent("甲信上杉");
    const active = [...container.querySelectorAll("line.edge.active")].map((l) =>
      l.getAttribute("data-edge"),
    );
    expect(active.sort()).toEqual(["3-4", "4-5", "4-6", "4-7", "4-8"]);

    // もう一度タップすると選択を解除する
    fireEvent.click(screen.getByRole("button", { name: "甲信：上杉" }));
    expect(onHighlightOwner).toHaveBeenLastCalledWith(null);
    expect(container.querySelectorAll("line.edge.active")).toHaveLength(0);
  });
});
