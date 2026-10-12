use gbmj::model::Meld;
use gbmj::plan::{plan, Direction};
use gbmj::tiles::*;

// 说明（**刻意**，非疏漏）：本文件内 `plan(…, 2)` 的 `2` 是**契约护栏**用的字面量，与
// `src/plan.rs` 的 `MAX_DIST = 2` 同值却**不改为常量引用**——那个常量一旦被改动，
// 这里的快照断言必须立刻失败报警，而不是静默跟随。

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

#[test]
fn qing_long_zero() {
    // 123456789m + 11m + 22m：清龙已经成形，只差把 11m/22m 补成面子+将
    let h = parse_hand("123456789m11m22m");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);
    let d = find(&ds, "清龙(万门)").expect("应有清龙(万门)");
    assert!(d.distance <= 2, "{:?}", d);
}

#[test]
fn hua_long_zero() {
    let h = parse_hand("123m456s789p11m22p");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);
    let d = find(&ds, "花龙").expect("应有花龙");
    assert_eq!(d.distance, 0, "{:?}", d);
}

#[test]
fn san_se_san_bu_gao_zero() {
    let h = parse_hand("123m234s345p11m22s");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);
    let d = find(&ds, "三色三步高").expect("应有三色三步高");
    assert_eq!(d.distance, 0, "{:?}", d);

    // T52/F1 反控：d=2（递增二位）不被计番侧认定 ⇒ plan 不得给出该方向。
    // 该手的三色三步高 witness 只能是 (n=2,d=2) 的 234m/456s/678p + 111m + 22s。
    let h2 = parse_hand("123m 456s 789p 11m 22s");
    assert_eq!(total(&h2), 13);
    let ds2 = plan(&h2, &[], 2);
    assert!(
        find(&ds2, "三色三步高").is_none(),
        "d=2（递增二位）不被计番认定（T52/F1），不得给出该方向: {:?}",
        ds2.iter().map(|d| d.name.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn zuhe_long_zero() {
    let h = parse_hand("147m258s369p11m22p");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);
    let d = find(&ds, "组合龙").expect("应有组合龙");
    assert_eq!(d.distance, 0, "{:?}", d);
}

#[test]
fn pungs_block_run_family() {
    // 已经两副碰（或杠）时，顺子系骨架放不下
    let h = parse_hand("12345m12345s1p");
    let melds = vec![Meld::pon(0), Meld::pon(9)]; // 碰1万 + 碰1条
    let ds = plan(&h, &melds, 2);
    let names: Vec<&str> = ds.iter().map(|d| d.name.as_str()).collect();
    assert!(find(&ds, "清龙").is_none(), "{:?}", names);
    assert!(find(&ds, "花龙").is_none(), "{:?}", names);
    assert!(find(&ds, "三色三同顺").is_none(), "{:?}", names);
    assert!(find(&ds, "组合龙").is_none(), "{:?}", names);
}

#[test]
fn san_an_ke_zero() {
    // 111m222m333m + 44m55m：三个暗刻已成形，44m 补成 444m 靠点和是明刻，不影响前三个
    let h = parse_hand("111m222m333m4455m");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);
    let d = find(&ds, "三暗刻").expect("应有三暗刻");
    assert_eq!(d.distance, 0, "{:?}", d);

    // T52/F5 反控：四张同牌（1111m）同样能提供一副暗刻，不得因 `hand[t] == 3` 被排除。
    let h4 = parse_hand("1111m222m333m99s5p");
    assert_eq!(total(&h4), 13);
    let ds4 = plan(&h4, &[], 2);
    assert!(
        find(&ds4, "三暗刻").is_some(),
        "四张同牌应可作暗刻源（T52/F5）: {:?}",
        ds4.iter().map(|d| d.name.as_str()).collect::<Vec<_>>()
    );

    // T54/F2 反例：明碰不是暗刻，不得被 `Carrier::Open` 当成暗刻源（fans.rs 暗刻统计要求 !open）。
    // 注：该断言同时锁死一条结构性不变量——三/四暗刻的 forced 由「手内 ≥3 张」的牌生成，而同牌再有
    // 副露会超过每张 4 张上限（输入校验拒绝），故匹配本就不会命中；即便将来 `full` 取法变化也必须仍不出现。
    let ds_pon = plan(&parse_hand("222m333m99s5p5p"), &[Meld::pon(0)], 2);
    assert!(
        find(&ds_pon, "三暗刻").is_none() && find(&ds_pon, "四暗刻").is_none(),
        "明碰不得充当暗刻源: {:?}",
        ds_pon.iter().map(|d| d.name.as_str()).collect::<Vec<_>>()
    );

    // T74：`Carrier::Concealed` 正例——暗杠不是「被承载」，而是由 `ankan` 单独扣除（need3 = 3 - ankan），
    // 故「暗杠 + 2 暗刻」仍应给出 三暗刻。
    let h_kan = parse_hand("222m333m99s55p");
    assert_eq!(total(&h_kan), 10, "10 张暗牌 + 1 副暗杠");
    let ds_kan = plan(&h_kan, &[Meld::kan(0, false)], 2);
    let san = find(&ds_kan, "三暗刻")
        .unwrap_or_else(|| panic!("暗杠 + 2 暗刻 应仍有三暗刻（T74）: {:?}", ds_kan));
    assert_eq!(san.value, 16, "{:?}", san);
    assert_eq!(san.distance, 0, "{:?}", san);

    // T79/F7a：`ankan == 3`（need3 = 0）时方向**仍应生成**——`combos(full, 0)` 返回 `[[]]` ⇒ `forced = []`，
    // 方向给出「差0 + 该补的将牌」。修复前 `if ankan < 3` 把整块跳过 ⇒ 恒缺。
    let h3 = parse_hand("5567p");
    assert_eq!(total(&h3), 4, "4 张暗牌 + 3 副暗杠");
    let ds3 = plan(
        &h3,
        &[
            Meld::kan(0, false),
            Meld::kan(1, false),
            Meld::kan(2, false),
        ],
        2,
    );
    let s3 = find(&ds3, "三暗刻")
        .unwrap_or_else(|| panic!("3 副暗杠（need3=0）应仍有三暗刻（T79/F7a）: {:?}", ds3));
    assert_eq!(s3.value, 16, "{:?}", s3);
    assert_eq!(s3.distance, 0, "{:?}", s3);
    assert_eq!(
        s3.need.iter().filter(|&&t| t == 22).count(),
        1,
        "need 应为 1 张 5筒: {s3:?}"
    );
}

