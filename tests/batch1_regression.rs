//! 第 1 批修复的回归与覆盖用例（DL-2 / DL-3）
//!
//! 范围：
//! - DL-2：R-01（三态）/ R-02（荣和暗刻）/ R-11（组合龙分支杠系番+刻子类番）/ R-12（三条互斥）回归
//! - DL-3：T-2 批 1 的 8 个番种覆盖（双暗刻、四暗刻、四归一、断幺、全大、全小、大于五、小于五）
//!
//! 约定（依 Lead 工作指令）：
//! - **全部使用库级断言**（`analyze()` 取番名/番数），**不断言 CLI 文本**（CLI 仅在部分行打印番表明细）。
//! - 只新增，不修改既有 `tests/engine.rs` / `tests/plan.rs` 的 47 条用例。
//! - 期望值来自 `docs/测试方案.md` v2.4（规则推导）+ 验证轮实测。

use gbmj::calc::{analyze, Analysis, WaitResult, WinVerdict};
use gbmj::model::Meld;
use gbmj::tiles::*;

fn by_name<'a>(an: &'a Analysis, name: &str) -> &'a WaitResult {
    an.waits
        .iter()
        .find(|w| tile_name(w.tile) == name)
        .unwrap_or_else(|| panic!("无此和牌张: {}", name))
}

fn names(w: &WaitResult) -> Vec<String> {
    w.fans.iter().map(|f| f.name.to_string()).collect()
}

fn has_fan(w: &WaitResult, n: &str) -> bool {
    w.fans.iter().any(|f| f.name == n)
}

fn fan_value(w: &WaitResult, n: &str) -> u32 {
    w.fans.iter().filter(|f| f.name == n).map(|f| f.value).sum()
}

// ============================ DL-2：R-01 三态 ============================

/// R-01 回归（锁死原缺陷 C）：`can_win()` 必须门控绝张栏。
/// 手 `111m 234s 45s 789s 99m`：和 3条 时暗牌里已有 1 张 3条 ⇒ `can_last=false`
/// ⇒ 隐藏的绝张栏（9 番）不得使能「可和」。
#[test]
fn r01_guards_three_state_no_win_when_last_hidden() {
    let c = parse_hand("111m 234s 45s 789s 99m");
    assert_eq!(total(&c), 13);
    let an = analyze(&c, &[], 27, 27);
    let w = by_name(&an, "3条");
    assert_eq!((w.normal, w.tsumo), (5, 7), "{:?}", names(w));
    assert!(!w.can_last, "暗牌含 1 张 3条 ⇒ 绝张不成立");
    assert_eq!(w.verdict(), WinVerdict::NoWin, "{:?}", names(w));
    assert!(!w.can_win(), "三点和/自摸均 <8 且绝张栏隐藏 ⇒ 不得可和");
    // 同手 6条：自摸 8 番可和，未受波及
    let w6 = by_name(&an, "6条");
    assert_eq!((w6.normal, w6.tsumo), (6, 8), "{:?}", names(w6));
    assert!(w6.can_last);
    assert_eq!(w6.verdict(), WinVerdict::Win);
}

