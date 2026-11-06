use eh3d_drillpro::config::{rack_line, MachineSettings, RackSetup};
use eh3d_drillpro::i18n::{str_width, Lang};
use eh3d_drillpro::job::{job_rack, job_rack_for, JobOptions, RackRefusal};
use eh3d_drillpro::server_link::{with_chain, without_chain};
use eh3d_drillpro::work_contract::{
    chain_answer, chain_options, check_rack_options, rack_ignored, refusal_detail,
    unknown_rack_fields, warning, warning_lines, warning_text, wrap_cells, AnswerWarning,
    RackProblem, BAD_RACK, MAX_RACK_PLACES,
};
use serde_json::json;

fn slots(list: &[(u8, usize)]) -> JobOptions {
    JobOptions {
        rack_slots: list.to_vec(),
        ..JobOptions::default()
    }
}

#[test]
fn a_rack_that_fits_passes_every_check() {
    assert_eq!(
        check_rack_options(&JobOptions::default(), MAX_RACK_PLACES),
        Ok(())
    );
    let chained = JobOptions {
        rack_first: 14,
        rack_carry_in: true,
        rack_carry_out: true,
        rack_slots: vec![(1, 6), (2, 2)],
        rack_carry_slot: Some(4),
        ..JobOptions::default()
    };
    assert_eq!(check_rack_options(&chained, MAX_RACK_PLACES), Ok(()));
}

#[test]
fn each_rack_that_does_not_fit_is_told_with_the_sentence_of_the_server() {
    let carry = |slot: Option<usize>, carry_in: bool, given: &[(u8, usize)]| JobOptions {
        rack_carry_in: carry_in,
        rack_carry_slot: slot,
        rack_slots: given.to_vec(),
        ..JobOptions::default()
    };
    let cases: Vec<(JobOptions, usize, RackProblem, &str)> = vec![
        (
            JobOptions {
                rack_first: 15,
                ..JobOptions::default()
            },
            14,
            RackProblem::FirstTooBig(14),
            "rack.first may be up to 14 places",
        ),
        (
            slots(&[(1, 1), (2, 2), (3, 3)]),
            2,
            RackProblem::TooManySlots(2),
            "rack.slots may name up to 2 places",
        ),
        (
            slots(&[(5, 0)]),
            14,
            RackProblem::TestPinSlot(5),
            "rack.slots: place 0 is the test pin, not a place for tool 5",
        ),
        (
            slots(&[(1, 1), (1, 2)]),
            14,
            RackProblem::ToolTwice(1),
            "rack.slots: tool 1 is given two places",
        ),
        (
            slots(&[(1, 1), (2, 1)]),
            14,
            RackProblem::PlaceTwice(1),
            "rack.slots: place 1 is given to two tools",
        ),
        (
            carry(Some(4), false, &[]),
            14,
            RackProblem::CarrySlotNeedsCarryIn,
            "rack.carry_slot needs rack.carry_in",
        ),
        (
            carry(Some(0), true, &[]),
            14,
            RackProblem::CarrySlotTestPin,
            "rack.carry_slot: place 0 is the test pin",
        ),
        (
            carry(Some(4), true, &[(1, 4)]),
            14,
            RackProblem::CarrySlotTaken(4),
            "rack.carry_slot: place 4 is also the place of a tool of this job",
        ),
    ];
    for (options, most, problem, detail) in cases {
        let found = check_rack_options(&options, most).unwrap_err();
        assert_eq!(found, problem);
        assert_eq!(found.detail(), detail);
        assert_eq!(found.code(), BAD_RACK);
        assert_eq!(found.to_string(), format!("bad_rack: {detail}"));
    }
}

