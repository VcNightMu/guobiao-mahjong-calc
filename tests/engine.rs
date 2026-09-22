use gbmj::calc::{analyze, WaitResult};
use gbmj::model::Meld;
use gbmj::tiles::*;

fn names(w: &WaitResult) -> Vec<String> {
    w.fans.iter().map(|f| f.name.to_string()).collect()
}

fn has(w: &WaitResult, n: &str) -> bool {
    w.fans.iter().any(|f| f.name == n)
}

fn find<'a>(an: &'a gbmj::calc::Analysis, t: usize) -> &'a WaitResult {
    an.waits.iter().find(|w| w.tile == t).expect("无此和牌张")
}

#[test]
fn thirteen_orphans_is_88() {
    let c = parse_hand("19m19s19p东南西北中发白");
    let an = analyze(&c, &[], 27, 27);
    assert_eq!(an.structural_waits.len(), 13, "十三幺应听 13 种");
    for w in &an.waits {
        assert!(has(w, "十三幺"), "{:?}", names(w));
        assert_eq!(w.normal, 88, "十三幺点炮应 88（不计门前清）");
        assert!(w.can_win());
    }
}

#[test]
fn nine_gates_recognized() {
    let c = parse_hand("1112345678999m");
    let an = analyze(&c, &[], 27, 27);
    assert_eq!(an.structural_waits.len(), 9, "九莲宝灯应听 1-9 万");
    for w in &an.waits {
        assert!(has(w, "九莲宝灯"), "{:?}", names(w));
        assert!(w.can_win());
    }
}

#[test]
fn pure_dragon_and_pinfu() {
    // 123m 456m 789m 55p 78p，听 6p/9p
    let c = parse_hand("123456789m 55p 78p");
    let an = analyze(&c, &[], 27, 27);
    let w6 = find(&an, 18 + 5); // 6筒
    assert!(has(w6, "清龙"), "{:?}", names(w6));
    assert!(has(w6, "平和"), "{:?}", names(w6));
    assert!(has(w6, "缺一门"), "{:?}", names(w6));
    let w9 = find(&an, 18 + 8); // 9筒 -> 789p，与 789m 构成喜相逢
    assert!(has(w9, "喜相逢"), "{:?}", names(w9));
}

#[test]
fn menqing_tsumo_is_buqiu_ren() {
    let c = parse_hand("123456789m 55p 78p");
    let an = analyze(&c, &[], 27, 27);
    let w = &an.waits[0];
    // 门清：点炮 门前清+2，自摸 不求人+4 → 差 2
    assert_eq!(w.tsumo - w.normal, 2);
    // 绝张：门上再多 4
    assert_eq!(w.last - w.normal, 4);
    assert_eq!(w.tsumo_last - w.tsumo, 4);
}

#[test]
fn open_hand_is_not_menqing() {
    // 副露：吃 123m、碰 888s，暗牌 567s 东，听东
    let melds = vec![Meld::chi(0), Meld::pon(9 + 7)];
    let c = parse_hand("567s 东");
    assert_eq!(total(&c), 4, "3 副露时暗牌应为 4 张");
    let an = analyze(&c, &melds, 27, 27);
    assert!(!an.menqing, "有吃碰应非门清");
}

#[test]
fn ankan_keeps_menqing() {
    // 暗杠 888s 不破门清
    let melds = vec![Meld::kan(9 + 7, false)];
    let c = parse_hand("123789m 99p 东东");
    assert_eq!(total(&c), 10, "1 副露时暗牌应为 10 张");
    let an = analyze(&c, &melds, 27, 27);
    assert!(an.menqing, "暗杠不应破门清");
}

#[test]
fn wu_fan_he_only_by_discard() {
    // 副露：吃 234m、碰 555p、碰 666s，暗牌 东东 77s，和 7s 成刻子
    // 手牌 0 番 → 只有点炮可按无番和 8 番
    let melds = vec![Meld::chi(1), Meld::pon(18 + 4), Meld::pon(9 + 5)];
    let c = parse_hand("77s 东东");
    assert_eq!(total(&c), 4);
    let an = analyze(&c, &melds, 27, 27);
    let w = find(&an, 9 + 6); // 7条
    assert!(has(w, "无番和"), "{:?}", names(w));
    assert_eq!(w.normal, 8, "无番和 = 8");
    assert!(w.tsumo < 8, "自摸引入番，反而不到 8：{}", w.tsumo);
    assert!(w.can_win());
}

