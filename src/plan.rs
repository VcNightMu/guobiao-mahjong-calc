//! 未听牌方向扫描：不做摸打搜索，只算「这手牌离某个番种还差几张」。
//!
//! 距离定义：把手牌换成该番种的「和牌型」最少需要替换几张牌
//!     distance = 手牌张数 − 目标牌型能覆盖的手牌张数
//! 覆盖 = Σ_t min(手牌[t], 目标[t])。
//! 于是「要补的牌」= 目标比手牌多的部分，「要换掉的牌」= 手牌比目标多的部分，
//! 两者张数恰好差 1（那 1 张就是最终的和牌张）。
//!
//! 口径提醒：距离是「离和牌型」而不是「离花色/骨架」，否则会给出假的近。

use crate::decompose::{knit_tiles, PERMS, YAOJIU};
use crate::model::{Meld, MeldKind};
use crate::tiles::*;

pub const SUIT_NAME: [&str; 3] = ["万", "条", "筒"];

#[derive(Clone, Debug)]
pub struct Direction {
    pub name: String,
    pub value: u32,
    pub distance: u32,
    pub need: Vec<usize>,
    pub drop: Vec<usize>,
    pub tag: &'static str,
}

impl Direction {
    pub fn need_str(&self) -> String {
        tile_list(&self.need)
    }
    pub fn drop_str(&self) -> String {
        tile_list(&self.drop)
    }
}

fn tile_list(v: &[usize]) -> String {
    if v.is_empty() {
        return "—".to_string();
    }
    v.iter().map(|&t| tile_name(t)).collect::<Vec<_>>().join(" ")
}

/// 牌种限制
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    Any,
    /// 单门花色（清一色）
    Suit(usize),
    /// 单门花色 + 字牌（混一色）
    Mixed(usize),
    Honors,
    /// 幺九牌（含字牌）——混幺九
    TermHonor,
    /// 序数牌的幺九（不含字牌）——清幺九
    TermNum,
    Big,
    Mid,
    Small,
    Above5,
    Below5,
    /// 推不倒：筒1234589 / 条245689 / 白
    Tuibu,
    /// 双数序数牌 2468
    Even,
    /// 绿一色：条23468 / 发
    Green,
}

impl Filter {
    pub fn ok(self, t: usize) -> bool {
        match self {
            Filter::Any => true,
            Filter::Suit(s) => suit(t) == s,
            Filter::Mixed(s) => suit(t) == s || suit(t) == 3,
            Filter::Honors => suit(t) == 3,
            Filter::TermHonor => is_terminal_or_honor(t),
            Filter::TermNum => is_terminal(t),
            Filter::Big => is_suited(t) && num(t) >= 7,
            Filter::Mid => is_suited(t) && (4..=6).contains(&num(t)),
            Filter::Small => is_suited(t) && num(t) <= 3,
            Filter::Above5 => is_suited(t) && num(t) >= 6,
            Filter::Below5 => is_suited(t) && num(t) <= 4,
            Filter::Even => is_suited(t) && num(t) % 2 == 0,
            Filter::Tuibu => match suit(t) {
                2 => matches!(num(t), 1 | 2 | 3 | 4 | 5 | 8 | 9),
                1 => matches!(num(t), 2 | 4 | 5 | 6 | 8 | 9),
                _ => t == 33,
            },
            Filter::Green => match suit(t) {
                1 => matches!(num(t), 2 | 3 | 4 | 6 | 8),
                _ => t == 32,
            },
        }
    }
}

fn cover(hand: &Counts, target: &Counts) -> u32 {
    (0..NUM_TILES)
        .map(|t| hand[t].min(target[t]) as u32)
        .sum()
}

fn gain(hand: &Counts, cur: &Counts, t: usize, k: u8) -> u32 {
    let before = hand[t].min(cur[t]) as u32;
    let after = hand[t].min(cur[t].saturating_add(k)) as u32;
    after - before
}

fn has_five_gates(t: &Counts, melds: &[Meld]) -> bool {
    let mut c = *t;
    for m in melds {
        for x in m.tiles() {
            c[x] = c[x].saturating_add(1);
        }
    }
    (0..3).all(|s| (s * 9..s * 9 + 9).any(|x| c[x] > 0))
        && (27..=30).any(|x| c[x] > 0)
        && (31..=33).any(|x| c[x] > 0)
}

