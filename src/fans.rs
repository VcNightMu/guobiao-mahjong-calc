//! 番种识别（牌型番）
//!
//! 说明：这里只算「牌型番」。和牌方式相关的番（门前清 / 不求人 / 自摸 /
//! 和绝张）在 score() 里按和法叠加；边张 / 坎张 / 单钓将是结构性的（依赖和牌张），
//! 放在本文件的 collect() 里。
//!
//! 未实现/待核：组合龙、推不倒、一色四步高细化判定等（见文件末尾 TODO）。

use crate::decompose::*;
use crate::model::*;
use crate::tiles::*;

#[derive(Clone, Copy, Debug)]
pub struct Fan {
    pub name: &'static str,
    pub value: u32,
}

fn f(name: &'static str, value: u32) -> Fan {
    Fan { name, value }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WinMode {
    /// 点炮（常态）
    Normal,
    /// 自摸
    Tsumo,
    /// 点炮 + 绝张
    LastTile,
    /// 自摸 + 绝张
    TsumoLast,
}

#[derive(Clone, Copy)]
pub struct WinCtx<'a> {
    pub all: &'a Counts,
    pub melds: &'a [Meld],
    pub decomp: &'a Decomp,
    pub round_wind: usize,
    pub seat_wind: usize,
    pub win_tile: usize,
    pub menqing: bool,
    pub single_wait: bool,
    /// 是否自摸（影响：荣和补成的刻子算明刻）
    pub tsumo: bool,
}

fn suits_present(all: &Counts) -> [bool; 3] {
    let mut p = [false; 3];
    for t in 0..27usize {
        if all[t] > 0 {
            p[t / 9] = true;
        }
    }
    p
}

fn has_honor(all: &Counts) -> bool {
    (27..34).any(|t| all[t] > 0)
}

fn runs_of(d: &Decomp) -> Vec<usize> {
    d.sets.iter().filter(|s| s.is_run()).map(|s| s.tile).collect()
}

fn trips_of(d: &Decomp) -> Vec<usize> {
    d.sets.iter().filter(|s| s.is_triplet()).map(|s| s.tile).collect()
}

fn is_green(t: usize) -> bool {
    matches!(t, 10 | 11 | 12 | 14 | 16 | 32) // 2/3/4/6/8条 + 发
}

/// 收集牌型番
pub fn collect(ctx: &WinCtx) -> Vec<Fan> {
    if let Some(sp) = ctx.decomp.special {
        return collect_special(ctx, sp);
    }
    collect_standard(ctx)
}

fn collect_special(ctx: &WinCtx, sp: Special) -> Vec<Fan> {
    let mut v: Vec<Fan> = Vec::new();
    let all = ctx.all;
    let suits = suits_present(all);
    let nsuit = suits.iter().filter(|&&x| x).count();
    let honor = has_honor(all);

    match sp {
        Special::SevenPairsStraight => v.push(f("连七对", 88)),
        Special::SevenPairs => v.push(f("七对", 24)),
        Special::ThirteenOrphans => v.push(f("十三幺", 88)),
        Special::SevenStars => v.push(f("七星不靠", 24)),
        Special::BuKao => v.push(f("全不靠", 12)),
        Special::Zuhelong => {
            v.push(f("组合龙", 12));
            let d = ctx.decomp;
            // 第四副面子：m=0 在暗牌里；m=1 即那副副露
            let fourth: Option<Set> = if d.sets.len() == 1 {
                Some(d.sets[0])
            } else if ctx.melds.len() == 1 {
                Some(ctx.melds[0].to_set())
            } else {
                None
            };
            if let Some(s) = fourth {
                if s.is_run() && d.pair.map(is_suited).unwrap_or(false) {
                    v.push(f("平和", 2));
                }
            }
            if !has_honor(all) {
                v.push(f("无字", 1));
            }
        }
    }

    // 颜色类（七对 / 不靠 可叠）
    match sp {
        Special::SevenPairs | Special::SevenPairsStraight => {
            if nsuit == 1 && !honor {
                v.push(f("清一色", 24));
                v.push(f("无字", 1));
            } else if nsuit == 1 && honor {
                v.push(f("混一色", 6));
            } else if nsuit == 0 {
                v.push(f("字一色", 64));
            }
            if nsuit == 2 {
                v.push(f("缺一门", 1));
            }
            if !honor {
                v.push(f("无字", 1));
            }
            // 断幺
            if (0..34usize).all(|t| {
                if all[t] == 0 {
                    return true;
                }
                !is_terminal_or_honor(t)
            }) {
                v.push(f("断幺", 2));
            }
        }
        Special::ThirteenOrphans => {
            // 十三幺不计 五门齐 / 门前清 / 单钓将
        }
        Special::BuKao => {
            // 全不靠：若含完整 147/258/369，加计组合龙
            for perm in PERMS.iter() {
                if knit_tiles(*perm).iter().all(|&t| all[t] > 0) {
                    v.push(f("组合龙", 12));
                    break;
                }
            }
        }
        Special::SevenStars => {
            // 七星不靠 不计 五门齐 / 门前清
        }
        Special::Zuhelong => {}
    }

    // 五门齐（组合龙手常见）
    {
        let suits = suits_present(all);
        let nsuit = suits.iter().filter(|&&x| x).count();
        if nsuit == 3
            && (0..34usize).any(|t| all[t] > 0 && is_wind(t))
            && (0..34usize).any(|t| all[t] > 0 && is_dragon(t))
        {
            v.push(f("五门齐", 6));
        }
    }

    apply_exclusions(&mut v, ctx);
    v
}

