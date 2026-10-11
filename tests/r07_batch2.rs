//! R-07 批 2：杠系 / 刻子系 番种覆盖兜底（DL-3）
//!
//! 口径（B1/B3）：
//! - **每个番种至少一条「名字级断言」的最小正例**，断言**存在性 + 取值**（`has`/`fan_value`），**不用总分代替**。
//! - **互斥族必须同时加反向断言**（`!has_fan`），因为互斥表是最高风险区。
//! - 全部库级断言（`analyze()` 取 `fans` / `fans_tsumo`）。
//!
//! 本批 11 个：`四杠`、`三杠`、`双暗杠`、`双明杠`、`明杠`、`三同刻`、`双同刻`、`双箭刻`、`小三元`、`小四喜`、`清幺九`
//! （`混幺九` / `字一色` 已由 `tests/batch1_regression.rs` 覆盖，不重复。）

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

fn names(w: &WaitResult) -> Vec<String> {
    w.fans.iter().map(|f| f.name.to_string()).collect()
}

fn has_tsumo(w: &WaitResult, n: &str) -> bool {
    w.fans_tsumo.iter().any(|f| f.name == n)
}

/// 一手一测：正向（存在性 + 取值班 + 多处计数）+ 反向（互斥不得出现）
struct Case {
    label: &'static str,
    hand: &'static str,
    melds: Vec<Meld>,
    win: &'static str,
    /// (番名, 期望值和)
    must: &'static [(&'static str, u32)],
    /// 互斥/应被吞掉的番名
    must_not: &'static [&'static str],
    /// 该番名应出现 ≥1 次（用于「幺九刻×3」这类重复计数断言），(番名, 次数)
    repeat: &'static [(&'static str, usize)],
}

fn run(c: &Case) {
    let h = parse_hand(c.hand);
    let an = analyze(&h, &c.melds, 27, 27);
    let w = by_name(&an, c.win);
    for &(n, v) in c.must {
        assert!(
            has(w, n),
            "[{}] 缺番种 {}（实际 {:?}）",
            c.label,
            n,
            names(w)
        );
        if v > 0 {
            assert_eq!(
                val(w, n),
                v,
                "[{}] {} 番值不符（实际 {:?}）",
                c.label,
                n,
                names(w)
            );
        }
    }
    for &(n, times) in c.repeat {
        let got = w.fans.iter().filter(|f| f.name == n).count();
        assert_eq!(
            got, times,
            "[{}] {} 出现 {} 次（期望 {}）",
            c.label, n, got, times
        );
    }
    for &n in c.must_not {
        assert!(
            !has(w, n),
            "[{}] 不应出现 {}（互斥/被吞掉；实际 {:?}）",
            c.label,
            n,
            names(w)
        );
    }
}