#[allow(clippy::too_many_arguments)]
fn rec(
    i: usize,
    left: usize,
    cur: &mut Counts,
    cov: u32,
    hand: &Counts,
    f: Filter,
    five_gates: bool,
    melds: &[Meld],
    opts: &[[usize; 3]],
    best: &mut Option<(u32, Counts)>,
) {
    let ub = cov + 3 * left as u32 + 2;
    if let Some((bc, _)) = *best {
        if ub <= bc {
            return;
        }
    }
    if left == 0 {
        for p in 0..NUM_TILES {
            if !f.ok(p) {
                continue;
            }
            let mut t = *cur;
            t[p] = t[p].saturating_add(2);
            if five_gates && !has_five_gates(&t, melds) {
                continue;
            }
            let c = cov + gain(hand, cur, p, 2);
            if best.as_ref().map_or(true, |(bc, _)| c > *bc) {
                *best = Some((c, t));
            }
        }
        return;
    }
    for k in i..opts.len() {
        let s = opts[k];
        let mut g = 0u32;
        for &t in s.iter() {
            g += gain(hand, cur, t, 1);
            cur[t] += 1;
        }
        rec(k, left - 1, cur, cov + g, hand, f, five_gates, melds, opts, best);
        for &t in s.iter() {
            cur[t] -= 1;
        }
    }
}

/// 4 面子 + 1 将 的「最大覆盖」。返回 (cover, 暗牌部分的目标牌型)
pub fn best_standard(
    hand: &Counts,
    melds: &[Meld],
    f: Filter,
    allow_runs: bool,
    five_gates: bool,
) -> Option<(u32, Counts)> {
    for m in melds {
        if !allow_runs && m.kind == MeldKind::Chi {
            return None;
        }
        if !m.tiles().iter().all(|&t| f.ok(t)) {
            return None;
        }
    }
    let left0 = 4usize.checked_sub(melds.len())?;

    let mut opts: Vec<[usize; 3]> = Vec::new();
    for t in 0..NUM_TILES {
        if f.ok(t) {
            opts.push([t, t, t]);
        }
        if allow_runs && is_suited(t) && num(t) <= 7 && f.ok(t + 1) && f.ok(t + 2) {
            opts.push([t, t + 1, t + 2]);
        }
    }
    // 先试“能吃上手牌最多的面子”，让第一轮回溯就拿到好解，剪枝才能早生效
    opts.sort_by_key(|s| std::cmp::Reverse(s.iter().map(|&t| hand[t] as u32).sum::<u32>()));

    let mut best: Option<(u32, Counts)> = None;
    let mut cur = counts();
    rec(0, left0, &mut cur, 0, hand, f, five_gates, melds, &opts, &mut best);
    best
}

/// 七对：目标 7 个对子（4 张当两对）
pub fn seven_pairs(hand: &Counts, melds: &[Meld]) -> Option<(u32, Counts)> {
    if !melds.is_empty() || total(hand) != 13 {
        return None;
    }
    let mut t = counts();
    let mut slots = 7u32;
    let mut order: Vec<usize> = (0..NUM_TILES).collect();
    order.sort_by_key(|&x| std::cmp::Reverse(hand[x]));
    for &x in order.iter() {
        if slots >= 2 && hand[x] >= 4 {
            t[x] += 4;
            slots -= 2;
        }
    }
    for &x in order.iter() {
        if slots >= 1 && hand[x] >= 2 && t[x] == 0 {
            t[x] += 2;
            slots -= 1;
        }
    }
    for &x in order.iter() {
        if slots >= 1 && hand[x] >= 1 && t[x] == 0 {
            t[x] += 2;
            slots -= 1;
        }
    }
    Some((cover(hand, &t), t))
}

/// 连七对：同一门花色连续 7 张各一对
pub fn seven_pairs_straight(hand: &Counts, melds: &[Meld]) -> Option<(u32, Counts)> {
    if !melds.is_empty() || total(hand) != 13 {
        return None;
    }
    let mut best: Option<(u32, Counts)> = None;
    for s in 0..3usize {
        for n in 1..=3usize {
            let mut t = counts();
            for k in 0..7usize {
                t[s * 9 + (n - 1) + k] = 2;
            }
            let c = cover(hand, &t);
            if best.as_ref().map_or(true, |(bc, _)| c > *bc) {
                best = Some((c, t));
            }
        }
    }
    best
}

/// 十三幺：13 种幺九牌各一张 + 其中一种作将
pub fn thirteen_orphans(hand: &Counts, melds: &[Meld]) -> Option<(u32, Counts)> {
    if !melds.is_empty() || total(hand) != 13 {
        return None;
    }
    let kinds = YAOJIU.iter().filter(|&&t| hand[t] > 0).count() as u32;
    let pair_flag = if YAOJIU.iter().any(|&t| hand[t] >= 2) { 1 } else { 0 };
    let mut t = counts();
    for &x in YAOJIU.iter() {
        t[x] = 1;
    }
    if let Some(&x) = YAOJIU.iter().find(|&&x| hand[x] >= 2) {
        t[x] = 2;
    } else {
        t[YAOJIU[0]] = 2;
    }
    Some((kinds + pair_flag, t))
}