#[test]
fn seven_pairs_recognized() {
    // 11m 33m 55m 77m 99m 11s 7s，听 7s 成七对
    let c = parse_hand("1133557799m 11s 7s");
    assert_eq!(total(&c), 13);
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 9 + 6); // 7条
    assert!(has(w, "七对"), "{:?}", names(w));
    assert_eq!(w.normal, 24 + 1 + 1, "七对 24 + 缺一门 1 + 无字 1");
}

#[test]
fn da_san_yuan() {
    // 中中中 发发发 白白白 456m 5m，和 5m
    let c = parse_hand("中中中发发发白白白 456m 5m");
    assert_eq!(total(&c), 13);
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 4); // 5万
    assert!(has(w, "大三元"), "{:?}", names(w));
    assert!(w.normal >= 88);
}

#[test]
fn da_si_xi_exclusions() {
    // 东东东 南南南 西西西 北北北 5m，和 5m
    let c = parse_hand("东东东南南南西西西北北北 5m");
    assert_eq!(total(&c), 13);
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 4);
    assert!(has(w, "大四喜"), "{:?}", names(w));
    assert!(!has(w, "三风刻"), "{:?}", names(w));
    assert!(!has(w, "碰碰和"), "{:?}", names(w));
}

#[test]
fn tui_bu_dao() {
    // 112233p 456s 888p 白，和 白
    let c = parse_hand("112233p 456s 888p 白");
    assert_eq!(total(&c), 13);
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 33); // 白
    assert!(has(w, "推不倒"), "{:?}", names(w));
    assert!(!has(w, "缺一门"), "推不倒不计缺一门: {:?}", names(w));
}

#[test]
fn yi_se_shuang_long_hui() {
    // 112233m 778899m 5m，和 5m
    let c = parse_hand("112233m 778899m 5m");
    assert_eq!(total(&c), 13);
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 4);
    assert!(has(w, "一色双龙会"), "{:?}", names(w));
    assert!(!has(w, "清一色"), "{:?}", names(w));
    assert!(!has(w, "一般高"), "{:?}", names(w));
}

#[test]
fn san_se_shuang_long_hui() {
    // 123789m 123789p 5s，和 5s
    let c = parse_hand("123789m 123789p 5s");
    assert_eq!(total(&c), 13);
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 9 + 4); // 5条
    assert!(has(w, "三色双龙会"), "{:?}", names(w));
    assert!(!has(w, "喜相逢"), "{:?}", names(w));
    assert!(!has(w, "老少副"), "{:?}", names(w));
}

#[test]
fn zuhe_long() {
    // 147m 258s 369p 456m 9m，和 9m
    let c = parse_hand("147m 258s 369p 456m 9m");
    assert_eq!(total(&c), 13);
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 8); // 9万
    assert!(has(w, "组合龙"), "{:?}", names(w));
    assert!(has(w, "平和"), "{:?}", names(w));
}

#[test]
fn ankou_differs_by_win_method() {
    // 555m 66s 777p 123m 东东，双碰听 6s / 东
    let c = parse_hand("555m 66s 777p 123m 东东");
    assert_eq!(total(&c), 13);
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 9 + 5); // 6条
    // 三色三节高(8) 固定；差异在暗刻：
    // 自摸：666s 算暗刻 → 三暗刻 16 + 不求人 4 → 8+16+4 = 28
    assert_eq!(w.tsumo, 28, "自摸：三色三节高+三暗刻+不求人");
    // 点炮：666s 算明刻 → 双暗刻 2 + 门前清 2 → 8+2+2 = 12
    assert_eq!(w.normal, 12, "点炮：三色三节高+双暗刻+门前清");
}

#[test]
fn bu_chai_yi_qinglong() {
    // 123456789m 55p 78s，和 6s → 清龙，不计连六/老少副（不拆移）
    let c = parse_hand("123456789m 55p 78s");
    assert_eq!(total(&c), 13);
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 9 + 5); // 6条
    assert!(has(w, "清龙"), "{:?}", names(w));
    assert!(!has(w, "连六"), "{:?}", names(w));
    assert!(!has(w, "老少副"), "{:?}", names(w));
}

#[test]
fn tao_suan_yi_ci_huolong() {
    // 123m 456p 789s 123s 5m，和 5m → 花龙 + 只能套算一次（喜相逢或老少副）
    let c = parse_hand("123m 456p 789s 123s 5m");
    assert_eq!(total(&c), 13);
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 4); // 5万
    assert!(has(w, "花龙"), "{:?}", names(w));
    let small = has(w, "喜相逢") as u8 + has(w, "老少副") as u8;
    assert_eq!(small, 1, "只可套算一次: {:?}", names(w));
}