/// R-01 三态：`△ 条件可和`（仅非绝张世界）与对照（`g_last=false` ⇒ 无条件可和）。
/// 副露 吃234m / 碰5筒 / 碰6条 + 暗牌 `78s 东东`。
#[test]
fn r01_conditional_last_and_unconditional_control() {
    let chi_234m = Meld::chi(1);
    let pon_5p = Meld::pon(18 + 4);
    let pon_6s = Meld::pon(9 + 5);

    // A1：concealed[6条]==0（3 张全在碰里）、concealed[9条]==0
    let c = parse_hand("78s 东东");
    assert_eq!(total(&c), 4, "3 副露 ⇒ 暗牌 4 张");
    let melds = vec![chi_234m, pon_5p, pon_6s];
    let an = analyze(&c, &melds, 27, 27);
    assert!(!an.menqing);

    // 9条：base=0 ⇒ 8/1/4/5，非绝张世界可和、绝张世界不可和 ⇒ 条件可和
    let w9 = by_name(&an, "9条");
    assert_eq!(
        (w9.normal, w9.tsumo, w9.last, w9.tsumo_last),
        (8, 1, 4, 5),
        "{:?}",
        names(w9)
    );
    assert!(w9.can_last);
    assert_eq!(w9.verdict(), WinVerdict::ConditionalLast, "{:?}", names(w9));
    assert!(!w9.can_win(), "条件可和 ≠ 确定能和（D3）");
    assert!(has_fan(w9, "无番和"), "{:?}", names(w9));

    // 6条：和牌张使 6条 达 4 张且未成杠 ⇒ 四归一2 ⇒ base=2 ⇒ 不可和
    let w6 = by_name(&an, "6条");
    assert_eq!(
        (w6.normal, w6.tsumo, w6.last, w6.tsumo_last),
        (2, 3, 6, 7),
        "{:?}",
        names(w6)
    );
    assert!(w6.can_last);
    assert_eq!(w6.verdict(), WinVerdict::NoWin, "{:?}", names(w6));
    // 库级确证：6条 行的 base=2 完全来自 四归一2（而非算术推测）
    assert_eq!(fan_value(w6, "四归一"), 2, "{:?}", names(w6));

    // B 对照：暗牌换成 77s ⇒ concealed[7条]==2 ⇒ g_last=false ⇒ 无条件可和
    let c2 = parse_hand("77s 东东");
    let an2 = analyze(&c2, &melds, 27, 27);
    let w7 = by_name(&an2, "7条");
    assert_eq!(
        (w7.normal, w7.tsumo, w7.last, w7.tsumo_last),
        (8, 1, 4, 5),
        "{:?}",
        names(w7)
    );
    assert!(!w7.can_last);
    assert_eq!(w7.verdict(), WinVerdict::Win, "{:?}", names(w7));
    assert!(w7.can_win());
}

/// R-01（撤销缺陷 B 后锁定）：暗牌握 3 张同一张牌时，绝张**不成立**、两栏不得参与判定。
/// 用户裁决口径：绝张 = 牌池与副露区出现 3 张的结果；手上有的一定不是绝张。
#[test]
fn r01_holding_three_copies_is_never_last_tile() {
    let c = parse_hand("111m 23m 456m 999s 55p");
    let an = analyze(&c, &[], 27, 27);
    let w = by_name(&an, "1万");
    assert_eq!(total(&c), 13);
    assert!(!w.can_last, "暗牌已握 3 张 1万 ⇒ 不是绝张");
    assert_eq!((w.normal, w.tsumo), (10, 12), "{:?}", names(w));
    assert_eq!(w.verdict(), WinVerdict::Win, "由点和 10 番支撑");
    // 7万 暗牌 0 张 ⇒ 绝张假设成立，两栏参与
    let w7 = by_name(&an, "7万");
    assert!(w7.can_last);
    assert_eq!((w7.last, w7.tsumo_last), (12, 14), "{:?}", names(w7));
}

// ============================ DL-2：R-02 荣和暗刻 ============================

/// R-02 回归：和牌张补进**顺子**时，手中既有暗刻不得被误判为明刻。
/// `111m 23m 456m 999s 55p` 和 1万：唯一分解为 将55筒/111万/123万/456万/999条，
/// 和牌张 1万 进 123万 顺子 ⇒ 111万 仍是暗刻 ⇒ 双暗刻 成立、点和 = 10。
#[test]
fn r02_ron_win_tile_in_run_keeps_concealed_triplet() {
    let c = parse_hand("111m 23m 456m 999s 55p");
    let an = analyze(&c, &[], 27, 27);
    let w = by_name(&an, "1万");
    assert_eq!(w.normal, 10, "{:?}", names(w));
    assert!(has_fan(w, "双暗刻"), "{:?}", names(w));
    assert!(has_fan(w, "四归一"), "四归一与双暗刻不互斥: {:?}", names(w));
    // 同手其它和牌张未受修复波及
    assert_eq!(by_name(&an, "4万").normal, 7);
    assert_eq!(by_name(&an, "7万").normal, 8);
}