/// 全不靠 / 七星不靠：三花色各取 147/258/369 之一（互不错位）+ 字牌
pub fn bu_kao(hand: &Counts, melds: &[Meld], seven_stars: bool) -> Option<(u32, Counts)> {
    if !melds.is_empty() || total(hand) != 13 {
        return None;
    }
    let held_honors = (27..=33).filter(|&x| hand[x] > 0).count();
    let mut best: Option<(u32, Counts)> = None;
    for perm in PERMS.iter() {
        let knit = knit_tiles(*perm);
        let honor_take = if seven_stars {
            7
        } else {
            held_honors.clamp(5, 7)
        };
        let mut t = counts();
        let mut hs: Vec<usize> = (27..=33).collect();
        hs.sort_by_key(|&x| std::cmp::Reverse(hand[x]));
        for &x in hs.iter().take(honor_take) {
            t[x] = 1;
        }
        let mut ks = knit.clone();
        ks.sort_by_key(|&x| std::cmp::Reverse(hand[x]));
        for &x in ks.iter().take(14 - honor_take) {
            if t[x] == 0 {
                t[x] = 1;
            }
        }
        let c = cover(hand, &t);
        if best.as_ref().map_or(true, |(bc, _)| c > *bc) {
            best = Some((c, t));
        }
    }
    best
}

/// 4 面子 + 1 将 的主番（牌种限制型）
const STANDARD_PATTERNS: &[(&str, u32, Filter, bool, bool, &str)] = &[
    ("字一色", 64, Filter::Honors, false, false, ""),
    ("清幺九", 64, Filter::TermNum, true, false, ""),
    ("混幺九", 32, Filter::TermHonor, true, false, ""),
    ("全大", 24, Filter::Big, true, false, ""),
    ("全中", 24, Filter::Mid, true, false, ""),
    ("全小", 24, Filter::Small, true, false, ""),
    ("全双刻", 24, Filter::Even, false, false, "对子须为双数"),
    ("大于五", 12, Filter::Above5, true, false, ""),
    ("小于五", 12, Filter::Below5, true, false, ""),
    ("绿一色", 88, Filter::Green, true, false, ""),
    ("推不倒", 8, Filter::Tuibu, true, false, ""),
    ("碰碰和", 6, Filter::Any, false, false, ""),
    ("五门齐", 6, Filter::Any, true, true, "万条筒风箭俱全"),
];

fn push_dir(
    out: &mut Vec<Direction>,
    hand: &Counts,
    name: String,
    value: u32,
    res: Option<(u32, Counts)>,
    tag: &'static str,
    max_dist: u32,
) {
    let Some((cov, target)) = res else { return };
    let dist = total(hand).saturating_sub(cov);
    if dist > max_dist {
        return;
    }
    let mut need = Vec::new();
    let mut drop = Vec::new();
    for t in 0..NUM_TILES {
        for _ in 0..hand[t].saturating_sub(target[t]) {
            drop.push(t);
        }
        for _ in 0..target[t].saturating_sub(hand[t]) {
            need.push(t);
        }
    }
    out.push(Direction {
        name,
        value,
        distance: dist,
        need,
        drop,
        tag,
    });
}

/// 扫描这手牌离各个番种还差几张。只返回距离 ≤ max_dist 的方向。
pub fn plan(hand: &Counts, melds: &[Meld], max_dist: u32) -> Vec<Direction> {
    let mut out: Vec<Direction> = Vec::new();

    for s in 0..3 {
        push_dir(
            &mut out,
            hand,
            format!("清一色({}门)", SUIT_NAME[s]),
            24,
            best_standard(hand, melds, Filter::Suit(s), true, false),
            "",
            max_dist,
        );
        push_dir(
            &mut out,
            hand,
            format!("混一色({}门)", SUIT_NAME[s]),
            6,
            best_standard(hand, melds, Filter::Mixed(s), true, false),
            "",
            max_dist,
        );
    }
    for &(name, value, f, runs, gates, tag) in STANDARD_PATTERNS {
        push_dir(
            &mut out,
            hand,
            name.to_string(),
            value,
            best_standard(hand, melds, f, runs, gates),
            tag,
            max_dist,
        );
    }

    push_dir(&mut out, hand, "七对".into(), 24, seven_pairs(hand, melds), "门清", max_dist);
    push_dir(
        &mut out,
        hand,
        "连七对".into(),
        88,
        seven_pairs_straight(hand, melds),
        "门清",
        max_dist,
    );
    push_dir(
        &mut out,
        hand,
        "十三幺".into(),
        88,
        thirteen_orphans(hand, melds),
        "门清",
        max_dist,
    );
    push_dir(
        &mut out,
        hand,
        "全不靠".into(),
        12,
        bu_kao(hand, melds, false),
        "门清",
        max_dist,
    );
    push_dir(
        &mut out,
        hand,
        "七星不靠".into(),
        24,
        bu_kao(hand, melds, true),
        "门清",
        max_dist,
    );

    out.sort_by(|a, b| a.distance.cmp(&b.distance).then(b.value.cmp(&a.value)));
    out
}
