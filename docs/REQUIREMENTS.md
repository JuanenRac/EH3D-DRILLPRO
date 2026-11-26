# EH3D-DRILLPRO: requirements

This register is the single source of the state of each capability: `STATUS.md` and the README files cite it and never state a state that it does not. A state is one of `planned`, `prototype`, `implemented` and `validated`. `validated` means that a person has run the capability on a real machine and the evidence of the manifest says `bench` or `field`; nothing here is validated, because a whole drilling job has not run on a real machine, and the manifest of the repository records the evidence as `simulator`.

The register has 48 requirements: 33 implemented, 9 prototype, 6 planned and 0 validated. By progress: 33 done, 9 partial, 0 pending and 6 blocked. High coverage is not proof of correctness, so each row says how it is checked: the test that names it, and what it shows.

## How to read the register

* **ID:** `DRPR-NNN`, never reused and never renumbered; a new capability takes the next number.
* **State:** `implemented` is code that is complete and tested on a computer, `prototype` is part of the capability, `planned` is not written.
* **Progress:** `done`, `partial`, `pending` (can be written without hardware) or `blocked` (waits for hardware or for another program of the suite).
* **Evidence:** `simulator` when the test runs against the simulated machine and control unit of the protocol crate, a fault-injecting wrapper of it or a server in memory, `host-test` when it checks a calculation, a reader or a screen-less piece, `none` when nothing is written yet.
* **Test:** the acceptance criterion: `file::name` is the test `name` of `tests/file.rs` (`src/file::name` is a test inside `src/file.rs`), which must exist. For a capability that is not written the cell says `planned:` and the name of the test that will show it; the day that test exists, the row must change.
* **Limits:** what is not done or not proven; a row that is `partial` or `blocked` always says it.

The check is `tests/reqs.rs`: it reads this table, finds each test and fails when a state, a test or a count does not agree.

## Files and the plan

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| DRPR-001 | Modal coordinates keep the previous axis in a drill file. | implemented | done | host-test | `formats::modal_coordinates_keep_previous_axis` | Small sample files written for the tests, not files from real boards. |
| DRPR-002 | The sample files of the machines are inside the limits and a file that is too big is turned down by its size. | implemented | done | host-test | `input_guard::a_file_that_is_too_big_is_turned_down_by_its_size` | - |
| DRPR-003 | A character zero marks a binary file and the line is told. | implemented | done | host-test | `input_guard::a_character_zero_is_a_binary_file_and_the_line_is_told` | - |
| DRPR-004 | Tools stay grouped in order of first use. | implemented | done | host-test | `job::tools_stay_grouped_in_order_of_first_use` | - |
| DRPR-005 | A tool whose line cannot be read runs with the cautious values and the person is told which. | implemented | done | host-test | `job_safety::a_tool_whose_line_cannot_be_read_runs_with_the_cautious_values_and_the_person_is_told_which` | - |
| DRPR-006 | A file with slots is not taken without a word. | implemented | done | host-test | `job_safety::a_file_with_slots_is_not_taken_without_a_word` | - |
| DRPR-007 | The plan of each sample file is the stored one, and the same files give the same plan twice. | implemented | done | host-test | `plan_golden::the_plan_of_each_sample_file_is_the_stored_one` | - |
| DRPR-008 | The plan on paper repeats are left out only when asked and counted. | implemented | done | host-test | `plan_options::the_repeats_are_left_out_only_when_asked_and_counted` | - |
| DRPR-009 | An unchanged configuration is rewritten without a difference in its values, and a tool table keeps its comment lines. | implemented | done | host-test | `files_roundtrip::a_tool_table_keeps_its_comment_lines` | - |
| DRPR-010 | A job can be loaded from a ZIP archive. | implemented | done | host-test | `classic_zip::the_command_line_loads_the_first_drill_file_of_an_archive` | - |
| DRPR-011 | Every example of the release is read with its holes and tools and planned whole. | implemented | done | host-test | `release_files::the_examples_are_planned_whole_in_the_order_of_the_machine` | - |

