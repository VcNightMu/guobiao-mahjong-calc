//! 第 5 批（R-08 方向扫描假阳性 + 镜像假阴性 + 同类扩展）的回归与 `--plan` 基线用例
//!
//! **刻意保留字面量**（非疏漏）：本文件 `plan(…, 2)` 的 `2` 与 `src/plan.rs` 的 `MAX_DIST = 2`
//! 同值，但**不改为常量引用**——`MAX_DIST` 一旦变动，快照族必须立刻失败报警（契约护栏）。
//!
//! 约定（依 Lead 工作指令）：
//! - **全部库级断言**（直接调 `gbmj::plan::plan()` 取 `Direction`），**不断言 CLI 文本**。
//! - 只**新增**，不修改既有 78 条用例（含 `tests/plan.rs` 的 16 条）。
//! - 基线快照断言（`assert_golden`）用于**长期护栏**：本次修复**无关**的方向必须逐字不变。
//!
//! 规则口径（B1/B3，供断言依据）：
//! - `distance` = 把手牌换成该番种和牌型**最少需替换的张数** = `total(hand) − 覆盖张数`；
//!   `差0张` 意为「和牌型已被暗牌覆盖满，只差最后一张和牌张」。
//! - **混一色**的必要条件有**两条**：① 至少 1 张字牌；② 至少 1 张该门数牌
//!   （二者都可由**固定副露**提供 —— 镜像用例）。
//! - **混幺九**要求至少 1 张字牌（否则是**清幺九**，二者在计番时互斥）。

use gbmj::model::Meld;
use gbmj::plan::{plan, Direction};
use gbmj::tiles::*;

// ==================== 番值常量（逐字对齐 `src/fans.rs` 计番 call site） ====================
// 每个常量对应 `src/fans.rs` 中一处 `v.push(f("<番名>", <值>))`；**同值不同番必须分开命名**，
// 禁止引入 `FAN_64` 之类的值合并别名（那样会丢掉番名语义）。
/// 混一色 6 番 —— `src/fans.rs`: `v.push(f("混一色", 6));`（七对分支 / 标准分支两处）
const FAN_HUN_YI_SE: u32 = 6;
/// 清一色 24 番 —— `src/fans.rs`: `v.push(f("清一色", 24));`（七对分支 / 标准分支两处）
const FAN_QING_YI_SE: u32 = 24;
/// 清幺九 64 番 —— `src/fans.rs`: `v.push(f("清幺九", 64));`（清幺九分支）
const FAN_QING_YAO_JIU: u32 = 64;
/// 字一色 64 番 —— `src/fans.rs`: `v.push(f("字一色", 64));`（七对分支 / 标准分支两处；与清幺九同值不同番）
const FAN_ZI_YI_SE: u32 = 64;
/// 混幺九 32 番 —— `src/fans.rs`: `v.push(f("混幺九", 32));`（混幺九分支，紧跟清幺九的 else-if 分支）
const FAN_HUN_YAO_JIU: u32 = 32;

fn find<'a>(ds: &'a [Direction], kw: &str) -> Option<&'a Direction> {
    ds.iter().find(|d| d.name == kw)
}

fn get<'a>(ds: &'a [Direction], kw: &str) -> &'a Direction {
    find(ds, kw).unwrap_or_else(|| panic!("应有方向 {}", kw))
}

fn names(v: &[usize]) -> String {
    v.iter()
        .map(|&t| tile_name(t))
        .collect::<Vec<_>>()
        .join(",")
}

/// 完整的「逐方向快照」：`名称|番数|距离|补|换掉|标签`
fn dump(ds: &[Direction]) -> Vec<String> {
    ds.iter()
        .map(|d| {
            format!(
                "{}|{}|{}|{}|{}|{}",
                d.name,
                d.value,
                d.distance,
                names(&d.need),
                names(&d.drop),
                d.tag
            )
        })
        .collect()
}

fn assert_golden(hand: &str, melds: Vec<Meld>, expected: &[&str]) {
    let h = parse_hand(hand);
    let got = dump(&plan(&h, &melds, 2));
    let exp: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    assert_eq!(got, exp, "手牌 {hand} 的 --plan 方向快照变化");
}

