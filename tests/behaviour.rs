//! Behaviour of the checks of the files that are read, of the languages and their keys, and of the progress of a plan, at the exact edge of each rule. Seeded and without a clock.

use eh3d_drillpro::i18n::{cell_width, fold_key, str_width, Lang};
use eh3d_drillpro::input_guard::{
    check, check_len, InputKind, InputProblem, MAX_INPUT_BYTES, MAX_LINE_CHARS, MAX_NUMBER_DIGITS,
};
use eh3d_drillpro::progress::{item_seconds, Progress};
use eh3d_drillpro::protocol::job::{Item, Pause};
use eh3d_drillpro::protocol::latches::Op;
use eh3d_drillpro::protocol::motion::Geometry;
use eh3d_drillpro::protocol::{bits, Step};

const KINDS: [InputKind; 4] = [
    InputKind::Ncd,
    InputKind::Hpgl,
    InputKind::Dxf,
    InputKind::Gerber,
];

/// A small generator with a fixed seed (31 bits of state).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = (self.0 * 1_103_515_245 + 12_345) & 0x7FFF_FFFF;
        self.0 >> 8
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

// ---------------------------------------------------------------- the limits of the files

#[test]
fn a_file_of_exactly_the_most_bytes_is_allowed_and_one_more_is_turned_down_with_its_size() {
    assert_eq!(check_len(0), Ok(()));
    assert_eq!(check_len(MAX_INPUT_BYTES as u64), Ok(()));
    let over = MAX_INPUT_BYTES as u64 + 1;
    assert_eq!(check_len(over), Err(InputProblem::TooLarge(over)));
    assert_eq!(check_len(u64::MAX), Err(InputProblem::TooLarge(u64::MAX)));
}

#[test]
fn an_empty_text_and_a_text_with_dos_line_ends_are_inside_the_limits_of_every_kind() {
    for kind in KINDS {
        assert_eq!(check(kind, ""), Ok(()));
        assert_eq!(check(kind, "\n\n\n"), Ok(()));
        assert_eq!(check(kind, "M48\r\nT01C0.8\r\n%\r\n"), Ok(()));
    }
}

#[test]
fn a_number_may_have_forty_digits_and_not_one_more_and_a_break_restarts_the_count() {
    let digits = |n: usize| "7".repeat(n);
    for kind in KINDS {
        assert_eq!(check(kind, &digits(MAX_NUMBER_DIGITS)), Ok(()));
        assert_eq!(
            check(kind, &digits(MAX_NUMBER_DIGITS + 1)),
            Err(InputProblem::NumberTooLong(1))
        );
        let broken = format!(
            "{}.{}",
            digits(MAX_NUMBER_DIGITS),
            digits(MAX_NUMBER_DIGITS)
        );
        assert_eq!(check(kind, &broken), Ok(()));
        let two_lines = format!(
            "{}\n{}",
            digits(MAX_NUMBER_DIGITS),
            digits(MAX_NUMBER_DIGITS)
        );
        assert_eq!(check(kind, &two_lines), Ok(()));
    }
}

#[test]
fn the_line_of_a_problem_counts_from_one_and_is_the_line_it_is_on() {
    assert_eq!(check(InputKind::Ncd, "a\n\0"), Err(InputProblem::Binary(2)));
    assert_eq!(check(InputKind::Hpgl, "\0"), Err(InputProblem::Binary(1)));
    let long_number = format!("ok\nok\nX{}", "1".repeat(MAX_NUMBER_DIGITS + 1));
    assert_eq!(
        check(InputKind::Gerber, &long_number),
        Err(InputProblem::NumberTooLong(3))
    );
    let at_the_end = format!("{}\n\n\n\0", "x".repeat(10));
    assert_eq!(
        check(InputKind::Dxf, &at_the_end),
        Err(InputProblem::Binary(4))
    );
}

#[test]
fn a_long_line_is_a_problem_only_in_the_formats_that_are_written_by_lines() {
    let line = "x".repeat(MAX_LINE_CHARS);
    let too_long = "x".repeat(MAX_LINE_CHARS + 1);
    for kind in KINDS {
        assert_eq!(check(kind, &line), Ok(()));
    }
    for kind in [InputKind::Ncd, InputKind::Dxf] {
        assert_eq!(check(kind, &too_long), Err(InputProblem::LineTooLong(1)));
        assert_eq!(
            check(kind, &format!("short\n{too_long}")),
            Err(InputProblem::LineTooLong(2))
        );
    }
    for kind in [InputKind::Hpgl, InputKind::Gerber] {
        assert_eq!(check(kind, &too_long), Ok(()));
    }
}

