//! R-07 批 4：其余 10 个零覆盖番种覆盖兜底（DL-3）
//!
//! 覆盖：`五门齐`6、`全带幺`4、`全带五`16、`全求人`6、`全中`24、`全双刻`24、`自摸`1、`全不靠`12、`七星不靠`24
//! —— 以及 `和绝张` 的**API 限制观察**（见文件末尾；已按 B2 升级，未改 `src/`）。

use gbmj::calc::{analyze, Analysis, WaitResult};
use gbmj::model::Meld;
use gbmj::tiles::*;

fn by_name<'a>(an: &'a Analysis, name: &str) -> &'a WaitResult {
    an.waits
        .iter()
        .find(|w| tile_name(w.tile) == name)
        .unwrap_or_else(|| panic!("无此和牌张: {}", name))
}

fn has(w: &WaitResult, n: &str) -> bool {
    w.fans.iter().any(|f| f.name == n)
}

fn val(w: &WaitResult, n: &str) -> u32 {
    w.fans.iter().filter(|f| f.name == n).map(|f| f.value).sum()
}

fn has_t(w: &WaitResult, n: &str) -> bool {
    w.fans_tsumo.iter().any(|f| f.name == n)
}

fn val_t(w: &WaitResult, n: &str) -> u32 {
    w.fans_tsumo
        .iter()
        .filter(|f| f.name == n)
        .map(|f| f.value)
        .sum()
}

fn names(w: &WaitResult) -> Vec<String> {
    w.fans.iter().map(|f| f.name.to_string()).collect()
}

fn names_t(w: &WaitResult) -> Vec<String> {
    w.fans_tsumo.iter().map(|f| f.name.to_string()).collect()
}

// ============================ 番值常量（for 表：番名→期望番值和；出处为 src/fans.rs 的 f("番名", N) call site）============================
// 同一数字不同番种分开命名；动态生成的番另有标注。
const FAN_DAN_DIAO_JIANG: u32 = 1; // 单钓将（1 番）  — 出处：src/fans.rs 的 f("单钓将", 1)
const FAN_HUA_LONG: u32 = 8; // 花龙（8 番）    — 出处：src/fans.rs 的 f("花龙", 8)
const FAN_LAO_SHAO_FU: u32 = 1; // 老少副（1 番）  — 出处：动态生成（src/fans.rs 的 for name in ["一般高","喜相逢","连六","老少副"]）
const FAN_MEN_FENG_KE: u32 = 2; // 门风刻（2 番）  — 出处：src/fans.rs 的 f("门风刻", 2)
const FAN_MEN_QIAN_QING: u32 = 2; // 门前清（2 番，和法番）— 出处：src/fans.rs 的 f("门前清", 2)
const FAN_QUAN_DAI_WU: u32 = 16; // 全带五（16 番） — 出处：src/fans.rs 的 f("全带五", 16)
const FAN_QUAN_DAI_YAO: u32 = 4; // 全带幺（4 番）  — 出处：src/fans.rs 的 f("全带幺", 4)
const FAN_QUAN_FENG_KE: u32 = 2; // 圈风刻（2 番）  — 出处：src/fans.rs 的 f("圈风刻", 2)
const FAN_QUAN_QIU_REN: u32 = 6; // 全求人（6 番）  — 出处：src/fans.rs 的 f("全求人", 6)
const FAN_QUAN_ZHONG: u32 = 24; // 全中（24 番）   — 出处：src/fans.rs 的 f("全中", 24)
const FAN_SAN_AN_KE: u32 = 16; // 三暗刻（16 番） — 出处：src/fans.rs 的 f("三暗刻", 16)
const FAN_SAN_SE_SAN_BU_GAO: u32 = 6; // 三色三步高（6 番）— 出处：src/fans.rs 的 f("三色三步高", 6)
const FAN_SAN_SE_SAN_TONG_SHUN: u32 = 8; // 三色三同顺（8 番）— 出处：src/fans.rs 的 f("三色三同顺", 8)
const FAN_SHUANG_AN_KE: u32 = 2; // 双暗刻（2 番）  — 出处：src/fans.rs 的 f("双暗刻", 2)
const FAN_SI_GUI_YI: u32 = 2; // 四归一（2 番）  — 出处：src/fans.rs 的 f("四归一", 2)
const FAN_WU_MEN_QI: u32 = 6; // 五门齐（6 番）  — 出处：src/fans.rs 的 f("五门齐", 6)
const FAN_WU_ZI: u32 = 1; // 无字（1 番）    — 出处：src/fans.rs 的 f("无字", 1)
const FAN_YAO_JIU_KE: u32 = 1; // 幺九刻（1 番）  — 出处：src/fans.rs 的 f("幺九刻", 1)
const FAN_QUAN_BU_KAO: u32 = 12; // 全不靠（12 番） — 出处：src/fans.rs 的 Special::BuKao 分支 f("全不靠", 12)
const FAN_ZUHE_LONG: u32 = 12; // 组合龙（12 番） — 出处：src/fans.rs 的 f("组合龙", 12)（两处）
const FAN_QI_XING_BU_KAO: u32 = 24; // 七星不靠（24 番）— 出处：src/fans.rs 的 Special::SevenStars 分支 f("七星不靠", 24)

