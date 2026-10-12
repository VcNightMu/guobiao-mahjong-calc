//! CLI 参数校验回归（R1/R4）：非法副露 token 必须 exit 2 且文案明确；合法输入不得被误杀。
//! 用 `env!("CARGO_BIN_EXE_gbmj")` 直调构建出的二进制，不依赖 CWD 与外部脚本。

use std::process::Command;

fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_gbmj"))
        .args(args)
        .output()
        .expect("spawn gbmj");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// R1：`--chi 8p` 会构造出 `8筒/9筒/东`（越界到字牌区）⇒ 必须拒绝。
#[test]
fn chi_crossing_into_honors_is_rejected() {
    let (code, _o, err) = run(&["123456789m", "1s", "--chi", "8p"]);
    assert_eq!(code, 2, "非法吃必须 exit 2；stderr={err}");
    assert!(
        err.contains("副露参数非法"),
        "stderr 应含「副露参数非法」；实得 {err}"
    );
}

/// 防误杀：合法吃（7p8p9p）必须仍然 exit 0。
#[test]
fn chi_legal_still_succeeds() {
    let (code, out, err) = run(&["123456789m", "1s", "--chi", "7p"]);
    assert_eq!(code, 0, "合法吃不得被拒；stderr={err}");
    assert!(out.contains("副露 1 副"), "应正常输出；实得 {out}");
}

/// 防误杀 2：三张连续的合法写法（`--chi 234m`）也必须 exit 0。
#[test]
fn chi_three_tile_run_still_succeeds() {
    let (code, _o, err) = run(&["78s", "东东", "--chi", "234m", "--pon", "5p", "--pon", "6s"]);
    assert_eq!(code, 0, "三张连续写法不得被拒；stderr={err}");
}

/// R4：多牌 token（`--pon 12m`）必须明确报错，不得静默取首张。
#[test]
fn pon_with_multi_tile_token_is_rejected() {
    let (code, _o, err) = run(&["123456789m", "1s", "--pon", "12m"]);
    assert_eq!(code, 2, "多牌 token 必须 exit 2；stderr={err}");
    assert!(
        err.contains("副露参数非法"),
        "stderr 应含「副露参数非法」；实得 {err}"
    );
}

/// 防误杀：单张 token（`--pon 1m`）必须仍然 exit 0。
#[test]
fn pon_single_tile_still_succeeds() {
    let (code, _o, err) = run(&["123456789m", "1s", "--pon", "1m"]);
    assert_eq!(code, 0, "合法碰不得被拒；stderr={err}");
}

/// R4 第二形态：副露选项之后的散牌不得再报成「暗牌张数不符」。
#[test]
fn bare_tile_after_meld_option_is_reported_as_meld_error() {
    let (code, _o, err) = run(&["123456789m", "1s", "--pon", "1m", "2m"]);
    assert_eq!(code, 2, "应 exit 2；stderr={err}");
    assert!(
        err.contains("副露选项之后"),
        "应报副露/位置参数错误；实得 {err}"
    );
    assert!(
        !err.contains("暗牌张数不符"),
        "不得再报成手牌张数问题；实得 {err}"
    );
}
