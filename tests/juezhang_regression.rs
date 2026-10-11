//! R-07 收尾：`和绝张` 的名字级断言（开发补齐 `fans_last` / `fans_tsumo_last` 后）
//!
//! 覆盖与口径：
//! - `和绝张` = **4 番**，出现在 `fans_last`（点和绝张）与 `fans_tsumo_last`（自摸绝张）两张新表中。
//! - **取值语义（开发定，我已复核认可，见 `docs/测试评估报告.md` §20）**：
//!   两表**无条件按 `score()` 结果填充**，`can_last == false` 时**照样存表、不清空**——
//!   与 `last`/`tsumo_last` 数值「照算、由展示层决定显示 `—`」的既有做法同源同规则；
//!   **适用性由消费方按 `can_last` / `verdict()` 判断**。
//! - 数值护栏（辅助）：常规口径下 `L == N + 4`、`TL == T + 4`；**例外**：非门清且 base=0 时
//!   `N = 8`（无番和，仅点和）而 `L = 4 = 和绝张`，`L ≠ N + 4`（见 `juezhang_no_wufanhe_on_last_path`）。
//!
//! 只**新增**，不修改既有 136 条用例。

use gbmj::calc::{analyze, Analysis, WaitResult};
use gbmj::fans::Fan;
use gbmj::model::Meld;
use gbmj::tiles::*;

fn by_name<'a>(an: &'a Analysis, name: &str) -> &'a WaitResult {
    an.waits
        .iter()
        .find(|w| tile_name(w.tile) == name)
        .unwrap_or_else(|| panic!("无此和牌张: {}", name))
}

fn names_of(v: &[Fan]) -> Vec<String> {
    v.iter().map(|f| f.name.to_string()).collect()
}

fn has_of(v: &[Fan], n: &str) -> bool {
    v.iter().any(|f| f.name == n)
}

fn val_of(v: &[Fan], n: &str) -> u32 {
    v.iter().filter(|f| f.name == n).map(|f| f.value).sum()
}

/// **主用例**：`can_last == true` 的行（DEFECTC 的 `6条`）——
/// `fans_last` 必须**含 `和绝张` 且值 4**，且**恰为「点和番表 + 和绝张」**（同源组合）。
#[test]
fn juezhang_fans_last_contains_he_jue_zhang() {
    let h = parse_hand("111m 234s 45s 789s 99m");
    let an = analyze(&h, &[], 27, 27);
    let w = by_name(&an, "6条");

    assert!(w.can_last, "6条 应可绝张: {:?}", w);
    assert!(has_of(&w.fans_last, "和绝张"), "fans_last 应含和绝张: {:?}", names_of(&w.fans_last));
    assert_eq!(val_of(&w.fans_last, "和绝张"), 4, "和绝张 = 4 番");

    // 组合关系：fans_last == fans + 一个 和绝张4
    let mut expect = names_of(&w.fans);
    expect.push("和绝张".to_string());
    assert_eq!(names_of(&w.fans_last), expect, "fans_last 应为 点和番表 + 和绝张");

    // 数值护栏（辅助）
    assert_eq!(w.last, w.normal + 4, "L == N + 4");
}

/// `fans_tsumo_last`（自摸绝张）同样含 `和绝张4`，且为「自摸番表 + 和绝张」。
#[test]
fn juezhang_fans_tsumo_last_contains_he_jue_zhang() {
    let h = parse_hand("111m 234s 45s 789s 99m");
    let an = analyze(&h, &[], 27, 27);
    let w = by_name(&an, "6条");

    assert!(has_of(&w.fans_tsumo_last, "和绝张"), "{:?}", names_of(&w.fans_tsumo_last));
    assert_eq!(val_of(&w.fans_tsumo_last, "和绝张"), 4);
    let mut expect = names_of(&w.fans_tsumo);
    expect.push("和绝张".to_string());
    assert_eq!(names_of(&w.fans_tsumo_last), expect);
    assert_eq!(w.tsumo_last, w.tsumo + 4, "TL == T + 4");
}