// ==================== 花色整手番 ====================

/// `五门齐`（6）：万/条/筒/风/箭俱全；顺带核对 圈风刻+门风刻（东=圈=门）与 `花龙`。
#[test]
fn r07b4_wu_men_qi() {
    let h = parse_hand("123m 456s 789p 东东东 中");
    let an = analyze(&h, &[], 27, 27);
    let w = by_name(&an, "中");
    for (n, v) in [
        ("五门齐", FAN_WU_MEN_QI),
        ("圈风刻", FAN_QUAN_FENG_KE),
        ("门风刻", FAN_MEN_FENG_KE),
        ("花龙", FAN_HUA_LONG),
        ("单钓将", FAN_DAN_DIAO_JIANG),
        ("门前清", FAN_MEN_QIAN_QING),
    ] {
        assert!(has(w, n), "缺 {}: {:?}", n, names(w));
        assert_eq!(val(w, n), v, "{} 番值: {:?}", n, names(w));
    }
}

/// `全带幺`（4）：每副面子与将牌都含幺九或字牌。
#[test]
fn r07b4_quan_dai_yao() {
    let h = parse_hand("123m 789m 111s 999p 东");
    let an = analyze(&h, &[], 27, 27);
    let w = by_name(&an, "东");
    for (n, v) in [
        ("全带幺", FAN_QUAN_DAI_YAO),
        ("双暗刻", FAN_SHUANG_AN_KE),
        ("老少副", FAN_LAO_SHAO_FU),
        ("单钓将", FAN_DAN_DIAO_JIANG),
        ("门前清", FAN_MEN_QIAN_QING),
    ] {
        assert!(has(w, n), "缺 {}: {:?}", n, names(w));
        assert_eq!(val(w, n), v, "{} 番值: {:?}", n, names(w));
    }
    assert_eq!(
        w.fans.iter().filter(|f| f.name == "幺九刻").count(),
        2,
        "应有两处幺九刻: {:?}",
        names(w)
    );
}

/// `全带五`（16）：每副面子与将牌都含 5；应吞 `断幺`/`无字`（互斥表 `:932`）。
#[test]
fn r07b4_quan_dai_wu() {
    let h = parse_hand("345m 456s 567p 555m 5s");
    let an = analyze(&h, &[], 27, 27);
    let w = by_name(&an, "5条");
    for (n, v) in [
        ("全带五", FAN_QUAN_DAI_WU),
        ("三色三步高", FAN_SAN_SE_SAN_BU_GAO),
        ("四归一", FAN_SI_GUI_YI),
        ("单钓将", FAN_DAN_DIAO_JIANG),
        ("门前清", FAN_MEN_QIAN_QING),
    ] {
        assert!(has(w, n), "缺 {}: {:?}", n, names(w));
        assert_eq!(val(w, n), v, "{} 番值: {:?}", n, names(w));
    }
    for n in ["断幺", "无字"] {
        assert!(!has(w, n), "全带五 应吞 {}: {:?}", n, names(w));
    }
}