// ==================== R-08 反向锁定：必要条件下的距离不得被算成 0 ====================

/// **R-08 本体**：合计**零字牌**时，混一色必然至少需换入 1 张字牌 ⇒ `distance ≥ 1`。
/// 手 `123m456m789m 11m22m`（13 张万子）：修复前报 `差0张`（假阳性），修复后 = `差1张`，
/// 且 `补: 东 东`、`换掉: 9万`（把将牌换成字牌对）。
#[test]
fn plan_r08_zero_honor_mixed_suit_is_not_zero() {
    let h = parse_hand("123m456m789m 11m22m");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);

    let m = get(&ds, "混一色(万门)");
    assert!(m.distance >= 1, "零字牌手混一色不得为 0 张差: {:?}", m);
    assert_eq!(m.distance, 1, "应为 1 张差: {:?}", m);
    assert_eq!(
        (m.value, names(&m.need).as_str(), m.drop.len()),
        // value ⇒ 混一色番值；末位 1 = 换掉 1 张（drop 张数阈值，保持字面量）
        (FAN_HUN_YI_SE, "东,东", 1),
        "{:?}",
        m
    );

    // 同手「清一色」确实可以 0 张差（对照，说明并非把整手判远）
    let q = get(&ds, "清一色(万门)");
    assert_eq!(
        (q.value, q.distance, names(&q.need).as_str()),
        // value ⇒ 清一色番值；中间 0 = 距离阈值（口径③ 保持字面量）
        (FAN_QING_YI_SE, 0, "9万"),
        "{:?}",
        q
    );
}

/// **同类扩展 · 混幺九**：全幺九但**零字牌**时必须 ≥1（否则是清幺九）。
/// 修复前报 `混幺九 差0张`（假阳性），修复后 = `差2张`（补 9条 + 字牌对）。
#[test]
fn plan_hunyaojiu_requires_honor() {
    let h = parse_hand("111m 999m 111s 99s 11p");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);

    // 清幺九是真 0 张差（无字牌正是其特征）
    let q = get(&ds, "清幺九");
    // value ⇒ 清幺九番值；0 = 距离阈值（口径③ 保持字面量）
    assert_eq!((q.value, q.distance), (FAN_QING_YAO_JIU, 0), "{:?}", q);

    let m = get(&ds, "混幺九");
    assert!(m.distance >= 1, "零字牌手混幺九不得为 0 张差: {:?}", m);
    assert_eq!(m.distance, 2, "{:?}", m);
    assert_eq!(names(&m.need).as_str(), "9条,东,东", "{:?}", m);
}

/// **第三形态**：纯字牌手（有字牌但**无任何该门数牌**）⇒ 三门混一色**各差 1 张**，不得为 0。
/// 修复前三门**全部报 差0张**（假阳性）。
#[test]
fn plan_pure_honor_hand_mixed_suit_each_one_away() {
    let h = parse_hand("东东东 南南南 西西西 中中中 发");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);

    for (name, suit_first) in [("万", "1万"), ("条", "1条"), ("筒", "1筒")] {
        let d = get(&ds, &format!("混一色({}门)", name));
        assert!(d.distance >= 1, "纯字牌手 {} 不得为 0: {:?}", name, d);
        assert_eq!(d.distance, 1, "{}: {:?}", name, d);
        assert_eq!(
            names(&d.need),
            format!("{},{}", suit_first, suit_first),
            "{}",
            name
        );
    }
    // 字一色本身是真的 0 张差
    let z = get(&ds, "字一色");
    assert_eq!(
        (z.value, z.distance, names(&z.need).as_str()),
        // value ⇒ 字一色番值（与清幺九同为 64，命名分开）；0 = 距离阈值（口径③）
        (FAN_ZI_YI_SE, 0, "发"),
        "{:?}",
        z
    );
}

// ==================== 正向护栏：不得修过头 ====================

