//! 第 2 批（R-04 输入校验 + R-05 参数健壮性）的库级回归用例（DL-4 / T-4）
//!
//! 约定（依 Lead 工作指令）：
//! - **全部使用库级断言**（`try_parse_hand` / `validate_counts`），**不断言 CLI 文本**；
//!   CLI 侧的分流/退出码由验证记录中的命令证据覆盖（见 `docs/测试评估报告.md`）。
//! - 只新增，不修改既有 68 条用例。
//! - 期望值来自 `docs/测试方案.md` C1/C2/C3/C5/C6/C7 与项目文档 v1.4 的验收口径。

use gbmj::model::{validate_counts, InputError, Meld};
use gbmj::tiles::*;

// ==================== try_parse_hand：接受 / 拒绝矩阵 ====================

/// 合法串必须接受；且与旧 `parse_hand` 同语法同结果（向后兼容）。
#[test]
fn b2_parse_accepts_legal_forms() {
    for s in [
        "123456789m 55p 78p",
        "1m",
        "东东",
        "123m456s789p",
        "147m 258s 369p 5p",
    ] {
        assert!(try_parse_hand(s).is_ok(), "应接受合法串: {:?}", s);
    }

    let c_new = try_parse_hand("123456789m 55p 78p").unwrap();
    let c_old = parse_hand("123456789m 55p 78p");
    assert_eq!(c_new, c_old, "合法串上新旧解析器结果应一致");
    assert_eq!(total(&c_new), 13);
}

/// 非法字符：必须报**该字符本身**（C5：不要求指出位置）。
#[test]
fn b2_parse_reports_bad_char_only() {
    // 半角字母：花色后缀打错
    assert_eq!(
        try_parse_hand("123n 456s 789p 东东").unwrap_err(),
        ParseError::BadChar('n')
    );
    // 全角字符
    assert_eq!(try_parse_hand("１２３ｍ").unwrap_err(), ParseError::BadChar('１'));
    // 任意未识别符号
    assert_eq!(try_parse_hand("12*m").unwrap_err(), ParseError::BadChar('*'));

    // 文案只含字符本身，不应含位置/索引类措辞
    let msg = try_parse_hand("123n").unwrap_err().to_string();
    assert!(msg.contains('n'), "应报出该字符: {}", msg);
    assert!(
        !msg.contains("位置") && !msg.contains("index") && !msg.contains("offset"),
        "C5：不得要求指出位置: {}",
        msg
    );
}

/// 数字 0 与悬空牌点必须报错（旧 `parse_hand` 会静默忽略）。
#[test]
fn b2_parse_rejects_zero_and_dangling_digits() {
    assert_eq!(try_parse_hand("0m").unwrap_err(), ParseError::ZeroDigit);
    assert_eq!(try_parse_hand("000").unwrap_err(), ParseError::ZeroDigit);
    assert_eq!(try_parse_hand("123").unwrap_err(), ParseError::DanglingDigits);
    assert_eq!(
        try_parse_hand("123456789m 55p 78p 9").unwrap_err(),
        ParseError::DanglingDigits
    );
}

/// 空白允许并忽略；**纯空白 ⇒ Ok(空手牌)**——
/// 「0 张」的拒绝属 CLI 层（`没有解析出任何牌`），库层不重复承担。
#[test]
fn b2_parse_allows_whitespace_and_empty_contract() {
    assert_eq!(total(&try_parse_hand("123m 456s").unwrap()), 6);
    assert_eq!(total(&try_parse_hand("   \t ").unwrap()), 0);
    assert!(try_parse_hand("").is_ok(), "空串在库层为 Ok(0 张)");
}

/// 旧 `parse_hand` 行为保持不变（向后兼容），与新的严格解析形成对照。
#[test]
fn b2_parse_hand_backward_compatible() {
    // `n` 会被静默跳过 ⇒ 数字 456 与前面的 123 合并：123456s + 789p + 东东 = 11 张
    assert_eq!(total(&parse_hand("123n 456s 789p 东东")), 11);
    // 悬空数字被忽略
    assert_eq!(total(&parse_hand("123")), 0);
    // 新解析器对同样输入报错（严格化）
    assert!(try_parse_hand("123n 456s 789p 东东").is_err());
}

// ==================== validate_counts：全局 ≤4（C7）与张数（C6） ====================

