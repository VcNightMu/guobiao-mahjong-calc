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
use crate::fans::coverage_pairs;
use crate::model::{Meld, MeldKind};
use crate::tiles::*;

pub const SUIT_NAME: [&str; 3] = ["万", "条", "筒"];

/// 方向扫描的默认搜索半径（**策略常量**）：CLI 的 `--plan` 只列出距离不超过它的方向。
/// 与 `plan()` 的 `max_dist` 参数同义。测试侧**刻意**保留字面量作为契约护栏：
/// 一旦本值变动，方向快照族会立刻失败报警，而不是静默跟随。
pub const MAX_DIST: u32 = 2;

#[derive(Clone, Debug)]
pub struct Direction {
    pub name: String,
    pub value: u32,
    pub distance: u32,
    pub need: Vec<usize>,
    pub drop: Vec<usize>,
    pub tag: &'static str,
    /// R-13：本方向被哪些「已完成（distance==0）」的主番覆盖（仅标注用，不影响 distance/need/drop/value）
    pub covered_by: Vec<String>,
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
    v.iter()
        .map(|&t| tile_name(t))
        .collect::<Vec<_>>()
        .join(" ")
}

/// 牌种限制
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    Any,
    /// 单门花色（清一色）
    Suit(usize),
    /// 单门花色 + 字牌（混一色）
    Mixed(usize),
    /// 单门花色 + 字牌，且目标必须含至少一张字牌（混一色）
    MixedHonor(usize),
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
            Filter::Mixed(s) | Filter::MixedHonor(s) => suit(t) == s || suit(t) == 3,
            Filter::Honors => suit(t) == 3,
            Filter::TermHonor => is_terminal_or_honor(t),
            Filter::TermNum => is_terminal(t),
            Filter::Big => is_suited(t) && num(t) >= 7,
            Filter::Mid => is_suited(t) && (4..=6).contains(&num(t)),
            Filter::Small => is_suited(t) && num(t) <= 3,
            Filter::Above5 => is_suited(t) && num(t) >= 6,
            Filter::Below5 => is_suited(t) && num(t) <= 4,
            Filter::Even => is_suited(t) && num(t).is_multiple_of(2),
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

    /// 存在性约束：目标必须含至少一张字牌（混一色 / 混幺九）。
    /// 与 `has_five_gates`（五门齐的同类强制）同型：`Filter` 本身只是「允许集」，
    /// 规则额外要求的「必须存在」在搜索终点（加将之后）显式校验，避免 `gain()`
    /// 在 `hand[t] == 0` 时恒 0、搜索只最大化覆盖而回避「非必需但规则必需」的牌类。
    pub fn requires_honor(self) -> bool {
        matches!(self, Filter::MixedHonor(_) | Filter::TermHonor)
    }

    /// 存在性约束：目标必须含至少一张该门数牌（混一色）。
    /// 同 `has_five_gates` 型强制：`MixedHonor(s)` 的允许集含 s 门数牌与字牌，
    /// 但规则要求两者都真实出现，故在终点校验 s 门数牌的存在性。
    pub fn required_suit(self) -> Option<usize> {
        match self {
            Filter::MixedHonor(s) => Some(s),
            _ => None,
        }
    }
}