// ============================ 番值常量（must 表：番名→期望番值和；出处为 src/fans.rs 的 f("番名", N) call site）============================
// 同一数字不同番种分开命名；动态生成的番另有标注。
const FAN_BIAN_ZHANG: u32 = 1; // 边张（1 番）    — 出处：src/fans.rs 的 f("边张", 1)
const FAN_DAN_DIAO_JIANG: u32 = 1; // 单钓将（1 番）  — 出处：src/fans.rs 的 f("单钓将", 1)
const FAN_HUA_LONG: u32 = 8; // 花龙（8 番）    — 出处：src/fans.rs 的 f("花龙", 8)
const FAN_HUN_YI_SE: u32 = 6; // 混一色（6 番）  — 出处：src/fans.rs 的 f("混一色", 6)
const FAN_MEN_FENG_KE: u32 = 2; // 门风刻（2 番）  — 出处：src/fans.rs 的 f("门风刻", 2)
const FAN_MEN_QIAN_QING: u32 = 2; // 门前清（2 番，和法番）— 出处：src/fans.rs 的 f("门前清", 2)
const FAN_MING_GANG: u32 = 1; // 明杠（1 番）    — 出处：src/fans.rs 的 f("明杠", 1)
const FAN_QUAN_DAI_YAO: u32 = 4; // 全带幺（4 番）  — 出处：src/fans.rs 的 f("全带幺", 4)
const FAN_QUAN_FENG_KE: u32 = 2; // 圈风刻（2 番）  — 出处：src/fans.rs 的 f("圈风刻", 2)
const FAN_QUAN_QIU_REN: u32 = 6; // 全求人（6 番）  — 出处：src/fans.rs 的 f("全求人", 6)
const FAN_QUE_YI_MEN: u32 = 1; // 缺一门（1 番）  — 出处：src/fans.rs 的 f("缺一门", 1)
const FAN_SAN_AN_KE: u32 = 16; // 三暗刻（16 番） — 出处：src/fans.rs 的 f("三暗刻", 16)
const FAN_SAN_GANG: u32 = 32; // 三杠（32 番）   — 出处：src/fans.rs 的 f("三杠", 32)
const FAN_SAN_TONG_KE: u32 = 16; // 三同刻（16 番） — 出处：src/fans.rs 的 f("三同刻", 16)
const FAN_SHUANG_AN_GANG: u32 = 6; // 双暗杠（6 番）  — 出处：src/fans.rs 的 f("双暗杠", 6)
const FAN_SHUANG_AN_KE: u32 = 2; // 双暗刻（2 番）  — 出处：src/fans.rs 的 f("双暗刻", 2)
const FAN_SHUANG_JIAN_KE: u32 = 6; // 双箭刻（6 番）  — 出处：src/fans.rs 的 f("双箭刻", 6)
const FAN_SHUANG_MING_GANG: u32 = 4; // 双明杠（4 番）  — 出处：src/fans.rs 的 f("双明杠", 4)
const FAN_SHUANG_TONG_KE: u32 = 2; // 双同刻（2 番）  — 出处：src/fans.rs 的 f("双同刻", 2)
const FAN_SI_GANG: u32 = 88; // 四杠（88 番）   — 出处：src/fans.rs 的 f("四杠", 88)
const FAN_WU_ZI: u32 = 1; // 无字（1 番）    — 出处：src/fans.rs 的 f("无字", 1)
const FAN_XIAO_SAN_YUAN: u32 = 64; // 小三元（64 番） — 出处：src/fans.rs 的 f("小三元", 64)
const FAN_XIAO_SI_XI: u32 = 64; // 小四喜（64 番） — 出处：src/fans.rs 的 f("小四喜", 64)
const FAN_YAO_JIU_KE: u32 = 1; // 幺九刻（1 番）  — 出处：src/fans.rs 的 f("幺九刻", 1)
                               // ============================ 出现次数常量（repeat 表：番名→应出现次数；**不是番值**）============================
const TIMES_YAO_JIU_KE_4: usize = 4; // 幺九刻 应出现 4 次（每副各计 1 次）—— 非番值
const TIMES_YAO_JIU_KE_3: usize = 3; // 幺九刻 应出现 3 次（每副各计 1 次）—— 非番值
const TIMES_YAO_JIU_KE_2: usize = 2; // 幺九刻 应出现 2 次（每副各计 1 次）—— 非番值

// ==================== 杠系（4 条 + 明杠） ====================
// 注：原预留但未使用的常量 `KAN_FAMILY_OTHERS` 已删除（消除 dead_code 警告）；
//     各杠系用例的互斥反向断言在各自 `must_not` 中显式列出（更具体、可审计）。

/// `四杠`：4 副杠 ⇒ 暗牌 1 张；杠系互斥（不得另计 三杠/双暗杠/双明杠/一明一暗杠/暗杠/明杠）；
/// 点炮为 4 副露 + 单钓 ⇒ `全求人` 吞 `单钓将`；`四杠` 吞 `碰碰和`。
#[test]
fn r07b2_si_gang_and_kan_family_exclusion() {
    run(&Case {
        label: "四杠",
        hand: "5p",
        melds: vec![
            Meld::kan(0, true),  // 明杠 1万
            Meld::kan(8, true),  // 明杠 9万
            Meld::kan(9, true),  // 明杠 1条
            Meld::kan(17, true), // 明杠 9条
        ],
        win: "5筒",
        must: &[
            ("四杠", FAN_SI_GANG),
            ("双同刻", FAN_SHUANG_TONG_KE),
            ("无字", FAN_WU_ZI),
            ("全求人", FAN_QUAN_QIU_REN),
        ],
        must_not: &[
            "碰碰和",
            "单钓将",
            "三杠",
            "双暗杠",
            "双明杠",
            "一明一暗杠",
            "暗杠",
            "明杠",
        ],
        repeat: &[("幺九刻", TIMES_YAO_JIU_KE_4)],
    });
}

