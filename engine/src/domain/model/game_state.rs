use crate::domain::error::DomainError;
use crate::domain::model::value_objects::{ActionOrderIndex, DaimyoId, KuniId, TurnNumber};

/// ゲームの進行フェーズ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GamePhase {
    /// 内政フェーズ
    #[default]
    Domestic,
    /// 合戦フェーズ
    Battle,
    /// ゲームオーバー
    GameOver,
    /// 天下一統（クリア）
    GameClear,
}

/// ゲームの進行状態全体を表すドメインモデル
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameState {
    /// 現在のターン（季節）
    current_turn: TurnNumber,
    /// このターンで行動する国の順番
    action_order: Vec<KuniId>,
    /// 現在行動中の国のインデックス（`action_order`のインデックス）
    current_action_index: ActionOrderIndex,
    /// 現在のフェーズにおいて、既に行動（内政や合戦）が実行されたか
    action_performed: bool,
    /// 現在のフェーズ
    phase: GamePhase,
    /// 勝者（領土をすべて失った場合などの判定用）
    winner_id: Option<DaimyoId>,
}

impl GameState {
    /// 新規ゲーム状態を生成します。
    pub fn new(
        current_turn: TurnNumber,
        action_order: Vec<KuniId>,
        current_action_index: ActionOrderIndex,
    ) -> Result<Self, DomainError> {
        if current_action_index.value() > action_order.len() {
            return Err(DomainError::ValidationError(
                "current_action_index must be <= action_order.len()".to_string(),
            ));
        }
        Ok(Self {
            current_turn,
            action_order,
            current_action_index,
            action_performed: false,
            phase: GamePhase::Domestic,
            winner_id: None,
        })
    }

    /// 明示的に状態を指定して生成（テストや永続化からの復元用）
    pub fn with_all_fields(
        current_turn: TurnNumber,
        action_order: Vec<KuniId>,
        current_action_index: ActionOrderIndex,
        action_performed: bool,
        phase: GamePhase,
        winner_id: Option<DaimyoId>,
    ) -> Self {
        Self {
            current_turn,
            action_order,
            current_action_index,
            action_performed,
            phase,
            winner_id,
        }
    }

    pub fn current_turn(&self) -> TurnNumber {
        self.current_turn
    }

    pub fn action_order(&self) -> &[KuniId] {
        &self.action_order
    }

    pub fn current_action_index(&self) -> ActionOrderIndex {
        self.current_action_index
    }

    pub fn phase(&self) -> GamePhase {
        self.phase
    }

    pub fn winner(&self) -> Option<DaimyoId> {
        self.winner_id
    }

    pub fn set_phase(&mut self, phase: GamePhase) {
        self.phase = phase;
    }

    pub fn set_winner(&mut self, daimyo_id: DaimyoId) {
        self.winner_id = Some(daimyo_id);
        self.phase = GamePhase::GameOver;
    }

    /// 現在行動中の国IDを取得します。
    /// 順番が終了している場合は `None` を返します。
    pub fn current_kuni_id(&self) -> Option<KuniId> {
        self.action_order
            .get(self.current_action_index.value())
            .copied()
    }

    /// 次の行動国に進みます。
    /// 行動済みフラグをリセットし、フェーズを内政に戻します。
    pub fn advance_action(&mut self) {
        if self.current_action_index.value() < self.action_order.len() {
            self.current_action_index =
                ActionOrderIndex::new(self.current_action_index.value() + 1);
            self.action_performed = false;
            // ゲーム終了・クリア・合戦時はフェーズを戻さない
            if !matches!(
                self.phase,
                GamePhase::GameOver | GamePhase::GameClear | GamePhase::Battle
            ) {
                self.phase = GamePhase::Domestic;
            }
        }
    }

    /// 現在の行動が完了したことをマークします。
    pub fn mark_action_performed(&mut self) {
        self.action_performed = true;
    }

    /// 現在の行動が既に実行済みかを確認します。
    pub fn is_action_performed(&self) -> bool {
        self.action_performed
    }

    /// ターン内のすべての大名が行動を完了したかを判定します。
    pub fn is_turn_completed(&self) -> bool {
        self.current_action_index.value() >= self.action_order.len()
    }

    /// 新しいターンを開始し、行動順序をリセットします。
    pub fn start_new_turn(&mut self, new_order: Vec<KuniId>) {
        self.current_turn = TurnNumber::new(self.current_turn.value() + 1);
        self.action_order = new_order;
        self.current_action_index = ActionOrderIndex::new(0);
        self.action_performed = false;
        self.phase = GamePhase::Domestic;
    }

    /// 合戦を開始し、合戦フェーズに移行します。
    pub fn start_war(
        &mut self,
        _attacker_id: KuniId,
        _defender_id: KuniId,
    ) -> Result<(), DomainError> {
        self.phase = GamePhase::Battle;
        Ok(())
    }

    /// 指定された国IDが現在の手番であるかを確認します。
    pub fn check_turn(&self, kuni_id: KuniId) -> Result<(), DomainError> {
        match self.current_kuni_id() {
            Some(current) if current == kuni_id => Ok(()),
            Some(current) => Err(DomainError::NotYourTurn(current)),
            None => Err(DomainError::ValidationError(
                "Turn is already finished".to_string(),
            )),
        }
    }
}