#[test]
fn the_length_of_a_line_is_counted_in_characters_and_not_in_bytes() {
    let wide = "\u{e9}".repeat(MAX_LINE_CHARS);
    assert!(wide.len() > MAX_LINE_CHARS);
    assert_eq!(check(InputKind::Ncd, &wide), Ok(()));
    let wider = "\u{e9}".repeat(MAX_LINE_CHARS + 1);
    assert_eq!(
        check(InputKind::Ncd, &wider),
        Err(InputProblem::LineTooLong(1))
    );
}

#[test]
fn the_first_problem_in_reading_order_is_the_one_told_and_the_codes_never_change() {
    let both = format!("\0{}", "9".repeat(MAX_NUMBER_DIGITS + 1));
    assert_eq!(check(InputKind::Ncd, &both), Err(InputProblem::Binary(1)));
    let number_first = format!("{}\n\0", "9".repeat(MAX_NUMBER_DIGITS + 1));
    assert_eq!(
        check(InputKind::Ncd, &number_first),
        Err(InputProblem::NumberTooLong(1))
    );
    let codes = [
        (InputProblem::TooLarge(1), "input_too_large", None),
        (InputProblem::Binary(5), "input_binary", Some(5)),
        (InputProblem::LineTooLong(6), "input_line_too_long", Some(6)),
        (
            InputProblem::NumberTooLong(7),
            "input_number_too_long",
            Some(7),
        ),
    ];
    for (problem, code, line) in codes {
        assert_eq!(problem.code(), code);
        assert_eq!(problem.line(), line);
        assert!(problem.to_string().starts_with(code));
        assert_eq!(problem.to_string().contains("(line "), line.is_some());
    }
    assert_eq!(InputProblem::Binary(2).to_string(), "input_binary (line 2)");
}

#[test]
fn seeded_garbage_is_judged_without_a_panic_and_a_clean_text_is_always_allowed() {
    let mut rng = Lcg(2025);
    for round in 0..200 {
        let len = rng.below(300) as usize;
        let text: String = (0..len)
            .map(|_| char::from_u32(rng.below(256) as u32).unwrap_or('?'))
            .collect();
        for kind in KINDS {
            let verdict = check(kind, &text);
            if !text.contains('\0') && !text.bytes().any(|b| b.is_ascii_digit()) {
                assert_eq!(verdict, Ok(()), "round {round}");
            }
        }
    }
}

// ---------------------------------------------------------------- the languages and their keys

#[test]
fn the_eight_languages_are_in_the_fixed_order_and_every_language_has_its_place() {
    let codes: Vec<&str> = Lang::ALL.iter().map(|l| l.code()).collect();
    assert_eq!(codes, vec!["en", "es", "it", "de", "fr", "pt", "zh", "ja"]);
    for (i, lang) in Lang::ALL.iter().enumerate() {
        assert_eq!(*lang as usize, i);
    }
    assert_eq!(Lang::default(), Lang::En);
}

#[test]
fn the_codes_and_the_names_of_the_languages_are_all_different() {
    for (i, a) in Lang::ALL.iter().enumerate() {
        assert!(!a.native_name().is_empty());
        for b in &Lang::ALL[i + 1..] {
            assert_ne!(a.code(), b.code());
            assert_ne!(a.native_name(), b.native_name());
        }
    }
}

#[test]
fn a_code_of_a_locale_gives_its_language_and_a_region_gives_the_main_one() {
    for (text, lang) in [
        ("es_ES.UTF-8", Some(Lang::Es)),
        ("PT-pt", Some(Lang::Pt)),
        ("pt-br", Some(Lang::Pt)),
        ("pt_BR", Some(Lang::Pt)),
        ("fr-ca", Some(Lang::Fr)),
        ("fr_CA.UTF-8", Some(Lang::Fr)),
        ("fr_CH", Some(Lang::Fr)),
        ("  de  ", Some(Lang::De)),
        ("JA_JP.eucJP", Some(Lang::Ja)),
        ("zh-Hans-CN", Some(Lang::Zh)),
        ("ru", None),
        ("pl_PL.UTF-8", None),
        ("xx", None),
        ("", None),
        ("C", None),
    ] {
        assert_eq!(Lang::from_code(text), lang, "{text:?}");
    }
    assert_eq!(Lang::from_locale("POSIX"), Lang::En);
    assert_eq!(Lang::from_locale("it_IT"), Lang::It);
    assert_eq!(Lang::from_locale("ru_RU"), Lang::En);
    for lang in Lang::ALL {
        assert_eq!(Lang::from_code(lang.code()), Some(lang));
    }
}