/// **取值语义锁定（`can_last == false`）**：DEFECTC 的 `3条`（该张已握在暗牌里 ⇒ 绝张世界不成立）
/// ⇒ **数值照旧非零、两表照样填充**（与数值同源），适用性由消费方按 `can_last` 判断（展示为 `—`）。
#[test]
fn juezhang_can_last_false_still_filled_semantics() {
    let h = parse_hand("111m 234s 45s 789s 99m");
    let an = analyze(&h, &[], 27, 27);
    let w = by_name(&an, "3条");

    assert!(!w.can_last, "3条 暗牌已握 1 张 ⇒ 绝张不成立");
    // 数值与该栏语义：照算、非 0
    assert!(w.last > 0 && w.tsumo_last > 0, "L/TL 仍照算: {:?}", w);
    assert_eq!(w.last, w.normal + 4);
    // 两表照样填充（**不清空**）——这正是开发登记的语义
    assert!(has_of(&w.fans_last, "和绝张"), "can_last=false 时 fans_last 仍填充: {:?}", names_of(&w.fans_last));
    assert!(has_of(&w.fans_tsumo_last, "和绝张"), "{:?}", names_of(&w.fans_tsumo_last));
    // 适用性由消费方门控：该行 verdict 为 NoWin（绝张世界不参与判定）
    assert_eq!(w.verdict(), gbmj::calc::WinVerdict::NoWin);
}

/// **非门清 base=0 的关键例外**（T1-14A 的 `9条`）：`N = 8`（无番和，仅点和）而 `L = 4`。
/// ⇒ 绝张路径上**只有 `和绝张4`、不得出现 `无番和`**（口径：无番和仅点和）。
/// 这是该口径首次以**名字级**证据锁定。
#[test]
fn juezhang_no_wufanhe_on_last_path() {
    let h = parse_hand("78s 东东");
    let melds = vec![Meld::chi(1), Meld::pon(22), Meld::pon(14)]; // 吃234m / 碰5筒 / 碰6条
    let an = analyze(&h, &melds, 27, 27);
    let w = by_name(&an, "9条");

    assert!(w.can_last, "{:?}", w);
    assert!(has_of(&w.fans, "无番和") && val_of(&w.fans, "无番和") == 8, "点和为无番和8: {:?}", names_of(&w.fans));
    // 绝张路径：仅 和绝张4，**无无番和**
    assert_eq!(names_of(&w.fans_last), vec!["和绝张".to_string()], "{:?}", w);
    assert!(!has_of(&w.fans_last, "无番和"), "无番和仅点和，不得出现在绝张路径: {:?}", names_of(&w.fans_last));
    assert_eq!((w.normal, w.last), (8, 4), "N=8 / L=4（L ≠ N+4 的例外）");
    assert_eq!(w.last, 4, "L 必须为 4（绝张路径：和绝张4 起）");
    // 自摸绝张：自摸1 + 和绝张4（亦无无番和）
    assert_eq!(names_of(&w.fans_tsumo_last), vec!["自摸".to_string(), "和绝张".to_string()], "{:?}", w);
    // **数值-番表同源**（直接校验，而非结构推断）：L/TL 应等于各自番表的番值和
    let sum = |v: &[Fan]| v.iter().map(|f| f.value).sum::<u32>();
    assert_eq!(w.last, sum(&w.fans_last), "L 应等于 fans_last 番值和: {:?}", names_of(&w.fans_last));
    assert_eq!(w.normal, sum(&w.fans), "N 应等于 fans 番值和");
    assert_eq!(w.tsumo_last, sum(&w.fans_tsumo_last), "TL 应等于 fans_tsumo_last 番值和");
    assert_eq!(w.tsumo, sum(&w.fans_tsumo), "T 应等于 fans_tsumo 番值和");
}

/// 非门清 + `can_last == true` 的另一族（三杠手）：`L == N + 4` 与 `和绝张4` 同时成立。
#[test]
fn juezhang_non_menqing_can_last_true() {
    let h = parse_hand("55p 67m");
    let melds = vec![Meld::kan(0, true), Meld::kan(8, true), Meld::kan(9, true)]; // 3 副明杠
    let an = analyze(&h, &melds, 27, 27);
    let w = by_name(&an, "5万");

    assert!(w.can_last, "{:?}", w);
    assert!(!an.menqing, "本手非门清");
    assert_eq!(val_of(&w.fans_last, "和绝张"), 4, "{:?}", names_of(&w.fans_last));
    assert_eq!(w.last, w.normal + 4, "非门清 base>0：L == N + 4");
    assert_eq!(w.tsumo_last, w.tsumo + 4, "TL == T + 4");
    // 组合关系（与其余用例对齐）：fans_last == fans + 和绝张
    let mut expect = names_of(&w.fans);
    expect.push("和绝张".to_string());
    assert_eq!(names_of(&w.fans_last), expect, "非门清行同样满足「常规番表 + 和绝张」");
}