/// 含字牌手：混一色仍为 **0 张差**（且与清一色 2 张差形成对照）。
#[test]
fn plan_with_honor_mixed_suit_stays_zero() {
    let h = parse_hand("123m456m789m 11m 东东");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);

    let m = get(&ds, "混一色(万门)");
    assert_eq!(
        (m.value, m.distance, names(&m.need).as_str(), m.drop.len()),
        // value ⇒ 混一色番值；0 = 距离阈值（口径③）；末位 0 = 换掉 0 张（drop 张数阈值）
        (FAN_HUN_YI_SE, 0, "1万", 0),
        "{:?}",
        m
    );

    let q = get(&ds, "清一色(万门)");
    assert_eq!(
        (q.distance, names(&q.drop).as_str()),
        // 2 = 距离阈值（口径③ 保持字面量；本处只断言 distance，与番值无关）
        (2, "东,东"),
        "{:?}",
        q
    );
}

/// 含字牌的幺九手：混幺九仍为 0 张差（清幺九反需换掉字牌）。
#[test]
fn plan_hunyaojiu_with_honor_stays_zero() {
    let h = parse_hand("111m 999m 111s 99s 中中");
    let ds = plan(&h, &[], 2);
    let m = get(&ds, "混幺九");
    assert_eq!(
        (m.value, m.distance, names(&m.need).as_str()),
        // value ⇒ 混幺九番值；0 = 距离阈值（口径③ 保持字面量）
        (FAN_HUN_YAO_JIU, 0, "9条"),
        "{:?}",
        m
    );
    assert_eq!(get(&ds, "清幺九").distance, 2, "清幺九需换掉中中");
}

/// **镜像假阴性回归（副露供字牌）**：`--pon 1m` 已提供该门数牌，暗牌全是字牌 ⇒
/// 混一色(万门) 必须 **0 张差**（首次实现只看暗牌时误报 2 张）。
#[test]
fn plan_mirror_meld_supplies_honor() {
    let h = parse_hand("东东东南南南西西中中");
    assert_eq!(total(&h), 10);
    let melds = vec![Meld::pon(0)]; // 碰 1万
    let ds = plan(&h, &melds, 2);
    let m = get(&ds, "混一色(万门)");
    assert_eq!(
        (m.value, m.distance, names(&m.need).as_str(), m.drop.len()),
        // value ⇒ 混一色番值；0 = 距离阈值（口径③）；末位 0 = 换掉 0 张（drop 张数阈值）
        (FAN_HUN_YI_SE, 0, "西", 0),
        "{:?}",
        m
    );
}

/// **镜像假阴性回归（副露供该门数牌）**：`--pon 9m` 已提供该门数牌，暗牌含字牌 ⇒
/// 混一色(万门) 必须 **0 张差**。
#[test]
fn plan_mirror_meld_supplies_suit_tile() {
    let h = parse_hand("1234567m 东东 8m");
    assert_eq!(total(&h), 10);
    let melds = vec![Meld::pon(8)]; // 碰 9万
    let ds = plan(&h, &melds, 2);
    let m = get(&ds, "混一色(万门)");
    assert_eq!(
        (m.value, m.distance, names(&m.need).as_str(), m.drop.len()),
        // value ⇒ 混一色番值；0 = 距离阈值（口径③）；末位 0 = 换掉 0 张（drop 张数阈值）
        (FAN_HUN_YI_SE, 0, "3万", 0),
        "{:?}",
        m
    );
}

/// **口径记录（非缺陷）**：`23456789m 55p --pon 1m` 下 清一色 与 混一色 **同为 2 张差** ——
/// 混一色额外需要的字牌**可由将牌承担**（将本就要凑），故**不增加换牌数**。
/// 此条已由 Lead/开发纠正（我此前误判为「把 >2 报成 ≤2」的假阳性），此处以快照固化正确口径。
#[test]
fn plan_mixed_suit_same_distance_as_pure_suit_is_not_defect() {
    let h = parse_hand("23456789m 55p");
    assert_eq!(total(&h), 10);
    let melds = vec![Meld::pon(0)]; // 碰 1万
    let ds = plan(&h, &melds, 2);
    let q = get(&ds, "清一色(万门)");
    let m = get(&ds, "混一色(万门)");
    assert_eq!(q.distance, 2, "{:?}", q);
    assert_eq!(
        m.distance, 2,
        "混一色的字牌可由将牌承担 ⇒ 与清一色同距: {:?}",
        m
    );
    assert_eq!(names(&m.need).as_str(), "4万,东,东", "{:?}", m);
}

