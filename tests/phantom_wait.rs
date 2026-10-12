//! T16 幻影听牌回归：副露/杠已占满 4 张的牌，不得再被算作结构听牌。
//!
//! import 风格照抄 `tests/engine.rs`（`gbmj::calc::analyze` + `gbmj::model::Meld`
//! + `gbmj::tiles::*` 的 `parse_hand` / `tile_name`），helper 风格照抄
//!   `tests/plan_regression.rs` 的 `names`（本文件独立，不共用其它测试文件的 item）。
//!
//! 索引表：万 0-8、条 9-17、筒 18-26、风 27-30、箭 31-33。
//! 本文用到：5筒=22、6筒=23、2筒=19、5条=13、6条=14、9条=17。

use gbmj::calc::analyze;
use gbmj::model::Meld;
use gbmj::tiles::*;

fn waits(hand: &str, melds: Vec<Meld>) -> Vec<usize> {
    analyze(&parse_hand(hand), &melds, 0, 0).structural_waits
}

fn names(v: &[usize]) -> String {
    v.iter()
        .map(|&t| tile_name(t))
        .collect::<Vec<_>>()
        .join(",")
}

/// 147m 258s 369p 5p --pon 5p：碰占 5筒×3 + 暗牌 1，和牌后 5 张 ⇒ 22 不是听牌。
#[test]
fn phantom_pong_occupied_tile_is_dropped() {
    let w = waits("147m 258s 369p 5p", vec![Meld::pon(22)]);
    assert!(
        !w.contains(&22),
        "5筒已被碰占满 4 张，不得算听牌：{}",
        names(&w)
    );
    assert!(w.is_empty(), "该手不应有结构听牌，实得 {}", names(&w));
}

/// 123s 5s --chi 345s ×3：吃占 5条×3 + 暗牌 1，和牌后 5 张 ⇒ 无结构听牌。
#[test]
fn phantom_chi_occupied_tile_is_dropped() {
    let melds = vec![Meld::chi(11), Meld::chi(11), Meld::chi(11)];
    let w = waits("123s 5s", melds);
    assert!(
        w.is_empty(),
        "5条已被吃占满 4 张，不应有结构听牌：{}",
        names(&w)
    );
}

/// A2 原例：123m456m34p99p --ankan 5p（暗杠 5筒×4）⇒ 5筒被剔除、2筒保留。
#[test]
fn phantom_ankan_occupied_tile_is_dropped() {
    let w = waits("123m456m34p99p", vec![Meld::kan(22, false)]);
    assert!(!w.contains(&22), "5筒已被暗杠占满 4 张：{}", names(&w));
    assert!(w.contains(&19), "2筒是合法听牌，必须保留：{}", names(&w));
}

/// ⭐下边界：碰占 5筒×3、暗牌 0 张 5筒、听 5筒 嵌张，和牌后恰 4 张 ⇒ 必须保留。
#[test]
fn borderline_three_occupied_then_waiting_that_tile_is_kept() {
    let w = waits("456m 789s 46p 东东", vec![Meld::pon(22)]);
    assert!(
        w.contains(&22),
        "占用恰 3 且听该张，和牌后 4 张应合法保留：{}",
        names(&w)
    );
}

/// 反例：占用别的牌不得遮蔽其它听牌。
#[test]
fn counter_example_occupied_meld_does_not_hide_other_waits() {
    let a = waits("147m 258s 369p 6p", vec![Meld::pon(22)]);
    assert!(a.contains(&23), "占用 5筒不应遮蔽 6筒 听牌：{}", names(&a));

    let b = waits("147m 258s 369p 5p", vec![Meld::chi(0)]);
    assert!(
        b.contains(&22),
        "吃 1万2万3万 不应遮蔽 5筒 听牌：{}",
        names(&b)
    );
}

/// 门禁基线 T1-14A：78s 东东 --chi 234m --pon 5p --pon 6s ⇒ 听 6条、9条。
#[test]
fn meld_occupancy_edge_case_gate_baseline_t1_14a() {
    let melds = vec![Meld::chi(1), Meld::pon(22), Meld::pon(14)];
    let w = waits("78s 东东", melds);
    assert!(w.contains(&14), "6条应保留：{}", names(&w));
    assert!(w.contains(&17), "9条应保留：{}", names(&w));
    assert_eq!(w.len(), 2, "结构听牌应恰为 6条/9条：{}", names(&w));
}