fn collect_standard(ctx: &WinCtx) -> Vec<Fan> {
    let mut v: Vec<Fan> = Vec::new();
    let all = ctx.all;
    let d = ctx.decomp;
    let pair = d.pair.unwrap();
    let runs = runs_of(d);
    let trips = trips_of(d);
    let suits = suits_present(all);
    let nsuit = suits.iter().filter(|&&x| x).count();
    let honor = has_honor(all);

    let wind_tri = trips.iter().filter(|&&t| is_wind(t)).count();
    let dragon_tri = trips.iter().filter(|&&t| is_dragon(t)).count();

    // ---------- 字牌系列 ----------
    let mut big4 = false;
    if wind_tri == 4 {
        v.push(f("大四喜", 88));
        big4 = true;
    }
    if dragon_tri == 3 {
        v.push(f("大三元", 88));
    }
    if wind_tri == 3 && is_wind(pair) {
        v.push(f("小四喜", 64));
    }
    if dragon_tri == 2 && is_dragon(pair) {
        v.push(f("小三元", 64));
    }

    // ---------- 花色色系 ----------
    let mut qingyise = false;
    let mut ziyise = false;
    if nsuit == 0 {
        v.push(f("字一色", 64));
        ziyise = true;
    } else if nsuit == 1 && !honor {
        v.push(f("清一色", 24));
        qingyise = true;
    } else if nsuit == 1 && honor {
        v.push(f("混一色", 6));
    }

    // 绿一色
    if (0..34usize).all(|t| all[t] == 0 || is_green(t)) {
        v.push(f("绿一色", 88));
    }

    // 清幺九 / 混幺九
    let all_yaojiu = (0..34usize).all(|t| all[t] == 0 || is_terminal_or_honor(t));
    let all_terminals = (0..34usize).all(|t| all[t] == 0 || is_terminal(t));
    if all_terminals {
        v.push(f("清幺九", 64));
    } else if all_yaojiu {
        v.push(f("混幺九", 32));
    }

    // 九莲宝灯（单花色 + 1112345678999 + 任意一张）
    for s in 0..3usize {
        let b = s * 9;
        let mut others = 0u32;
        for i in 0..34usize {
            if !(i >= b && i < b + 9) {
                others += all[i] as u32;
            }
        }
        if others == 0
            && all[b] >= 3
            && all[b + 8] >= 3
            && (1..=7).all(|i| all[b + i] >= 1)
            && total(all) == 14
        {
            v.push(f("九莲宝灯", 88));
            break;
        }
    }

    // ---------- 刻子/杠 系列 ----------
    let kan_cnt = d.sets.iter().filter(|s| s.is_kan).count();
    let ankan = d.sets.iter().filter(|s| s.is_kan && !s.open).count();
    let mingkan = d.sets.iter().filter(|s| s.is_kan && s.open).count();

    if kan_cnt >= 4 {
        v.push(f("四杠", 88));
    } else if kan_cnt == 3 {
        v.push(f("三杠", 32));
    } else if kan_cnt == 2 {
        if ankan == 2 {
            v.push(f("双暗杠", 6));
        } else if mingkan == 2 {
            v.push(f("双明杠", 4));
        } else {
            // 一明一暗杠：98 规则未列，现行网络国标普遍计 5 番
            v.push(f("一明一暗杠", 5));
        }
    } else if kan_cnt == 1 {
        if ankan == 1 {
            v.push(f("暗杠", 2));
        } else {
            v.push(f("明杠", 1));
        }
    }

    // 碰碰和
    let all_trips = d.sets.iter().all(|s| s.is_triplet());
    if all_trips {
        v.push(f("碰碰和", 6));
    }

    // 暗刻统计（手中的刻子 + 暗杠）；荣和补成的刻子算明刻
    let concealed_trips = d
        .sets
        .iter()
        .filter(|s| {
            if !s.is_triplet() {
                return false;
            }
            let is_conc = !s.open || (s.is_kan && !s.open);
            if !is_conc {
                return false;
            }
            if !ctx.tsumo && !s.is_kan && s.tile == ctx.win_tile {
                return false; // 荣和补成的刻子→明刻
            }
            true
        })
        .count();
    if concealed_trips == 4 {
        v.push(f("四暗刻", 64));
    } else if concealed_trips == 3 {
        v.push(f("三暗刻", 16));
    } else if concealed_trips == 2 {
        v.push(f("双暗刻", 2));
    }

    // 箭刻 / 风刻 / 幺九刻
    for &t in trips.iter() {
        if is_dragon(t) {
            if dragon_tri == 1 {
                v.push(f("箭刻", 2));
            }
        } else if is_wind(t) {
            if t == ctx.round_wind {
                v.push(f("圈风刻", 2));
            }
            if t == ctx.seat_wind {
                v.push(f("门风刻", 2));
            }
            if t != ctx.round_wind && t != ctx.seat_wind {
                v.push(f("幺九刻", 1));
            }
        } else if is_terminal(t) {
            v.push(f("幺九刻", 1));
        }
    }
    if wind_tri == 3 && !is_wind(pair) && !big4 {
        v.push(f("三风刻", 12));
    }

    // 双箭刻
    if dragon_tri == 2 {
        v.push(f("双箭刻", 6));
    }

    // 双同刻 / 三同刻
    {
        let mut by_num: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        for &t in trips.iter() {
            if is_suited(t) {
                *by_num.entry(num(t)).or_insert(0) += 1;
            }
        }
        let mut san = false;
        for (_, &c) in by_num.iter() {
            if c >= 3 {
                san = true;
            }
        }
        if san {
            v.push(f("三同刻", 16));
        } else if by_num.values().any(|&c| c == 2) {
            v.push(f("双同刻", 2));
        }
    }

    // ---------- 顺子系列 ----------
    let mut used_run_fan = false;

    // 一色四同顺 / 一色三同顺 / 一般高
    {
        let mut best = 1usize; // 同花色相同顺子的最大重复数
        for s in 0..3usize {
            let mut cnt: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
            for &t in runs.iter() {
                if suit(t) == s {
                    *cnt.entry(t).or_insert(0) += 1;
                }
            }
            for (_, &c) in cnt.iter() {
                if c > best {
                    best = c;
                }
            }
        }
        if best == 4 {
            v.push(f("一色四同顺", 48));
            used_run_fan = true;
        } else if best == 3 {
            v.push(f("一色三同顺", 24));
            used_run_fan = true;
        }
    }

    // 一色四节高 / 一色三节高
    {
        let mut best = 1usize;
        for s in 0..3usize {
            let mut nums: Vec<usize> = trips.iter().filter(|&&t| suit(t) == s).map(|&t| num(t)).collect();
            nums.sort();
            nums.dedup();
            // 最长连续（步长1）
            let mut cur = 1usize;
            for w in nums.windows(2) {
                if w[1] == w[0] + 1 {
                    cur += 1;
                    best = best.max(cur);
                } else {
                    cur = 1;
                }
            }
        }
        if best == 4 {
            v.push(f("一色四节高", 48));
        } else if best == 3 {
            v.push(f("一色三节高", 24));
        }
    }

    // 清龙 / 花龙
    {
        let mut qinglong = false;
        for s in 0..3usize {
            let st: std::collections::HashSet<usize> =
                runs.iter().filter(|&&t| suit(t) == s).map(|&t| num(t)).collect();
            if st.contains(&1) && st.contains(&4) && st.contains(&7) {
                qinglong = true;
            }
        }
        if qinglong {
            v.push(f("清龙", 16));
        }
        // 花龙：三种花色分别有以 1/4/7 开头的顺子
        let mut huolong = false;
        'outer: for perm in [[1usize, 4, 7], [1, 7, 4], [4, 1, 7], [4, 7, 1], [7, 1, 4], [7, 4, 1]].iter() {
            let mut ok = true;
            for s in 0..3usize {
                let st: std::collections::HashSet<usize> =
                    runs.iter().filter(|&&t| suit(t) == s).map(|&t| num(t)).collect();
                if !st.contains(&perm[s]) {
                    ok = false;
                    break;
                }
            }
            if ok {
                huolong = true;
                break 'outer;
            }
        }
        if huolong {
            v.push(f("花龙", 8));
        }
    }

    // 一色三步高 / 一色四步高
    {
        let mut best = 1usize;
        for s in 0..3usize {
            let mut nums: Vec<usize> =
                runs.iter().filter(|&&t| suit(t) == s).map(|&t| num(t)).collect();
            nums.sort();
            nums.dedup();
            for step in [1usize, 2] {
                let mut cur = 1usize;
                for w in nums.windows(2) {
                    if w[1] == w[0] + step {
                        cur += 1;
                        best = best.max(cur);
                    } else {
                        cur = 1;
                    }
                }
            }
        }
        if best == 4 {
            v.push(f("一色四步高", 32));
        } else if best == 3 {
            v.push(f("一色三步高", 16));
        }
    }

    // 三色三同顺 / 三色三节高 / 三色三步高
    {
        // 三色三同顺：三种花色相同数字的顺子
        let mut found6 = false;
        for n in 1..=7usize {
            let mut cnt = 0;
            for s in 0..3usize {
                if runs.iter().any(|&t| suit(t) == s && num(t) == n) {
                    cnt += 1;
                }
            }
            if cnt == 3 {
                found6 = true;
            }
        }
        if found6 {
            v.push(f("三色三同顺", 8));
        }

        // 三色三节高：三种花色刻子，数字依次 +1
        let tnum: Vec<Vec<usize>> = (0..3usize)
            .map(|s| trips.iter().filter(|&&t| suit(t) == s).map(|&t| num(t)).collect())
            .collect();
        let mut found7 = false;
        for a in tnum[0].iter() {
            for b in tnum[1].iter() {
                for c in tnum[2].iter() {
                    let mut arr = [*a, *b, *c];
                    arr.sort();
                    if arr[1] == arr[0] + 1 && arr[2] == arr[1] + 1 {
                        found7 = true;
                    }
                }
            }
        }
        if found7 {
            v.push(f("三色三节高", 8));
        }

        // 三色三步高：三门顺子数字依次 +1
        let mut found8 = false;
        let nums_by_suit: Vec<Vec<usize>> = (0..3usize)
            .map(|s| {
                let mut vv: Vec<usize> =
                    runs.iter().filter(|&&t| suit(t) == s).map(|&t| num(t)).collect();
                vv.sort();
                vv.dedup();
                vv
            })
            .collect();
        'o8: for a in nums_by_suit[0].iter() {
            for b in nums_by_suit[1].iter() {
                for c in nums_by_suit[2].iter() {
                    let mut arr = [*a, *b, *c];
                    arr.sort();
                    if arr[1] == arr[0] + 1 && arr[2] == arr[1] + 1 {
                        found8 = true;
                        break 'o8;
                    }
                }
            }
        }
        if found8 {
            v.push(f("三色三步高", 6));
        }
    }

    // 一般高 / 喜相逢 / 连六 / 老少副：同一副顺子只能用一次（对顺子做最大匹配）
    {
        fn fan_between(a: usize, b: usize) -> Option<&'static str> {
            let (sa, na) = (suit(a), num(a));
            let (sb, nb) = (suit(b), num(b));
            if sa == sb && na == nb {
                Some("一般高")
            } else if sa != sb && na == nb {
                Some("喜相逢")
            } else if sa == sb && (na as i32 - nb as i32).abs() == 3 {
                Some("连六")
            } else if sa == sb && ((na == 1 && nb == 7) || (na == 7 && nb == 1)) {
                Some("老少副")
            } else {
                None
            }
        }
        fn rec(
            start: usize,
            used: &mut [bool],
            pairs: &mut Vec<(usize, usize)>,
            runs: &[usize],
            best: &mut usize,
            best_pairs: &mut Vec<(usize, usize)>,
        ) {
            let mut i = start;
            while i < runs.len() && used[i] {
                i += 1;
            }
            if i >= runs.len() {
                if pairs.len() > *best {
                    *best = pairs.len();
                    *best_pairs = pairs.clone();
                }
                return;
            }
            used[i] = true;
            rec(i + 1, used, pairs, runs, best, best_pairs);
            used[i] = false;
            for j in (i + 1)..runs.len() {
                if used[j] {
                    continue;
                }
                if fan_between(runs[i], runs[j]).is_some() {
                    used[i] = true;
                    used[j] = true;
                    pairs.push((i, j));
                    rec(i + 1, used, pairs, runs, best, best_pairs);
                    pairs.pop();
                    used[i] = false;
                    used[j] = false;
                }
            }
        }
        let mut used = vec![false; runs.len()];
        let mut best = 0usize;
        let mut best_pairs: Vec<(usize, usize)> = Vec::new();
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        rec(0, &mut used, &mut pairs, &runs, &mut best, &mut best_pairs);

        let mut counts: std::collections::HashMap<&'static str, u32> = std::collections::HashMap::new();
        for (i, j) in best_pairs.iter() {
            if let Some(n) = fan_between(runs[*i], runs[*j]) {
                *counts.entry(n).or_insert(0) += 1;
            }
        }
        for name in ["一般高", "喜相逢", "连六", "老少副"].iter() {
            if let Some(&c) = counts.get(name) {
                v.push(f(name, c));
            }
        }
    }

    // ---------- 特殊全带 ----------
    // 全带五
    let all_with_five = d.sets.iter().all(|s| match s.kind {
        SetKind::Run => num(s.tile) >= 3 && num(s.tile) <= 5,
        SetKind::Triplet => is_suited(s.tile) && num(s.tile) == 5,
    }) && is_suited(pair)
        && num(pair) == 5;
    if all_with_five {
        v.push(f("全带五", 16));
    }

    // 全带幺
    let all_with_yao = d.sets.iter().all(|s| match s.kind {
        SetKind::Run => {
            let n = num(s.tile);
            n == 1 || n == 7
        }
        SetKind::Triplet => is_terminal_or_honor(s.tile),
    }) && is_terminal_or_honor(pair);
    if all_with_yao {
        v.push(f("全带幺", 4));
    }

    // 全大 / 全中 / 全小
    let only_num = |lo: usize, hi: usize| {
        (0..34usize).all(|t| {
            all[t] == 0 || (is_suited(t) && num(t) >= lo && num(t) <= hi)
        })
    };
    if only_num(7, 9) {
        v.push(f("全大", 24));
    } else if only_num(4, 6) {
        v.push(f("全中", 24));
    } else if only_num(1, 3) {
        v.push(f("全小", 24));
    }

    // 大于五 / 小于五
    let gt5 = (0..34usize).all(|t| all[t] == 0 || (is_suited(t) && num(t) >= 6));
    let lt5 = (0..34usize).all(|t| all[t] == 0 || (is_suited(t) && num(t) <= 4));
    if gt5 {
        v.push(f("大于五", 12));
    }
    if lt5 {
        v.push(f("小于五", 12));
    }

    // 全双刻
    let all_even_trip = all_trips
        && d.sets.iter().all(|s| is_suited(s.tile) && num(s.tile) % 2 == 0)
        && is_suited(pair)
        && num(pair) % 2 == 0;
    if all_even_trip {
        v.push(f("全双刻", 24));
    }

    // 五门齐：三花色 + 风 + 箭 齐
    if nsuit == 3
        && (0..34usize).any(|t| all[t] > 0 && is_wind(t))
        && (0..34usize).any(|t| all[t] > 0 && is_dragon(t))
    {
        v.push(f("五门齐", 6));
    }

    // 平和：4 顺子 + 数牌将
    if d.sets.iter().all(|s| s.is_run()) && is_suited(pair) {
        v.push(f("平和", 2));
    }

    // 断幺
    if (0..34usize).all(|t| all[t] == 0 || !is_terminal_or_honor(t)) {
        v.push(f("断幺", 2));
    }

    // 缺一门
    if nsuit == 2 {
        v.push(f("缺一门", 1));
    }

    // 无字
    if !honor {
        v.push(f("无字", 1));
    }
    let _ = (qingyise, ziyise, used_run_fan);

    // 四归一：某张牌 4 张且未成杠
    for t in 0..34usize {
        if all[t] == 4 && !d.sets.iter().any(|s| s.is_kan && s.tile == t) {
            v.push(f("四归一", 2));
        }
    }

    // ---------- 推不倒 ----------
    {
        const TBD: [usize; 14] = [10, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 25, 26, 33];
        if (0..34usize).all(|t| all[t] == 0 || TBD.contains(&t)) {
            v.push(f("推不倒", 8));
        }
    }

    // ---------- 一色双龙会 / 三色双龙会 ----------
    if d.sets.len() == 4 && d.sets.iter().all(|s| s.is_run()) {
        if is_suited(pair) && num(pair) == 5 {
            let ps = suit(pair);
            // 一色双龙会：同一花色 123,123,789,789 + 5 作将
            let same = runs.iter().all(|&t| suit(t) == ps);
            let c1 = runs.iter().filter(|&&t| num(t) == 1).count();
            let c7 = runs.iter().filter(|&&t| num(t) == 7).count();
            if same && c1 == 2 && c7 == 2 {
                v.push(f("一色双龙会", 64));
            }
            // 三色双龙会：两花色各一老少副，第三花色 5 作将
            let mut full = 0;
            for s in 0..3usize {
                if s == ps {
                    continue;
                }
                let has1 = runs.iter().any(|&t| suit(t) == s && num(t) == 1);
                let has7 = runs.iter().any(|&t| suit(t) == s && num(t) == 7);
                if has1 && has7 {
                    full += 1;
                }
            }
            if full == 2 && runs.iter().all(|&t| suit(t) != ps) {
                v.push(f("三色双龙会", 16));
            }
        }
    }

    // ---------- 和牌张结构：边张 / 坎张 / 单钓将 ----------
    if ctx.single_wait {
        if pair == ctx.win_tile {
            v.push(f("单钓将", 1));
        } else {
            // 找出含和牌张的顺子
            for s in d.sets.iter() {
                if s.is_run() && (s.tile == ctx.win_tile || s.tile + 1 == ctx.win_tile || s.tile + 2 == ctx.win_tile) {
                    let n = num(s.tile);
                    if ctx.win_tile == s.tile + 2 && n == 1 {
                        v.push(f("边张", 1));
                    } else if ctx.win_tile == s.tile && n == 7 {
                        v.push(f("边张", 1));
                    } else if ctx.win_tile == s.tile + 1 {
                        v.push(f("坎张", 1));
                    }
                    break;
                }
            }
        }
    }

    apply_exclusions(&mut v, ctx);
    v
}

