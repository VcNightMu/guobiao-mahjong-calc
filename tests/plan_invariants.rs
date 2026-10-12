//! T69（=T61）：`--plan` 的 `distance` / `need` / `drop` **全量不变量回归**。
//!
//! 本文件的不变量**先读码导出、后落为断言**（不许先猜后测）；语料 34 手，全部为规范张数
//! `13 - 3 x 副露数`，覆盖 0/1/2/3/4 副露、吃/碰/明杠/暗杠、以及 F2 的五个正例手。
//!
//! 语料裁剪说明（Lead 要求目标测试 ≤ 12 s，开发者体验）：从首版 45 手裁到 34 手，剔除的都是
//! debug 下秒级的纯色大计算手（`1112345678999m` / `1122334455667m` / `1112223334445m` /
//! `123m456m789m 11m22m` / `111m222m333m44m55m` 等）；**必留样本一个不少**：F2 五个正例手 +
//! 两个反例手、`GUARD75`/`M1`/`M2`/`R02`、T3-06 四杠手，0/1/2/3/4 副露各至少 1 手。
//! 不变量验证不依赖极端规模 ⇒ 裁剪不影响 A~F 的覆盖面。
//!
//! ## A. `distance == drop.len()`
//! 口径：**`distance` = 需要换掉的暗牌张数**（不是 `need` 张数、也不是 `need - drop`）。
//! 依据：`src/plan.rs` 的 `push_dir` 用 `dist = total(hand).saturating_sub(cov)`；而 `cov` 在搜索里
//! 由 `rec` 累加 `gain()`，`gain(hand, cur, t, k) = min(hand[t], cur[t] + k) - min(hand[t], cur[t])`
//! （见 `gain` 函数体）⇒ 累加结果恒等于 `cover(hand, target) = sum(min(hand[t], target[t]))`
//! （见 `cover` 函数体，特殊型番种走同一函数）⇒ `dist = sum(max(0, hand[t] - target[t]))`，
//! 正是 `push_dir` 里 `drop` 的构造式 `hand[t].saturating_sub(target[t])` 的总张数。
//!
//! ## B. `need.len() - drop.len() == 14 - 3 * melds.len() - total(hand)`
//! 口径：目标牌型恒为 `3 * (4 - 副露数) + 2` 张（4 面子 + 1 将，副露占其中的面子）。
//! 依据：`best_forced` 的 `used = 4 - melds.len()` 与 `left0 = used - hand_forced`；`七对` / `连七对` /
//! `十三幺` 入口均要求 `melds.is_empty()`；`组合龙` 仅在 `melds.len() <= 1` 时生成，且 `melded` 时
//! 第 4 面子由副露承载（目标暗牌部分少 3 张）；`全带五` 用 `left = 4 - melds.len()`。
//! ⇒ 对规范张数手牌（本语料全部满足 `total(hand) == 13 - 3 * m`），该式恒等于 **1**。
//!
//! ## C. 同一张牌不得同时出现在 `need` 与 `drop`
//! 依据：`push_dir` 内 `drop` 由 `hand[t].saturating_sub(target[t])`、`need` 由
//! `target[t].saturating_sub(hand[t])` 生成，两者互补 ⇒ 任一张牌至多一侧非空。
//!
//! ## D. 物理可达性（T3-06）：对每张 `need` 的牌 `t`，
//! `副露已占(t) + hand[t] + need(t) <= 4`
//! 依据：`target_feasible(target, melds)` 要求 `target[t] + mpt[t] <= 4`（`mpt` 由 `Meld::tiles()` 统计，
//! **杠算 4 张**）；而 `target[t] = hand[t] - drop[t] + need[t]`，C 保证 `need[t] > 0` 时
//! `drop[t] == 0` ⇒ 即本式。
//!
//! ## E. `distance <= max_dist`（本语料固定 2）
//! 依据：`push_dir` 入口 `if dist > max_dist { return; }`。
//!
//! ## F. 每种牌的 `drop` 张数 `<=` 该牌的暗牌张数
//! 依据：`drop` 用 `hand[t].saturating_sub(target[t])` 生成 ⇒ `|drop(t)| <= hand[t]`。
//!
//! 不在不变量内：`covered_by`（R-13 覆盖标注）只做标注，不改 `distance/need/drop/value`。
//!
//! 约定（与 `tests/plan_regression.rs` 同）：`plan(..., 2)` 的字面量 `2` 是契约护栏，
//! `MAX_DIST` 一旦变动，本文件的 `distance <= 2` 断言必须立刻失败报警。

