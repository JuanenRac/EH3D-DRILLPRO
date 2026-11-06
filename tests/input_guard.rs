use eh3d_drillpro::input_guard::{
    check, check_len, InputKind, InputProblem, MAX_INPUT_BYTES, MAX_LINE_CHARS, MAX_NUMBER_DIGITS,
};

fn read(name: &str) -> String {
    let b = std::fs::read(format!("{}/testdata/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    b.iter().map(|&c| c as char).collect()
}

#[test]
fn the_sample_files_of_the_machines_are_inside_the_limits() {
    for name in ["14TOOLS.NCD", "speedtst.ncd", "I8d.ncd", "refholes.ncd"] {
        assert_eq!(check(InputKind::Ncd, &read(name)), Ok(()), "{name}");
    }
    assert_eq!(check(InputKind::Ncd, ""), Ok(()));
}

#[test]
fn a_file_that_is_too_big_is_turned_down_by_its_size() {
    assert_eq!(check_len(MAX_INPUT_BYTES as u64), Ok(()));
    let over = MAX_INPUT_BYTES as u64 + 1;
    assert_eq!(check_len(over), Err(InputProblem::TooLarge(over)));
    assert_eq!(InputProblem::TooLarge(over).code(), "input_too_large");
    assert_eq!(InputProblem::TooLarge(over).line(), None);
    assert_eq!(InputProblem::TooLarge(over).to_string(), "input_too_large");
}

#[test]
fn a_character_zero_is_a_binary_file_and_the_line_is_told() {
    let text = "%\nT01\nX100Y100\nX2\u{0}0Y5\n";
    let problem = check(InputKind::Ncd, text).unwrap_err();
    assert_eq!(problem, InputProblem::Binary(4));
    assert_eq!(problem.code(), "input_binary");
    assert_eq!(problem.to_string(), "input_binary (line 4)");
    // plots and outlines are text too
    assert_eq!(
        check(InputKind::Hpgl, "IN;\u{0}PU0,0;"),
        Err(InputProblem::Binary(1))
    );
    assert_eq!(
        check(InputKind::Gerber, "%FSLAX24Y24*%\n\u{0}"),
        Err(InputProblem::Binary(2))
    );
}

#[test]
fn a_line_that_never_ends_is_turned_down_in_the_formats_written_by_lines() {
    let long = "X".repeat(MAX_LINE_CHARS + 1);
    let exact = "X".repeat(MAX_LINE_CHARS);
    let text = format!("%\n{exact}\n{long}\n");
    for kind in [InputKind::Ncd, InputKind::Dxf] {
        let problem = check(kind, &text).unwrap_err();
        assert_eq!(problem, InputProblem::LineTooLong(3));
        assert_eq!(problem.to_string(), "input_line_too_long (line 3)");
        assert_eq!(check(kind, &format!("%\n{exact}\n")), Ok(()));
    }
    // a plot and an outline put many commands on one line: only their size is limited
    for kind in [InputKind::Hpgl, InputKind::Gerber] {
        assert_eq!(check(kind, &text), Ok(()));
    }
}

#[test]
fn a_number_of_absurd_length_is_turned_down_with_its_line() {
    let longest = "9".repeat(MAX_NUMBER_DIGITS);
    let absurd = "9".repeat(MAX_NUMBER_DIGITS + 1);
    for kind in [
        InputKind::Ncd,
        InputKind::Hpgl,
        InputKind::Dxf,
        InputKind::Gerber,
    ] {
        assert_eq!(check(kind, &format!("T01\nX{longest}Y5\n")), Ok(()));
        let problem = check(kind, &format!("T01\nX1Y1\nX{absurd}Y5\n")).unwrap_err();
        assert_eq!(problem, InputProblem::NumberTooLong(3));
        assert_eq!(problem.code(), "input_number_too_long");
        assert_eq!(problem.to_string(), "input_number_too_long (line 3)");
    }
    // the digits of two numbers do not add up, whatever stands between them
    let apart = format!("X{longest}Y{longest}\n0.{longest}\n");
    assert_eq!(check(InputKind::Dxf, &apart), Ok(()));
}

#[test]
fn the_first_problem_in_the_file_is_the_one_told() {
    let absurd = "7".repeat(MAX_NUMBER_DIGITS + 1);
    // a number on line 2 is met before a character zero on line 3
    let text = format!("a\nX{absurd}\nb\u{0}\n");
    assert_eq!(
        check(InputKind::Ncd, &text),
        Err(InputProblem::NumberTooLong(2))
    );
    // in one line a character zero comes before the length of the line
    let both = format!("{}\u{0}", "X".repeat(MAX_LINE_CHARS + 1));
    assert_eq!(check(InputKind::Ncd, &both), Err(InputProblem::Binary(1)));
    // the lines are counted by their line feeds, with or without a carriage return before them
    let crlf = "a\r\nb\r\nX\u{0}\r\n";
    assert_eq!(check(InputKind::Ncd, crlf), Err(InputProblem::Binary(3)));
}