#[test]
fn the_first_problem_found_is_the_one_told_in_the_order_of_the_server() {
    // `first` is judged before the places, and the number of places before each of them
    let both = JobOptions {
        rack_first: 20,
        rack_slots: vec![(1, 0)],
        ..JobOptions::default()
    };
    assert_eq!(
        check_rack_options(&both, 14),
        Err(RackProblem::FirstTooBig(14))
    );
    // place 0 is judged before a repeated tool, whichever slot has it
    assert_eq!(
        check_rack_options(&slots(&[(1, 0), (1, 2)]), 14),
        Err(RackProblem::TestPinSlot(1))
    );
    assert_eq!(
        check_rack_options(&slots(&[(1, 2), (1, 0)]), 14),
        Err(RackProblem::TestPinSlot(1))
    );
    // a repeated tool is judged before a repeated place
    assert_eq!(
        check_rack_options(&slots(&[(1, 2), (1, 2)]), 14),
        Err(RackProblem::ToolTwice(1))
    );
    // the slots are judged before the carry slot
    let mixed = JobOptions {
        rack_slots: vec![(1, 2), (2, 2)],
        rack_carry_slot: Some(0),
        ..JobOptions::default()
    };
    assert_eq!(
        check_rack_options(&mixed, 14),
        Err(RackProblem::PlaceTwice(2))
    );
}

fn changer_machine(places: usize) -> MachineSettings {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("config/DRILLPRO.DCF");
    let text = eh3d_drillpro::config::bytes_to_text(&std::fs::read(path).unwrap());
    let mut m = eh3d_drillpro::config::parse(&text)
        .unwrap()
        .settings()
        .unwrap();
    m.clip_mm = (300.0, 200.0);
    m.z_max_depth_mm = 40.0;
    m.auto_change = true;
    m.rack = RackSetup::default();
    m.atc = rack_line(
        (320.0, 40.0, 30.0),
        (320.0, 40.0 + 20.0 * (places as f64 - 1.0), 30.0),
        places,
    );
    m
}

#[test]
fn the_local_planner_refuses_with_the_words_the_server_uses() {
    let few = changer_machine(3);
    let too_many = job_rack(Some(&few), 4).unwrap_err();
    assert_eq!(too_many, RackRefusal::TooManyTools(3));
    assert_eq!(
        refusal_detail(&too_many),
        "the rack of the machine holds 3 tools at the most"
    );
    let mut gap = changer_machine(3);
    gap.atc.retain(|a| a.index != 2);
    let missing = job_rack(Some(&gap), 3).unwrap_err();
    assert_eq!(
        refusal_detail(&missing),
        "place 2 of the rack is not defined in the configuration file of the machine"
    );
    // the same through the places the person gave to the tools
    let given = slots(&[(1, 2)]);
    assert_eq!(
        refusal_detail(&job_rack_for(Some(&gap), 1, &given).unwrap_err()),
        "place 2 of the rack is not defined in the configuration file of the machine"
    );
    let mut far = changer_machine(3);
    far.rack.reach_x_mm = 5.0;
    let outside = job_rack(Some(&far), 3).unwrap_err();
    let RackRefusal::Outside(place) = outside else {
        panic!("a bar past the reach must be Outside, not {outside:?}");
    };
    assert_eq!(
        refusal_detail(&outside),
        format!("place {place} of the rack is out of the reach of the machine")
    );
}

#[test]
fn a_request_with_places_that_pass_the_checks_is_the_one_the_server_reads() {
    let options = JobOptions {
        rack_first: 2,
        rack_carry_in: true,
        rack_slots: vec![(1, 6), (2, 3)],
        rack_carry_slot: Some(4),
        ..JobOptions::default()
    };
    assert_eq!(check_rack_options(&options, MAX_RACK_PLACES), Ok(()));
    let base = json!({"kind": "route", "name": "board"});
    let request = with_chain(base.clone(), &options);
    assert_eq!(request["request_version"], 2);
    assert_eq!(request["rack"]["slots"].as_array().unwrap().len(), 2);
    // the places in the request are those of the options, in the same order
    assert_eq!(request["rack"]["slots"][0], json!({"tool": 1, "place": 6}));
    assert_eq!(request["rack"]["slots"][1], json!({"tool": 2, "place": 3}));
    // and for an older server the request goes back to what it always was
    assert_eq!(without_chain(&request), Some(base));
}