use gbmj::model::Meld;
use gbmj::plan::{plan, Direction};
use gbmj::tiles::*;

/// 单个副露 token 的首张牌（与 CLI 同口径：吃取顺子首张、碰/杠取该张）。
fn first_tile_of(token: &str) -> usize {
    let c = parse_hand(token);
    (0..NUM_TILES)
        .find(|&t| c[t] > 0)
        .unwrap_or_else(|| panic!("非法副露 token: {token}"))
}

fn chi_of(token: &str) -> Meld {
    Meld::chi(first_tile_of(token))
}

fn pon_of(token: &str) -> Meld {
    Meld::pon(first_tile_of(token))
}

fn kan_of(token: &str, open: bool) -> Meld {
    Meld::kan(first_tile_of(token), open)
}

/// 语料：34 手规范张数手牌（含 F2 五个正例手；裁剪依据见文件头）。
fn corpus() -> Vec<(&'static str, Vec<Meld>)> {
    vec![
        // ---- 0 副露（12 手）----
        ("111m 999m 111s 99s 11p", vec![]),
        ("123m 456s 789p 11m 22s", vec![]),
        ("345m456s567p55m34s", vec![]),
        ("19m 19s 19p 东南西北中发白", vec![]),
        ("东东东 南南南 西西西 中中中 发", vec![]),
        ("147m 258s 369p 11m22p", vec![]),
        ("123456789m 11p 白白", vec![]),
        ("中中中发发发白白123m44m", vec![]),
        ("19m 19m 19m 258s 369p 5p", vec![]),
        ("南南南西西西北北北东东中中", vec![]),
        ("222m 222s 345m 99m 67s", vec![]),
        // R02（BASELINE 用例手）
        ("111m 23m 456m 999s 55p", vec![]),
        // ---- 1 副露（12 手，吃/碰/明杠/暗杠）----
        // F2 正例：大四喜（碰东 + 手内 南/西/北 三刻 + 中将）
        ("南南南西西西北北北中", vec![pon_of("东")]),
        // GUARD75（暗杠）
        ("111m999m中中中5p", vec![kan_of("1p", false)]),
        ("147m 258s 369p 5p", vec![kan_of("1p", true)]),
        ("147m 258s 369p 5p", vec![kan_of("1p", false)]),
        // F2 正例：三风刻（碰北 + 手内 南南南/西西西）
        ("南南南西西西99p55p", vec![pon_of("北")]),
        // F2 正例：三同刻（碰1筒 + 手内 111m/111s）
        ("111m111s99p55p", vec![pon_of("1p")]),
        // F2 反例：明碰不得充当暗刻源
        ("222m333m99s5p5p", vec![pon_of("1m")]),
        // F2 形状反例：吃 123m 不得充当 111m 刻子
        ("111s111p99m55m", vec![chi_of("123m")]),
        ("123456789m 1s", vec![chi_of("7p")]),
        ("78s 东东 456m 中中中", vec![pon_of("5p")]),
        ("111222333m 4p", vec![pon_of("1s")]),
        ("12345678m 11p", vec![pon_of("白")]),
        // ---- 2 副露（3 手）----
        // F2 正例：大三元（碰中 + 碰发 + 手内 白白白）
        ("白白白东东5m5m", vec![pon_of("中"), pon_of("发")]),
        ("1234567m", vec![pon_of("1p"), pon_of("9p")]),
        ("5p 中中中 东东 1m", vec![pon_of("南"), pon_of("北")]),
        // ---- 3 副露（4 手）----
        ("78s 东东", vec![chi_of("234m"), pon_of("5p"), pon_of("6s")]),
        (
            "5m 白白白",
            vec![pon_of("中"), pon_of("发"), chi_of("123m")],
        ),
        ("7p 东东 中", vec![pon_of("1m"), pon_of("9m"), pon_of("1s")]),
        (
            "5s 中中中",
            vec![
                kan_of("2m", false),
                kan_of("3m", false),
                kan_of("4m", false),
            ],
        ),
        // ---- 4 副露（3 手）----
        // F2 正例：一色双龙会（4 副明吃 + 5 万作将）
        (
            "5m",
            vec![
                chi_of("123m"),
                chi_of("123m"),
                chi_of("789m"),
                chi_of("789m"),
            ],
        ),
        // T3-06 四杠手
        (
            "5p",
            vec![
                kan_of("1m", true),
                kan_of("9m", true),
                kan_of("1s", true),
                kan_of("9s", true),
            ],
        ),
        (
            "5p",
            vec![chi_of("123s"), chi_of("456s"), chi_of("789s"), pon_of("白")],
        ),
    ]
}