## The screens

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| DRPR-012 | The main screen matches the original, cell by cell. | implemented | done | host-test | `classic_golden::main_screen_matches_the_original` | Checked against recordings of the original. |
| DRPR-013 | Hot letters are unique on every screen and every classic text has its hot letter in every language. | implemented | done | host-test | `classic::every_classic_text_has_its_hot_letter_in_every_language` | - |
| DRPR-014 | Every text exists in all eight languages and every language renders every screen without panicking. | implemented | done | host-test | `i18n::every_text_exists_in_all_eight_languages` | - |
| DRPR-015 | A line edit takes typing, backspace and the cursor keys; a pasted text stops at the length of the field. | implemented | done | host-test | `src/classic/edit::a_pasted_text_stops_at_the_length_of_the_field` | - |
| DRPR-016 | The jog never leaves the table. | implemented | done | host-test | `src/classic/teach::the_jog_never_leaves_the_table` | - |
| DRPR-017 | The teach session homes, follows the jogs and homes again, and a jog outside the reach never hangs. | implemented | done | host-test | `teach_session::a_jog_outside_the_reach_or_after_the_session_is_dropped_never_hangs` | - |
| DRPR-018 | The terminal version draws the same screens with the keyboard and the mouse. | implemented | done | host-test | `console::the_picture_has_every_row_placed_and_the_colours_of_the_cells` | No terminal of an old PC has been tried. |
| DRPR-019 | A panic in the work becomes an error that names it, and the crash log is in the configuration folder. | implemented | done | host-test | `crash::the_crash_log_is_in_the_configuration_folder` | - |

## The protocol and the simulator

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| DRPR-020 | A whole job reaches the machine byte for byte. | prototype | partial | simulator | `protocol/tests/execute::a_whole_job_reaches_the_machine_byte_for_byte` | Against the simulator and recordings; a whole job has not run on a real machine. |
| DRPR-021 | Three holes with one tool match the recorded job exactly. | prototype | partial | simulator | `protocol/tests/job_golden::three_holes_one_tool_match_the_recorded_job_exactly` | Matches a recording of the original, not a run on a machine. |
| DRPR-022 | The first move to the first hole is 1098 diagonal steps. | prototype | partial | simulator | `protocol/tests/golden::first_move_to_the_first_hole_is_1098_diagonal_steps` | The steps per millimetre are only checked against the configuration of the original. |
| DRPR-023 | Motion is refused until everything is verified, and an unarmed machine moves nothing. | implemented | done | simulator | `protocol/tests/safety::motion_is_refused_until_everything_is_verified` | - |
| DRPR-024 | Cancelling stops the machine with the drivers off. | implemented | done | simulator | `protocol/tests/execute::cancelling_stops_the_machine_with_the_drivers_off` | On the simulator; the stop of a real machine was not recorded. |
| DRPR-025 | Z homing stops on the limit switch, and a missing switch is reported like the original home signal error. | prototype | partial | simulator | `protocol/tests/homing::z_homing_stops_on_the_limit_switch` | The reference run was tried on one real machine, as the README says; it is not recorded as bench evidence. |
| DRPR-026 | A silent unit, a fault line stuck high, a cable pulled in the middle of a run and a dead limit switch are told, and a second run works. | implemented | done | simulator | `protocol/tests/faults::a_cable_pulled_in_the_middle_of_a_run_stops_it_and_a_second_run_works` | A wrapper of the simulator makes the link fail on purpose. |
| DRPR-027 | The frames and the checksum of the link to the node are the ones of the firmware vectors. | implemented | done | host-test | `protocol/tests/node::crc_and_frame_match_the_firmware_vectors` | - |
| DRPR-028 | The spindle start sequence orders latch B before A, and with the spindle of the node the job asks the node. | prototype | partial | simulator | `protocol/tests/job_golden::with_the_nodes_spindle_the_job_asks_the_node_instead_of_writing_the_latches` | The spindle and vacuum outputs are untried on a machine. |
| DRPR-029 | The used tool is left in its place before the next is taken, and nothing moves across the bar below the height of the tools. | prototype | partial | simulator | `protocol/tests/rack_travel::nothing_moves_across_the_bar_below_the_height_of_the_tools_except_into_and_out_of_a_place_from_its_free_side` | The changer of the ATC models has not run on a real machine. |
| DRPR-030 | A job with three tools runs to the end changing them by itself. | prototype | partial | simulator | `classic_atc::a_job_with_three_tools_runs_to_the_end_changing_them_by_itself` | The changer of the ATC models has not run on a real machine. |
| DRPR-031 | A move past the far end of the course is a violation with its item, going behind the switch is swallowed. | implemented | done | host-test | `src/dryrun::a_move_past_the_far_end_is_a_violation_with_its_item` | - |

