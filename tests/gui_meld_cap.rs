//! L1 回归：副露组数超过 4（真实 UI 可达：反复碰不同牌，per-tile 限 4 张但不限组数）时，
//! 引擎不得 panic；GUI 侧 `src/bin/gbmj_gui.rs` 已改为不把预期暗牌张数静默饱和成 0。
//! import 风格照抄 `tests/phantom_wait.rs`（`gbmj::calc::analyze` + `gbmj::model::Meld` + `gbmj::tiles::*`）。
//!
//! L3-C 第一步实测（`cargo test --no-default-features --test gui_meld_cap -- --nocapture`）：
//!   5 组副露 + 空暗牌 ⇒ structural_waits.len()=0、waits.len()=0（引擎不 panic、返回空结果，
//!   故无需改引擎；L1 缺陷纯在 GUI 的 saturating_sub 饱和）；
//!   4 组副露 + 单张暗牌 ⇒ structural_waits=[0]（1万，单钓）。
use gbmj::calc::analyze;
use gbmj::model::Meld;
use gbmj::tiles::*;

/// ① 5 组副露 + 空暗牌（GUI 在 melds=5 时正是把空暗牌送进来）：
///    不 panic，且结构听牌与和牌张均为空，不得给出会被当成「可计算」的荒谬结果。
#[test]
fn analyze_with_five_melds_does_not_panic() {
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
fn analyze_with_four_melds_still_works() {
    let hand = parse_hand("1m"); // 13 - 3*4 = 1 张暗牌
    let melds = vec![Meld::pon(1), Meld::pon(2), Meld::pon(3), Meld::pon(4)];
    let a = analyze(&hand, &melds, 0, 0);
    assert_eq!(a.structural_waits, vec![0], "4 组副露时 1万 应仍是结构听牌");
}