/// `全中`（24）：全部 4/5/6；应吞 `断幺`/`无字`（互斥表 `:930`）。
#[test]
fn r07b4_quan_zhong() {
    let h = parse_hand("456m 456s 456p 55m 66s");
    let an = analyze(&h, &[], 27, 27);
    let w = by_name(&an, "5万");
    for (n, v) in [
        ("全中", FAN_QUAN_ZHONG),
        ("三色三同顺", FAN_SAN_SE_SAN_TONG_SHUN),
        ("四归一", FAN_SI_GUI_YI),
        ("门前清", FAN_MEN_QIAN_QING),
    ] {
        assert!(has(w, n), "缺 {}: {:?}", n, names(w));
        assert_eq!(val(w, n), v, "{} 番值: {:?}", n, names(w));
    }
    for n in ["断幺", "无字"] {
        assert!(!has(w, n), "全中 应吞 {}: {:?}", n, names(w));
    }
}

// 全双刻（24 番）— 出处：src/fans.rs 的 f("全双刻", 24)
const FAN_QUAN_SHUANG_KE: u32 = 24;
// 自摸（1 番，和法番）— 出处：src/fans.rs 的 f("自摸", 1)
const FAN_ZI_MO: u32 = 1;

/// `全双刻`（24）：全部双数刻子 + 双数将；应吞 `碰碰和`/`断幺`（互斥表 `:925`）。
/// 点炮为 `三暗刻`、自摸为 `四暗刻`（四暗刻吞 碰碰和/三暗刻）。
#[test]
fn r07b4_quan_shuang_ke() {
    let h = parse_hand("222m 444s 666p 88m 22s");
    let an = analyze(&h, &[], 27, 27);
    let w = by_name(&an, "8万");
    for (n, v) in [
        ("全双刻", FAN_QUAN_SHUANG_KE),
        ("三暗刻", FAN_SAN_AN_KE),
        ("门前清", FAN_MEN_QIAN_QING),
    ] {
        assert!(has(w, n), "缺 {}: {:?}", n, names(w));
        assert_eq!(val(w, n), v, "{} 番值: {:?}", n, names(w));
    }
    for n in ["碰碰和", "断幺"] {
        assert!(!has(w, n), "全双刻 应吞 {}: {:?}", n, names(w));
    }
    // 自摸分支：四暗刻 成立并吞 碰碰和/三暗刻
    assert!(has_t(w, "四暗刻"), "自摸应四暗刻: {:?}", names_t(w));
    assert_eq!(val_t(w, "全双刻"), FAN_QUAN_SHUANG_KE);
    assert!(!has_t(w, "碰碰和"), "四暗刻应吞碰碰和: {:?}", names_t(w));
}

// ==================== 和法与副露番 ====================

/// `全求人`（6）：4 副**明**副露 + 单钓，**仅荣和**成立；自摸时反而不计（互斥/成立条件差异）。
#[test]
fn r07b4_quan_qiu_ren() {
    let h = parse_hand("5p");
    let melds = vec![
        Meld::chi(0),  // 吃 123万
        Meld::pon(13), // 碰 5条
        Meld::pon(24), // 碰 8筒
        Meld::pon(17), // 碰 9条
    ];
    let an = analyze(&h, &melds, 27, 27);
    let w = by_name(&an, "5筒");
    for (n, v) in [
        ("全求人", FAN_QUAN_QIU_REN),
        ("幺九刻", FAN_YAO_JIU_KE),
        ("无字", FAN_WU_ZI),
    ] {
        assert!(has(w, n), "缺 {}: {:?}", n, names(w));
        assert_eq!(val(w, n), v, "{} 番值: {:?}", n, names(w));
    }
    assert!(!has(w, "门前清"), "非门清: {:?}", names(w));
    assert!(!has(w, "单钓将"), "全求人 应吞 单钓将: {:?}", names(w));
    // 自摸：全求人不成立，改为 单钓将 + 自摸
    assert!(!has_t(w, "全求人"), "全求人仅荣和: {:?}", names_t(w));
    assert!(
        has_t(w, "单钓将") && has_t(w, "自摸"),
        "自摸应 单钓将+自摸: {:?}",
        names_t(w)
    );
}

