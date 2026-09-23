use gbmj::model::Meld;
use gbmj::plan::{plan, Direction};
use gbmj::tiles::*;

fn find<'a>(ds: &'a [Direction], kw: &str) -> Option<&'a Direction> {
    ds.iter().find(|d| d.name.contains(kw))
}

#[test]
fn qi_dui_distance() {
    // 6 对 + 单张 = 七对听牌
    let h = parse_hand("1122334455667m");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);
    let d = find(&ds, "七对").expect("应有七对方向");
    assert_eq!(d.distance, 0, "{:?}", d);

    // 5 对 + 3 单张 = 差 1 张
    let h2 = parse_hand("1122334455m678m");
    assert_eq!(total(&h2), 13);
    let ds2 = plan(&h2, &[], 2);
    let d2 = find(&ds2, "七对").expect("应有七对方向");
    assert_eq!(d2.distance, 1, "{:?}", d2);
}

#[test]
fn qing_yi_se_two_away() {
    // 11 张万子 + 东东：清一色最多只能覆盖 11 张，差 2
    let h = parse_hand("11123456789m东东");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);
    let d = find(&ds, "清一色(万门)").expect("应有清一色(万门)");
    assert_eq!(d.distance, 2, "{:?}", d);
    // 混一色只需把东东当将，是 0 张差
    let m = find(&ds, "混一色(万门)").expect("应有混一色(万门)");
    assert_eq!(m.distance, 0, "{:?}", m);
}

#[test]
fn peng_peng_he_zero() {
    let h = parse_hand("111m222m333m44m55m");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);
    let d = find(&ds, "碰碰和").expect("应有碰碰和");
    assert_eq!(d.distance, 0, "{:?}", d);
}

#[test]
fn shi_san_yao_distance() {
    // 13 种幺九各一张 = 十三幺听牌
    let h = parse_hand("19m19s19p东南西北中发白");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);
    let d = find(&ds, "十三幺").expect("应有十三幺");
    assert_eq!(d.distance, 0, "{:?}", d);

    // 12 种 + 1 对
    let h2 = parse_hand("19m19s19p东南西北中白白");
    assert_eq!(total(&h2), 13);
    let ds2 = plan(&h2, &[], 2);
    assert_eq!(find(&ds2, "十三幺").unwrap().distance, 0);

    // 11 种 + 2 对 = 差 1
    let h3 = parse_hand("19m19s1p东南西北白白发发");
    assert_eq!(total(&h3), 13);
    let ds3 = plan(&h3, &[], 2);
    assert_eq!(find(&ds3, "十三幺").unwrap().distance, 1);
}

#[test]
fn quan_bu_kao_and_qi_xing() {
    // 9 张骨架 + 4 张字：全不靠听牌（等第 5 张字牌）
    let h = parse_hand("147m258s369p东南西北");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);
    assert_eq!(find(&ds, "全不靠").unwrap().distance, 0, "{:?}", ds);

    // 7 张骨架 + 6 张字：七星不靠听牌（等白板）
    let h2 = parse_hand("147m258s3p东南西北中发");
    assert_eq!(total(&h2), 13);
    let ds2 = plan(&h2, &[], 2);
    assert_eq!(find(&ds2, "七星不靠").unwrap().distance, 0, "{:?}", ds2);
}

#[test]
fn melds_block_menqing_patterns() {
    // 有副露（吃）时，七对/十三幺这类必然门清的番种不该出现；碰碰和也要求全刻子
    let h = parse_hand("1112345678m");
    let melds = vec![Meld::chi(0)]; // 吃 123万
    let ds = plan(&h, &melds, 2);
    let names: Vec<&str> = ds.iter().map(|d| d.name.as_str()).collect();
    assert!(find(&ds, "七对").is_none(), "{:?}", names);
    assert!(find(&ds, "十三幺").is_none(), "{:?}", names);
    assert!(find(&ds, "碰碰和").is_none(), "{:?}", names);
}