/// `三杠`：3 副杠 ⇒ 暗牌 4 张；不得另计其它杠系番。
#[test]
fn r07b2_san_gang() {
    run(&Case {
        label: "三杠",
        hand: "55p 67m",
        melds: vec![Meld::kan(0, true), Meld::kan(8, true), Meld::kan(9, true)],
        win: "5万",
        must: &[
            ("三杠", FAN_SAN_GANG),
            ("双同刻", FAN_SHUANG_TONG_KE),
            ("无字", FAN_WU_ZI),
        ],
        must_not: &["四杠", "双暗杠", "双明杠", "一明一暗杠", "暗杠", "明杠"],
        repeat: &[("幺九刻", TIMES_YAO_JIU_KE_3)],
    });
    // 同一手的另一张听牌（8万）番表应一致
    let h = parse_hand("55p 67m");
    let an = analyze(
        &h,
        &[Meld::kan(0, true), Meld::kan(8, true), Meld::kan(9, true)],
        27,
        27,
    );
    assert_eq!(val(by_name(&an, "8万"), "三杠"), 32);
}

/// `双暗杠`：2 副暗杠；**不得另计 `暗杠`**；暗杠计入暗刻 ⇒ `双暗刻`；门清保持。
#[test]
fn r07b2_shuang_an_gang() {
    run(&Case {
        label: "双暗杠",
        hand: "123m 456s 9p",
        melds: vec![Meld::kan(18, false), Meld::kan(24, false)],
        win: "9筒",
        must: &[
            ("双暗杠", FAN_SHUANG_AN_GANG),
            ("双暗刻", FAN_SHUANG_AN_KE),
            ("单钓将", FAN_DAN_DIAO_JIANG),
            ("门前清", FAN_MEN_QIAN_QING),
            ("幺九刻", FAN_YAO_JIU_KE),
            ("无字", FAN_WU_ZI),
        ],
        must_not: &["暗杠", "明杠", "双明杠", "一明一暗杠"],
        repeat: &[],
    });
}

/// `双明杠`：2 副明杠；**不得另计 `明杠`**；明杠不算暗刻 ⇒ 无 `双暗刻`。
#[test]
fn r07b2_shuang_ming_gang() {
    run(&Case {
        label: "双明杠",
        hand: "123m 456s 9p",
        melds: vec![Meld::kan(18, true), Meld::kan(24, true)],
        win: "9筒",
        must: &[
            ("双明杠", FAN_SHUANG_MING_GANG),
            ("单钓将", FAN_DAN_DIAO_JIANG),
            ("幺九刻", FAN_YAO_JIU_KE),
            ("无字", FAN_WU_ZI),
        ],
        must_not: &["明杠", "暗杠", "双暗杠", "一明一暗杠", "双暗刻", "门前清"],
        repeat: &[],
    });
}

/// `明杠`：单副明杠（1 番）；不得误计 `暗杠`/双杠系；同手顺带核对 `花龙`。
#[test]
fn r07b2_ming_gang() {
    run(&Case {
        label: "明杠",
        hand: "123m 456s 789p 9s",
        melds: vec![Meld::kan(22, true)], // 明杠 5筒
        win: "9条",
        must: &[
            ("明杠", FAN_MING_GANG),
            ("花龙", FAN_HUA_LONG),
            ("单钓将", FAN_DAN_DIAO_JIANG),
            ("无字", FAN_WU_ZI),
        ],
        must_not: &["暗杠", "双明杠", "双暗杠", "一明一暗杠", "三杠", "四杠"],
        repeat: &[],
    });
}

// ==================== 同刻系 ====================

