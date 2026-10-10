import { useEffect, useState } from "react";

interface Props {
  joinCode?: string;
  /** 参加コードの有効期限（ISO 8601） */
  expiresAt?: string;
}

/** 残り時間を「m:ss」で表す（期限切れなら null） */
export function formatRemaining(expiresAt: string | undefined, now: number): string | null {
  if (!expiresAt) return null;
  const ms = new Date(expiresAt).getTime() - now;
  if (Number.isNaN(ms) || ms <= 0) return null;
  const totalSeconds = Math.floor(ms / 1000);
  return `${Math.floor(totalSeconds / 60)}:${String(totalSeconds % 60).padStart(2, "0")}`;
}

/** チャット側の参加待ちの間、参加コードを表示する（チャットに貼る文はコピーできる） */
export function JoinPanel({ joinCode, expiresAt }: Props) {
  const [now, setNow] = useState(() => Date.now());
  const [copied, setCopied] = useState(false);

  // 残り時間の表示を1秒ごとに更新する
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  if (!joinCode) {
    return (
      <section className="card notice">
        <h1>参加待ち</h1>
      </section>
    );
  }

  const left = formatRemaining(expiresAt, now);
  const message = `参加コード ${joinCode} でゲームに参加して`;
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(message);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  };

  return (
    <section className="card notice join" aria-label="参加コード">
      <p className="join-code" aria-label={`参加コード ${joinCode.split("").join(" ")}`}>
        {joinCode}
      </p>
      <button type="button" onClick={copy}>
        {copied ? "コピーしました" : "チャット用の文をコピー"}
      </button>
      <p className={left ? "muted" : "error"} role="timer">
        {left ?? "期限切れ"}
      </p>
    </section>
  );
}
