import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { sampleStatus } from "../test/fixtures";
import { buildTerritories } from "../territory";
import { SchematicMap } from "./SchematicMap";

const territories = buildTerritories(sampleStatus);

function renderMap() {
  const onHighlightOwner = vi.fn();
  const utils = render(
    <SchematicMap
      territories={territories}
      highlightedOwnerId={null}
      onHighlightOwner={onHighlightOwner}
    />,
  );
  return { ...utils, onHighlightOwner };
}

describe("SchematicMap", () => {
  it("タップで選択を切り替え、もう一度タップすると解除する", () => {
    const { container, onHighlightOwner } = renderMap();
    // 未選択の間は説明文を出さない
    expect(screen.queryByRole("status")).toBeNull();

    const koshin = screen.getByRole("button", { name: "甲信：上杉" });
    fireEvent.click(koshin);
    expect(onHighlightOwner).toHaveBeenLastCalledWith(3);
    expect(container.querySelectorAll("line.edge.active")).toHaveLength(5);

    fireEvent.click(koshin);
    expect(onHighlightOwner).toHaveBeenLastCalledWith(null);
    expect(container.querySelectorAll("line.edge.active")).toHaveLength(0);
  });

  it("マウスはホバーで選択し、離れると解除する（クリックで解除しない）", () => {
    const { container, onHighlightOwner } = renderMap();
    const owari = screen.getByRole("button", { name: "尾張：織田" });

    fireEvent.pointerEnter(owari, { pointerType: "mouse" });
    expect(onHighlightOwner).toHaveBeenLastCalledWith(7);
    fireEvent.pointerDown(owari, { pointerType: "mouse" });
    fireEvent.click(owari);
    expect(container.querySelectorAll("line.edge.active")).toHaveLength(3);

    fireEvent.pointerLeave(owari, { pointerType: "mouse" });
    expect(onHighlightOwner).toHaveBeenLastCalledWith(null);
  });

  it("キーボードでも選択できる", () => {
    const { onHighlightOwner } = renderMap();
    fireEvent.keyDown(screen.getByRole("button", { name: "武蔵：北条" }), { key: "Enter" });
    expect(onHighlightOwner).toHaveBeenLastCalledWith(5);
  });
});
