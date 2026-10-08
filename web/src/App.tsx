import { useState } from "react";
import { createGame } from "./api/client";
import { JoinPanel } from "./components/JoinPanel";
import { StatusView } from "./components/StatusView";
import { loadSession, saveSession, type StoredSession } from "./session";
import { useGameStatus } from "./useGameStatus";

/** 新しいゲームを作成するボタン（作成中は二重送信を防ぐ） */
function NewGameButton({
  onCreated,
  label = "新しいゲームを始める",
}: {
  onCreated: (session: StoredSession) => void;
  label?: string;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const start = async () => {
    setBusy(true);
    setError(null);
    try {
      const created = await createGame();
      onCreated({
        token: created.view_token,
        joinCode: created.join_code,
        joinCodeExpiresAt: created.join_code_expires_at,
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : "ゲームを作成できませんでした");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="new-game">
      <button type="button" onClick={start} disabled={busy}>
        {busy ? "作成中…" : label}
      </button>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

export function App() {
  const [session, setSession] = useState<StoredSession | null>(() => loadSession());
  const { result } = useGameStatus(session?.token ?? null);

  const changeSession = (next: StoredSession | null) => {
    saveSession(next);
    setSession(next);
  };

  // ゲーム未作成：開始画面
  if (!session) {
    return (
      <main className="centered">
        <section className="card notice">
          <h1>戦国 勢力図</h1>
          <NewGameButton onCreated={changeSession} />
        </section>
      </main>
    );
  }

  // 初回の取得中
  if (!result) {
    return (
      <main className="centered">
        <p className="muted" role="status">
          読み込み中…
        </p>
      </main>
    );
  }

  switch (result.kind) {
    case "ok":
      return <StatusView status={result.status} />;
    case "waiting_for_join":
      return (
        <main className="centered">
          <JoinPanel joinCode={session.joinCode} expiresAt={session.joinCodeExpiresAt} />
        </main>
      );
    case "daimyo_not_selected":
      return (
        <main className="centered">
          <section className="card notice">
            <h1>大名を選んでください</h1>
          </section>
        </main>
      );
    case "not_found":
      return (
        <main className="centered">
          <section className="card notice">
            <h1>このゲームは終了しています</h1>
            <NewGameButton onCreated={changeSession} />
          </section>
        </main>
      );
    case "error":
      return (
        <main className="centered">
          <section className="card notice">
            <h1>接続できません</h1>
            <p className="error" role="alert">
              {result.message}
            </p>
          </section>
        </main>
      );
  }
}