// ==================== 同类风险独立抽查：单类别 Filter 无同类假阳性 ====================

/// 逐张「单类别」过滤器（字一色 / 绿一色 / 全大 / 全双刻 / 推不倒 / 清幺九 / 混幺九）：
/// 对**明显不满足**的手牌**不得**出现在 2 张以内的方向里。
#[test]
fn plan_single_category_filters_have_no_false_positive() {
    // 全万、且含低数牌与 6/7 等，明显不是 字一色 / 绿一色 / 全大 / 推不倒 / 全双刻
    let h = parse_hand("123m456m789m 11m22m");
    let ds = plan(&h, &[], 2);
    for f in [
        "字一色",
        "绿一色",
        "全大",
        "全双刻",
        "推不倒",
        "清幺九",
        "混幺九",
    ] {
        assert!(
            find(&ds, f).is_none(),
            "{} 不应在 2 张内可达: {:?}",
            f,
            dump(&ds)
        );
    }

    // 零幺九、零字牌手：清幺九 / 混幺九 均不可达
    let h2 = parse_hand("23456789m 23456s");
    let ds2 = plan(&h2, &[], 2);
    for f in ["清幺九", "混幺九", "字一色"] {
        assert!(find(&ds2, f).is_none(), "{} 不应出现: {:?}", f, dump(&ds2));
    }
}

// ==================== `--plan` 回归基线（golden：无关方向必须逐字不变） ====================

/// 基线 1：`123m456m789m 11m22m` —— 覆盖 清一色/清龙/九莲宝灯(门清)/一色三步高/混一色/
/// 一色四步高/一色三同顺/一色三节高 的完整快照（含 R-08 修复后的混一色=1）。
#[test]
fn plan_golden_baseline_a() {
    assert_golden(
        "123m456m789m 11m22m",
        vec![],
        &[
            "清一色(万门)|24|0|9万||",
            "清龙(万门)|16|0|1万||",
            "九莲宝灯(万门)|88|1|9万,9万|2万|门清",
            "一色三步高(万门)|16|1|4万,6万|9万|",
            "混一色(万门)|6|1|东,东|9万|",
            "一色四步高(万门)|32|2|3万,5万,7万|2万,2万|",
            "一色三同顺(万门)|24|2|3万,3万,7万|8万,9万|",
            "一色三节高(万门)|24|2|3万,3万,7万|8万,9万|",
        ],
    );
}

/// 基线 2：含字牌手（既有 `tests/plan.rs::qing_yi_se_two_away` 的同手牌）——
/// `混一色(万门)` 仍为 0 张差（防修复过头），`清一色(万门)` 2 张差。
#[test]
fn plan_golden_baseline_with_honor() {
    assert_golden(
        "11123456789m东东",
        vec![],
        &[
            "清龙(万门)|16|0|1万||",
            "混一色(万门)|6|0|1万||",
            "一色三步高(万门)|16|1|4万,6万|9万|",
            "九莲宝灯(万门)|88|2|1万,9万,9万|东,东|门清",
            "一色四步高(万门)|32|2|3万,5万,7万|东,东|",
            "清一色(万门)|24|2|1万,3万,9万|东,东|",
        ],
    );
}

/// 基线 3：与本次修复**无关**的三手牌，方向逐字不变（覆盖 花龙/三色三步高/组合龙/
/// 全带五/三色三同顺/十三幺）。
#[test]
fn plan_golden_baseline_unrelated_patterns() {
    assert_golden("123m 456s 789p 11m 22s", vec![], &["花龙|8|0|1万||"]);
    assert_golden(
        "147m 258s 369p 147p 东",
        vec![],
        &["组合龙|12|2|1筒,1筒,4筒|7筒,东|"],
    );
    assert_golden(
        "234m 567m 345s 678s 5p",
        vec![],
        &[
            "全带五|16|2|5万,5条,5筒|2万,8条|",
            "三色三同顺|8|2|2万,3筒,4筒|6万,7万|",
            "三色三步高|6|2|6条,4筒,6筒|7条,8条|",
        ],
    );
    assert_golden(
        "19m 19s 19p 东南西北中发白",
        vec![],
        &["十三幺|88|0|1万||门清"],
    );
}

