use assert_cmd;

fn setup_command() -> assert_cmd::cmd::Command {
    assert_cmd::Command::cargo_bin("lax").unwrap()
}

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

#[test]
fn no_args_check() {
    setup_command().arg("ls").assert().success();
}

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

#[test]
fn fails_when_file_not_found() {
    setup_command()
        .arg("echo")
        .arg("@great_googly_moogly.txt")
        .assert()
        .failure();
}