/// 结构性互斥：去掉被高阶番种"吃掉"的低阶番
fn apply_exclusions(v: &mut Vec<Fan>, _ctx: &WinCtx) {
    let has = |v: &Vec<Fan>, n: &str| v.iter().any(|x| x.name == n);
    let mut drop: Vec<&'static str> = Vec::new();

    if has(v, "大四喜") {
        drop.extend(["三风刻", "碰碰和", "圈风刻", "门风刻", "幺九刻"]);
    }
    if has(v, "大三元") {
        drop.extend(["双箭刻", "箭刻"]);
    }
    if has(v, "绿一色") {
        drop.extend(["混一色"]);
    }
    if has(v, "九莲宝灯") {
        drop.extend(["清一色", "幺九刻"]);
    }
    if has(v, "四杠") {
        drop.extend([
            "碰碰和",
            "单钓将",
            "三杠",
            "双暗杠",
            "双明杠",
            "一明一暗杠",
            "暗杠",
            "明杠",
        ]);
    }
    if has(v, "三杠") {
        drop.extend(["双暗杠", "双明杠", "一明一暗杠", "暗杠", "明杠"]);
    }
    if has(v, "连七对") {
        drop.extend(["清一色", "七对", "单钓将"]);
    }
    if has(v, "十三幺") {
        drop.extend(["五门齐", "单钓将"]);
    }
    if has(v, "清幺九") {
        drop.extend(["碰碰和", "全带幺", "幺九刻", "无字", "双同刻", "三同刻"]);
    }
    if has(v, "混幺九") {
        drop.extend(["碰碰和", "全带幺", "幺九刻"]);
    }
    if has(v, "字一色") {
        drop.extend(["碰碰和", "全带幺", "幺九刻"]);
    }
    if has(v, "小四喜") {
        drop.extend(["三风刻", "幺九刻"]);
    }
    if has(v, "小三元") {
        drop.extend(["双箭刻", "箭刻"]);
    }
    if has(v, "四暗刻") {
        drop.extend(["碰碰和", "三暗刻", "双暗刻"]);
    }
    if has(v, "三暗刻") {
        drop.extend(["双暗刻"]);
    }
    if has(v, "一色双龙会") {
        drop.extend(["七对", "清一色", "平和", "一般高", "老少副", "无字"]);
    }
    if has(v, "一色四同顺") {
        drop.extend(["一色三节高", "一色三同顺", "七对", "四归一", "一般高"]);
    }
    if has(v, "一色四节高") {
        drop.extend(["一色三同顺", "碰碰和"]);
    }
    if has(v, "清龙") {
        drop.extend(["连六", "老少副"]);
    }
    if has(v, "一色四步高") {
        drop.extend(["连六", "老少副"]);
    }
    if has(v, "七对") {
        drop.push("单钓将");
    }
    if has(v, "七星不靠") {
        drop.push("五门齐");
    }
    if has(v, "全双刻") {
        drop.extend(["碰碰和", "断幺"]);
    }
    if has(v, "清一色") {
        drop.push("无字");
    }
    if has(v, "一色三同顺") {
        drop.push("一般高");
    }
    if has(v, "全大") || has(v, "全小") {
        drop.push("无字");
    }
    if has(v, "全中") {
        drop.extend(["断幺", "无字"]);
    }
    if has(v, "三色双龙会") {
        drop.extend(["平和", "无字", "喜相逢", "老少副"]);
    }
    if has(v, "全带五") {
        drop.extend(["断幺", "无字"]);
    }
    if has(v, "全不靠") {
        drop.push("五门齐");
    }
    if has(v, "推不倒") {
        drop.push("缺一门");
    }
    if has(v, "三色三同顺") {
        drop.push("喜相逢");
    }
    if has(v, "大于五") || has(v, "小于五") {
        drop.push("无字");
    }
    if has(v, "断幺") {
        drop.push("无字");
    }
    if has(v, "平和") {
        drop.push("无字");
    }
    if has(v, "全求人") {
        drop.push("单钓将");
    }

    v.retain(|x| !drop.contains(&x.name));
}

