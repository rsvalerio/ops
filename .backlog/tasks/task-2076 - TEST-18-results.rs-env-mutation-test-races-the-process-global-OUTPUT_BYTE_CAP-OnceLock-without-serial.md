---
id: TASK-2076
title: 'TEST-18: results.rs env-mutation test races the process-global OUTPUT_BYTE_CAP OnceLock without serial'
status: To Do
assignee: []
created_date: '2026-09-07 22:57'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - tests
dependencies: []
parent_task_id: 'TASK-2241'
modified_files:
  - crates/runner/src/command/results.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/results.rs:427-445`

**What**: `output_byte_cap_is_memoized_across_calls` mutates the process-global `OPS_OUTPUT_BYTE_CAP` env var with no `#[serial_test::serial]` guard, unlike the sibling env-knob tests in `command/parallel.rs` (`resolve_max_parallel_*`) and `exec.rs` (`drain_grace_knob_tests`), which all carry serial annotations. The `SAFETY:` comment justifying the bare `set_var` — "tests under `cargo test` run on a single thread per binary" — is factually wrong: the default libtest harness runs tests on multiple threads; only the project's nextest gate isolates per process.

**Why it matters**: TEST-18 — unserialised process-env mutation is a cross-test race. The `OUTPUT_BYTE_CAP` `OnceLock` is read by every `from_streamed`/`spawn_capped` test in the same binary; if this test's `set_var(.., "1")` lands before another test's first `output_byte_cap()` call initializes the OnceLock, that test sees `cap = 1` and `command_output_from_streamed_success`-style assertions (`stdout == "hello world"`, 11 bytes > cap 1 → truncated) fail intermittently under plain `cargo test`. The race is dormant only because the CI gate uses nextest; any dev or tool running `cargo test` directly hits it. The incorrect SAFETY rationale also invites copy-paste of the same unsound claim into future env tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test carries #[serial_test::serial(env_output_cap)] (a fresh serial key or the existing env group), matching the parallel.rs / exec.rs env-knob tests
- [ ] #2 The SAFETY comment states the real isolation mechanism (serialisation and/or nextest per-process isolation), not the false single-thread-per-binary claim
- [ ] #3 Plain
running 245 tests
test command::build::tests::apply_escape_policy_deny_returns_permission_denied ... ok
test command::build::tests::apply_escape_policy_warn_is_ok ... ok
test command::abort::tests::cancelled_returns_immediately_when_already_set ... ok
test command::abort::tests::cancelled_resolves_after_set ... ok
test command::abort::tests::set_is_idempotent ... ok
test command::abort::tests::many_waiters_all_wake_on_set ... ok
test command::build::tests::cache_eviction_by_many_distinct_cwds_does_not_change_escape_outcomes ... ok
test command::build::tests::deny_canonicalizes_absolute_inside_workspace ... ok
test command::build::tests::deny_still_admits_a_missing_but_contained_cwd ... ok
test command::build::tests::detect_workspace_escape_inside_is_inside ... ok
test command::build::tests::expand_err_to_io_does_not_leak_variable_name_in_message ... ok
test command::build::tests::resolve_spec_cwd_absolute_inside_workspace_is_returned_verbatim ... ok
test command::build::tests::invalidate_workspace_cache_changes_subsequent_spawn_canonicalize ... ok
test command::build::tests::detect_workspace_escape_via_symlink_still_fires_with_cached_workspace ... ok
test command::build::tests::deny_re_resolves_the_joined_path_on_every_spawn ... ok
test command::build::tests::build_command_async_preserves_program_name ... ok
test command::build::tests::detect_workspace_escape_parent_escapes ... ok
test command::build::tests::deny_returns_canonical_path_to_shrink_toctou_window ... ok
test command::build::tests::resolve_spec_cwd_absolute_outside_workspace_is_denied ... ok
test command::build::tests::resolve_spec_cwd_absolute_outside_workspace_warns_under_warn_and_allow ... ok
test command::build::tests::resolve_spec_cwd_deny_rejects_escape ... ok
test command::build::tests::build_command_async_does_not_starve_concurrent_tokio_task ... ok
test command::build::tests::resolve_spec_cwd_none_returns_workspace ... ok
test command::events::tests::step_failed_serializes_with_message ... ok
test command::build::tests::workspace_canonical_cache_evicts_lru_at_cap ... ok
test command::events::tests::step_finished_serializes_with_duration ... ok
test command::events::tests::step_output_serializes_stderr_flag ... ok
test command::events::tests::step_skipped_serializes ... ok
test command::exec::drain_grace_knob_tests::a_valid_value_is_honoured ... ok
test command::build::tests::workspace_canonical_cache_re_canonicalizes_after_symlink_swap_and_invalidate ... ok
test command::exec::drain_grace_knob_tests::out_of_range_clamps_to_the_ceiling ... ok
test command::exec::drain_grace_knob_tests::unset_uses_the_default ... ok
test command::exec::drain_grace_knob_tests::unusable_values_fall_back_to_the_default ... ok
test command::exec::spawn_error_log_format_tests::command_id_field_debug_escapes_control_characters ... ok
test command::exec::spawn_error_log_format_tests::program_field_debug_escapes_control_characters ... ok
test command::parallel::resolve_tests::channel_capacity_capped_at_max_parallel_for_large_plan ... ok
test command::build::tests::deny_refuses_a_joined_path_that_cannot_be_canonicalized ... ok
test command::builtins::tests::builtin_exec_invokes_current_binary_with_subcommand ... ok
test command::build::tests::resolve_spec_cwd_relative_inside_workspace_is_joined ... ok
test command::build::tests::resolve_spec_cwd_warn_allows_escape ... ok
test command::parallel::resolve_tests::channel_capacity_clamped_to_steps_len_for_small_plan ... ok
test command::parallel::resolve_tests::channel_capacity_empty_plan_has_floor ... ok
test command::parallel::resolve_tests::resolve_max_parallel_clamps_above_ceiling ... ok
test command::builtins::tests::registers_config_checkers ... ok
test command::builtins::tests::registers_text_fixers_with_aliases ... ok
test command::build::tests::resolve_spec_cwd_rejects_non_utf8_cwd_loudly ... ok
test command::events::tests::run_finished_serializes_success_flag ... ok
test command::build::tests::warn_and_allow_canonicalizes_inside_workspace ... ok
test command::parallel::resolve_tests::resolve_max_parallel_falls_back_on_zero_or_unparseable ... ok
test command::parallel::resolve_tests::resolve_max_parallel_treats_empty_as_unset ... ok
test command::results::tests::command_output_from_streamed_failure ... ok
test command::results::tests::cap_streamed_caps_oversized_input ... ok
test command::results::tests::command_output_under_cap_is_unchanged ... ok
test command::results::tests::command_output_from_streamed_invalid_utf8 ... ok
test command::parallel::resolve_tests::resolve_max_parallel_is_memoised_across_env_mutation ... ok
test command::results::tests::command_output_from_streamed_success ... ok
test command::parallel::resolve_tests::zero_warn_message_distinguishes_sequential_intent ... ok
test command::results::tests::command_output_status_message ... ok
test command::events::tests::runner_event_serializes_to_json ... ok
test command::results::tests::from_streamed_marker_reflects_sinked_bytes ... ok
test command::builtins::tests::builtin_exec_displays_as_ops_not_absolute_path ... ok
test command::builtins::tests::registers_sec_scanner ... ok
test command::results::tests::parse_output_byte_cap_warns_on_invalid_inputs ... ok
test command::results::tests::step_result_failure_constructor ... ok
test command::results::tests::output_byte_cap_is_memoized_across_calls ... ok
test command::results::tests::step_result_clone_produces_equal_copy ... ok
test command::secret_patterns::tests::git_sha_does_not_look_like_secret ... ok
test command::secret_patterns::tests::env_key_field_debug_escapes_control_characters ... ok
test command::results::tests::step_result_debug_includes_fields ... ok
test command::secret_patterns::tests::ascii_contains_ignore_case_basics ... ok
test command::tests::build_cmd::build_command_error_tests::build_command_with_empty_args ... ok
test command::tests::build_cmd::build_command_error_tests::build_command_with_special_chars_in_args ... ok
test command::secret_patterns::tests::aws_shaped_secret_still_flagged ... ok
test command::tests::build_cmd::build_command_error_tests::build_command_with_relative_cwd ... ok
test command::tests::build_cmd::build_command_error_tests::build_command_with_nonexistent_cwd_still_builds ... ok
test command::tests::build_cmd::build_command_error_tests::build_command_with_many_args ... ok
test command::tests::build_cmd::step_result_failure_creates_correct_result ... ok
test command::tests::build_cmd::build_command_sets_program_and_args ... ok
test command::secret_patterns::tests::redaction_patterns_is_subset_of_warn_patterns ... ok
test command::tests::build_cmd::build_command_error_tests::build_command_with_absolute_cwd ... ok
test command::secret_patterns::tests::sensitive_key_detection_parity_across_case ... ok
test command::tests::build_cmd::build_command_uses_spec_cwd_when_provided ... ok
test command::secret_patterns::tests::bounded_prefix_respects_char_boundaries ... ok
test command::tests::events::emit_output_events_emits_stdout_and_stderr ... ok
test command::tests::events::emit_output_edge_tests::emit_output_events_with_unicode ... ok
test command::build::tests::canonical_workspace_cached_collapses_burst_to_single_canonicalize ... ok
test command::tests::events::emit_output_edge_tests::emit_output_events_with_many_lines ... ok
test command::tests::events::emit_output_edge_tests::emit_output_events_with_very_long_line ... ok
test command::tests::exec::exec_unit_tests::build_command_includes_env_vars ... ok
test command::secret_patterns::tests::looks_like_secret_value_does_not_scan_past_cap ... ok
test command::tests::exec::exec_unit_tests::build_step_result_from_success_output ... ok
test command::tests::exec::exec_unit_tests::build_step_result_from_failure_output ... ok
test command::tests::exec::exec_unit_tests::emit_output_events_arc_ptr_eq_per_stream ... ok
test command::tests::exec::exec_unit_tests::emit_output_events_crlf_handling ... ok
test command::tests::exec::exec_unit_tests::emit_output_events_empty_input ... ok
test command::tests::exec::exec_unit_tests::emit_output_events_trailing_newline ... ok
test command::tests::exec::exec_unit_tests::emit_step_completion_failure ... ok
test command::tests::exec::exec_unit_tests::emit_step_completion_success ... ok
test command::tests::data::query_data_unknown_provider_errors ... ok
test command::tests::data::query_data_caches_results ... ok
test command::tests::data::query_data_shares_cwd_arc_with_provider ... ok
test command::tests::data::query_data_shares_inner_cache_across_outer_calls ... ok
test command::tests::data::re_registering_providers_invalidates_cache ... ok
test command::tests::data::query_data_failing_provider_errors ... ok
test command::tests::exec::error_path_tests::list_command_ids_includes_all_commands ... ok
test command::tests::exec::error_path_tests::list_command_ids_with_extension_commands ... ok
test command::tests::data::query_data_returns_provider_value ... ok
test command::tests::exec::exec_unit_tests::execute_with_timeout_no_timeout_succeeds ... ok
test command::tests::exec::exec_unit_tests::execute_with_timeout_with_timeout_returns_output ... ok
test command::tests::expand::builtin_commands_resolve_by_name_and_alias ... ok
test command::tests::exec::error_path_tests::run_exec_nonexistent_program ... ok
test command::tests::exec::run_unknown_command_returns_error ... ok
test command::tests::expand::composite_can_reference_builtin_aliases ... ok
test command::tests::expand::cycle_detection_tests::expand_to_leaves_cycle_2_nodes ... ok
test command::tests::expand::cycle_detection_tests::expand_to_leaves_cycle_3_nodes ... ok
test command::tests::expand::cycle_detection_tests::expand_to_leaves_self_reference ... ok
test command::tests::exec::exec_unit_tests::spawn_capped_gives_child_null_stdin_so_reader_cannot_hang ... ok
test command::tests::expand::expand_to_leaves_composite ... ok
test command::tests::expand::expand_to_leaves_single ... ok
test command::tests::expand::depth_limit_tests::expand_to_leaves_at_depth_limit_succeeds ... ok
test command::tests::expand::expand_to_leaves_unknown ... ok
test command::tests::exec::error_path_tests::run_exec_permission_denied ... ok
test command::tests::expand::expand_to_leaves_via_alias ... ok
test command::tests::expand::nested_composite_tests::expand_to_leaves_deep_cycle ... ok
test command::tests::expand::depth_limit_tests::expand_to_leaves_shallow_nesting_succeeds ... ok
test command::tests::expand::depth_limit_tests::expand_to_leaves_exceeds_depth_limit_returns_none ... ok
test command::tests::expand::depth_limit_tests::expand_to_leaves_microbench_does_not_regress ... ok
test command::tests::exec::run_exec_fails_loudly_on_non_utf8_env_var ... ok
test command::tests::exec::error_path_tests::run_exec_invalid_cwd ... ok
test command::tests::expand::nested_composite_tests::expand_to_leaves_deeply_nested_composite ... ok
test command::tests::expand::orphan_config_alias_falls_through_to_stack_default ... ok
test command::tests::expand::nested_composite_tests::expand_to_leaves_diamond_composite_succeeds ... ok
test command::tests::exec::exec_unit_tests::spawn_capped_bounds_collected_bytes_near_cap ... ok
test command::tests::expand::register_commands_incremental_alias_merge_preserves_resolution ... ok
test command::tests::expand::register_commands_re_registration_prunes_stale_aliases ... ok
test command::tests::expand::register_commands_warns_on_cross_store_alias_collision_with_config ... ok
test command::tests::expand::nested_composite_tests::expand_to_leaves_nested_missing_intermediate ... ok
test command::tests::expand::schedule_flag_agreement_tests::aggregated_flags_match_the_agreed_declaration ... ok
test command::tests::expand::schedule_flag_agreement_tests::conflict_message_is_actionable ... ok
test command::tests::expand::schedule_flag_agreement_tests::agreeing_nested_composites_still_expand ... ok
test command::tests::expand::schedule_flag_agreement_tests::diamond_revisit_of_agreeing_node_is_not_a_conflict ... ok
test command::tests::expand::resolve_by_alias ... ok
test command::tests::expand::schedule_flag_agreement_tests::fail_fast_disagreement_is_rejected ... ok
test command::tests::exec::run_plan_echo_success ... ok
test command::tests::expand::schedule_flag_agreement_tests::sequential_child_under_parallel_parent_is_rejected ... ok
test command::tests::expand::schedule_flag_agreement_tests::parallel_child_under_sequential_parent_is_rejected ... ok
test command::tests::exec::exec_unit_tests::emit_output_events_shares_buffer_across_lines ... ok
test command::tests::parallel::run_parallel_composite ... ok
test command::tests::parallel::exec_standalone_aborts_forwarder_on_outer_cancellation ... ok
test command::tests::exec::exec_unit_tests::spawn_capped_drains_die_with_parent_joinset_abort ... ok
test command::tests::parallel::exec_standalone_skips_when_abort_set ... ok
test command::tests::parallel::run_plan_parallel_duplicate_id_pairs_every_started_with_terminal_under_fail_fast ... ok
test command::tests::parallel::run_plan_parallel_fail_fast_emits_terminal_for_every_started_step ... ok
test command::tests::parallel::exec_standalone_terminal_send_aborts_on_full_outer_channel ... ok
test command::tests::parallel::run_plan_parallel_resolution_failure ... ok
test command::tests::parallel::exec_standalone_delivers_terminal_event_under_high_volume_load ... ok
test command::tests::parallel::parallel_failure_tests::run_plan_parallel_all_fail ... ok
test command::tests::parallel::run_plan_parallel_fail_fast_emits_failure ... ok
test command::tests::parallel::run_plan_parallel_no_fail_fast ... ok
test command::tests::parallel_infra::collect_join_results_redacts_panic_payload ... ok
test command::tests::parallel::exec_standalone_emits_step_output_dropped_under_burst ... ok
test command::tests::parallel::run_plan_parallel_singleton_matches_sequential_event_order ... ok
test command::tests::parallel_infra::handle_parallel_events_sets_abort_on_fail_fast ... ok
test command::tests::parallel_infra::fail_fast_aborts_siblings_when_a_task_panics ... ok
test command::tests::parallel_infra::handle_parallel_events_no_abort_without_fail_fast ... ok
test command::tests::parallel::run_plan_parallel_verify_event_content ... ok
test command::tests::parallel_infra::handle_parallel_events_receives_all ... ok
test command::tests::parallel::run_plan_parallel_success ... ok
test command::tests::raw_mode::exec_command_raw_returns_success_for_true ... ok
test command::tests::secrets::has_high_entropy_detects_random_strings ... ok
test command::tests::secrets::has_high_entropy_rejects_simple_strings ... ok
test command::tests::secrets::is_sensitive_env_key_case_insensitive ... ok
test command::tests::raw_mode::exec_command_raw_returns_failure_for_false ... ok
test command::tests::secrets::looks_like_aws_key_detects_40_char_keys ... ok
test command::tests::raw_mode::run_raw_expands_composite_and_runs_leaves ... ok
test command::tests::raw_mode::run_plan_raw_unknown_command_returns_failure ... ok
test command::tests::secrets::looks_like_jwt_rejects_non_jwt ... ok
test command::tests::secrets::looks_like_jwt_detects_jwt_format ... ok
test command::tests::secrets::looks_like_aws_key_rejects_wrong_length ... ok
test command::tests::secrets::looks_like_uuid_detects_uuid_format ... ok
test command::tests::secrets::looks_like_secret_value_combines_all_checks ... ok
test command::tests::secrets::looks_like_uuid_rejects_non_uuid ... ok
test command::tests::secrets::looks_like_secret_value_rejects_short_values ... ok
test command::tests::secrets::looks_like_uuid_rejects_wrong_segment_lengths ... ok
test command::tests::raw_mode::run_plan_raw_fail_fast_stops_on_first_failure ... ok
test command::tests::sequential::run_plan_unknown_command_emits_failure ... ok
test command::tests::sequential::deny_policy_refuses_escaping_cwd_on_hook_path ... ok
test display::progress_state::tests::consume_step_index_on_unknown_id_returns_none ... ok
test display::progress_state::tests::duplicate_ids_route_to_distinct_bars_via_consume ... ok
test command::tests::raw_mode::run_plan_raw_runs_sequentially_and_collects_results ... ok
test display::progress_state::tests::record_stderr_accumulates_per_id ... ok
test display::progress_state::tests::record_stderr_does_not_pin_the_shared_capture_buffer ... ok
test command::tests::parallel_infra::spawn_parallel_tasks_creates_correct_count ... ok
test display::progress_state::tests::record_stderr_high_cap_preserves_full_tail ... ok
test display::progress_state::tests::record_stderr_zero_cap_records_nothing ... ok
test display::progress_state::tests::reset_for_plan_clears_previous_state_and_seeds_steps ... ok
test display::progress_state::tests::resolve_step_display_falls_back_to_id_when_no_override ... ok
test display::progress_state::tests::resolve_step_display_uses_override_when_present ... ok
test display::progress_state::tests::step_index_returns_position_of_registered_id ... ok
test command::tests::sequential::warn_policy_does_not_refuse_escaping_cwd_on_interactive_path ... ok
test display::progress_state::tests::step_index_resolves_via_o1_map_for_large_plan ... ok
test display::tap::tests::append_marker_still_attempts_reopen_on_other_kinds ... ok
test display::tap::tests::append_marker_skips_reopen_on_broken_pipe ... ok
test display::tap::tests::append_marker_skips_reopen_on_storage_full ... ok
test display::tap::tests::tap_path_debug_escapes_control_characters ... ok
test display::tap::tests::first_write_failure_still_disables_the_tap_once ... ok
test display::tap::tests::buffered_lines_reach_disk_at_the_explicit_flush_point ... ok
test display::tests::edge_case_tests::extract_stderr_tail_extracts_correct_count ... ok
test display::tests::edge_case_tests::extract_stderr_tail_handles_empty ... ok
test display::tests::edge_case_tests::extract_stderr_tail_handles_fewer_lines ... ok
test display::tests::edge_case_tests::extract_stderr_tail_unlimited_returns_all ... ok
test display::tests::edge_case_tests::write_stderr_handles_none_and_some ... ok
test display::tests::concurrent_event_tests::handle_event_interleaved_failure_sequence ... ok
test display::tests::emit_line_non_tty_writes_to_stderr ... ok
test display::tests::error_path_tests::progress_display_invalid_theme_returns_error ... ok
test display::tests::concurrent_event_tests::handle_event_rapid_sequence_no_panic ... ok
test display::tests::error_path_tests::progress_display_valid_theme_succeeds ... ok
test display::tests::edge_case_tests::finish_step_returns_none_for_unknown_id ... ok
test display::tests::progress_display_handles_failure_with_error_detail ... ok
test command::tests::sequential::run_sequential_composite ... ok
test display::tests::progress_display_handles_full_lifecycle ... ok
test display::tests::render_config_uses_output_settings ... ok
test display::tests::progress_display_render_step ... ok
test display::tests::tap_none_produces_no_file ... ok
test display::tests::step_stderr_captures_output ... ok
test display::tests::progress_display_handles_step_skipped ... ok
test display::tests::tap_file_captures_raw_output ... ok
test display::tests::unknown_command_tests::handle_event_step_output_for_unknown_command_no_panic ... ok
test display::tests::unknown_command_tests::finish_step_unknown_id_returns_none ... ok
test display::tests::unknown_command_tests::handle_event_unknown_command_id_no_panic ... ok
test display::tests::unknown_command_tests::handle_event_step_failed_for_unknown_command_no_panic ... ok
test display::tests::verbose_overrides_to_unbounded_without_mutating_config ... ok
test terminal::tests::disable_echo_is_noop_when_stderr_is_not_a_tty ... ok
test command::tests::expand::proptest_tests::expand_to_leaves_single_exec_returns_self ... ok
test display::tests::unknown_command_tests::run_finished_finalizes_orphan_running_bars ... ok
test command::tests::expand::proptest_tests::expand_to_leaves_unknown_returns_none ... ok
test command::tests::sequential::retained_capture_bytes_scale_with_plan_length ... ok
test command::tests::parallel::parallel_timing_tests::run_plan_parallel_executes_concurrently ... ok
test command::tests::parallel_infra::fail_fast_trips_on_panic_while_a_sibling_floods_output ... ok
test command::tests::expand::proptest_tests::expand_to_leaves_composite_flattens ... ok
test command::tests::parallel::exec_standalone_logs_dropped_count_when_outer_receiver_closed ... ok
test display::progress_state::tests::record_stderr_bounded_ring_keeps_only_tail ... ok
test command::tests::exec::run_exec_timeout ... ok
test command::tests::exec::exec_unit_tests::timed_out_step_kills_the_whole_process_group ... ok
test command::tests::exec::exec_unit_tests::post_exit_drain_is_bounded_when_a_grandchild_holds_the_pipe ... ok

test result: ok. 245 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.03s


running 1 test
test crates/runner/src/display.rs - display::ProgressDisplay::handle_event (line 259) - compile fail ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s passes repeatedly (e.g. 10 consecutive runs) with no intermittent from_streamed failures
<!-- AC:END -->
