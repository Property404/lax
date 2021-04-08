use assert_cmd;

fn setup_command() -> assert_cmd::cmd::Command {
    assert_cmd::Command::cargo_bin("lax").unwrap()
}

// Lex should only work when presented with a binary
#[test]
fn fails_with_no_binary() {
    setup_command().assert().failure();
}
#[test]
fn fails_with_nonexistant_binary() {
    setup_command()
        .arg("Great googly moogly!")
        .assert()
        .failure();
}

// Ensure argument parser is working correctly
#[test]
fn help_flag() {
    setup_command().arg("--help").assert().success();
    setup_command().arg("-h").assert().success();
    // This should pass because 'h' and 'h' are both valid short flags
    // and there's no point in checking for duplicates
    setup_command().arg("-hh").assert().success();
}
#[test]
fn no_such_argument() {
    setup_command().arg("--tinkleberries").assert().failure();
    setup_command()
        .arg("-abcdefghijklmnopqrstuvwxyz")
        .assert()
        .failure();
}

// Lex should just work as the regular program when not presented with additional arguments beyond
// the binary
//
// "lax ls" should be functionally the same as "ls"
#[test]
fn no_args_check() {
    setup_command().arg("ls").assert().success();
}

// Test most basic functionality
#[test]
fn single_substitution_check() {
    setup_command()
        .arg("echo")
        .arg("@foo")
        .assert()
        .success()
        .stdout("./tests/foobar/foo\n");
}
#[test]
fn mixed_args_check() {
    setup_command()
        .arg("echo")
        .arg("@foo")
        .arg("foo")
        .arg("@foo")
        .assert()
        .success()
        .stdout("./tests/foobar/foo foo ./tests/foobar/foo\n");
}

// Lax will fail if it can't transform an '@' argument
#[test]
fn fails_when_file_not_found() {
    setup_command()
        .arg("echo")
        .arg("@great_googly_moogly.txt")
        .assert()
        .failure();
}

// Ensure the 'match with directories' functionality is working, but not on by default
#[test]
fn match_with_dirs() {
    setup_command()
        .arg("-d")
        .arg("echo")
        .arg("@this_is_a_directory")
        .assert()
        .success();
    setup_command()
        .arg("echo")
        .arg("@this_is_a_directory")
        .assert()
        .failure();
}