/// 合法形态必须通过：含**四杠**（C6，反向护栏）、单暗杠、无副露。
#[test]
fn b2_validate_accepts_legal_including_four_kongs() {
    // 四杠：4 副杠 ⇒ 暗牌 = 13 - 3×4 = 1 张
    let c = parse_hand("5p");
    let melds = vec![
        Meld::kan(0, true),  // 明杠 1万
        Meld::kan(8, true),  // 明杠 9万
        Meld::kan(9, true),  // 明杠 1条
        Meld::kan(17, true), // 明杠 9条
    ];
    assert_eq!(total(&c), 1);
    assert!(
        validate_counts(&c, &melds).is_ok(),
        "四杠必须合法（C6），不得被新校验误杀"
    );

    // 1 副暗杠 + 10 张暗牌
    let c2 = parse_hand("147m 258s 369p 5p");
    assert_eq!(total(&c2), 10);
    assert!(validate_counts(&c2, &[Meld::kan(18, false)]).is_ok());

    // 4 副副露（吃/碰混合）+ 1 张暗牌
    let c3 = parse_hand("5p");
    let melds3 = vec![
        Meld::chi(0),
        Meld::pon(9),
        Meld::pon(18),
        Meld::chi(19),
    ];
    assert!(validate_counts(&c3, &melds3).is_ok(), "4 副副露 + 1 张暗牌应合法");

    // 无副露 13 张
    assert!(validate_counts(&parse_hand("123456789m 55p 78p"), &[]).is_ok());
}

/// 「同一张 ≤4」为**全局计数**（暗牌＋副露＋杠合计，C7）：5 张即拒，恰 4 张应接受。
#[test]
fn b2_validate_rejects_over4_globally_but_allows_exactly4() {
    // 暗牌内 5 张 9万
    match validate_counts(&parse_hand("111m 999m 23m 456m 99m"), &[]) {
        Err(InputError::Over4(t, n)) => {
            assert_eq!(tile_name(t), "9万");
            assert_eq!(n, 5);
        }
        other => panic!("应报 Over4(9万, 5)，实际 {:?}", other),
    }

    // 杠与暗牌**合并**计数：暗牌 1 张 5筒 + 明杠 4 张 5筒 = 5 张 ⇒ 拒
    match validate_counts(&parse_hand("5p"), &[Meld::kan(18 + 4, true)]) {
        Err(InputError::Over4(t, n)) => {
            assert_eq!(tile_name(t), "5筒");
            assert_eq!(n, 5);
        }
        other => panic!("C7 应合并杠计数并报 Over4(5筒, 5)，实际 {:?}", other),
    }

    // 暗杠同理
    assert!(matches!(
        validate_counts(&parse_hand("5p"), &[Meld::kan(18 + 4, false)]),
        Err(InputError::Over4(_, 5))
    ));

    // 边界：恰好 4 张 9万（13 张）⇒ 接受
    let c4 = parse_hand("9999m123p456p789p");
    assert_eq!(total(&c4), 13);
    assert!(
        validate_counts(&c4, &[]).is_ok(),
        "恰好 4 张应接受（边界）"
    );
}

/// 张数必须等于 `13 - 3×副露面子数`（面子可为杠，C6）。
#[test]
fn b2_validate_rejects_count_mismatch() {
    // 14 张、无副露 ⇒ 应为 13
    match validate_counts(&parse_hand("123456789m 55p 78p 9p"), &[]) {
        Err(InputError::Count { expected, got }) => assert_eq!((expected, got), (13, 14)),
        other => panic!("应报 Count(13,14)，实际 {:?}", other),
    }

    // 1 副碰 + 11 张 ⇒ 应为 10
    match validate_counts(&parse_hand("123456789m 55p"), &[Meld::pon(0)]) {
        Err(InputError::Count { expected, got }) => assert_eq!((expected, got), (10, 11)),
        other => panic!("应报 Count(10,11)，实际 {:?}", other),
    }

    // 杠按 1 个面子计：1 副杠 ⇒ 应为 10（不是 9）
    match validate_counts(&parse_hand("5p"), &[Meld::kan(18, false)]) {
        Err(InputError::Count { expected, got }) => assert_eq!((expected, got), (10, 1)),
        other => panic!("杠应按 1 副面子 ⇒ Count(10,1)，实际 {:?}", other),
    }
}

/// 校验**顺序**：同一手同时违规时，先报「全局 ≤4」（C7）再报张数（C6）。
#[test]
fn b2_validate_checks_over4_before_count() {
    // 暗牌 1 张 5筒 + 明杠 5筒：既超 4，又张数不符 ⇒ 必须报 Over4
    match validate_counts(&parse_hand("5p"), &[Meld::kan(18 + 4, true)]) {
        Err(InputError::Over4(..)) => {}
        other => panic!("应先报 Over4，实际 {:?}", other),
    }
}

/// 记录分工（防将来误改）：`>4 副副露` 的拒绝在 **CLI 层**（`melds.len() > 4`），
/// `validate_counts` 只负责「全局 ≤4」与「13−3n」；5 副时 expected 饱和为 0。
#[test]
fn b2_five_melds_gate_is_cli_level() {
    let melds: Vec<Meld> = vec![
        Meld::pon(0),
        Meld::pon(1),
        Meld::pon(2),
        Meld::pon(3),
        Meld::pon(4),
    ];
    assert!(
        validate_counts(&parse_hand(""), &melds).is_ok(),
        "5 副副露的拒绝由 CLI 层 melds.len()>4 承担（本断言记录该分工）"
    );
}