/// R-02：三暗刻场景不得被降级为双暗刻（同缺陷的另一表现面）。
#[test]
fn r02_three_concealed_triplets_not_downgraded() {
    let c = parse_hand("111m 23m 999m 888s 55p");
    let an = analyze(&c, &[], 27, 27);
    let w = by_name(&an, "1万");
    assert_eq!(w.normal, 23, "{:?}", names(w));
    assert!(has_fan(w, "三暗刻"), "{:?}", names(w));
    assert!(!has_fan(w, "双暗刻"), "三暗刻已含，不应并列: {:?}", names(w));
}

/// R-02 反向护栏（防修复过头）：和牌张确实把**对子补成刻子**时（`concealed[t]==2`），
/// 该刻子必须仍按**明刻**处理。
#[test]
fn r02_reverse_guard_pair_completed_triplet_is_open() {
    // 双碰听：荣和 6条 补成 666s ⇒ 明刻（点炮 12），自摸才算暗刻（28）
    let c = parse_hand("555m 66s 777p 123m 东东");
    let an = analyze(&c, &[], 27, 27);
    let w = by_name(&an, "6条");
    assert_eq!(w.normal, 12, "{:?}", names(w));
    assert_eq!(w.tsumo, 28, "{:?}", names(w));
    assert!(!has_fan(w, "三暗刻"), "荣和补成的刻子是明刻: {:?}", names(w));
    assert!(has_fan(w, "双暗刻"), "{:?}", names(w));
    // 肯定式对照（自摸）：666s 由自摸补成 ⇒ 暗刻 ⇒ 出现 三暗刻
    assert!(
        w.fans_tsumo.iter().any(|f| f.name == "三暗刻"),
        "自摸时 666s 为暗刻 ⇒ 应有 三暗刻: {:?}",
        w.fans_tsumo.iter().map(|f| f.name).collect::<Vec<_>>()
    );

    // 单钓将：和 5筒 把 55筒 补成 555筒 ⇒ 荣和算明刻、自摸才算暗刻（肯定式双向对照）
    let c2 = parse_hand("111m 23m 456m 999s 55p");
    let an2 = analyze(&c2, &[], 27, 27);
    let w5 = by_name(&an2, "5筒");
    assert_eq!(w5.normal, 5, "{:?}", names(w5));
    // 点炮：555筒 是明刻 ⇒ 只剩 999条 一个暗刻，不得出现暗刻类番
    assert!(!has_fan(w5, "双暗刻"), "荣和补成的 555筒 是明刻: {:?}", names(w5));
    assert!(!has_fan(w5, "三暗刻"), "{:?}", names(w5));
    // 自摸对照：555筒 变暗刻 ⇒ 双暗刻 必须出现（且差值 = 双暗刻2 + 不求人4 − 门前清2）
    assert!(
        w5.fans_tsumo.iter().any(|f| f.name == "双暗刻"),
        "自摸时 555筒 为暗刻: {:?}",
        w5.fans_tsumo.iter().map(|f| f.name).collect::<Vec<_>>()
    );
    assert_eq!(w5.tsumo - w5.normal, 4, "{:?} vs {:?}", names(w5), w5.fans_tsumo.iter().map(|f| f.name).collect::<Vec<_>>());
}

// ============================ DL-2：R-11 组合龙×杠 ============================

/// R-11 主验收：`M1` 暗杠 → 19/21，且补上 `暗杠2` + `幺九刻1`（组合龙分支此前完全不算）。
#[test]
fn r11_zuhelong_with_ankan_includes_kan_and_triplet_fans() {
    let c = parse_hand("147m 258s 369p 5p");
    assert_eq!(total(&c), 10);
    let melds = vec![Meld::kan(18, false)]; // 暗杠 1筒
    let an = analyze(&c, &melds, 27, 27);
    assert!(an.menqing, "暗杠不破门清");
    let w = by_name(&an, "5筒");
    assert_eq!((w.normal, w.tsumo), (19, 21), "{:?}", names(w));
    for f in ["组合龙", "暗杠", "幺九刻", "单钓将", "无字", "门前清"] {
        assert!(has_fan(w, f), "缺 {} : {:?}", f, names(w));
    }
}

