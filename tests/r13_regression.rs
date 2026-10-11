//! 第 6 批（R-13 覆盖标注）回归用例
//!
//! 约定（依 Lead 工作指令）：
//! - **全部库级断言**（`plan()` 取 `Direction.covered_by` / `distance`），**不断言 CLI 文本**。
//! - 只**新增**，不修改既有 91 条用例。
//!
//! 规则口径（用户确认的 **A 口径**）：
//! - **coverer 已达成（`distance == 0`）** 即标注；**不要求被覆盖方向也达成**（`混幺九 差2` 被 `清幺九 差0` 标注即为明证）。
//! - 标注**只加标注，不隐藏、不改距离**。
//! - 覆盖对来源 = `src/fans.rs::coverage_pairs()`（= 计番 `EXCLUSIONS` ∪ 分支 `BRANCH_EXCLUSIONS`），**非手工枚举**。
//! - 同门规则：双方都有门后缀须相同；一方无后缀视为通配。

use gbmj::fans::{coverage_pairs, BRANCH_EXCLUSIONS, EXCLUSIONS};
use gbmj::plan::{plan, Direction};
use gbmj::tiles::*;

fn find<'a>(ds: &'a [Direction], kw: &str) -> Option<&'a Direction> {
    ds.iter().find(|d| d.name == kw)
}

fn get<'a>(ds: &'a [Direction], kw: &str) -> &'a Direction {
    find(ds, kw).unwrap_or_else(|| panic!("应有方向 {}", kw))
}

fn coverers(d: &Direction) -> Vec<String> {
    let mut v = d.covered_by.clone();
    v.sort();
    v
}

/// 反向锁定 ★新增 1：**清幺九 → 混幺九**（分支互斥表来源）。
/// 注意被覆盖方向 `混幺九` 为 **差2张**（A 口径：coverer 达成即可标注，被覆盖方向不必达成）。
#[test]
fn r13_qingyaojiu_covers_hunyaojiu_even_when_far() {
    let h = parse_hand("111m 999m 111s 99s 11p");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);

    let q = get(&ds, "清幺九");
    assert_eq!(q.distance, 0, "{:?}", q);
    assert!(q.covered_by.is_empty(), "coverer 自身不应被标注: {:?}", q);

    let m = get(&ds, "混幺九");
    assert_eq!(m.distance, 2, "被覆盖方向不必为 0: {:?}", m);
    assert_eq!(coverers(m), vec!["清幺九".to_string()], "{:?}", m);

    // 同一 coverer 也覆盖 碰碰和 / 三同刻（drop 表条目）
    assert_eq!(coverers(get(&ds, "碰碰和")), vec!["清幺九".to_string()]);
    assert_eq!(coverers(get(&ds, "三同刻")), vec!["清幺九".to_string()]);
    // 三暗刻 不在 清幺九 的 drop 名单内 ⇒ 不标注
    assert!(get(&ds, "三暗刻").covered_by.is_empty());
}

/// 反向锁定 ★新增 2：**一色四节高 → 一色三节高**（分支互斥表来源，`best==4/==3`）。
#[test]
fn r13_yise_sijiegao_covers_sanjiegao() {
    let h = parse_hand("111m222m333m444m5m");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);

    let c = get(&ds, "一色四节高(万门)");
    assert_eq!(c.distance, 0, "{:?}", c);
    assert!(c.covered_by.is_empty());

    let d1 = get(&ds, "一色三节高(万门)");
    assert_eq!(coverers(d1), vec!["一色四节高(万门)".to_string()], "{:?}", d1);
    let d2 = get(&ds, "一色三同顺(万门)");
    assert_eq!(coverers(d2), vec!["一色四节高(万门)".to_string()], "{:?}", d2);

    // 四暗刻（差0）亦覆盖 三暗刻 / 碰碰和；碰碰和 为多覆盖者
    assert_eq!(coverers(get(&ds, "三暗刻")), vec!["四暗刻".to_string()]);
    let mut pk = coverers(get(&ds, "碰碰和"));
    pk.retain(|x| x != "一色四节高(万门)");
    assert_eq!(pk, vec!["四暗刻".to_string()], "碰碰和 应含 四暗刻 与 一色四节高(万门) 两个覆盖者");
}