/// 基线 4：纯字牌手（T2）与两个镜像（副露供字牌 / 供该门数牌）的完整快照。
#[test]
fn plan_golden_baseline_honor_and_mirrors() {
    assert_golden(
        "东东东 南南南 西西西 中中中 发",
        vec![],
        &[
            "字一色|64|0|发||",
            "四暗刻|64|0|发||",
            "混幺九|32|0|发||",
            "三暗刻|16|0|发||",
            "三风刻|12|0|发||",
            "碰碰和|6|0|发||",
            "小四喜|64|1|北,北|发|",
            "混一色(万门)|6|1|1万,1万|发|",
            "混一色(条门)|6|1|1条,1条|发|",
            "混一色(筒门)|6|1|1筒,1筒|发|",
            "双箭刻|6|1|发,发|西|",
            "大四喜|88|2|北,北,北|中,发|",
        ],
    );
    assert_golden(
        "东东东南南南西西中中",
        vec![Meld::pon(0)],
        &[
            "混幺九|32|0|西||",
            "三风刻|12|0|西||",
            "混一色(万门)|6|0|西||",
            "碰碰和|6|0|西||",
            "小四喜|64|2|北,北,北|中,中|",
        ],
    );
    assert_golden(
        "1234567m 东东 8m",
        vec![Meld::pon(8)],
        &[
            "清龙(万门)|16|0|9万||",
            "混一色(万门)|6|0|3万||",
            "一色三步高(万门)|16|1|3万,5万|8万|",
            "清一色(万门)|24|2|1万,1万,8万|东,东|",
        ],
    );
}
// ==================== T3-06：目标物理可达性（暗牌 ± drop/need + 副露已占 ≤ 4） ====================

/// T3-06：`--plan` 的每个方向，其目标在「暗牌 ± drop/need + 副露已占」下每张 ≤4。
fn assert_targets_feasible(hand: &str, melds: &[Meld], dirs: &[Direction]) {
    let h = parse_hand(hand);
    let mut mpt = [0u8; 34];
    for m in melds {
        for t in m.tiles() {
            mpt[t] = mpt[t].saturating_add(1);
        }
    }
    for d in dirs {
        for t in 0..34usize {
            let dropped = d.drop.iter().filter(|&&x| x == t).count() as u8;
            let needed = d.need.iter().filter(|&&x| x == t).count() as u8;
            let target = h[t].saturating_sub(dropped).saturating_add(needed);
            assert!(
                // 4 = 每种牌的物理上限（与 src/plan.rs 的同一不变量同源）⇒ 按 v0.2.0 口径保持字面量，测试单方常量化会造成「测试有常量、src 没有」的不同源
                target.saturating_add(mpt[t]) <= 4,
                "方向 {} 目标在 {} 需 {} 张（>4）",
                d.name,
                tile_name(t),
                target
            );
        }
    }
}

#[test]
fn plan_targets_are_physically_reachable_with_kans() {
    let hand = "5p";
    let melds = vec![
        Meld::kan(0, true),
        Meld::kan(8, true),
        Meld::kan(9, true),
        Meld::kan(17, true),
    ];
    let ds = plan(&parse_hand(hand), &melds, 2);
    assert_targets_feasible(hand, &melds, &ds);
    if let Some(d) = find(&ds, "清幺九") {
        assert!(
            !names(&d.need).contains("1万"),
            "清幺九 不得要求已被杠占满的 1万：{:?}",
            d
        );
    }
}

#[test]
fn plan_targets_never_exceed_four_copies_without_melds() {
    let hand = "1111m 222m 333m 44m 5m";
    let ds = plan(&parse_hand(hand), &[], 2);
    assert_targets_feasible(hand, &[], &ds);
    for d in &ds {
        assert!(
            !names(&d.need).contains("1万 1万"),
            "不得给出需 6 张 1万 的目标：{:?}",
            d
        );
    }
}

#[test]
fn plan_keeps_reachable_qingyaojiu_direction() {
    let hand = "111m 999m 111s 99s 中中";
    let ds = plan(&parse_hand(hand), &[], 2);
    let m = get(&ds, "清幺九");
    assert_eq!(m.distance, 2, "可达方向不得被误杀：{:?}", m);
    assert_targets_feasible(hand, &[], &ds);
}