#[test]
fn the_width_of_a_character_changes_exactly_at_the_edges_of_the_wide_ranges() {
    let width = |u: u32| cell_width(char::from_u32(u).unwrap());
    for (u, w) in [
        (0x0041, 1),
        (0x00E9, 1),
        (0x10FF, 1),
        (0x1100, 2),
        (0x115F, 2),
        (0x1160, 1),
        (0x2E7F, 1),
        (0x2E80, 2),
        (0x65E5, 2),
        (0x30A2, 2),
        (0xA4CF, 2),
        (0xA4D0, 1),
        (0xAC00, 2),
        (0xD7A3, 2),
        (0xD7A4, 1),
        (0xFF00, 2),
        (0xFF60, 2),
        (0xFF61, 1),
        (0xFFE0, 2),
        (0xFFE6, 2),
        (0xFFE7, 1),
    ] {
        assert_eq!(width(u), w, "{u:#x}");
    }
}

#[test]
fn the_width_of_a_text_ignores_the_marks_of_the_hot_key() {
    assert_eq!(str_width(""), 0);
    assert_eq!(str_width("~L~oad"), 4);
    assert_eq!(str_width("Load"), 4);
    assert_eq!(str_width("\u{8F7D}\u{5165}(~L~)"), 7);
    assert_eq!(str_width("~~~"), 0);
    let text = "Ab\u{65E5}~x~";
    assert_eq!(str_width(text), 1 + 1 + 2 + 1);
}

#[test]
fn a_typed_letter_is_folded_to_lower_case() {
    for (typed, folded) in [
        ('A', 'a'),
        ('z', 'z'),
        ('\u{C9}', '\u{E9}'),
        ('1', '1'),
        ('~', '~'),
    ] {
        assert_eq!(fold_key(typed), folded, "{typed}");
    }
    for lang in Lang::ALL {
        for key in 'A'..='Z' {
            assert_eq!(lang.hot_key(key), key.to_ascii_lowercase());
        }
    }
}

// ---------------------------------------------------------------- the progress of a plan

fn run_of(n: usize, delay_us: f64) -> Item {
    Item::Steps(vec![
        Step {
            byte: bits::X_POS,
            delay_us
        };
        n
    ])
}