/// 反向锁定 ★新增 3：**一色四步高 → 一色三步高**（分支互斥表来源）。
#[test]
fn r13_yise_sibugao_covers_sanbugao() {
    let h = parse_hand("123m234m345m456m5m");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);

    assert_eq!(get(&ds, "一色四步高(万门)").distance, 0);
    let d = get(&ds, "一色三步高(万门)");
    assert_eq!(coverers(d), vec!["一色四步高(万门)".to_string()], "{:?}", d);
}

/// 正向护栏 1：**清一色 差0 不得标注 混一色**（二者互斥；此项若被标注等于复活 R-08 假阳性）。
#[test]
fn r13_pure_suit_does_not_cover_mixed_suit() {
    let h = parse_hand("123456789m 11m 22m");
    let ds = plan(&h, &[], 2);
    let q = get(&ds, "清一色(万门)");
    assert_eq!(q.distance, 0, "{:?}", q);
    let m = get(&ds, "混一色(万门)");
    assert_eq!(m.distance, 1, "{:?}", m);
    assert!(
        m.covered_by.is_empty(),
        "清一色 不得覆盖 混一色（互斥）: {:?}",
        m
    );
}

/// 正向护栏 2：**无 coverer 达成（无 distance==0）时全表零标注**。
#[test]
fn r13_no_annotation_without_achieved_coverer() {
    let h = parse_hand("123m456m789m 11m 东东");
    let ds = plan(&h, &[], 2);
    assert!(ds.iter().any(|d| d.distance == 0), "本手存在差0方向（清龙/混一色）");
    // 该手差0的方向均非任一差0方向在覆盖表中的 coverer
    for d in ds.iter() {
        assert!(
            d.covered_by.is_empty(),
            "{} 不应被标注: {:?}",
            d.name,
            d.covered_by
        );
    }
}

/// 正向护栏 3：**只有 `distance == 0` 的方向可作 coverer**（差1 的方向不标注他人）。
#[test]
fn r13_only_achieved_coverer_annotates() {
    // 该手：一色四节高 差0（L2 手）、一色四步高 差1 ⇒ 差1 者不得作为 coverer
    let h = parse_hand("111m222m333m444m5m");
    let ds = plan(&h, &[], 2);
    let far = get(&ds, "一色四步高(万门)");
    assert_eq!(far.distance, 1, "{:?}", far);
    for d in ds.iter() {
        assert!(
            !d.covered_by.iter().any(|c| c == "一色四步高(万门)"),
            "差1 的 一色四步高(万门) 不应作为 coverer: {:?}",
            d
        );
    }
    // A 口径的另一半（被覆盖方向不必为 0）由 `r13_qingyaojiu_covers_hunyaojiu_even_when_far` 锁定
}

/// 多覆盖者 + 字一色分支：`字一色` 覆盖 `混幺九`，但**不覆盖 混一色(各门)**（后者差1）。
#[test]
fn r13_zipaiyise_covers_hunyaojiu_but_not_mixed_suit() {
    let h = parse_hand("东东东 南南南 西西西 中中中 发");
    let ds = plan(&h, &[], 2);

    assert_eq!(get(&ds, "字一色").distance, 0);
    assert_eq!(coverers(get(&ds, "混幺九")), vec!["字一色".to_string()]);
    for m in ["混一色(万门)", "混一色(条门)", "混一色(筒门)"] {
        let d = get(&ds, m);
        assert_eq!(d.distance, 1, "{:?}", d);
        assert!(d.covered_by.is_empty(), "字一色 不得覆盖 {}（该门数牌为 0）", m);
    }
    // 碰碰和 为多覆盖者（番数降序、同名升序）
    assert_eq!(
        get(&ds, "碰碰和").covered_by,
        vec!["四暗刻".to_string(), "字一色".to_string(), "混幺九".to_string()]
    );
    assert_eq!(coverers(get(&ds, "三暗刻")), vec!["四暗刻".to_string()]);
}