#[test]
fn the_answer_says_whether_the_rack_was_used() {
    let used = json!({"id": "j1", "rack_applied": true, "view": {"warnings": []}});
    assert_eq!(chain_answer(&used).applied, Some(true));
    assert!(chain_answer(&used).warnings.is_empty());
    let left_out = json!({"rack_applied": false});
    assert_eq!(chain_answer(&left_out).applied, Some(false));
    // the answer of a request that had no rack, or of an older server, does not say
    assert_eq!(chain_answer(&json!({"id": "j1"})).applied, None);
    assert_eq!(chain_answer(&json!(null)), Default::default());
    // a field of another type is read as not said
    assert_eq!(chain_answer(&json!({"rack_applied": "yes"})).applied, None);
    // the view carries it too
    assert_eq!(
        chain_answer(&json!({"view": {"rack_applied": true}})).applied,
        Some(true)
    );
}

#[test]
fn the_warnings_of_the_request_are_read_by_their_codes() {
    let job = json!({"rack_applied": false, "view": {"warnings": [
        {"code": "rack_needs_v2", "values": []},
        {"code": "rack_no_changer"},
        {"code": "rack_unknown_field", "detail": "speed", "values": []},
        {"code": "request_version_newer", "values": [3.0]},
        {"code": "a_later_warning", "values": [1.0]},
        {"detail": "a warning with no code"},
    ]}});
    assert_eq!(
        chain_answer(&job).warnings,
        vec![
            AnswerWarning::RackNeedsV2,
            AnswerWarning::RackNoChanger,
            AnswerWarning::RackUnknownField("speed".into()),
            AnswerWarning::RequestVersionNewer(3),
            AnswerWarning::Other("a_later_warning".into()),
        ]
    );
    // a version asked for that is missing or negative is read as 0, never as a panic
    let odd = json!({"view": {"warnings": [
        {"code": "request_version_newer"},
        {"code": "request_version_newer", "values": [-4.0]},
    ]}});
    assert_eq!(
        chain_answer(&odd).warnings,
        vec![
            AnswerWarning::RequestVersionNewer(0),
            AnswerWarning::RequestVersionNewer(0)
        ]
    );
}

#[test]
fn the_chaining_is_noticed_as_left_out_when_the_server_does_not_say_it_used_it() {
    let chained = json!({"kind": "drill", "request_version": 2, "rack": {"first": 1}});
    let plain = json!({"kind": "drill"});
    assert!(!rack_ignored(&chained, &json!({"rack_applied": true})));
    assert!(rack_ignored(&chained, &json!({"rack_applied": false})));
    assert!(rack_ignored(&chained, &json!({"id": "j1"})));
    assert!(rack_ignored(&chained, &json!({"rack_applied": "true"})));
    // a request that asked for nothing has nothing left out
    assert!(!rack_ignored(&plain, &json!({"id": "j1"})));
    assert!(!rack_ignored(&plain, &json!({"rack_applied": false})));
}

/// The contract page of the server, when the repository of the server sits next to this one.
fn server_page() -> Option<String> {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../EH3D-SERVER/docs/ATC_CHAIN.md");
    std::fs::read_to_string(path).ok()
}

#[test]
fn the_sentences_and_codes_are_those_the_server_documents() {
    // the placeholders of the page (P a place, T a tool, N a limit) are the numbers 777, 33 and 99 here
    let back = |text: String| {
        text.replace("777", "P")
            .replace("33", "T")
            .replace("99", "N")
    };
    let sentences = [
        RackProblem::FirstTooBig(99).detail(),
        RackProblem::TooManySlots(99).detail(),
        RackProblem::TestPinSlot(33).detail(),
        RackProblem::ToolTwice(33).detail(),
        RackProblem::PlaceTwice(777).detail(),
        RackProblem::CarrySlotNeedsCarryIn.detail(),
        RackProblem::CarrySlotTestPin.detail(),
        RackProblem::CarrySlotTaken(777).detail(),
        refusal_detail(&RackRefusal::Gap(777)),
        refusal_detail(&RackRefusal::Outside(777)),
        refusal_detail(&RackRefusal::TooManyTools(99)),
    ];
    let codes = [
        BAD_RACK,
        warning::RACK_NEEDS_V2,
        warning::RACK_NO_CHANGER,
        warning::RACK_UNKNOWN_FIELD,
        warning::REQUEST_VERSION_NEWER,
    ];
    // the stable words never change, whether or not the page of the server is at hand
    assert_eq!(codes[0], "bad_rack");
    assert_eq!(codes[1], "rack_needs_v2");
    assert_eq!(codes[2], "rack_no_changer");
    assert_eq!(codes[3], "rack_unknown_field");
    assert_eq!(codes[4], "request_version_newer");
    assert_eq!(sentences.len(), 11);
    let Some(page) = server_page() else {
        return;
    };
    for code in codes {
        assert!(page.contains(code), "{code}");
    }
    for sentence in sentences {
        let template = back(sentence);
        assert!(page.contains(&format!("`{template}`")), "{template}");
    }
}

