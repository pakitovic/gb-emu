use super::super::FrameSequencerState;

#[test]
fn frame_sequencer_advance_emits_expected_clock_pattern() {
    let mut sequencer = FrameSequencerState::default();

    let mut pattern = Vec::new();
    for _ in 0..8 {
        let clocks = sequencer.advance();
        pattern.push((
            clocks.clock_length,
            clocks.clock_sweep,
            clocks.clock_envelope,
        ));
    }

    assert_eq!(
        pattern,
        vec![
            (true, false, false),
            (false, false, false),
            (true, true, false),
            (false, false, false),
            (true, false, false),
            (false, false, false),
            (true, true, false),
            (false, false, true),
        ]
    );
    assert_eq!(sequencer.step, 0);
    assert_eq!(sequencer.ticks, 8);
    assert_eq!(sequencer.length_tick_count, 4);
    assert_eq!(sequencer.sweep_tick_count, 2);
    assert_eq!(sequencer.envelope_tick_count, 1);
}

#[test]
fn frame_sequencer_next_step_helpers_follow_internal_step() {
    let mut sequencer = FrameSequencerState::default();
    assert!(sequencer.length_clocks_on_next_step());
    assert!(!sequencer.envelope_clocks_on_next_step());

    for _ in 0..7 {
        sequencer.advance();
    }

    assert!(!sequencer.length_clocks_on_next_step());
    assert!(sequencer.envelope_clocks_on_next_step());
}
