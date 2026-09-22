//! 输入数据模型：副露、门清判定

use crate::decompose::{Set, SetKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeldKind {
    Chi,
    Pon,
    Kan,
}

#[derive(Clone, Debug)]
pub struct Meld {
    pub kind: MeldKind,
    /// Chi: 最小牌; Pon/Kan: 该牌
    pub tile: usize,
    /// 仅 Kan 有意义：明杠 true / 暗杠 false
    pub open: bool,
}

impl Meld {
    pub fn chi(tile: usize) -> Self {
        Meld { kind: MeldKind::Chi, tile, open: true }
    }
    pub fn pon(tile: usize) -> Self {
        Meld { kind: MeldKind::Pon, tile, open: true }
    }
    pub fn kan(tile: usize, open: bool) -> Self {
        Meld { kind: MeldKind::Kan, tile, open }
    }

    /// 转成面子的统一表示
    pub fn to_set(&self) -> Set {
        match self.kind {
            MeldKind::Chi => Set { kind: SetKind::Run, tile: self.tile, is_kan: false, open: true },
            MeldKind::Pon => {
                Set { kind: SetKind::Triplet, tile: self.tile, is_kan: false, open: true }
            }
            MeldKind::Kan => {
                Set { kind: SetKind::Triplet, tile: self.tile, is_kan: true, open: self.open }
            }
        }
    }

    /// 占用的牌
    pub fn tiles(&self) -> Vec<usize> {
        match self.kind {
            MeldKind::Chi => vec![self.tile, self.tile + 1, self.tile + 2],
            MeldKind::Pon => vec![self.tile; 3],
            MeldKind::Kan => vec![self.tile; 4],
        }
    }
}

/// 副露是否破坏门清：吃、碰、明杠 → 破坏；暗杠不破坏
pub fn breaks_menqing(melds: &[Meld]) -> bool {
    melds.iter().any(|m| match m.kind {
        MeldKind::Chi | MeldKind::Pon => true,
        MeldKind::Kan => m.open,
    })
}
