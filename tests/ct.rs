extern crate compiletest_rs as compiletest;

use std::path::PathBuf;

use compiletest::common::Mode;

#[test]
fn compile_fail() {
    let mut config = compiletest::Config::default().tempdir();

    config.mode = Mode::CompileFail;
    config.src_base = PathBuf::from("tests/compile-fail");
    config.link_deps();
    config.clean_rmeta();

    compiletest::run_tests(&config);
}
