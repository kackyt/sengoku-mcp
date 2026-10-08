import { useEffect, useState } from "react";

interface Props {
  joinCode?: string;
  /** 参加コードの有効期限（ISO 8601） */
  expiresAt?: string;
}

/** 残り時間を「m分s秒」で表す（期限切れなら null） */
function remaining(expiresAt: string | undefined, now: number): string | null {
  if (!expiresAt) return null;
  const ms = new Date(expiresAt).getTime() - now;
  if (Number.isNaN(ms) || ms <= 0) return null;
  const totalSeconds = Math.floor(ms / 1000);
  return `${Math.floor(totalSeconds / 60)}分${String(totalSeconds % 60).padStart(2, "0")}秒`;
}

/** チャット側の参加待ちの間、参加コードと手順を表示する */
export function JoinPanel({ joinCode, expiresAt }: Props) {
  const [now, setNow] = useState(() => Date.now());
  const [copied, setCopied] = useState(false);

  // 残り時間の表示を1秒ごとに更新する
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  const left = remaining(expiresAt, now);
  const message = joinCode ? `参加コード ${joinCode} でゲームに参加して` : "";

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(message);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  };

  if (!joinCode) {
    return (
      <section className="card notice">
        <h1>チャット側の参加待ちです</h1>
        <p>このゲームの参加コードは、作成したブラウザでのみ表示できます。</p>
      </section>
    );
  }

  return (
    <section className="card notice" aria-labelledby="join-title">
      <h1 id="join-title">チャットで参加コードを伝えてください</h1>
      <p className="join-code" aria-label={`参加コード ${joinCode.split("").join(" ")}`}>
        {joinCode}
      </p>
      <p>LLM とのチャットで、次のように伝えてください。</p>
      <div className="join-message">
        <code>{message}</code>
        <button type="button" className="secondary" onClick={copy}>
          {copied ? "コピーしました" : "コピー"}
        </button>
      </div>
      <p className="muted">
        {left
          ? `参加コードの有効期限：残り ${left}（1回限り有効）`
          : "参加コードの有効期限が切れました。新しいゲームを作成してください。"}
      </p>
      <p className="muted">参加すると、この画面は自動で切り替わります。</p>
    </section>
  );
}
