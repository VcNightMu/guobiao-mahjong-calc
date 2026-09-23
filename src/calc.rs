//! 听牌算番主流程
//!
//! 输入是**听牌型**：暗牌张数 = 13 - 3×副露面子数（开杠每个 +1，已含在公式里）。
//! 对 34 种候补逐张试和，先求完整听牌集（用于「独听」判断），再对每个和牌张算四种和法的番。

use crate::decompose::*;
use crate::fans::*;
use crate::model::*;
use crate::tiles::*;

/// 检测组合龙：9 张 147/258/369 + 1 面子 + 将
/// m 为副露数：m=0 时第四副面子在暗牌里；m=1 时第四副面子即那副副露
pub fn detect_zuhelong(c: &Counts, m: usize) -> Vec<Decomp> {
    let mut out = Vec::new();
    if m > 1 {
        return out;
    }
    if total(c) != (14 - 3 * m) as u32 {
        return out;
    }
    for perm in PERMS.iter() {
        let nine = knit_tiles(*perm);
        let mut cc = *c;
        let mut ok = true;
        for &t in nine.iter() {
            if cc[t] == 0 {
                ok = false;
                break;
            }
            cc[t] -= 1;
        }
        if !ok {
            continue;
        }
        if m == 1 {
            // 余下 2 张即将
            for p in 0..34usize {
                if cc[p] == 2 {
                    out.push(Decomp {
                        pair: Some(p),
                        sets: vec![],
                        special: Some(Special::Zuhelong),
                    });
                }
            }
            continue;
        }
        for p in 0..34usize {
            if cc[p] < 2 {
                continue;
            }
            let mut c2 = cc;
            c2[p] -= 2;
            for t in 0..27usize {
                if t % 9 <= 6 && c2[t] > 0 && c2[t + 1] > 0 && c2[t + 2] > 0 {
                    out.push(Decomp {
                        pair: Some(p),
                        sets: vec![Set::run(t)],
                        special: Some(Special::Zuhelong),
                    });
                }
            }
            for t in 0..34usize {
                if c2[t] >= 3 {
                    out.push(Decomp {
                        pair: Some(p),
                        sets: vec![Set::triplet(t)],
                        special: Some(Special::Zuhelong),
                    });
                }
            }
        }
    }
    out
}

#[derive(Clone, Debug)]
pub struct WaitResult {
    pub tile: usize,
    pub normal: u32,
    pub tsumo: u32,
    pub last: u32,
    pub tsumo_last: u32,
    /// 是否可能「和绝张」：暗牌里握着该张时绝张不成立（双碰听、单钓听、或听张已在暗牌中）
    pub can_last: bool,
    /// 点和时的完整番表（含和法番：门前清 / 和绝张 / 无番和 等）
    pub fans: Vec<Fan>,
    /// 自摸时的完整番表（与 fans 可能因暗刻/自摸类番而不同）
    pub fans_tsumo: Vec<Fan>,
    pub special: bool,
}

impl WaitResult {
    pub fn can_win(&self) -> bool {
        self.normal >= 8 || self.tsumo >= 8 || self.last >= 8 || self.tsumo_last >= 8
    }
    pub fn only_tsumo(&self) -> bool {
        self.normal < 8 && self.tsumo >= 8
    }
    pub fn only_last(&self) -> bool {
        self.can_last
            && self.normal < 8
            && self.tsumo < 8
            && (self.last >= 8 || self.tsumo_last >= 8)
    }
}

#[derive(Clone, Debug)]
pub struct Analysis {
    pub menqing: bool,
    pub meld_count: usize,
    /// 结构听牌（存在和牌型），未必够 8 番
    pub structural_waits: Vec<usize>,
    pub waits: Vec<WaitResult>,
}

/// 把暗牌 + 副露合成全部牌
pub fn build_all(c: &Counts, melds: &[Meld]) -> Counts {
    let mut a = *c;
    for m in melds {
        for t in m.tiles() {
            a[t] += 1;
        }
    }
    a
}

/// 给定（含和牌张的）暗牌与副露，返回所有和牌解释
pub fn winning_decomps(c: &Counts, melds: &[Meld]) -> Vec<Decomp> {
    let m = melds.len();
    let need = 4usize.saturating_sub(m);
    let mut out = Vec::new();

    if total(c) == (need * 3 + 2) as u32 {
        for mut d in decompose_standard(c, need) {
            let mut sets: Vec<Set> = melds.iter().map(|x| x.to_set()).collect();
            sets.extend(d.sets);
            d.sets = sets;
            out.push(d);
        }
    }

    if m == 0 {
        if is_seven_pairs_straight(c) {
            out.push(Decomp::special(Special::SevenPairsStraight));
        } else if is_seven_pairs(c) {
            out.push(Decomp::special(Special::SevenPairs));
        }
        if is_thirteen_orphans(c) {
            out.push(Decomp::special(Special::ThirteenOrphans));
        }
        if is_seven_stars(c) {
            out.push(Decomp::special(Special::SevenStars));
        } else if is_bu_kao(c) {
            out.push(Decomp::special(Special::BuKao));
        }
    }

    // 组合龙可带 1 副副露（m<=1）
    for d in detect_zuhelong(c, m) {
        out.push(d);
    }

    out
}

pub fn analyze(
    concealed: &Counts,
    melds: &[Meld],
    round_wind: usize,
    seat_wind: usize,
) -> Analysis {
    let menqing = !breaks_menqing(melds);

    // 第一遍：结构听牌集
    let mut structural = Vec::new();
    for t in 0..34usize {
        if concealed[t] >= 4 {
            continue;
        }
        let mut c = *concealed;
        c[t] += 1;
        if !winning_decomps(&c, melds).is_empty() {
            structural.push(t);
        }
    }
    let single_wait = structural.len() == 1;

    let mut waits = Vec::new();
    for &t in structural.iter() {
        let mut c = *concealed;
        c[t] += 1;
        let all = build_all(&c, melds);
        let decomps = winning_decomps(&c, melds);

        let mut best = [0u32; 4]; // normal, tsumo, last, tsumo_last
        let mut best_fans: Vec<Fan> = Vec::new();
        let mut best_fans_tsumo: Vec<Fan> = Vec::new();
        let mut best_normal = i64::MIN;
        let mut best_tsumo = i64::MIN;
        let mut special = false;

        for d in decomps.iter() {
            let ctx = WinCtx {
                all: &all,
                melds,
                decomp: d,
                round_wind,
                seat_wind,
                win_tile: t,
                menqing,
                single_wait,
                tsumo: false,
            };
            let (n, nf) = score(&ctx, WinMode::Normal);
            let (ts, tf) = score(&ctx, WinMode::Tsumo);
            let (lt, _) = score(&ctx, WinMode::LastTile);
            let (tl, _) = score(&ctx, WinMode::TsumoLast);
            best[0] = best[0].max(n);
            best[1] = best[1].max(ts);
            best[2] = best[2].max(lt);
            best[3] = best[3].max(tl);
            if (n as i64) > best_normal {
                best_normal = n as i64;
                best_fans = nf;
                special = d.special.is_some();
            }
            if (ts as i64) > best_tsumo {
                best_tsumo = ts as i64;
                best_fans_tsumo = tf;
            }
        }

        waits.push(WaitResult {
            tile: t,
            normal: best[0],
            tsumo: best[1],
            last: best[2],
            tsumo_last: best[3],
            can_last: concealed[t] == 0,
            fans: best_fans,
            fans_tsumo: best_fans_tsumo,
            special,
        });
    }

    Analysis {
        menqing,
        meld_count: melds.len(),
        structural_waits: structural,
        waits,
    }
}
