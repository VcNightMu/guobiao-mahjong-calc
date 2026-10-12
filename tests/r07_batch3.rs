//! R-07 批 3：组合型大番覆盖兜底（DL-3）
//!
//! 口径（B1/B3）：
//! - 每个番种一条**名字级存在性 + 取值**断言（助手 `run()` 统一消费数据表 ⇒ 表驱动但可审计）。
//! - **套算关系反向断言**：高阶番成立时低阶是否按互斥表 drop（`一般高`/`连六`/`老少副`/`喜相逢` 是最易算错处）。
//! - **控制行**：只含低阶、无高阶的手牌，锁住「不可误删低阶」。
//!
//! 覆盖：`一色四同顺`48、`一色四节高`48、`一色四步高`32、`一色三节高`24、`一色三步高`16、
//! `三色三同顺`8、`三色三节高`8、`三色三步高`6、`连七对`88 —— 以及一条 **`一色三同顺` 不可达观察**（见文件末尾）。

use gbmj::calc::{analyze, Analysis, WaitResult};
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

struct Case {
    label: &'static str,
    hand: &'static str,
    win: &'static str,
    must: &'static [(&'static str, u32)],
    must_not: &'static [&'static str],
    repeat: &'static [(&'static str, usize)],
}

fn run(c: &Case) {
    let h = parse_hand(c.hand);
    let an = analyze(&h, &[], 27, 27);
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
    for &(n, t) in c.repeat {
        let got = w.fans.iter().filter(|f| f.name == n).count();
        assert_eq!(got, t, "[{}] {} 出现 {} 次（期望 {}）", c.label, n, got, t);
    }
    for &n in c.must_not {
        assert!(
            !has(w, n),
            "[{}] 不应出现 {}（实际 {:?}）",
            c.label,
            n,
            names(w)
        );
    }
}

// ============================ 番值常量（must 表：番名→期望番值和；出处为 src/fans.rs 的 f("番名", N) call site）============================
// 同一数字不同番种分开命名；动态生成的番另有标注。
const FAN_DAN_DIAO_JIANG: u32 = 1; // 单钓将（1 番）  — 出处：src/fans.rs 的 f("单钓将", 1)
const FAN_LIAN_LIU: u32 = 1; // 连六（1 番）    — 出处：动态生成（src/fans.rs 的 for name in ["一般高","喜相逢","连六","老少副"]）
const FAN_LIAN_QI_DUI: u32 = 88; // 连七对（88 番） — 出处：src/fans.rs 的 f("连七对", 88)
const FAN_MEN_QIAN_QING: u32 = 2; // 门前清（2 番，和法番）— 出处：src/fans.rs 的 f("门前清", 2)
const FAN_PING_HE: u32 = 2; // 平和（2 番）    — 出处：src/fans.rs 的 f("平和", 2)
const FAN_QI_DUI: u32 = 24; // 七对（24 番）   — 出处：src/fans.rs 的 f("七对", 24)
const FAN_QING_YI_SE: u32 = 24; // 清一色（24 番） — 出处：src/fans.rs 的 f("清一色", 24)
const FAN_SAN_AN_KE: u32 = 16; // 三暗刻（16 番） — 出处：src/fans.rs 的 f("三暗刻", 16)
const FAN_SAN_SE_SAN_BU_GAO: u32 = 6; // 三色三步高（6 番）— 出处：src/fans.rs 的 f("三色三步高", 6)
const FAN_SAN_SE_SAN_JIE_GAO: u32 = 8; // 三色三节高（8 番）— 出处：src/fans.rs 的 f("三色三节高", 8)
const FAN_SAN_SE_SAN_TONG_SHUN: u32 = 8; // 三色三同顺（8 番）— 出处：src/fans.rs 的 f("三色三同顺", 8)
const FAN_SI_AN_KE: u32 = 64; // 四暗刻（64 番） — 出处：src/fans.rs 的 f("四暗刻", 64)
const FAN_SI_GUI_YI: u32 = 2; // 四归一（2 番）  — 出处：src/fans.rs 的 f("四归一", 2)
const FAN_WU_ZI: u32 = 1; // 无字（1 番）    — 出处：src/fans.rs 的 f("无字", 1)
const FAN_XI_XIANG_FENG: u32 = 1; // 喜相逢（1 番）  — 出处：动态生成（src/fans.rs 的 for name in ["一般高","喜相逢","连六","老少副"]）
const FAN_YAO_JIU_KE: u32 = 1; // 幺九刻（1 番）  — 出处：src/fans.rs 的 f("幺九刻", 1)
const FAN_YI_BAN_GAO: u32 = 1; // 一般高（1 番）  — 出处：动态生成（src/fans.rs 的 for name in ["一般高","喜相逢","连六","老少副"]）
const FAN_YI_SE_SAN_BU_GAO: u32 = 16; // 一色三步高（16 番）— 出处：src/fans.rs 的 f("一色三步高", 16)
const FAN_YI_SE_SAN_JIE_GAO: u32 = 24; // 一色三节高（24 番）— 出处：src/fans.rs 的 f("一色三节高", 24)
const FAN_YI_SE_SI_BU_GAO: u32 = 32; // 一色四步高（32 番）— 出处：src/fans.rs 的 f("一色四步高", 32)
const FAN_YI_SE_SI_JIE_GAO: u32 = 48; // 一色四节高（48 番）— 出处：src/fans.rs 的 f("一色四节高", 48)
const FAN_YI_SE_SI_TONG_SHUN: u32 = 48; // 一色四同顺（48 番）— 出处：src/fans.rs 的 f("一色四同顺", 48)