#[test]
fn what_the_program_sends_is_what_the_server_reads_back() {
    let cases = [
        JobOptions {
            rack_first: 3,
            rack_carry_in: true,
            ..JobOptions::default()
        },
        JobOptions {
            rack_carry_out: true,
            ..JobOptions::default()
        },
        JobOptions {
            rack_first: 1,
            rack_carry_in: true,
            rack_slots: vec![(5, 9), (2, 1)],
            rack_carry_slot: Some(4),
            ..JobOptions::default()
        },
        JobOptions {
            rack_slots: vec![(1, 2)],
            ..JobOptions::default()
        },
    ];
    for options in cases {
        let request = with_chain(json!({"kind": "drill"}), &options);
        // every field of the rack is one the server knows
        assert_eq!(unknown_rack_fields(&request), Vec::<String>::new());
        // and reading the request gives the options back
        assert_eq!(chain_options(&request), Some(options));
    }
    // a request with no chaining has no rack to read
    let plain = with_chain(json!({"kind": "drill"}), &JobOptions::default());
    assert_eq!(chain_options(&plain), None);
    assert!(unknown_rack_fields(&plain).is_empty());
}

#[test]
fn a_rack_from_a_later_version_is_read_for_what_it_has_and_its_new_fields_are_named() {
    let request = json!({"request_version": 3, "rack": {
        "first": 2, "speed": 1, "carry_in": "yes", "slots": [
            {"tool": 1, "place": 5}, {"tool": 300, "place": 6}, {"tool": 2}, {"tool": 3, "place": -1}],
        "carry_slot": -4, "later": {"a": 1}}});
    let mut unknown = unknown_rack_fields(&request);
    unknown.sort();
    assert_eq!(unknown, vec!["later".to_string(), "speed".to_string()]);
    let read = chain_options(&request).unwrap();
    assert_eq!(read.rack_first, 2);
    assert!(!read.rack_carry_in, "a flag of the wrong type is not set");
    assert_eq!(read.rack_slots, vec![(1, 5)], "bad entries are left out");
    assert_eq!(read.rack_carry_slot, None);
    assert_eq!(chain_options(&json!({"rack": 4})), None);
}

#[test]
fn the_server_warning_that_the_work_leaves_the_course_is_read_with_its_axis_and_excess() {
    let job = json!({"view": {"warnings": [
        {"code": "out_of_course", "axis": "X", "values": [2.5]},
        {"code": "out_of_course", "axis": "Z", "values": [-1.0]},
        {"code": "out_of_course"},
    ]}});
    assert_eq!(
        chain_answer(&job).warnings,
        vec![
            AnswerWarning::OutOfCourse {
                axis: "X".into(),
                excess_um: 2500
            },
            AnswerWarning::OutOfCourse {
                axis: "Z".into(),
                excess_um: 0
            },
            AnswerWarning::OutOfCourse {
                axis: String::new(),
                excess_um: 0
            },
        ]
    );
}