#[test]
fn si_an_ke_zero() {
    let h = parse_hand("111m222m333m444m5m");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);
    let d = find(&ds, "四暗刻").expect("应有四暗刻");
    assert_eq!(d.distance, 0, "{:?}", d);

    // T52/F5 反控：四暗刻的四副暗刻同样允许「四张同牌」提供其中一副。
    let h4 = parse_hand("1111m222m333m444m");
    assert_eq!(total(&h4), 13);
    let ds4 = plan(&h4, &[], 2);
    assert!(
        find(&ds4, "四暗刻").is_some(),
        "四张同牌应可作暗刻源（T52/F5）: {:?}",
        ds4.iter().map(|d| d.name.as_str()).collect::<Vec<_>>()
    );

    // T74：`Carrier::Concealed` 正例——「暗杠 + 3 暗刻」应给出 四暗刻（暗杠由 `ankan` 计入）。
    let h_kan = parse_hand("222m333m444m9s");
    assert_eq!(total(&h_kan), 10, "10 张暗牌 + 1 副暗杠");
    let ds_kan = plan(&h_kan, &[Meld::kan(18, false)], 2);
    let si = find(&ds_kan, "四暗刻")
        .unwrap_or_else(|| panic!("暗杠 + 3 暗刻 应有四暗刻（T74）: {:?}", ds_kan));
    assert_eq!(si.value, 64, "{:?}", si);
    assert_eq!(si.distance, 0, "{:?}", si);

    // T79/F7b：`ankan == 4`（need4 = 0）时方向**仍应生成**（`combos(full, 0) = [[]]` ⇒ `forced = []`）。
    // 修复前 `if ankan < 4` 把整块跳过 ⇒ 恒缺；引擎默认模式该手确实计 `四暗刻64`。
    let h4k = parse_hand("5p");
    assert_eq!(total(&h4k), 1, "1 张暗牌 + 4 副暗杠");
    let ds4k = plan(
        &h4k,
        &[
            Meld::kan(0, false),
            Meld::kan(1, false),
            Meld::kan(2, false),
            Meld::kan(3, false),
        ],
        2,
    );
    let s4k = find(&ds4k, "四暗刻")
        .unwrap_or_else(|| panic!("4 副暗杠（need4=0）应仍有四暗刻（T79/F7b）: {:?}", ds4k));
    assert_eq!(s4k.value, 64, "{:?}", s4k);
    assert_eq!(s4k.distance, 0, "{:?}", s4k);
    assert_eq!(
        s4k.need.iter().filter(|&&t| t == 22).count(),
        1,
        "need 应为 1 张 5筒: {s4k:?}"
    );
}