/// `一色四同顺`（48）：4 副相同顺子；**应吞 `一色三同顺`/`一般高`/`四归一`/`七对`**（互斥表 `EXCLUSIONS` 的 `一色四同顺` 条目）。
#[test]
fn r07b3_yi_se_si_tong_shun() {
    run(&Case {
        label: "一色四同顺",
        hand: "123m123m123m123m 5m",
        win: "5万",
        must: &[
            ("一色四同顺", FAN_YI_SE_SI_TONG_SHUN),
            ("清一色", FAN_QING_YI_SE),
            ("平和", FAN_PING_HE),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &["一色三同顺", "一般高", "四归一", "七对", "一色三节高"],
        repeat: &[],
    });
}

/// `一色四节高`（48）：4 副递增刻子；**应吞 `一色三节高`**；`四暗刻` 吞 `碰碰和`/`三暗刻`。
#[test]
fn r07b3_yi_se_si_jie_gao() {
    run(&Case {
        label: "一色四节高",
        hand: "111m222m333m444m 5m",
        win: "5万",
        must: &[
            ("一色四节高", FAN_YI_SE_SI_JIE_GAO),
            ("四暗刻", FAN_SI_AN_KE),
            ("清一色", FAN_QING_YI_SE),
            ("幺九刻", FAN_YAO_JIU_KE),
        ],
        must_not: &["一色三节高", "碰碰和", "三暗刻", "双暗刻", "门前清"],
        repeat: &[],
    });
}

/// `一色四步高`（32）：4 副递增顺子；**应吞 `连六`/`老少副`**（互斥表 `EXCLUSIONS` 的 `一色四步高` 条目）。
#[test]
fn r07b3_yi_se_si_bu_gao() {
    run(&Case {
        label: "一色四步高",
        hand: "123m234m345m456m 5m",
        win: "5万",
        must: &[
            ("一色四步高", FAN_YI_SE_SI_BU_GAO),
            ("清一色", FAN_QING_YI_SE),
            ("平和", FAN_PING_HE),
            ("四归一", FAN_SI_GUI_YI),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &["连六", "老少副", "一色三步高"],
        repeat: &[],
    });
}

/// 补强（批 1 已覆盖，此处加独立正例）：`一色三节高`（24）。
#[test]
fn r07b3_yi_se_san_jie_gao() {
    run(&Case {
        label: "一色三节高",
        hand: "111m222m333m 456s 9p",
        win: "9筒",
        must: &[
            ("一色三节高", FAN_YI_SE_SAN_JIE_GAO),
            ("三暗刻", FAN_SAN_AN_KE),
            ("幺九刻", FAN_YAO_JIU_KE),
            ("单钓将", FAN_DAN_DIAO_JIANG),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &["一色四节高", "一色三同顺"],
        repeat: &[],
    });
}

/// `一色三步高`（16）：3 副递增顺子。
#[test]
fn r07b3_yi_se_san_bu_gao() {
    run(&Case {
        label: "一色三步高",
        hand: "123m234m345m 456s 9p",
        win: "9筒",
        must: &[
            ("一色三步高", FAN_YI_SE_SAN_BU_GAO),
            ("平和", FAN_PING_HE),
            ("单钓将", FAN_DAN_DIAO_JIANG),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &["一色四步高", "一色三节高"],
        repeat: &[],
    });
}

/// `三色三同顺`（8）：三门各一副相同顺子；**应吞 `喜相逢`**（互斥表 `EXCLUSIONS` 的 `三色三同顺` 条目）。
#[test]
fn r07b3_san_se_san_tong_shun() {
    run(&Case {
        label: "三色三同顺",
        hand: "123m 123s 123p 456m 9s",
        win: "9条",
        must: &[
            ("三色三同顺", FAN_SAN_SE_SAN_TONG_SHUN),
            ("连六", FAN_LIAN_LIU),
            ("平和", FAN_PING_HE),
            ("单钓将", FAN_DAN_DIAO_JIANG),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &["喜相逢", "一般高", "三色三节高"],
        repeat: &[],
    });
}

/// `三色三节高`（8）：三门各一副递增刻子。
#[test]
fn r07b3_san_se_san_jie_gao() {
    run(&Case {
        label: "三色三节高",
        hand: "111m 222s 333p 456m 9s",
        win: "9条",
        must: &[
            ("三色三节高", FAN_SAN_SE_SAN_JIE_GAO),
            ("三暗刻", FAN_SAN_AN_KE),
            ("幺九刻", FAN_YAO_JIU_KE),
            ("单钓将", FAN_DAN_DIAO_JIANG),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &["三色三同顺", "三同刻", "一色三节高"],
        repeat: &[],
    });
}

/// `三色三步高`（6）：三门各一副递增顺子。
#[test]
fn r07b3_san_se_san_bu_gao() {
    run(&Case {
        label: "三色三步高",
        hand: "123m 234s 345p 456m 9s",
        win: "9条",
        must: &[
            ("三色三步高", FAN_SAN_SE_SAN_BU_GAO),
            ("连六", FAN_LIAN_LIU),
            ("平和", FAN_PING_HE),
            ("单钓将", FAN_DAN_DIAO_JIANG),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &["三色三节高", "三色三同顺"],
        repeat: &[],
    });
}

/// `连七对`（88）：同门连续 7 对；**应吞 `清一色`/`七对`/`单钓将`**（互斥表 `EXCLUSIONS` 的 `连七对` 条目）。
#[test]
fn r07b3_lian_qi_dui() {
    run(&Case {
        label: "连七对",
        hand: "1122334455667m",
        win: "7万",
        must: &[("连七对", FAN_LIAN_QI_DUI)],
        must_not: &["清一色", "七对", "单钓将", "门前清"],
        repeat: &[],
    });
}

// ==================== 控制行：只含低阶 ⇒ 低阶不得被误删 ====================

/// 控制行 1：`一般高`（两副相同顺子，无三/四同顺）。
#[test]
fn r07b3_control_ban_gao() {
    run(&Case {
        label: "控制-一般高",
        hand: "123m123m 456s 789p 9p",
        win: "9筒",
        must: &[
            ("一般高", FAN_YI_BAN_GAO),
            ("平和", FAN_PING_HE),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &["一色三同顺", "一色四同顺"],
        repeat: &[],
    });
}

/// 控制行 2：`连六`（同门相差 3 的两副顺子）。
#[test]
fn r07b3_control_lian_liu() {
    run(&Case {
        label: "控制-连六",
        hand: "123m 456m 789p 99s 55p",
        win: "5筒",
        must: &[("连六", FAN_LIAN_LIU), ("无字", FAN_WU_ZI)],
        must_not: &["一色四步高", "一色三步高", "老少副"],
        repeat: &[],
    });
}

/// 控制行 3：`喜相逢`（两门相同顺子，未达三色三同顺）。
#[test]
fn r07b3_control_xi_xiang_feng() {
    run(&Case {
        label: "控制-喜相逢",
        hand: "123m 123s 456m 789p 9s",
        win: "9条",
        must: &[
            ("喜相逢", FAN_XI_XIANG_FENG),
            ("平和", FAN_PING_HE),
            ("单钓将", FAN_DAN_DIAO_JIANG),
            ("门前清", FAN_MEN_QIAN_QING),
        ],
        must_not: &["三色三同顺"],
        repeat: &[],
    });
}

/// 控制行 4：普通 `七对`（非连七对）。
#[test]
fn r07b3_control_qi_dui() {
    run(&Case {
        label: "控制-七对",
        hand: "112233m 445566p 9s",
        win: "9条",
        must: &[("七对", FAN_QI_DUI), ("无字", FAN_WU_ZI)],
        must_not: &["连七对", "清一色"],
        repeat: &[],
    });
}

// ==================== 观察：`一色三同顺` 在本引擎不可达（**非缺陷**，待裁决） ====================

/// **观察（B2 已升级）**：任何「3 副相同且相邻的顺子」的手牌，都可等价重读为 3 副递增刻子
/// （`123m×3` ⇔ `111m+222m+333m`）；后者额外成立 `三暗刻 16`，总值**严格更高**（24+16 > 24），
/// 故按「就高不就低」原则，引擎**恒选刻子分解** ⇒ `一色三同顺`（24）**不会被产出**。
///
/// 本用例**固化当前行为**并留证：`一色三节高 24` + `三暗刻 16` 在场，`一色三同顺` **不在**。
/// **请 Lead/用户裁决**：这是否属「番种表可达性问题」（例如需要在等值/更高分解下保留番种表顺序、
/// 或改为并列展示），抑或按「就高原则」属正确行为（则 `一色三同顺` 记录为**结构性不可达**）。
#[test]
fn r07b3_observation_yi_se_san_tong_shun_unreachable() {
    let h = parse_hand("123m123m123m 456s 9p");
    let an = analyze(&h, &[], 27, 27);
    let w = by_name(&an, "9筒");
    assert!(
        has(w, "一色三节高") && val(w, "一色三节高") == FAN_YI_SE_SAN_JIE_GAO,
        "{:?}",
        names(w)
    );
    assert!(
        has(w, "三暗刻") && val(w, "三暗刻") == FAN_SAN_AN_KE,
        "{:?}",
        names(w)
    );
    assert!(
        !has(w, "一色三同顺"),
        "当前实现下 `一色三同顺` 不应出现（刻子分解更优）；若已改为可达，请更新本观察并回报: {:?}",
        names(w)
    );
    // 另一个形状（234m×3）同样不可达，佐证与具体数字无关
    let h2 = parse_hand("234m234m234m 567s 9p");
    let an2 = analyze(&h2, &[], 27, 27);
    let w2 = by_name(&an2, "9筒");
    assert!(
        has(w2, "一色三节高") && !has(w2, "一色三同顺"),
        "{:?}",
        names(w2)
    );
}
