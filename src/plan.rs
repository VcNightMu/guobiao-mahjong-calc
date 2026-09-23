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
        0,
        left0,
        &mut cur,
        cov0,
        hand,
        f,
        five_gates,
        melds,
        &opts,
        pair_tile,
        &mut best,
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
        for s in 0..3 {
            push_forced(
                &mut out,
                hand,
                melds,
                format!("清龙({}门)", SUIT_NAME[s]),
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
        for s in 0..3 {
            for d in 1..=2usize {
                for n in 1..=(7 - 2 * d) {
                    push_forced(
                        &mut out,
                        hand,
                        melds,
                        format!("一色三步高({}门)", SUIT_NAME[s]),
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
        for s in 0..3 {
            for n in 1..=7 {
                push_forced(
                    &mut out,
                    hand,
                    melds,
                    format!("一色三同顺({}门)", SUIT_NAME[s]),
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
        for s in 0..3 {
            for n in 1..=7 {
                push_forced(
                    &mut out,
                    hand,
                    melds,
                    format!("一色三节高({}门)", SUIT_NAME[s]),
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
        for s in 0..3 {
            for n in 1..=7 {
                push_forced(
                    &mut out,
                    hand,
                    melds,
                    format!("一色四同顺({}门)", SUIT_NAME[s]),
                    48,
                    &[run(s, n), run(s, n), run(s, n), run(s, n)],
                    None,
                    max_dist,
                );
            }
        }
        for s in 0..3 {
            for n in 1..=6 {
                push_forced(
                    &mut out,
                    hand,
                    melds,
                    format!("一色四节高({}门)", SUIT_NAME[s]),
                    48,
                    &[trip(s, n), trip(s, n + 1), trip(s, n + 2), trip(s, n + 3)],
                    None,
                    max_dist,
                );
            }
        }
        for s in 0..3 {
            for d in 1..=2usize {
                for n in 1..=(7 - 3 * d) {
                    push_forced(
                        &mut out,
                        hand,
                        melds,
                        format!("一色四步高({}门)", SUIT_NAME[s]),
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
        for s in 0..3 {
            push_forced(
                &mut out,
                hand,
                melds,
                format!("一色双龙会({}门)", SUIT_NAME[s]),
                64,
                &[run(s, 1), run(s, 1), run(s, 7), run(s, 7)],
                Some(s * 9 + 4),
                max_dist,
            );
        }
        for s in 0..3 {
            let a = (s + 1) % 3;
            let b = (s + 2) % 3;
            push_forced(
                &mut out,
                hand,
                melds,
                format!("三色双龙会({}门将)", SUIT_NAME[s]),
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
                        if best.as_ref().map_or(true, |(bc, _)| c > *bc) {
                            best = Some((c, t));
                        }
                    }
                }
                push_dir(&mut out, hand, "组合龙".into(), 12, best, "", max_dist);
            }
        }
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
    // 同名（同一骨架的不同排列）只留最近的那一条
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    out.retain(|d| seen.insert(d.name.clone()));
    out
}

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