fn cover(hand: &Counts, target: &Counts) -> u32 {
    (0..NUM_TILES).map(|t| hand[t].min(target[t]) as u32).sum()
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

/// T3-06：候选目标可达性——目标牌张 + 副露/杠已占张数 不得超过每种 4 张。
fn target_feasible(target: &Counts, melds: &[Meld]) -> bool {
    let mut mpt = [0u8; NUM_TILES];
    for m in melds {
        for t in m.tiles() {
            mpt[t] = mpt[t].saturating_add(1);
        }
    }
    (0..NUM_TILES).all(|t| target[t].saturating_add(mpt[t]) <= 4)
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
    pair_tile: Option<usize>,
    best: &mut Option<(u32, Counts)>,
) {
    let ub = cov + 3 * left as u32 + 2;
    if let Some((bc, _)) = *best {
        if ub <= bc {
            return;
        }
    }
    if left == 0 {
        let cands: Vec<usize> = match pair_tile {
            Some(p) => vec![p],
            None => (0..NUM_TILES).filter(|&p| f.ok(p)).collect(),
        };
        for p in cands {
            let mut t = *cur;
            t[p] = t[p].saturating_add(2);
            if five_gates && !has_five_gates(&t, melds) {
                continue;
            }
            // 存在性约束（混一色的字牌/该门数牌、混幺九的字牌）：与 has_five_gates 一致，把固定副露一并计入
            if f.requires_honor() || f.required_suit().is_some() {
                let mut tc = t;
                for m in melds {
                    for x in m.tiles() {
                        tc[x] = tc[x].saturating_add(1);
                    }
                }
                if f.requires_honor() && !(27..=33).any(|x| tc[x] > 0) {
                    continue;
                }
                if let Some(s) = f.required_suit() {
                    if !(s * 9..s * 9 + 9).any(|x| tc[x] > 0) {
                        continue;
                    }
                }
            }
            let c = cov + gain(hand, cur, p, 2);
            if target_feasible(&t, melds) && best.as_ref().is_none_or(|(bc, _)| c > *bc) {
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
        rec(
            k,
            left - 1,
            cur,
            cov + g,
            hand,
            f,
            five_gates,
            melds,
            opts,
            pair_tile,
            best,
        );
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
    best_forced(hand, melds, f, allow_runs, five_gates, &[], None)
}

// 骨架系用的小工具：run = 同门顺子，trip = 同门刻子
pub fn run(s: usize, n: usize) -> [usize; 3] {
    [s * 9 + n - 1, s * 9 + n, s * 9 + n + 1]
}
pub fn trip(s: usize, n: usize) -> [usize; 3] {
    let t = s * 9 + n - 1;
    [t, t, t]
}
fn is_run3(s: &[usize; 3]) -> bool {
    !(s[0] == s[1] && s[1] == s[2])
}

/// 同 best_standard，但可以先钉死若干「强制面子」（清龙、一色四步高、双龙会这类骨架番）。
/// pair_tile 给定时，将牌也被钉死（双龙会必须是 5）。
pub fn best_forced(
    hand: &Counts,
    melds: &[Meld],
    f: Filter,
    allow_runs: bool,
    five_gates: bool,
    forced: &[[usize; 3]],
    pair_tile: Option<usize>,
) -> Option<(u32, Counts)> {
    for m in melds {
        if !allow_runs && m.kind == MeldKind::Chi {
            return None;
        }
        if !m.tiles().iter().all(|&t| f.ok(t)) {
            return None;
        }
    }
    for s in forced {
        if s.iter().any(|&t| !f.ok(t)) {
            return None;
        }
        if !allow_runs && is_run3(s) {
            return None;
        }
    }
    // 副露 + 强制面子 已经占满 4 副 → 连骨架都放不下，直接不可能
    let used = 4usize.checked_sub(melds.len())?;
    if forced.len() > used {
        return None;
    }
    let left0 = used - forced.len();
    if let Some(p) = pair_tile {
        if !f.ok(p) {
            return None;
        }
    }

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
    let mut cov0 = 0u32;
    for s in forced {
        for &t in s.iter() {
            cov0 += gain(hand, &cur, t, 1);
            cur[t] += 1;
        }
    }
    rec(
        0, left0, &mut cur, cov0, hand, f, five_gates, melds, &opts, pair_tile, &mut best,
    );
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
            if best.as_ref().is_none_or(|(bc, _)| c > *bc) {
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
    let pair_flag = if YAOJIU.iter().any(|&t| hand[t] >= 2) {
        1
    } else {
        0
    };
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
        if best.as_ref().is_none_or(|(bc, _)| c > *bc) {
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
        covered_by: Vec::new(),
    });
}

/// 扫描这手牌离各个番种还差几张。只返回距离 ≤ max_dist 的方向。
pub fn plan(hand: &Counts, melds: &[Meld], max_dist: u32) -> Vec<Direction> {
    let mut out: Vec<Direction> = Vec::new();
    // R-13：覆盖对只从 fans::coverage_pairs() 取（完整来源 = drop 表 ∪ 分支互斥表），全程只用这一份
    let coverage: Vec<(&str, &str)> = coverage_pairs().collect();

    for (s, sname) in SUIT_NAME.iter().enumerate() {
        push_dir(
            &mut out,
            hand,
            format!("清一色({}门)", sname),
            24,
            best_standard(hand, melds, Filter::Suit(s), true, false),
            "",
            max_dist,
        );
        push_dir(
            &mut out,
            hand,
            format!("混一色({}门)", sname),
            6,
            best_standard(hand, melds, Filter::MixedHonor(s), true, false),
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

    // ── 骨架系：顺子/刻子被番种钉死 ──
    // 副露里已经有两个碰/杠时，顺子系骨架一概放不下（4 副面子装不下 3 个强制顺子），直接跳过
    let pungs = melds.iter().filter(|m| m.kind != MeldKind::Chi).count();
    if pungs < 2 {
        let perm3 = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        for (s, sname) in SUIT_NAME.iter().enumerate() {
            push_forced(
                &mut out,
                hand,
                melds,
                format!("清龙({}门)", sname),
                16,
                &[run(s, 1), run(s, 4), run(s, 7)],
                None,
                max_dist,
            );
        }
        for p in perm3.iter() {
            push_forced(
                &mut out,
                hand,
                melds,
                "花龙".into(),
                8,
                &[run(p[0], 1), run(p[1], 4), run(p[2], 7)],
                None,
                max_dist,
            );
        }
        for (s, sname) in SUIT_NAME.iter().enumerate() {
            for d in 1..=2usize {
                for n in 1..=(7 - 2 * d) {
                    push_forced(
                        &mut out,
                        hand,
                        melds,
                        format!("一色三步高({}门)", sname),
                        16,
                        &[run(s, n), run(s, n + d), run(s, n + 2 * d)],
                        None,
                        max_dist,
                    );
                }
            }
        }
        for p in perm3.iter() {
            for d in 1..=2usize {
                for n in 1..=(7 - 2 * d) {
                    push_forced(
                        &mut out,
                        hand,
                        melds,
                        "三色三步高".into(),
                        6,
                        &[run(p[0], n), run(p[1], n + d), run(p[2], n + 2 * d)],
                        None,
                        max_dist,
                    );
                }
            }
        }
        for (s, sname) in SUIT_NAME.iter().enumerate() {
            for n in 1..=7 {
                push_forced(
                    &mut out,
                    hand,
                    melds,
                    format!("一色三同顺({}门)", sname),
                    24,
                    &[run(s, n), run(s, n), run(s, n)],
                    None,
                    max_dist,
                );
            }
        }
        for n in 1..=7 {
            push_forced(
                &mut out,
                hand,
                melds,
                "三色三同顺".into(),
                8,
                &[run(0, n), run(1, n), run(2, n)],
                None,
                max_dist,
            );
        }
        for (s, sname) in SUIT_NAME.iter().enumerate() {
            for n in 1..=7 {
                push_forced(
                    &mut out,
                    hand,
                    melds,
                    format!("一色三节高({}门)", sname),
                    24,
                    &[trip(s, n), trip(s, n + 1), trip(s, n + 2)],
                    None,
                    max_dist,
                );
            }
        }
        for p in perm3.iter() {
            for n in 1..=7 {
                push_forced(
                    &mut out,
                    hand,
                    melds,
                    "三色三节高".into(),
                    8,
                    &[trip(p[0], n), trip(p[1], n + 1), trip(p[2], n + 2)],
                    None,
                    max_dist,
                );
            }
        }
        for (s, sname) in SUIT_NAME.iter().enumerate() {
            for n in 1..=7 {
                push_forced(
                    &mut out,
                    hand,
                    melds,
                    format!("一色四同顺({}门)", sname),
                    48,
                    &[run(s, n), run(s, n), run(s, n), run(s, n)],
                    None,
                    max_dist,
                );
            }
        }
        for (s, sname) in SUIT_NAME.iter().enumerate() {
            for n in 1..=6 {
                push_forced(
                    &mut out,
                    hand,
                    melds,
                    format!("一色四节高({}门)", sname),
                    48,
                    &[trip(s, n), trip(s, n + 1), trip(s, n + 2), trip(s, n + 3)],
                    None,
                    max_dist,
                );
            }
        }
        for (s, sname) in SUIT_NAME.iter().enumerate() {
            for d in 1..=2usize {
                for n in 1..=(7 - 3 * d) {
                    push_forced(
                        &mut out,
                        hand,
                        melds,
                        format!("一色四步高({}门)", sname),
                        32,
                        &[
                            run(s, n),
                            run(s, n + d),
                            run(s, n + 2 * d),
                            run(s, n + 3 * d),
                        ],
                        None,
                        max_dist,
                    );
                }
            }
        }
        // 双龙会：两个老少副 + 本门 5 作将
        for (s, sname) in SUIT_NAME.iter().enumerate() {
            push_forced(
                &mut out,
                hand,
                melds,
                format!("一色双龙会({}门)", sname),
                64,
                &[run(s, 1), run(s, 1), run(s, 7), run(s, 7)],
                Some(s * 9 + 4),
                max_dist,
            );
        }
        for (s, sname) in SUIT_NAME.iter().enumerate() {
            let a = (s + 1) % 3;
            let b = (s + 2) % 3;
            push_forced(
                &mut out,
                hand,
                melds,
                format!("三色双龙会({}门将)", sname),
                16,
                &[run(a, 1), run(a, 7), run(b, 1), run(b, 7)],
                Some(s * 9 + 4),
                max_dist,
            );
        }
        // 九莲宝灯：门清且暗牌 13 张时才有意义
        if melds.is_empty() {
            for s in 0..3 {
                for x in 1..=9usize {
                    let mut t = counts();
                    for n in 1..=9usize {
                        t[s * 9 + n - 1] = if n == 1 || n == 9 { 3 } else { 1 };
                    }
                    t[s * 9 + x - 1] += 1;
                    let c = cover(hand, &t);
                    push_dir(
                        &mut out,
                        hand,
                        format!("九莲宝灯({}门)", SUIT_NAME[s]),
                        88,
                        Some((c, t)),
                        "门清",
                        max_dist,
                    );
                }
            }
        }
        // 组合龙：147/258/369 九张不能错位 + 一副面子 + 将（可带一副副露）
        if melds.len() <= 1 {
            for perm in PERMS.iter() {
                let knit = knit_tiles(*perm);
                let mut base = counts();
                for &t in knit.iter() {
                    base[t] = 1;
                }
                let melded = melds.len() == 1;
                let mut best: Option<(u32, Counts)> = None;
                let mut set_opts: Vec<Option<Vec<usize>>> = Vec::new();
                if melded {
                    set_opts.push(None);
                } else {
                    for t in 0..NUM_TILES {
                        set_opts.push(Some(vec![t, t, t]));
                    }
                    for s in 0..3 {
                        for n in 1..=7usize {
                            set_opts.push(Some(vec![s * 9 + n - 1, s * 9 + n, s * 9 + n + 1]));
                        }
                    }
                }
                for so in set_opts.iter() {
                    for p in 0..NUM_TILES {
                        let mut t = base;
                        if let Some(v) = so {
                            for &x in v.iter() {
                                t[x] = t[x].saturating_add(1);
                            }
                        }
                        t[p] = t[p].saturating_add(2);
                        let c = cover(hand, &t);
                        if target_feasible(&t, melds) && best.as_ref().is_none_or(|(bc, _)| c > *bc)
                        {
                            best = Some((c, t));
                        }
                    }
                }
                push_dir(&mut out, hand, "组合龙".into(), 12, best, "", max_dist);
            }
        }
    }

    // ── 刻子系 ──
    // 暗刻系只做「点和能成」的形态：三/四暗刻的刻子必须已经在暗牌里成形，
    // 不能靠在和牌张上（点和补出来的刻子算明刻）；只有自摸才成立的那类不列。
    let ankan = melds
        .iter()
        .filter(|m| m.kind == MeldKind::Kan && !m.open)
        .count();
    let full: Vec<usize> = (0..NUM_TILES).filter(|&t| hand[t] == 3).collect();
    if ankan < 3 {
        let need3 = 3usize.saturating_sub(ankan);
        if full.len() >= need3 {
            for c in combos(&full, need3) {
                let forced: Vec<[usize; 3]> = c.iter().map(|&t| [t, t, t]).collect();
                push_forced(
                    &mut out,
                    hand,
                    melds,
                    "三暗刻".into(),
                    16,
                    &forced,
                    None,
                    max_dist,
                );
            }
        }
    }
    if ankan < 4 {
        let need4 = 4usize.saturating_sub(ankan);
        if full.len() >= need4 {
            for c in combos(&full, need4) {
                let forced: Vec<[usize; 3]> = c.iter().map(|&t| [t, t, t]).collect();
                push_forced(
                    &mut out,
                    hand,
                    melds,
                    "四暗刻".into(),
                    64,
                    &forced,
                    None,
                    max_dist,
                );
            }
        }
    }
    // 明暗皆可的刻子系（碰/杠/点和补成都可以）
    let winds = [1usize, 2, 3, 4];
    let dragons = [5usize, 6, 7];
    for n in 1..=9usize {
        push_forced(
            &mut out,
            hand,
            melds,
            "三同刻".into(),
            16,
            &[trip(0, n), trip(1, n), trip(2, n)],
            None,
            max_dist,
        );
    }
    for c in combos(&winds, 3) {
        let forced: Vec<[usize; 3]> = c.iter().map(|&n| trip(3, n)).collect();
        push_forced(
            &mut out,
            hand,
            melds,
            "三风刻".into(),
            12,
            &forced,
            None,
            max_dist,
        );
    }
    for c in combos(&dragons, 2) {
        let forced: Vec<[usize; 3]> = c.iter().map(|&n| trip(3, n)).collect();
        push_forced(
            &mut out,
            hand,
            melds,
            "双箭刻".into(),
            6,
            &forced,
            None,
            max_dist,
        );
    }
    push_forced(
        &mut out,
        hand,
        melds,
        "大三元".into(),
        88,
        &[trip(3, 5), trip(3, 6), trip(3, 7)],
        None,
        max_dist,
    );
    for k in 0..3usize {
        let forced: Vec<[usize; 3]> = (0..3usize)
            .filter(|&i| i != k)
            .map(|i| trip(3, dragons[i]))
            .collect();
        push_forced(
            &mut out,
            hand,
            melds,
            "小三元".into(),
            64,
            &forced,
            Some(26 + dragons[k]),
            max_dist,
        );
    }
    push_forced(
        &mut out,
        hand,
        melds,
        "大四喜".into(),
        88,
        &[trip(3, 1), trip(3, 2), trip(3, 3), trip(3, 4)],
        None,
        max_dist,
    );
    for k in 0..4usize {
        let forced: Vec<[usize; 3]> = (0..4usize)
            .filter(|&i| i != k)
            .map(|i| trip(3, winds[i]))
            .collect();
        push_forced(
            &mut out,
            hand,
            melds,
            "小四喜".into(),
            64,
            &forced,
            Some(27 + winds[k] - 1),
            max_dist,
        );
    }

    push_dir(
        &mut out,
        hand,
        "全带五".into(),
        16,
        quandaiwu(hand, melds),
        "",
        max_dist,
    );

    push_dir(
        &mut out,
        hand,
        "七对".into(),
        24,
        seven_pairs(hand, melds),
        "门清",
        max_dist,
    );
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
    // 同名（同一骨架的不同排列）只留最近的那一条
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    out.retain(|d| seen.insert(d.name.clone()));

    // ── R-13：已完成（distance==0）的主番 → 其覆盖的低番方向加标注（只标注、不隐藏、不改距离） ──
    let achieved: Vec<(String, u32)> = out
        .iter()
        .filter(|d| d.distance == 0)
        .map(|d| (d.name.clone(), d.value))
        .collect();
    for d in out.iter_mut() {
        let mut cov: Vec<(String, u32)> = achieved
            .iter()
            .filter(|(n, _)| covered_by_coverer(n, &d.name, &coverage))
            .cloned()
            .collect();
        // 番数降序、同名升序（口径同 out 的排序精神）
        cov.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        cov.dedup_by(|a, b| a.0 == b.0);
        d.covered_by = cov.into_iter().map(|(n, _)| n).collect();
    }
    out
}

/// 拆出「(X门)」后缀：`清一色(万门)` → (`清一色`, Some(`(万门)`))；无括号 → (`名`, None)。
fn split_suffix(s: &str) -> (&str, Option<&str>) {
    match s.find('(') {
        Some(i) => (&s[..i], Some(&s[i..])),
        None => (s, None),
    }
}

/// R-13 覆盖关系：coverer 已完成时，covered 方向不值得再考虑（只标注，不隐藏）。
///
/// **覆盖对来源 = `src/fans.rs::coverage_pairs()` 自动导出**（完整来源 = 计番 drop 表 `EXCLUSIONS`
/// ∪ 分支互斥表 `BRANCH_EXCLUSIONS`：清幺九/混幺九、一色四节高/一色三节高、一色四步高/一色三步高），
/// 此处不再手工枚举任何覆盖对，也不再直接引用 `EXCLUSIONS`（避免第二处手工枚举）。
///
/// 匹配规则：
/// 1. 拆后缀：`清一色(万门)` → base = `清一色`、suffix = `(万门)`；无括号则 suffix = None。
/// 2. 表内匹配：`coverage` 中存在覆盖对 `(coverer_base, covered_base)`（两端 base 完全相等）。
/// 3. **同门约束**：coverer 与 covered 双方都有后缀时，后缀必须**完全相同**；
///    至少一方无后缀时视为**通配**（可匹配任意门）。
///
/// **交集过滤（只保留两端都真实出现在本次 `out` 里的对）**：由 `plan()` 现有循环天然满足，
/// 无需任何静态名单 —— 见 `plan()` 末尾：只有 `distance == 0` 的方向才会进入 `achieved` 充当 coverer，
/// 而 covered 必须本身就是本次 `out` 中的方向才会被遍历到并接受标注。
fn covered_by_coverer(coverer: &str, covered: &str, coverage: &[(&str, &str)]) -> bool {
    if coverer == covered {
        return false;
    }
    let (cb, cs) = split_suffix(coverer);
    let (db, ds) = split_suffix(covered);
    // 同门判定：双方都带后缀必须相同；至少一方无后缀 → 通配
    let same_suit = match (cs, ds) {
        (Some(a), Some(b)) => a == b,
        _ => true,
    };
    same_suit
        && coverage
            .iter()
            .any(|&(c, cov_name)| c == cb && cov_name == db)
}

/// 从 v 里取 k 个的组合（用于枚举哪几个刻子/哪几个风箭）
fn combos(v: &[usize], k: usize) -> Vec<Vec<usize>> {
    fn go(v: &[usize], k: usize, i: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if cur.len() == k {
            out.push(cur.clone());
            return;
        }
        if v.len() - i < k - cur.len() {
            return;
        }
        for j in i..v.len() {
            cur.push(v[j]);
            go(v, k, j + 1, cur, out);
            cur.pop();
        }
    }
    let mut out = Vec::new();
    if k == 0 {
        out.push(Vec::new());
        return out;
    }
    let mut cur = Vec::new();
    go(v, k, 0, &mut cur, &mut out);
    out
}

/// 全带五：每一副面子和将都含 5 → 可用的只有 345/456/567/555 和 55。
/// 由此 1、2、8、9、字牌完全用不上：手里这种牌超过两张，距离必然 ≥ 3，交给 ≤2 张的过滤就看不见了。
pub fn quandaiwu(hand: &Counts, melds: &[Meld]) -> Option<(u32, Counts)> {
    // 副露每一副都必须含 5（5万/5条/5筒 分别为 4/13/22），否则这副牌永远不成立
    let fives = [4usize, 13, 22];
    for m in melds {
        let ts = m.tiles();
        if !fives.iter().any(|&f| ts.contains(&f)) {
            return None;
        }
    }
    let left = 4usize.checked_sub(melds.len())?;
    let mut sets: Vec<[usize; 3]> = Vec::new();
    for s in 0..3usize {
        sets.push([s * 9 + 2, s * 9 + 3, s * 9 + 4]);
        sets.push([s * 9 + 3, s * 9 + 4, s * 9 + 5]);
        sets.push([s * 9 + 4, s * 9 + 5, s * 9 + 6]);
        sets.push([s * 9 + 4, s * 9 + 4, s * 9 + 4]);
    }
    let mut best: Option<(u32, Counts)> = None;
    let mut cur = counts();
    go_qdw(0, left, &sets, &mut cur, 0, hand, melds, &mut best);
    best
}

// go_qdw 的 8 个参数全为原始类型/不可变借用（递归深度优先搜索的内部辅助函数），
// 仅因 T3-06 增加了「副露已占张数」所需的 melds 形参而达到 8 个；拆结构体不值得，故显式 allow。
#[allow(clippy::too_many_arguments)]
fn go_qdw(
    i: usize,
    left: usize,
    sets: &[[usize; 3]],
    cur: &mut Counts,
    cov: u32,
    hand: &Counts,
    melds: &[Meld],
    best: &mut Option<(u32, Counts)>,
) {
    if left == 0 {
        for s in 0..3usize {
            let p = s * 9 + 4;
            let mut t = *cur;
            t[p] = t[p].saturating_add(2);
            let c = cov + gain(hand, cur, p, 2);
            if target_feasible(&t, melds) && best.as_ref().is_none_or(|(bc, _)| c > *bc) {
                *best = Some((c, t));
            }
        }
        return;
    }
    for k in i..sets.len() {
        let s = sets[k];
        let mut g = 0u32;
        for &t in s.iter() {
            g += gain(hand, cur, t, 1);
            cur[t] += 1;
        }
        go_qdw(k, left - 1, sets, cur, cov + g, hand, melds, best);
        for &t in s.iter() {
            cur[t] -= 1;
        }
    }
}

// push_forced 的 8 个参数全为原始类型/不可变借用（&mut Vec / &Counts / &[Meld] / String / u32 / &[[usize;3]] / Option<usize> / u32），
// 且约有 22 处调用点（各番种分支中的 push_forced(…) 调用；2026-10-12 实测 22 处，不再逐一记行号）；
// 抽参数结构体会牵动全部调用点，而本轮以「行为分毫不变」为最高优先，故显式 allow（T5 处置依据见回报）。
#[allow(clippy::too_many_arguments)]
fn push_forced(
    out: &mut Vec<Direction>,
    hand: &Counts,
    melds: &[Meld],
    name: String,
    value: u32,
    forced: &[[usize; 3]],
    pair_tile: Option<usize>,
    max_dist: u32,
) {
    let res = best_forced(hand, melds, Filter::Any, true, false, forced, pair_tile);
    push_dir(out, hand, name, value, res, "", max_dist);
}
