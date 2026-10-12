//! 引擎特征化测试（characterization test）：记录 `analyze` 收到「副露超过 4 组」时的**既有行为**
//! ——安全空返回、**不 panic**。副露组数超过 4 在真实 UI 中可达（反复碰不同牌：per-tile 限 4 张，
//! 但不限组数），故这一行为值得被钉住。
//!
//! **本文件不构成 GUI 侧回归护栏**：它直接调引擎，而引擎在本批修复前后行为相同
//! （修复侧只改了 GUI 的预期张数计算，未动引擎）。GUI 侧的真护栏是：
//!   ① `expected_hidden() -> Option<usize>` 的**类型强制**（`None` 必须被显式处理）；
//!   ② 3 处 UI 分支的明确错误文案「副露最多 4 组（当前 N 组），无法计算」；
//!   ③ `GBMJ_DEMO=21` / `22` 的窗口级截图 + OCR 读图证据。
//! 详见 `docs/测试评估报告.md` §二十五（T23 独立点验 §25.2）与本批交付报告。
//!
//! 历史：本文件前身名为 `gui_meld_cap.rs`（**名实不符**——名字含 GUI、实际只测引擎），T24 改名。
//!
//! 实测记录（`cargo test --no-default-features --test meld_cap_engine -- --nocapture`）：
//!   5 组副露 + 空暗牌 ⇒ structural_waits.len()=0、waits.len()=0；
//!   4 组副露 + 单张暗牌 ⇒ structural_waits=[0]（1万 单钓）。
use gbmj::calc::analyze;
use gbmj::model::Meld;
use gbmj::tiles::*;

/// ① 5 组副露 + 空暗牌：不 panic，且结构听牌与和牌张均为空，
///    不得给出会被当成「可计算」的荒谬结果。
#[test]
fn analyze_with_five_melds_returns_empty_without_panic() {
    let hand = counts(); // 空暗牌
    let melds = vec![
        Meld::pon(0),
        Meld::pon(1),
        Meld::pon(2),
        Meld::pon(3),
        Meld::pon(4),
    ];
    let a = analyze(&hand, &melds, 0, 0);
    assert!(
        a.structural_waits.is_empty(),
        "5 组副露时应无结构听牌，实际 {:?}",
        a.structural_waits
    );
    assert!(
        a.waits.is_empty(),
        "5 组副露时应无和牌张行，实际 {} 行",
        a.waits.len()
    );
    assert!(
        a.structural_waits.len() <= NUM_TILES,
        "structural_waits 不应超上界"
    );
}

/// ② 4 组副露的正常情形仍应得到非空结果（防修过头）：暗牌 1万 单钓 1万。
#[test]
fn analyze_with_four_melds_still_produces_targets() {
    let hand = parse_hand("1m"); // 13 - 3*4 = 1 张暗牌
    let melds = vec![Meld::pon(1), Meld::pon(2), Meld::pon(3), Meld::pon(4)];
    let a = analyze(&hand, &melds, 0, 0);
    assert_eq!(a.structural_waits, vec![0], "4 组副露时 1万 应仍是结构听牌");
}

/// ③ S1 收口（**行为锁**，非「曾复现缺陷」）：**「2 张暗牌 + 5 组副露」必须得到空结果**。
/// 事实澄清（L2 负向自证）：**去掉** `winning_decomps` 的 `m > 4` 早返回后，本用例**仍然通过**
/// （连暗牌为对子 `1m 1m` 时亦然）⇒「m=5 时 `need` 饱和为 0 会产出 5 面子非法分解」属
/// **未证实的理论路径**（`decompose_standard(c, 0)` 本身不产出分解）。本用例把「空结果」钉住，
/// 防止将来该函数行为变化时静默产出非法分解。
#[test]
fn analyze_with_five_melds_and_two_tiles_is_empty() {
    let hand = parse_hand("1m 2m"); // 2 张暗牌（正是 m=5 时饱和式曾放行的张数）
    let melds = vec![
        Meld::pon(0),
        Meld::pon(1),
        Meld::pon(2),
        Meld::pon(3),
        Meld::pon(4),
    ];
    let a = analyze(&hand, &melds, 0, 0);
    assert!(
        a.structural_waits.is_empty(),
        "5 组副露 + 2 张暗牌不得产出结构听牌（防 5 面子非法分解），实际 {:?}",
        a.structural_waits
    );
    assert!(
        a.waits.is_empty(),
        "5 组副露 + 2 张暗牌不得产出和牌张行，实际 {} 行",
        a.waits.len()
    );
}