/// R-11 `M2`（D5 裁定）：明杠 → **16/17**、非门清（明杠破门清 ⇒ 无 门前清2）。
#[test]
fn r11_zuhelong_with_mingkan_is_not_menqing() {
    let c = parse_hand("147m 258s 369p 5p");
    let melds = vec![Meld::kan(18, true)]; // 明杠 1筒
    let an = analyze(&c, &melds, 27, 27);
    assert!(!an.menqing, "明杠破门清");
    let w = by_name(&an, "5筒");
    assert_eq!((w.normal, w.tsumo), (16, 17), "{:?}", names(w));
    for f in ["组合龙", "明杠", "幺九刻"] {
        assert!(has_fan(w, f), "缺 {} : {:?}", f, names(w));
    }
    assert!(!has_fan(w, "门前清"), "非门清不计门前清: {:?}", names(w));
    assert!(!has_fan(w, "暗杠"), "{:?}", names(w));
}

/// R-11 刻子类番的判别：非幺九杠**不得**计幺九刻；字牌杠使 `无字` 失效，且圈风/门风可双计。
#[test]
fn r11_zuhelong_triplet_fan_discriminants() {
    let c = parse_hand("147m 258s 369p 5p");

    // M4：明杠 2筒（非幺九） ⇒ 15/16，无幺九刻
    let an4 = analyze(&c, &[Meld::kan(19, true)], 27, 27);
    let w4 = by_name(&an4, "5筒");
    assert_eq!((w4.normal, w4.tsumo), (15, 16), "{:?}", names(w4));
    assert!(!has_fan(w4, "幺九刻"), "2筒非幺九: {:?}", names(w4));
    assert!(has_fan(w4, "无字"), "{:?}", names(w4));

    // M5：暗杠 东（默认 round=东 seat=东）⇒ 圈风刻2 + 门风刻2，且 无字 失效
    let an5 = analyze(&c, &[Meld::kan(27, false)], 27, 27);
    let w5 = by_name(&an5, "5筒");
    assert_eq!((w5.normal, w5.tsumo), (21, 23), "{:?}", names(w5));
    assert_eq!(fan_value(w5, "圈风刻"), 2, "{:?}", names(w5));
    assert_eq!(fan_value(w5, "门风刻"), 2, "{:?}", names(w5));
    assert!(!has_fan(w5, "无字"), "含字牌 ⇒ 无字失效: {:?}", names(w5));
    assert!(!has_fan(w5, "幺九刻"), "圈/门风刻与幺九刻互斥: {:?}", names(w5));

    // M5b：东 既非圈风也非门风 ⇒ 降为 幺九刻1（18/20）
    let an5b = analyze(&c, &[Meld::kan(27, false)], 28, 29); // round=南 seat=西
    let w5b = by_name(&an5b, "5筒");
    assert_eq!((w5b.normal, w5b.tsumo), (18, 20), "{:?}", names(w5b));
    assert_eq!(fan_value(w5b, "幺九刻"), 1, "{:?}", names(w5b));

    // M5e：暗杠 中（箭牌）⇒ 箭刻2（19/21）
    let an5e = analyze(&c, &[Meld::kan(31, false)], 27, 27);
    let w5e = by_name(&an5e, "5筒");
    assert_eq!((w5e.normal, w5e.tsumo), (19, 21), "{:?}", names(w5e));
    assert_eq!(fan_value(w5e, "箭刻"), 2, "{:?}", names(w5e));
}

/// R-11 结构边界：`m > 1` 时组合龙不生成（`detect_zuhelong` 的 `m>1` 早退）。
/// 含**正控**（`m=1` 必须生成组合龙），避免「`waits` 为空 ⇒ 循环体不执行」的空洞断言。
#[test]
fn r11_zuhelong_requires_at_most_one_meld() {
    // 正控：同 9 张针织牌 + 1 副露 ⇒ 组合龙必须出现，且确实存在结构听牌
    let c1 = parse_hand("147m 258s 369p 5p");
    let an1 = analyze(&c1, &[Meld::kan(18, false)], 27, 27);
    assert!(!an1.waits.is_empty(), "正控：m=1 应有结构听牌");
    assert!(
        an1.waits.iter().any(|w| has_fan(w, "组合龙")),
        "正控：m=1 应生成组合龙"
    );

    // 反例：m=2 ⇒ 组合龙不生成；该结构下亦无其它和牌型（实测「未听牌」）
    let c2 = parse_hand("147m 258s 5p");
    assert_eq!(total(&c2), 7, "2 副露 ⇒ 暗牌 7 张");
    let melds2 = vec![Meld::kan(18, false), Meld::kan(17, true)]; // 暗杠1筒 + 明杠9条
    let an2 = analyze(&c2, &melds2, 27, 27);
    assert!(
        an2.waits.iter().all(|w| !has_fan(w, "组合龙")),
        "m>1 不应生成组合龙"
    );
    assert!(
        an2.waits.is_empty(),
        "m=2 时该手应无和牌型（实测未听牌）: {:?}",
        an2.waits
            .iter()
            .map(|w| tile_name(w.tile))
            .collect::<Vec<_>>()
    );
}