#[test]
fn si_an_ke_not_offered_when_only_tsumo() {
    // 双碰听：点和补出来的第 4 个刻子是明刻，四暗刻只有自摸才算 → 不该出现
    let h = parse_hand("111m222m333m44m55m");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);
    let names: Vec<&str> = ds.iter().map(|d| d.name.as_str()).collect();
    assert!(find(&ds, "四暗刻").is_none(), "{:?}", names);
    assert!(find(&ds, "三暗刻").is_some(), "{:?}", names);
}

#[test]
fn da_san_yuan_and_xiao_san_yuan() {
    // 中中中发发发 + 白白（小三元听白，大三元差一张白）
    let h = parse_hand("中中中发发发白白123m44m");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);
    assert!(find(&ds, "小三元").is_some(), "{:?}", ds);
    assert!(find(&ds, "大三元").is_some(), "{:?}", ds);

    // ---- T54/F2 正例：副露可承载的强制面子（修复前这些方向恒缺）----
    // 引擎口径依据：和牌分解 = 副露 to_set() ∪ 暗牌分解（calc.rs::winning_decomps），故副露等价于同形面子；
    // fans.rs 的番种判定块内**无门清守卫** ⇒ 吃可承载顺子面子、碰/明杠/暗杠可承载刻子面子。
    let cases: [(&str, &str, Vec<Meld>, u32); 5] = [
        // 大四喜：碰东 + 手内 南/西/北 三刻 + 单张「中」作将
        ("大四喜", "南南南西西西北北北中", vec![Meld::pon(27)], 88),
        // 大三元：碰中/碰发 + 手内 白白白 + 5m5m（对子/第 4 面子待补）
        (
            "大三元",
            "白白白东东5m5m",
            vec![Meld::pon(31), Meld::pon(32)],
            88,
        ),
        // 三同刻：碰1筒 + 手内 111m/111s
        ("三同刻", "111m111s99p55p", vec![Meld::pon(18)], 16),
        // 三风刻：碰北 + 手内 南南南/西西西
        ("三风刻", "南南南西西西99p55p", vec![Meld::pon(30)], 12),
        // 一色双龙会：4 副明吃（123m×2 + 789m×2）+ 单张 5万作将（引擎实测计番 64，见 T54 报告）
        (
            "一色双龙会",
            "5m",
            vec![Meld::chi(0), Meld::chi(0), Meld::chi(6), Meld::chi(6)],
            64,
        ),
    ];
    for (name, hand, melds, value) in cases {
        let dirs = plan(&parse_hand(hand), &melds, 2);
        let d =
            find(&dirs, name).unwrap_or_else(|| panic!("{} 应由副露承载后出现: {:?}", name, dirs));
        assert_eq!(d.value, value, "{} 番值: {:?}", name, d);
        assert_eq!(d.distance, 0, "{} 距离: {:?}", name, d);
    }
    // ---- T54/F2 形状反例：吃不得承载「刻子型」强制面子 ----
    // 若 123m 的吃被误当作 111m 刻子，三同刻就会少补 3 张 1万 ⇒ 用 need 中 1万 的张数锁死。
    let ds_chi = plan(&parse_hand("111s111p99m55m"), &[Meld::chi(0)], 2);
    let tk = find(&ds_chi, "三同刻").expect("三同刻（111s/111p + 待补 111m）应出现");
    assert_eq!(
        tk.need.iter().filter(|&&t| t == 0).count(),
        3,
        "吃 123m 不得充当 111m 刻子: {:?}",
        tk
    );

    // ---- T74：杠承载（明杠/暗杠）充当「刻子型」强制面子 ----
    // 依据：`meld_carries` 对刻子型 forced 接受 `Pon | Kan`，与 `fans.rs::trips_of` 只过滤
    // `is_triplet()`（杠的 kind 亦为 Triplet）一致 ⇒ 明杠/暗杠都能承载一副刻子面子。
    let kan_cases: [(&str, &str, Vec<Meld>, u32); 4] = [
        // 明杠 1筒 承载 三同刻 的 111筒
        ("三同刻", "111m111s99p55p", vec![Meld::kan(18, true)], 16),
        // 暗杠 北 承载 三风刻 的 北北北
        (
            "三风刻",
            "南南南西西西99p55p",
            vec![Meld::kan(30, false)],
            12,
        ),
        // 暗杠 东 承载 大四喜 的 东东东
        (
            "大四喜",
            "南南南西西西北北北中",
            vec![Meld::kan(27, false)],
            88,
        ),
        // 双明杠 中/发 承载 大三元 的两副箭刻
        (
            "大三元",
            "白白白东东5m5m",
            vec![Meld::kan(31, true), Meld::kan(32, true)],
            88,
        ),
    ];
    for (name, hand, melds, value) in kan_cases {
        let dirs = plan(&parse_hand(hand), &melds, 2);
        let d = find(&dirs, name)
            .unwrap_or_else(|| panic!("{} 应由杠承载后出现（T74）: {:?}", name, dirs));
        assert_eq!(d.value, value, "{} 番值: {:?}", name, d);
        assert!(d.distance <= 2, "{} 距离: {:?}", name, d);
    }
    // ---- T74 反例：杠不得承载「顺子型」强制面子 ----
    // 两手的 1万/9万 已被杠占满 4 张 ⇒ 顺子 123m / 789m 物理不可达，清龙必须不出现；
    // 若 `meld_carries` 误把杠当作顺子面子（少一副顺子），该方向就会冒出来。
    let kan_run_neg: [(&str, Vec<Meld>, &str); 2] = [
        ("456m789m11p22p", vec![Meld::kan(0, true)], "1万明杠"),
        ("123m456m11p22p", vec![Meld::kan(8, false)], "9万暗杠"),
    ];
    for (hand, melds, label) in kan_run_neg {
        let dirs = plan(&parse_hand(hand), &melds, 2);
        assert!(
            find(&dirs, "清龙").is_none(),
            "{label}：杠不得承载顺子型强制面子，清龙不得出现: {:?}",
            dirs.iter().map(|d| d.name.as_str()).collect::<Vec<_>>()
        );
    }

    // ---- T75：多副露「分别承载」同一骨架番的多个强制面子 ----
    // 修复前 `plan()` 有一条「碰/杠 ≥ 2 ⇒ 骨架一概跳过」的前置守卫（其前提是强制面子只能由暗牌成形），
    // F2 之后强制面子可由副露承载 ⇒ 该守卫造成假阴性：2 个碰各承载一个强制刻子时方向恒缺。
    // 复现 A：碰 1m / 碰 2m 分别承载 111m / 222m，暗牌 333m/444m 成形 + 1筒作将 ⇒ 一色四节高 差0、need = 1 张 1筒。
    // 同类 1：4 个碰全部承载 111m/222m/333m/444m ⇒ 一色四节高 仍应出现（4 副露形态）。
    let t75: [(&str, &str, Vec<Meld>, u32); 2] = [
        (
            "一色四节高",
            "333m444m1p",
            vec![Meld::pon(0), Meld::pon(1)],
            48,
        ),
        (
            "一色四节高",
            "1p",
            vec![Meld::pon(0), Meld::pon(1), Meld::pon(2), Meld::pon(3)],
            48,
        ),
    ];
    for (name, hand, melds, value) in t75 {
        let dirs = plan(&parse_hand(hand), &melds, 2);
        let d = find(&dirs, name)
            .unwrap_or_else(|| panic!("{name} 应由多副露分别承载后出现（T75）: {dirs:?}"));
        assert_eq!(d.value, value, "{name} 番值: {d:?}");
        assert_eq!(d.distance, 0, "{name} 距离: {d:?}");
        assert_eq!(
            d.need.iter().filter(|&&t| t == 18).count(),
            1,
            "{name} 的 need 应为 1 张 1筒: {d:?}"
        );
    }
    // 同类 2：三色三节高（碰1m / 碰2s 分别承载 111m / 222s）⇒ 修复前恒缺。
    let ds_ss = plan(&parse_hand("333p444p1m"), &[Meld::pon(0), Meld::pon(10)], 2);
    let ss = find(&ds_ss, "三色三节高").expect("三色三节高 应由 2 副露承载后出现（T75）");
    assert_eq!(ss.value, 8, "{ss:?}");
    assert_eq!(ss.distance, 1, "{ss:?}");
}

#[test]
fn quan_dai_wu() {
    // 345m456s567p55m34s：差一张 5条 就是全带五
    let h = parse_hand("345m456s567p55m34s");
    assert_eq!(total(&h), 13);
    let ds = plan(&h, &[], 2);
    let d = find(&ds, "全带五").expect("应有全带五");
    assert_eq!(d.distance, 0, "{:?}", d);

    // 手里三张用不上的牌（東東東）→ 距离 ≥ 3，不该出现
    let h2 = parse_hand("345m456s567p5m东东东");
    assert_eq!(total(&h2), 13);
    let ds2 = plan(&h2, &[], 2);
    assert!(
        find(&ds2, "全带五").is_none(),
        "{:?}",
        ds2.iter().map(|d| &d.name).collect::<Vec<_>>()
    );

    // 副露里一副不含 5 → 直接不可能
    let h3 = parse_hand("345m456s55m34s");
    let melds = vec![Meld::chi(0)]; // 吃 1万2万3万
    let ds3 = plan(&h3, &melds, 2);
    assert!(
        find(&ds3, "全带五").is_none(),
        "{:?}",
        ds3.iter().map(|d| &d.name).collect::<Vec<_>>()
    );
}