/// 对一条方向逐条断言 A~F。
fn assert_direction_invariants(
    hand: &str,
    h: &Counts,
    melds: &[Meld],
    meld_used: &[u8; NUM_TILES],
    d: &Direction,
) {
    // A：distance == drop 张数
    assert_eq!(
        d.distance as usize,
        d.drop.len(),
        "[{hand}] A 违例：{} distance={} 应等于 drop 张数={}",
        d.name,
        d.distance,
        d.drop.len()
    );
    // B：need - drop == 目标张数 - 暗牌张数
    let lhs = d.need.len() as i64 - d.drop.len() as i64;
    let rhs = 14 - 3 * melds.len() as i64 - total(h) as i64;
    assert_eq!(
        lhs,
        rhs,
        "[{hand}] B 违例：{} need={} drop={}（期望差 {rhs}）",
        d.name,
        d.need.len(),
        d.drop.len()
    );
    // E：distance <= max_dist
    assert!(
        d.distance <= 2,
        "[{hand}] E 违例：{} distance={} > 2",
        d.name,
        d.distance
    );

    let mut need_cnt = [0u8; NUM_TILES];
    for &t in d.need.iter() {
        need_cnt[t] += 1;
    }
    let mut drop_cnt = [0u8; NUM_TILES];
    for &t in d.drop.iter() {
        drop_cnt[t] += 1;
    }
    for (t, &hand_t) in h.iter().enumerate() {
        // C：need / drop 互斥
        assert!(
            need_cnt[t] == 0 || drop_cnt[t] == 0,
            "[{hand}] C 违例：{} 的第 {} 张同时出现在 need 与 drop",
            d.name,
            t
        );
        // F：drop 张数 <= 暗牌张数
        assert!(
            drop_cnt[t] <= hand_t,
            "[{hand}] F 违例：{} 对第 {} 张 drop={} > hand={}",
            d.name,
            t,
            drop_cnt[t],
            hand_t
        );
        // D：物理可达性（副露已占 + 暗牌 + need <= 4）
        if need_cnt[t] > 0 {
            let used = meld_used[t] as u32 + hand_t as u32 + need_cnt[t] as u32;
            assert!(
                used <= 4,
                "[{hand}] D 违例：{} 第 {} 张 副露{} + 暗牌{} + need{} = {} > 4",
                d.name,
                t,
                meld_used[t],
                hand_t,
                need_cnt[t],
                used
            );
        }
    }
}

/// T69：34 手语料、逐方向断言 A~F（`distance` / `need` / `drop` 全量不变量）。
#[test]
fn plan_direction_invariants_over_corpus() {
    let mut total_dirs = 0usize;
    for (hand, melds) in corpus() {
        let h = parse_hand(hand);
        assert_eq!(
            total(&h),
            13 - 3 * melds.len() as u32,
            "语料必须为规范张数（13 - 3 x 副露数）：{hand}"
        );
        let mut meld_used = [0u8; NUM_TILES];
        for m in melds.iter() {
            for t in m.tiles() {
                meld_used[t] = meld_used[t].saturating_add(1);
            }
        }
        let dirs = plan(&h, &melds, 2);
        for d in dirs.iter() {
            assert_direction_invariants(hand, &h, &melds, &meld_used, d);
            total_dirs += 1;
        }
    }
    // 语料有效性护栏：裁剪后实测 136 个方向（首版 45 手语料为 206）⇒ 阈值由 150 调低为 130
    // （保留约 4% 余量，仅用于「语料真的产出了方向」的自检，不锁定具体方向集合）。
    assert!(
        total_dirs >= 130,
        "语料产出的方向数过少（{total_dirs}），不变量覆盖不足"
    );
}