/// `自摸`（1）：**非门清**手自摸得 `自摸`；门清手改计 `不求人`（此处以非门清手锁 `自摸` 之名）。
#[test]
fn r07b4_zi_mo() {
    let h = parse_hand("123m 456s 789p 9s");
    let an = analyze(&h, &[Meld::kan(22, true)], 27, 27);
    let w = by_name(&an, "9条");
    assert!(!has(w, "自摸"), "点和不得计自摸: {:?}", names(w));
    assert!(has_t(w, "自摸"), "自摸应计自摸: {:?}", names_t(w));
    assert_eq!(val_t(w, "自摸"), FAN_ZI_MO);
    assert!(!has_t(w, "不求人"), "非门清不计不求人: {:?}", names_t(w));
    // 门清对照：应以 不求人 取代 自摸
    let h2 = parse_hand("789m 789m 789s 99s 88p");
    let an2 = analyze(&h2, &[], 27, 27);
    let w2 = by_name(&an2, "9条");
    assert!(
        has_t(w2, "不求人") && !has_t(w2, "自摸"),
        "门清自摸应为不求人: {:?}",
        names_t(w2)
    );
}

// ==================== 不靠系 ====================

/// `全不靠`（12）：三门针织牌 + 字牌；听 9筒 时同时成立 `组合龙`（12）。
#[test]
fn r07b4_quan_bu_kao() {
    let h = parse_hand("147m 258s 36p 东南西北中");
    let an = analyze(&h, &[], 27, 27);
    let w9 = by_name(&an, "9筒");
    assert!(
        has(w9, "全不靠") && val(w9, "全不靠") == FAN_QUAN_BU_KAO,
        "{:?}",
        names(w9)
    );
    assert!(
        has(w9, "组合龙") && val(w9, "组合龙") == FAN_ZUHE_LONG,
        "全不靠+组合龙 可并列: {:?}",
        names(w9)
    );
    // 听字牌（发/白）时只有 全不靠
    let wf = by_name(&an, "发");
    assert!(has(wf, "全不靠") && !has(wf, "组合龙"), "{:?}", names(wf));
}

/// `七星不靠`（24）：全不靠 + 七种字牌齐全。
#[test]
fn r07b4_qi_xing_bu_kao() {
    let h = parse_hand("147m 258s 东南西北中发白");
    let an = analyze(&h, &[], 27, 27);
    for t in ["3筒", "6筒", "9筒"] {
        let w = by_name(&an, t);
        assert!(
            has(w, "七星不靠") && val(w, "七星不靠") == FAN_QI_XING_BU_KAO,
            "{}: {:?}",
            t,
            names(w)
        );
        assert!(!has(w, "全不靠"), "七星不靠应吞全不靠: {:?}", names(w));
    }
}

// ==================== 观察：`和绝张` 无公开番表（API 限制，B2 已升级） ====================

/// **观察（B2 已升级）**：`WaitResult` 只公开 `fans`（点和）与 `fans_tsumo`（自摸）两张番表；
/// 绝张两栏（点和绝张 / 自摸绝张）的番表在 `src/calc.rs:239-240` 被 `let (lt, _) = score(...)` **丢弃**
/// ⇒ **无法**对 `和绝张` 做名字级断言（无 src 改动不可解）。
///
/// 本用例改为**数值护栏**：对 `can_last == true` 的方向，绝张栏应比同方式常规栏**多 4 番**
/// （点炮 `L = N + 4`、自摸 `TL = T + 4`，与既有口径一致）。
/// **建议**：若要求 `和绝张` 具备名字级覆盖，需在 `WaitResult` 增加 `fans_last`/`fans_tsumo_last`
/// （属 `src/` 改动，**由 Lead/开发决定，测试方未改**）。
#[test]
fn r07b4_observation_he_jue_zhang_has_no_public_fan_list() {
    let h = parse_hand("123m123m 456s 789p 9p");
    let an = analyze(&h, &[], 27, 27);
    let w = by_name(&an, "6筒");
    assert!(w.can_last, "该方向应可绝张: {:?}", w);
    assert_eq!(
        w.last,
        w.normal + 4,
        "点炮绝张 = 点和 + 4（和绝张 4 番）: {:?}",
        w
    );
    assert_eq!(
        w.tsumo_last,
        w.tsumo + 4,
        "自摸绝张 = 自摸 + 4（和绝张 4 番）: {:?}",
        w
    );
}