fn plan() -> Vec<Item> {
    vec![
        run_of(10, 1000.0),
        Item::Latch(vec![Op::WaitUs(500_000), Op::Send(0x80)]),
        Item::Spindle {
            rpm: 30_000,
            wait_ms: 2000,
        },
        Item::Pause(Pause::ToolChange { tool: 1 }),
    ]
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn each_kind_of_item_takes_the_time_it_takes_on_the_machine_and_a_dialog_takes_none() {
    let items = plan();
    assert!(near(item_seconds(&items[0]), 0.01));
    assert!(near(item_seconds(&items[1]), 0.5));
    assert!(near(item_seconds(&items[2]), 2.0));
    assert_eq!(item_seconds(&items[3]), 0.0);
    assert_eq!(item_seconds(&Item::Pause(Pause::SpindleStart)), 0.0);
    assert_eq!(item_seconds(&Item::Steps(Vec::new())), 0.0);
    let one = Step {
        byte: bits::X_POS,
        delay_us: 250.0,
    };
    assert!(near(item_seconds(&Item::Search(vec![one])), 0.00025));
    assert_eq!(
        item_seconds(&Item::Latch(vec![Op::Send(1), Op::Send(2)])),
        0.0
    );
}

#[test]
fn the_time_done_is_the_time_of_the_items_before_plus_the_part_of_the_one_that_runs() {
    let progress = Progress::new(&plan(), &[0, 2]);
    assert!(near(progress.total_seconds(), 2.51));
    assert_eq!(progress.done_seconds(0, 0.0), 0.0);
    assert!(near(progress.done_seconds(1, 0.0), 0.01));
    assert!(near(progress.done_seconds(1, 0.3), 0.31));
    assert!(near(progress.done_seconds(1, 99.0), 0.51));
    assert!(near(progress.done_seconds(1, -5.0), 0.01));
    assert!(near(progress.done_seconds(2, 1.0), 1.51));
    assert!(near(progress.done_seconds(4, 0.0), 2.51));
    assert!(near(progress.done_seconds(100, 7.0), 2.51));
}

#[test]
fn the_time_left_is_what_the_plan_takes_less_what_is_done_and_never_negative() {
    let progress = Progress::new(&plan(), &[]);
    assert!(near(progress.remaining_seconds(0, 0.0), 2.51));
    assert!(near(progress.remaining_seconds(2, 0.5), 2.51 - 1.01));
    assert_eq!(progress.remaining_seconds(4, 0.0), 0.0);
    assert_eq!(progress.remaining_seconds(1000, 1e9), 0.0);
}

#[test]
fn the_fraction_goes_from_zero_to_one_and_never_goes_back_along_a_seeded_walk() {
    let progress = Progress::new(&plan(), &[0, 2]);
    assert_eq!(progress.fraction(0, 0.0), 0.0);
    assert!(near(progress.fraction(4, 0.0), 1.0));
    let mut rng = Lcg(14);
    let (mut next, mut into) = (0usize, 0.0f64);
    let mut last = 0.0;
    for _ in 0..200 {
        into += rng.below(400) as f64 / 1000.0;
        if rng.below(4) == 0 && next < 4 {
            next += 1;
            into = 0.0;
        }
        let f = progress.fraction(next, into);
        assert!((0.0..=1.0).contains(&f), "{f}");
        assert!(f + 1e-12 >= last, "{f} after {last}");
        last = f;
    }
}

#[test]
fn a_plan_that_takes_no_time_is_done_when_all_its_items_are() {
    let empty = Progress::new(&[], &[]);
    assert_eq!(empty.total_seconds(), 0.0);
    assert_eq!(empty.fraction(0, 0.0), 1.0);
    assert_eq!(empty.units_total(), 0);
    let dialogs = [
        Item::Pause(Pause::SpindleStart),
        Item::Pause(Pause::ToolChange { tool: 2 }),
    ];
    let progress = Progress::new(&dialogs, &[1]);
    assert_eq!(progress.fraction(0, 0.0), 0.0);
    assert_eq!(progress.fraction(1, 0.0), 0.0);
    assert_eq!(progress.fraction(2, 0.0), 1.0);
    assert_eq!(progress.fraction(50, 0.0), 1.0);
}

#[test]
fn a_unit_of_work_is_done_when_the_run_has_gone_past_the_item_that_ends_it() {
    let progress = Progress::new(&plan(), &[0, 2]);
    assert_eq!(progress.units_total(), 2);
    for (next, done) in [(0, 0), (1, 1), (2, 1), (3, 2), (4, 2), (99, 2)] {
        assert_eq!(progress.units_done(next), done, "{next}");
    }
}

#[test]
fn a_plan_that_was_not_followed_does_not_know_where_the_head_is() {
    let progress = Progress::new(&plan(), &[]);
    assert_eq!(progress.head_mm(0, 0.0), None);
    assert_eq!(progress.head_mm(3, 1.0), None);
    assert_eq!(Progress::default().head_mm(0, 0.0), None);
}

#[test]
fn a_followed_plan_puts_the_head_where_the_steps_have_taken_it() {
    let g = Geometry::default();
    let items = vec![run_of(100, 1000.0), run_of(100, 1000.0)];
    let progress = Progress::new(&items, &[0, 1]).with_track(&items, &g);
    let per_mm = g.steps_per_mm_xy;
    assert_eq!(progress.head_mm(0, 0.0), Some((0.0, 0.0, 0.0)));
    let (x, y, z) = progress.head_mm(1, 0.0).unwrap();
    assert!((x - 100.0 / per_mm).abs() < 1e-9 && y == 0.0 && z == 0.0);
    let (x, _, _) = progress.head_mm(2, 0.0).unwrap();
    assert!((x - 200.0 / per_mm).abs() < 1e-9);
    let (end, _, _) = progress.head_mm(50, 3.0).unwrap();
    assert!((end - 200.0 / per_mm).abs() < 1e-9);
}

#[test]
fn inside_an_item_the_head_is_on_the_way_between_its_ends_and_never_goes_behind_a_switch() {
    let g = Geometry::default();
    let items = vec![run_of(100, 1000.0)];
    let progress = Progress::new(&items, &[0]).with_track(&items, &g);
    let (x, _, _) = progress.head_mm(0, 0.05).unwrap();
    assert!((x - 50.0 / g.steps_per_mm_xy).abs() < 1e-6, "{x}");
    let (before, _, _) = progress.head_mm(0, -3.0).unwrap();
    assert_eq!(before, 0.0);
    let (after, _, _) = progress.head_mm(0, 99.0).unwrap();
    assert!((after - 100.0 / g.steps_per_mm_xy).abs() < 1e-9);
    let backwards = vec![Item::Steps(vec![
        Step {
            byte: bits::X_NEG,
            delay_us: 1000.0
        };
        20
    ])];
    let behind = Progress::new(&backwards, &[0]).with_track(&backwards, &g);
    assert_eq!(behind.head_mm(1, 0.0), Some((0.0, 0.0, 0.0)));
}