#[test]
fn a_warning_of_the_server_is_told_in_the_words_of_the_language() {
    let en = |w: AnswerWarning| warning_text(&w, Lang::En).unwrap();
    assert_eq!(
        en(AnswerWarning::RackNoChanger),
        "This machine changes its tools by hand: the tools are not chained"
    );
    assert_eq!(
        en(AnswerWarning::RackUnknownField("speed".into())),
        "The server ignored a field of the tool changer that it does not know: speed"
    );
    assert_eq!(
        en(AnswerWarning::RequestVersionNewer(3)),
        "The server read the request as the newest version it knows (request_version 3)"
    );
    assert_eq!(
        en(AnswerWarning::OutOfCourse {
            axis: "X".into(),
            excess_um: 2500
        }),
        "The work goes past the course of the machine on the axis: X (+2.500 mm)"
    );
    assert_eq!(
        en(AnswerWarning::OutOfCourse {
            axis: "Y".into(),
            excess_um: 12_007
        }),
        "The work goes past the course of the machine on the axis: Y (+12.007 mm)"
    );
    assert_eq!(
        warning_text(&AnswerWarning::RackNoChanger, Lang::Es).unwrap(),
        "Esta máquina cambia las herramientas a mano: no se encadenan"
    );
    // a code of a later version of the server is not shown
    assert_eq!(
        warning_text(&AnswerWarning::Other("x".into()), Lang::En),
        None
    );
    // every language has the words of every warning
    for lang in Lang::ALL {
        for w in [
            AnswerWarning::RackNeedsV2,
            AnswerWarning::RackNoChanger,
            AnswerWarning::RackUnknownField("a".into()),
            AnswerWarning::RequestVersionNewer(3),
            AnswerWarning::OutOfCourse {
                axis: "Z".into(),
                excess_um: 1,
            },
        ] {
            let text = warning_text(&w, lang).unwrap();
            assert!(text.chars().count() > 12, "{lang:?}: {text}");
        }
    }
}

#[test]
fn what_the_server_sends_along_cannot_break_the_line() {
    let long = "x".repeat(100);
    let name = format!("a\r\nb{long}");
    let text = warning_text(&AnswerWarning::RackUnknownField(name), Lang::En).unwrap();
    assert!(!text.contains('\n') && !text.contains('\r'));
    assert!(text.ends_with(&format!("ab{}", "x".repeat(30))), "{text}");
    let axis = warning_text(
        &AnswerWarning::OutOfCourse {
            axis: "XYZW\u{7}".into(),
            excess_um: 5,
        },
        Lang::En,
    )
    .unwrap();
    assert!(axis.ends_with("XYZ (+0.005 mm)"), "{axis}");
}

#[test]
fn a_text_is_cut_into_lines_of_cells() {
    assert_eq!(wrap_cells("aa bb cc", 5), ["aa bb", "cc"]);
    assert_eq!(wrap_cells("abcdefgh", 3), ["abc", "def", "gh"]);
    // a wide character takes two cells
    assert_eq!(
        wrap_cells("\u{4f60}\u{597d}\u{4e16}\u{754c}", 5),
        ["\u{4f60}\u{597d}", "\u{4e16}\u{754c}"]
    );
    assert!(wrap_cells("", 10).is_empty());
    assert_eq!(
        wrap_cells(
            "The server ignored the tool changer: the request was of the first version",
            52
        ),
        [
            "The server ignored the tool changer: the request was",
            "of the first version"
        ]
    );
}

#[test]
fn the_rows_of_the_warnings_fit_the_window_and_say_when_they_were_cut() {
    let all = [
        AnswerWarning::RackNeedsV2,
        AnswerWarning::RackNoChanger,
        AnswerWarning::RackUnknownField("spare_place".into()),
    ];
    let two = warning_lines(&all[..2], Lang::En, 52, 4);
    assert_eq!(
        two,
        [
            "The server ignored the tool changer: the request was",
            "of the first version",
            "This machine changes its tools by hand: the tools",
            "are not chained"
        ]
    );
    // more than the rows allow: the last row ends in three dots
    let cut = warning_lines(&all, Lang::En, 52, 4);
    assert_eq!(cut.len(), 4);
    assert_eq!(&cut[..3], &two[..3]);
    assert_eq!(cut[3], "are not chained...");
    let cut = warning_lines(&all, Lang::En, 52, 3);
    assert_eq!(
        cut,
        [
            two[0].as_str(),
            two[1].as_str(),
            "This machine changes its tools by hand: the tools..."
        ]
    );
    assert!(warning_lines(&all, Lang::En, 52, 0).is_empty());
    assert!(warning_lines(&[], Lang::En, 52, 4).is_empty());
    for lang in Lang::ALL {
        for line in warning_lines(&all, lang, 52, 4) {
            assert!(str_width(&line) <= 52, "{lang:?}: {line}");
        }
    }
}
