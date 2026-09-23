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

#[test]
fn jiu_gao_bu_jiu_di() {
    // 门清 555666777p 5556s 自摸 7s
    // 拆分A: 567p567p567p 567s 55s → 一色三同顺24+全带五16+平和2+喜相逢1+缺一门1 +不求人4 = 48
    // 拆分B: 555p666p777p 567s 55s → 一色三节高24+三暗刻16+断幺2+缺一门1 +不求人4 = 47
    let c = parse_hand("555p666p777p 555s 6s");
    assert_eq!(total(&c), 13);
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 15); // 7条
    assert_eq!(w.tsumo, 48, "就高不就低应取 48: {:?}", names(w));
}

#[test]
fn zuhe_long_with_meld() {
    // 副露 吃 123m；暗牌 147m 258s 369p 5p，和 5p → 组合龙 + 平和
    let melds = vec![Meld::chi(0)];
    let c = parse_hand("147m 258s 369p 5p");
    assert_eq!(total(&c), 10);
    let an = analyze(&c, &melds, 27, 27);
    let w = find(&an, 18 + 4); // 5筒
    assert!(has(w, "组合龙"), "{:?}", names(w));
    assert!(has(w, "平和"), "{:?}", names(w));
}

#[test]
fn yi_ming_yi_an_kan() {
    // 明杠 3万 + 暗杠 5筒；暗牌 123s456s 7s，和 7s → 一明一暗杠 5 番
    let melds = vec![Meld::kan(2, true), Meld::kan(22, false)];
    let c = parse_hand("123s456s 7s");
    assert_eq!(total(&c), 7);
    let an = analyze(&c, &melds, 27, 27);
    let w = find(&an, 9 + 6); // 7条
    assert!(has(w, "一明一暗杠"), "{:?}", names(w));
    assert!(
        !has(w, "明杠") && !has(w, "暗杠"),
        "不应再另计明杠/暗杠: {:?}",
        names(w)
    );
}

#[test]
fn tao_suan_yi_ci_run_can_serve_two_fans() {
    // 吃 123万 + 吃 123条；暗牌 1112567万。
    // 和 3万：暗牌里还有一副 123万，与副露的 123万 组「一般高」，同时 123万 与 123条 组「喜相逢」。
    // 一副顺子可以参与不同番种（只是同一番种内不重复用）。
    let c = parse_hand("1112567m");
    let melds = vec![Meld::chi(0), Meld::chi(9)];
    let an = analyze(&c, &melds, 27, 27);
    let w = find(&an, 2); // 3万
    assert!(has(w, "一般高"), "和3万应计一般高: {:?}", names(w));
    assert!(has(w, "喜相逢"), "和3万应计喜相逢: {:?}", names(w));
    // 和 2万 时是 111万+567万，凑不出两副 123万，故无一般高
    let w2 = find(&an, 1); // 2万
    assert!(!has(w2, "一般高"), "和2万不该有一般高: {:?}", names(w2));
    assert!(has(w2, "喜相逢"), "{:?}", names(w2));
}

#[test]
fn tao_suan_yi_ci_four_runs_three_pairs() {
    // 门清 123456m 123456s 5m，和 5万：四副顺子共可套算出 3 对（不是每副只能用一次）
    let c = parse_hand("123456m123456s5m");
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 4); // 5万
    let n = names(w);
    let val = |s: &str| {
        w.fans
            .iter()
            .filter(|f| f.name == s)
            .map(|f| f.value)
            .sum::<u32>()
    };
    assert_eq!(
        val("连六") + val("喜相逢"),
        3,
        "四副顺子应套算出 3 对: {:?}",
        n
    );
}

#[test]
fn bi_ran_menqing_tsumo_is_just_one() {
    // 七对（必然门清）自摸只加「自摸 1」，不计不求人
    let c = parse_hand("11m22m33m44m55m白白白");
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 33); // 白
    assert!(has(w, "七对"), "{:?}", names(w));
    assert_eq!(w.normal, 30, "七对24+混一色6: {:?}", names(w));
    assert_eq!(w.tsumo, 31, "必然门清自摸只应 +1: {:?}", names(w));
    assert!(!has(w, "不求人"), "{:?}", names(w));
}

#[test]
fn menqing_shows_qianmenqing_in_detail() {
    // 门清手点炮：完整番表里应含「门前清 2」
    let c = parse_hand("147m258s369p12m55p");
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 2); // 3万
    assert!(has(w, "门前清"), "{:?}", names(w));
    // 12万 听 3万 本身就是边张，组合龙手也照计
    assert_eq!(w.normal, 17, "组合龙12+平和2+边张1+门前清2: {:?}", names(w));
}

#[test]
fn tsumo_detail_differs_from_normal() {
    // 双碰听：荣和补的刻子算明刻、自摸才算暗刻 → 点炮与自摸的番表不同（界面需额外显示自摸番）
    let c = parse_hand("555m66s777p123m东东");
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 9 + 5); // 6条
    let strip = |v: &[gbmj::fans::Fan]| -> Vec<String> {
        v.iter()
            .filter(|f| !matches!(f.name, "门前清" | "不求人" | "自摸" | "和绝张"))
            .map(|f| f.name.to_string())
            .collect()
    };
    assert_ne!(
        strip(&w.fans),
        strip(&w.fans_tsumo),
        "点和与自摸的番表应不同: {:?} vs {:?}",
        names(w),
        w.fans_tsumo
            .iter()
            .map(|f| f.name)
            .collect::<Vec<_>>()
    );
}

#[test]
fn zuhelong_dan_diao_jiang() {
    // 吃 234万；暗牌 147m 258s 3679p，和 7筒：
    // 组合龙 + 平和 + 单钓将（听 7筒 正好补将牌）
    let melds = vec![Meld::chi(1)]; // 234万
    let c = parse_hand("147m258s3679p");
    assert_eq!(total(&c), 10);
    let an = analyze(&c, &melds, 27, 27);
    let w = find(&an, 18 + 6); // 7筒
    assert!(has(w, "组合龙"), "{:?}", names(w));
    assert!(has(w, "平和"), "{:?}", names(w));
    assert!(has(w, "单钓将"), "{:?}", names(w));
}

#[test]
fn zuhelong_bian_zhang() {
    // 门清 12247m 258s 12369p 和 3筒：
    // 3筒 既在组合龙（369筒）里，又构成 12筒 的边张 → 加计边张
    let c = parse_hand("12247m258s12369p");
    assert_eq!(total(&c), 13);
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 20); // 3筒
    assert!(has(w, "组合龙"), "{:?}", names(w));
    assert!(has(w, "边张"), "{:?}", names(w));
}

#[test]
fn zuhelong_kan_zhang() {
    // 门清 147m258s369p 13m 55p，和 2万：2万 既在组合龙体系的手牌里，又是 13万 的坎张
    let c = parse_hand("147m258s369p13m55p");
    assert_eq!(total(&c), 13);
    let an = analyze(&c, &[], 27, 27);
    let w = find(&an, 1); // 2万
    assert!(has(w, "组合龙"), "{:?}", names(w));
    assert!(has(w, "坎张"), "{:?}", names(w));
}