## Connection and server

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| DRPR-032 | The connection is chosen in the Config window and written into the machine file, which keeps its other lines. | implemented | done | host-test | `classic_connection::the_machine_file_keeps_its_other_lines_when_a_connection_is_saved` | - |
| DRPR-033 | A server that is not local or private gets a visible warning about the token. | implemented | done | host-test | `classic_connection::a_server_that_is_not_local_or_private_gets_a_visible_warning_about_the_token` | - |
| DRPR-034 | A post that timed out is not sent twice, but a get is tried again. | implemented | done | simulator | `src/server_wire::a_post_that_timed_out_is_not_sent_twice_but_a_get_is_tried_again` | Against a server in memory. |
| DRPR-035 | A rack the server would refuse is refused before the job is planned, with the words of the server. | implemented | done | host-test | `work_contract::the_local_planner_refuses_with_the_words_the_server_uses` | - |
| DRPR-036 | The whole fleet is asked in one request and a refusal is one line. | prototype | partial | simulator | `src/server_link::the_whole_fleet_is_asked_in_one_request_and_a_refusal_is_one_line` | Only exercised against a server in memory and the simulator. |
| DRPR-037 | A saved connection replaces the file whole, leaves no temporary file and is private. | implemented | done | host-test | `protocol/src/connection::a_saved_connection_replaces_the_file_whole_leaves_no_temporary_file_and_is_private` | - |

## Not written yet

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| DRPR-038 | A whole drilling job is run on a real machine and the holes land where the file says. | planned | blocked | none | `planned:real_job::a_whole_job_drills_a_test_board_on_a_real_machine` | Needs a Bungard CCD machine and a person. |
| DRPR-039 | The steps per millimetre are confirmed on a machine. | planned | blocked | none | `planned:real_steps::the_steps_per_millimetre_match_a_real_machine` | Needs a Bungard CCD machine and a person. |
| DRPR-040 | The tool changer of the ATC models runs on a real machine. | planned | blocked | none | `planned:real_changer::the_changer_works_on_an_atc_machine` | Needs an ATC machine. |
| DRPR-041 | The readers are tried with files from real boards. | planned | blocked | none | `planned:real_boards::files_from_real_boards_are_read` | Needs files from a person. |
| DRPR-042 | The hypotheses of `docs/PROTOCOL.md` are confirmed on the machine. | planned | blocked | none | `planned:protocol_hypotheses::each_hypothesis_of_the_protocol_is_confirmed` | Needs a machine and a recording. |
| DRPR-043 | The server drives a real machine. | planned | blocked | none | `planned:real_server::the_server_drives_a_real_machine` | Needs the server and a machine. |

## The project of the suite

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| DRPR-044 | The drill layers are read from the job of a `.eh3d` project and each plan is kept in the branch `drillpro/` only. | implemented | done | host-test | `project::plan_keeps_a_plan_for_each_drill_layer_in_its_own_branch_only` | Invented files; no job of a real board. The plan is a text, not a file for the machine. |
| DRPR-045 | A job that changes makes the plans out of date, and a plan made again keeps the others only while the job is the same. | implemented | done | host-test | `project::a_job_that_changes_makes_the_plans_out_of_date_until_they_are_made_again` | - |
| DRPR-046 | A refused plan leaves the project as it was. | implemented | done | host-test | `project::a_refused_plan_leaves_the_project_as_it_was` | - |
| DRPR-047 | The screens open the first drill layer of a project, say when its plans are out of date and keep its plan with `Ctrl+S`. | prototype | partial | host-test | `project::the_screens_open_the_first_drill_layer_and_keep_its_plan` | Tested without a screen; only the first drill layer of the job is opened; no person has tried it. |
| DRPR-048 | The screens ask before a write that would remove a signature of the project and write nothing unless it is agreed. | implemented | done | host-test | `project::the_classic_screen_asks_before_removing_a_signature` | The screens are tested without a window; no person has tried them. |
