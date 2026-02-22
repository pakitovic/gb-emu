use super::EnvelopeState;

pub(in crate::apu) fn write_envelope_and_update_dac_state(
    envelope: &mut EnvelopeState,
    channel_enabled: &mut bool,
    dac_enabled: &mut bool,
    value: u8,
) {
    envelope.write_register(value);
    *dac_enabled = (value & 0xF8) != 0;
    if !*dac_enabled {
        *channel_enabled = false;
    }
}

pub(in crate::apu) fn apply_length_enable_edge_u8(
    channel_enabled: &mut bool,
    length_counter: &mut u8,
    length_enabled: bool,
    old_length_enabled: bool,
    length_clocks_next: bool,
    trigger_requested: bool,
) {
    if old_length_enabled || !length_enabled || length_clocks_next || *length_counter == 0 {
        return;
    }
    *length_counter -= 1;
    if *length_counter == 0 && !trigger_requested {
        *channel_enabled = false;
    }
}

pub(in crate::apu) fn reload_length_on_trigger_u8(
    length_counter: &mut u8,
    length_enabled: bool,
    length_clocks_next: bool,
    max_length: u8,
) {
    if *length_counter != 0 {
        return;
    }
    *length_counter = max_length;
    if length_enabled && !length_clocks_next {
        *length_counter = length_counter.saturating_sub(1);
    }
}

pub(in crate::apu) fn clock_length_u8(
    channel_enabled: &mut bool,
    length_counter: &mut u8,
    length_enabled: bool,
) {
    if !length_enabled || *length_counter == 0 {
        return;
    }

    *length_counter -= 1;
    if *length_counter == 0 {
        *channel_enabled = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_write_disables_channel_when_dac_turns_off() {
        let mut envelope = EnvelopeState::default();
        let mut channel_enabled = true;
        let mut dac_enabled = true;

        write_envelope_and_update_dac_state(
            &mut envelope,
            &mut channel_enabled,
            &mut dac_enabled,
            0x00,
        );

        assert!(!dac_enabled);
        assert!(!channel_enabled);
        assert_eq!(envelope.initial_volume, 0);
        assert_eq!(envelope.period, 0);
    }

    #[test]
    fn length_edge_helper_matches_dmg_edge_clock_behavior() {
        let mut channel_enabled = true;
        let mut length_counter = 1u8;

        apply_length_enable_edge_u8(
            &mut channel_enabled,
            &mut length_counter,
            true,
            false,
            false,
            false,
        );

        assert_eq!(length_counter, 0);
        assert!(!channel_enabled);
    }

    #[test]
    fn trigger_length_reload_helper_applies_immediate_decrement_when_needed() {
        let mut length_counter = 0u8;

        reload_length_on_trigger_u8(&mut length_counter, true, false, 64);

        assert_eq!(length_counter, 63);
    }
}