/// 计算某种和法下的总分：牌型番 + 和法番
pub fn score(ctx: &WinCtx, mode: WinMode) -> (u32, Vec<Fan>) {
    let mut c2 = *ctx;
    c2.tsumo = matches!(mode, WinMode::Tsumo | WinMode::TsumoLast);
    let mut fans = collect(&c2);

    let mut extra: Vec<Fan> = Vec::new();
    let menqing = ctx.menqing;
    // 定死门清的牌型不计「门前清」
    let no_menqing_fan = ["七对", "连七对", "十三幺", "九莲宝灯", "七星不靠", "全不靠", "四暗刻"]
        .iter()
        .any(|n| fans.iter().any(|x| x.name == *n));

    match mode {
        WinMode::Normal => {
            if menqing && !no_menqing_fan {
                extra.push(f("门前清", 2));
            }
        }
        WinMode::Tsumo => {
            if menqing {
                extra.push(f("不求人", 4));
            } else {
                extra.push(f("自摸", 1));
            }
        }
        WinMode::LastTile => {
            if menqing && !no_menqing_fan {
                extra.push(f("门前清", 2));
            }
            extra.push(f("和绝张", 4));
        }
        WinMode::TsumoLast => {
            if menqing {
                extra.push(f("不求人", 4));
            } else {
                extra.push(f("自摸", 1));
            }
            extra.push(f("和绝张", 4));
        }
    }

    // 全求人：四副明副露 + 单钓（荣和）
    if mode == WinMode::Normal
        && ctx.melds.len() == 4
        && ctx.melds.iter().all(|m| m.open)
    {
        extra.push(f("全求人", 6));
        fans.retain(|x| x.name != "单钓将");
    }

    let base: u32 = fans.iter().map(|x| x.value).sum();
    let mut total = base + extra.iter().map(|x| x.value).sum::<u32>();

    // 无番和：仅点炮、非门清、且无任何番
    if mode == WinMode::Normal && base == 0 && !menqing && total == 0 {
        total = 8;
        extra.push(f("无番和", 8));
    }

    fans.extend(extra);
    (total, fans)
}