/// 九莲宝灯只覆盖 清一色（不覆盖 混一色）。
#[test]
fn r13_jiulian_covers_pure_suit_only() {
    let h = parse_hand("1112345678999m");
    let ds = plan(&h, &[], 2);
    assert_eq!(get(&ds, "九莲宝灯(万门)").distance, 0);
    assert_eq!(coverers(get(&ds, "清一色(万门)")), vec!["九莲宝灯(万门)".to_string()]);
    let m = get(&ds, "混一色(万门)");
    assert_eq!(m.distance, 1, "{:?}", m);
    assert!(m.covered_by.is_empty(), "{:?}", m);
}

// ==================== 覆盖对「来源已自动化」的结构性断言 ====================

/// `coverage_pairs()` = `EXCLUSIONS`(38) ∪ `BRANCH_EXCLUSIONS`(3) 展开 ⇒ **95 对**（无重复）；
/// 三条分支对均在来源内；`plan` 侧不再手工枚举。
#[test]
fn r13_coverage_source_is_automated_and_complete() {
    let all: Vec<(&str, &str)> = coverage_pairs().collect();
    assert_eq!(EXCLUSIONS.len(), 38, "计番 drop 表条目数");
    assert_eq!(BRANCH_EXCLUSIONS.len(), 3, "分支互斥表条目数");
    assert_eq!(all.len(), 95, "展开后的覆盖对总数");
    let mut s = all.clone();
    s.sort();
    s.dedup();
    assert_eq!(s.len(), 95, "不应有重复覆盖对");
    for &(c, d) in &[("清幺九", "混幺九"), ("一色四节高", "一色三节高"), ("一色四步高", "一色三步高")] {
        assert!(all.iter().any(|&(x, y)| x == c && y == d), "缺分支覆盖对 {} -> {}", c, d);
    }
}

/// **兄弟互斥链不得登记为覆盖对**（R-13 未加项评估的固化）：
/// **组内**三条链两两互斥（同一手不可能同时达成，且无方向性）⇒ 覆盖表中**不应出现**任何组内对：
/// ① 清一色 / 混一色 / 字一色；② 全大 / 全中 / 全小；③ 大于五 / 小于五。
/// ⚠️ **不可跨组比较**：`全大 ⊆ 大于五`、`全小 ⊆ 小于五` 是**真蕴含**，属合法覆盖对（见下方对照组）。
#[test]
fn r13_sibling_chains_are_not_coverage_pairs() {
    let pairs: Vec<(&str, &str)> = coverage_pairs().collect();
    let groups: [&[&str]; 3] = [
        &["清一色", "混一色", "字一色"],
        &["全大", "全中", "全小"],
        &["大于五", "小于五"],
    ];
    for g in groups {
        for a in g {
            for b in g {
                if a == b {
                    continue;
                }
                assert!(
                    !pairs.iter().any(|&(c, d)| c == *a && d == *b),
                    "兄弟互斥不应登记为覆盖对：{} -> {}",
                    a,
                    b
                );
            }
        }
    }
    // 对照组：跨组/同链中**确实存在**的蕴含关系必须在表中
    for &(c, d) in &[
        ("全大", "大于五"),
        ("全小", "小于五"),
        ("清幺九", "混幺九"),
        ("字一色", "混幺九"),
    ] {
        assert!(pairs.iter().any(|&(x, y)| x == c && y == d), "缺覆盖对 {} -> {}", c, d);
    }
}
