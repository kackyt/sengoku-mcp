import { useEffect, useState } from "react";
import { fetchStatus, type StatusResult } from "./api/client";

/** 状況のポーリング間隔（ミリ秒）。LLM 側の操作がすぐ反映されるよう短めにする */
export const POLL_INTERVAL_MS = 5000;

/** ポーリングの状態 */
export interface GameStatusState {
  /** 最新の取得結果（初回取得前は null） */
  result: StatusResult | null;
  /** 最後に取得できた時刻 */
  updatedAt: Date | null;
}

/**
 * 閲覧トークンに対応する状況を定期的に取得するフック
 *
 * - トークンが変わったら状態をリセットして取り直す
 * - タブが非表示の間は取得を止め、表示されたら即座に取り直す
 * - URL が無効（not_found）になったら以降のポーリングを止める
 */
export function useGameStatus(token: string | null): GameStatusState {
  const [state, setState] = useState<GameStatusState>({ result: null, updatedAt: null });

  useEffect(() => {
    setState({ result: null, updatedAt: null });
    if (!token) return;

    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const poll = async () => {
      clearTimeout(timer);
      const result = await fetchStatus(token);
      if (cancelled) return;
      setState({ result, updatedAt: new Date() });
      // 無効なURLになったら、それ以上問い合わせない
      if (result.kind !== "not_found" && document.visibilityState === "visible") {
        timer = setTimeout(poll, POLL_INTERVAL_MS);
      }
    };

    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") void poll();
      else clearTimeout(timer);
    };

    void poll();
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => {
      cancelled = true;
      clearTimeout(timer);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [token]);

  return state;
}