// ============================ DL-2：R-12 三条互斥 ============================

/// R-12（E）：字一色 **不计** 混幺九。
#[test]
fn r12_zipaiyise_excludes_hunyaojiu() {
    let c = parse_hand("东东东 南南南 中中中 发发发 白");
    let an = analyze(&c, &[], 27, 27);
    let w = by_name(&an, "白");
    assert_eq!((w.normal, w.tsumo), (197, 198), "{:?}", names(w));
    assert!(has_fan(w, "字一色"), "{:?}", names(w));
    assert!(!has_fan(w, "混幺九"), "字一色吞混幺九: {:?}", names(w));
}

/// R-12（F1）：全大 **不计** 大于五；无全大时大于五照计（控制行）。
#[test]
fn r12_quanda_excludes_dayuwu() {
    let c = parse_hand("789m 789m 789s 99s 88p");
    let an = analyze(&c, &[], 27, 27);

    let w9 = by_name(&an, "9条");
    assert_eq!((w9.normal, w9.tsumo), (31, 33), "{:?}", names(w9));
    assert!(has_fan(w9, "全大") && !has_fan(w9, "大于五"), "{:?}", names(w9));

    let w8 = by_name(&an, "8筒");
    assert_eq!((w8.normal, w8.tsumo), (28, 30), "{:?}", names(w8));
    assert!(has_fan(w8, "全大") && !has_fan(w8, "大于五"), "{:?}", names(w8));

    // 控制行：6条 行含 6条（非 7/8/9）⇒ 不成全大 ⇒ 大于五 12 照计、分值不变
    let w6 = by_name(&an, "6条");
    assert_eq!((w6.normal, w6.tsumo), (16, 18), "{:?}", names(w6));
    assert!(has_fan(w6, "大于五") && !has_fan(w6, "全大"), "{:?}", names(w6));
}

/// R-12（F2）：全小 **不计** 小于五；无全小时小于五照计（控制行）。
#[test]
fn r12_quanxiao_excludes_xiaoyuwu() {
    let c = parse_hand("111m222s333p111s 2p");
    let an = analyze(&c, &[], 27, 27);

    let w1 = by_name(&an, "1筒");
    assert_eq!((w1.normal, w1.tsumo), (46, 48), "{:?}", names(w1));
    assert!(has_fan(w1, "全小") && !has_fan(w1, "小于五"), "{:?}", names(w1));

    let w2 = by_name(&an, "2筒");
    assert_eq!((w2.normal, w2.tsumo), (100, 101), "{:?}", names(w2));
    assert!(has_fan(w2, "全小") && !has_fan(w2, "小于五"), "{:?}", names(w2));

    // 控制行：4筒 行含 4筒 ⇒ 不成全小 ⇒ 小于五 12 照计、分值不变
    let w4 = by_name(&an, "4筒");
    assert_eq!((w4.normal, w4.tsumo), (34, 36), "{:?}", names(w4));
    assert!(has_fan(w4, "小于五") && !has_fan(w4, "全小"), "{:?}", names(w4));
}

// ============================ DL-3：T-2 批 1（8 个番种覆盖） ============================

/// DL-3 · 双暗刻（2 番）
#[test]
fn t2b1_shuang_an_ke() {
    let c = parse_hand("111m 23m 456m 999s 55p");
    let an = analyze(&c, &[], 27, 27);
    let w = by_name(&an, "7万");
    assert_eq!(fan_value(w, "双暗刻"), 2, "{:?}", names(w));
}