/// `三同刻`：三副同数刻子（111万/111条/111筒）；`三同刻` 吞 `双同刻`（不得并列）。
#[test]
fn r07b2_san_tong_ke() {
    run(&Case {
        label: "三同刻",
        hand: "111m 111s 111p 456m 9s",
        melds: vec![],
        win: "9条",
        must: &[
            ("三同刻", FAN_SAN_TONG_KE),
            ("三暗刻", FAN_SAN_AN_KE),
            ("单钓将", FAN_DAN_DIAO_JIANG),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &["双同刻", "四暗刻"],
        repeat: &[("幺九刻", TIMES_YAO_JIU_KE_3)], // 三副 1 的刻子各计一次幺九刻
    });
}

/// `双同刻`：两副同数刻子（111万/111条）；`双同刻` 与 `双暗刻` 可并列。
#[test]
fn r07b2_shuang_tong_ke() {
    run(&Case {
        label: "双同刻",
        hand: "111m 111s 456m 789p 9s",
        melds: vec![],
        win: "9条",
        must: &[
            ("双同刻", FAN_SHUANG_TONG_KE),
            ("双暗刻", FAN_SHUANG_AN_KE),
            ("单钓将", FAN_DAN_DIAO_JIANG),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &["三同刻", "三暗刻"],
        repeat: &[("幺九刻", TIMES_YAO_JIU_KE_2)],
    });
}

/// `双箭刻`：中/发两副箭牌刻子；`双箭刻` 吞 `箭刻`（不得并列），也不得误计三元。
#[test]
fn r07b2_shuang_jian_ke() {
    run(&Case {
        label: "双箭刻",
        hand: "中中中 发发发 123m 456s 9p",
        melds: vec![],
        win: "9筒",
        must: &[
            ("双箭刻", FAN_SHUANG_JIAN_KE),
            ("双暗刻", FAN_SHUANG_AN_KE),
            ("单钓将", FAN_DAN_DIAO_JIANG),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &["箭刻", "大三元", "小三元"],
        repeat: &[],
    });
}

/// `小三元`：两副箭刻 + 箭牌将；应吞 `双箭刻`/`箭刻`；顺带核对 `缺一门`。
#[test]
fn r07b2_xiao_san_yuan() {
    run(&Case {
        label: "小三元",
        hand: "中中中 发发发 白白 123m 45s",
        melds: vec![],
        win: "3条",
        must: &[
            ("小三元", FAN_XIAO_SAN_YUAN),
            ("双暗刻", FAN_SHUANG_AN_KE),
            ("缺一门", FAN_QUE_YI_MEN),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &["双箭刻", "箭刻", "大三元"],
        repeat: &[],
    });
}

/// `小四喜`：三副风刻 + 风牌将；应吞 `三风刻`/`幺九刻`；顺带核对 圈风刻/门风刻（东=圈=门）、`全带幺`、`边张`。
#[test]
fn r07b2_xiao_si_xi() {
    run(&Case {
        label: "小四喜",
        hand: "东东东 南南南 西西西 北北 12m",
        melds: vec![],
        win: "3万",
        must: &[
            ("小四喜", FAN_XIAO_SI_XI),
            ("三暗刻", FAN_SAN_AN_KE),
            ("圈风刻", FAN_QUAN_FENG_KE),
            ("门风刻", FAN_MEN_FENG_KE),
            ("全带幺", FAN_QUAN_DAI_YAO),
            ("混一色", FAN_HUN_YI_SE),
            ("边张", FAN_BIAN_ZHANG),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &["三风刻", "幺九刻", "大四喜"],
        repeat: &[],
    });
}

// 清幺九（64 番）— 出处：src/fans.rs 的 f("清幺九", 64)
const FAN_QING_YAO_JIU: u32 = 64;

// ==================== 清幺九（含自摸分支） ====================

/// `清幺九`：全幺九数牌，**无字牌**；应吞 `碰碰和`/`全带幺`/`幺九刻`/`无字`/`双同刻`/`三同刻`。
/// 点炮：和牌张补成的刻子算明刻 ⇒ `三暗刻`；自摸：四副刻子全暗 ⇒ `四暗刻`。
#[test]
fn r07b2_qing_yao_jiu_and_exclusions() {
    run(&Case {
        label: "清幺九(点炮)",
        hand: "111m 999m 111s 99s 11p",
        melds: vec![],
        win: "9条",
        must: &[
            ("清幺九", FAN_QING_YAO_JIU),
            ("三暗刻", FAN_SAN_AN_KE),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &[
            "碰碰和",
            "全带幺",
            "幺九刻",
            "无字",
            "双同刻",
            "三同刻",
            "混幺九",
            "字一色",
        ],
        repeat: &[],
    });
    // 自摸分支：四暗刻 成立、三暗刻 被吞
    let h = parse_hand("111m 999m 111s 99s 11p");
    let an = analyze(&h, &[], 27, 27);
    let w = by_name(&an, "9条");
    assert!(
        has_tsumo(w, "四暗刻"),
        "自摸应四暗刻: {:?}",
        w.fans_tsumo.iter().map(|f| f.name).collect::<Vec<_>>()
    );
    assert!(
        !w.fans_tsumo.iter().any(|f| f.name == "三暗刻"),
        "自摸四暗刻应吞三暗刻"
    );
    assert!(has_tsumo(w, "自摸"), "自摸应计自摸番");
    assert_eq!(val(w, "清幺九"), FAN_QING_YAO_JIU);
}
