use crate::domain::model::value_objects::SessionId;
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 参加コードに使う文字（読み間違えやすい I / L / O / 0 / 1 を除外）
const JOIN_CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
/// 参加コードの文字数
const JOIN_CODE_LEN: usize = 6;

/// Webで作成したゲームにチャット（LLM）側から参加するための短い参加コード
///
/// プレイヤーが手入力・口頭で伝えやすいよう、短く紛らわしい文字を含まない形式にします。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct JoinCode(String);

impl JoinCode {
    /// 新しい参加コードをランダムに生成します
    pub fn generate() -> Self {
        let mut rng = rand::rngs::OsRng;
        let code = (0..JOIN_CODE_LEN)
            .map(|_| JOIN_CODE_ALPHABET[rng.gen_range(0..JOIN_CODE_ALPHABET.len())] as char)
            .collect();
        Self(code)
    }

    /// 入力された文字列を参加コードとして解釈します
    ///
    /// LLM やプレイヤーが入力する値のため、前後の空白・区切り文字（空白/ハイフン）を除去し、
    /// 小文字は大文字に正規化します。形式外の場合は `None` を返します。
    pub fn parse(input: &str) -> Option<Self> {
        let normalized: String = input
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .map(|c| c.to_ascii_uppercase())
            .collect();
        let valid = normalized.len() == JOIN_CODE_LEN
            && normalized.bytes().all(|b| JOIN_CODE_ALPHABET.contains(&b));
        valid.then_some(Self(normalized))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for JoinCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for JoinCode {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s).ok_or_else(|| format!("不正な参加コードです: {}", s))
    }
}

impl From<JoinCode> for String {
    fn from(code: JoinCode) -> Self {
        code.0
    }
}

/// 参加コードと、参加待ちのゲーム（セッション）の対応を表すチケット
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinTicket {
    /// 参加コード
    pub code: JoinCode,
    /// 参加待ちのゲームのセッションID
    pub session_id: SessionId,
    /// 有効期限 (UTC)
    pub expires_at: DateTime<Utc>,
}

impl JoinTicket {
    /// 現在時刻から `ttl` だけ有効なチケットを生成します
    pub fn new(code: JoinCode, session_id: SessionId, ttl: Duration) -> Self {
        Self {
            code,
            session_id,
            expires_at: Utc::now() + ttl,
        }
    }

    /// 指定時刻において有効期限切れかどうかを判定します
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_is_parsable() {
        for _ in 0..100 {
            let code = JoinCode::generate();
            assert_eq!(JoinCode::parse(code.value()), Some(code));
        }
    }

    #[test]
    fn test_parse_normalizes_input() {
        let expected = JoinCode::parse("KX7P2Q").unwrap();
        assert_eq!(JoinCode::parse(" kx7-p2q "), Some(expected.clone()));
        assert_eq!(JoinCode::parse("KX7 P2Q"), Some(expected));
    }

    #[test]
    fn test_parse_rejects_invalid() {
        assert!(JoinCode::parse("").is_none());
        assert!(JoinCode::parse("KX7P2").is_none());
        assert!(JoinCode::parse("KX7P2QQ").is_none());
        // 紛らわしい文字（O / 0 / I / 1）や記号は使わない
        assert!(JoinCode::parse("OX7P2Q").is_none());
        assert!(JoinCode::parse("1X7P2Q").is_none());
        assert!(JoinCode::parse("../../").is_none());
    }

    #[test]
    fn test_ticket_expiry() {
        let ticket = JoinTicket::new(
            JoinCode::generate(),
            SessionId::new("web_x"),
            Duration::minutes(30),
        );
        assert!(!ticket.is_expired(Utc::now()));
        assert!(ticket.is_expired(Utc::now() + Duration::minutes(31)));
    }
}