/// DL-3 · 四暗刻（64 番）
#[test]
fn t2b1_si_an_ke() {
    let c = parse_hand("111m222m333m444m 5m");
    let an = analyze(&c, &[], 27, 27);
    let w = by_name(&an, "5万");
    assert_eq!(fan_value(w, "四暗刻"), 64, "{:?}", names(w));
    assert!(has_fan(w, "四暗刻"), "{:?}", names(w));
}

/// DL-3 · 四归一（2 番）
#[test]
fn t2b1_si_gui_yi() {
    let c = parse_hand("111m 23m 456m 999s 55p");
    let an = analyze(&c, &[], 27, 27);
    let w = by_name(&an, "1万");
    assert_eq!(fan_value(w, "四归一"), 2, "{:?}", names(w));
}

/// DL-3 · 断幺（2 番）
///
/// **不依赖耦合总分**：逐番种断言 + 互斥护栏；末尾的总分由**已断言项**相加得出
/// （等价于「没有额外番种」，比裸快照 202 更可读、也不会因无关番种变更而误报）。
#[test]
fn t2b1_duan_yao() {
    let c = parse_hand("234s234s234s666s 8s");
    let an = analyze(&c, &[], 27, 27);
    let w = by_name(&an, "8条");

    // 逐番种
    assert_eq!(fan_value(w, "断幺"), 2, "{:?}", names(w));
    assert_eq!(fan_value(w, "清一色"), 24, "{:?}", names(w));
    assert_eq!(fan_value(w, "绿一色"), 88, "{:?}", names(w));
    assert_eq!(fan_value(w, "四暗刻"), 64, "{:?}", names(w));
    assert_eq!(fan_value(w, "一色三节高"), 24, "{:?}", names(w));

    // 互斥护栏：绿一色（无发）可计清一色、但不得计混一色；四暗刻吞 碰碰和/双暗刻；
    // 清一色与断幺均吞 无字；四暗刻不再计 门前清
    assert!(!has_fan(w, "混一色"), "{:?}", names(w));
    assert!(!has_fan(w, "碰碰和"), "{:?}", names(w));
    assert!(!has_fan(w, "双暗刻"), "{:?}", names(w));
    assert!(!has_fan(w, "无字"), "{:?}", names(w));
    assert!(!has_fan(w, "门前清"), "{:?}", names(w));

    // 总分 = 上述各项之和（= 无额外番种）
    let sum = 2 + 24 + 88 + 64 + 24;
    assert_eq!(w.normal, sum, "{:?}", names(w));
}

/// DL-3 · 全大（24 番）+ 大于五 控制
#[test]
fn t2b1_quan_da() {
    let c = parse_hand("789m 789m 789s 99s 88p");
    let an = analyze(&c, &[], 27, 27);
    let w = by_name(&an, "9条");
    assert_eq!(fan_value(w, "全大"), 24, "{:?}", names(w));
}

/// DL-3 · 全小（24 番）
#[test]
fn t2b1_quan_xiao() {
    let c = parse_hand("111m222s333p111s 2p");
    let an = analyze(&c, &[], 27, 27);
    let w = by_name(&an, "1筒");
    assert_eq!(fan_value(w, "全小"), 24, "{:?}", names(w));
}

/// DL-3 · 大于五（12 番）——只在**无全大**时出现
#[test]
fn t2b1_da_yu_wu() {
    let c = parse_hand("789m 789m 789s 99s 88p");
    let an = analyze(&c, &[], 27, 27);
    let w = by_name(&an, "6条");
    assert_eq!(fan_value(w, "大于五"), 12, "{:?}", names(w));
    assert!(!has_fan(w, "全大"), "{:?}", names(w));
}

/// DL-3 · 小于五（12 番）——只在**无全小**时出现
#[test]
fn t2b1_xiao_yu_wu() {
    let c = parse_hand("111m222s333p111s 2p");
    let an = analyze(&c, &[], 27, 27);
    let w = by_name(&an, "4筒");
    assert_eq!(fan_value(w, "小于五"), 12, "{:?}", names(w));
    assert!(!has_fan(w, "全小"), "{:?}", names(w));
}
