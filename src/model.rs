//! 输入数据模型：副露、门清判定

use crate::decompose::{Set, SetKind};
use crate::tiles::{tile_name, total, Counts, NUM_TILES};

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

/// 输入校验错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputError {
    /// 暗牌张数不符：应为 expected（= 13 - 3×副露面子数，面子可为杠），实际 got
    Count { expected: u32, got: u32 },
    /// 某张牌全局（暗牌 + 副露 + 杠）超过 4 张
    Over4(usize, u8),
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputError::Count { expected, got } => write!(
                f,
                "暗牌张数不符：应为 {} 张（13 - 3×副露面子数），实际 {} 张",
                expected, got
            ),
            InputError::Over4(t, n) => write!(f, "{} 共 {} 张，超过每种牌 4 张上限", tile_name(*t), n),
        }
    }
}

/// C7：全局计数（暗牌 + 副露；杠按 4 张计，由 Meld::tiles() 展开）先查超 4；
/// C6：再查暗牌张数 = 13 - 3×副露面子数（面子可以是杠，四杠合法）。
pub fn validate_counts(c: &Counts, melds: &[Meld]) -> Result<(), InputError> {
    let mut all = *c;
    for m in melds {
        for t in m.tiles() {
            all[t] = all[t].saturating_add(1);
        }
    }
    for t in 0..NUM_TILES {
        if all[t] > 4 {
            return Err(InputError::Over4(t, all[t]));
        }
    }
    let expected = 13u32.saturating_sub(3 * melds.len() as u32);
    let got = total(c);
    if got != expected {
        return Err(InputError::Count { expected, got });
    }
    Ok(())
}
