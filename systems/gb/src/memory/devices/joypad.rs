use super::super::Bus;
use crate::input::Button;
use crate::sgb::{CMD_MLT_REQ, SgbLink};

const P1_DIRECTION_SELECT: u8 = 1 << 4;
const P1_BUTTON_SELECT: u8 = 1 << 5;
const P1_UNUSED_HIGH_BITS: u8 = 0xC0;
const JOYPAD_INTERRUPT_BIT: u8 = 1 << 4;
const MAX_SGB_PLAYERS: usize = 4;

pub(in crate::memory) struct JoypadState {
    pressed_masks: [u8; MAX_SGB_PLAYERS],
    sgb_link: SgbLink,
    sgb_player_count: u8,
    sgb_current_player: u8,
}

impl Default for JoypadState {
    fn default() -> Self {
        Self {
            pressed_masks: [0; MAX_SGB_PLAYERS],
            sgb_link: SgbLink::new(),
            sgb_player_count: 1,
            sgb_current_player: 0,
        }
    }
}

impl JoypadState {
    fn bit(button: Button) -> u8 {
        match button {
            Button::Right => 1 << 0,
            Button::Left => 1 << 1,
            Button::Up => 1 << 2,
            Button::Down => 1 << 3,
            Button::A => 1 << 4,
            Button::B => 1 << 5,
            Button::Select => 1 << 6,
            Button::Start => 1 << 7,
        }
    }

    fn current_player_index(bus: &Bus) -> usize {
        if bus.hardware_model.supports_sgb_features() {
            (bus.joypad.sgb_current_player as usize).min(MAX_SGB_PLAYERS - 1)
        } else {
            0
        }
    }

    fn is_pressed(bus: &Bus, player_index: usize, button: Button) -> bool {
        (bus.joypad.pressed_masks[player_index] & Self::bit(button)) != 0
    }

    fn selected_low_nibble(bus: &Bus, p1_select_bits: u8) -> u8 {
        let player_index = Self::current_player_index(bus);
        let mut low = 0x0F;

        // P14 low selects directional buttons.
        if (p1_select_bits & P1_DIRECTION_SELECT) == 0 {
            if Self::is_pressed(bus, player_index, Button::Right) {
                low &= !0x01;
            }
            if Self::is_pressed(bus, player_index, Button::Left) {
                low &= !0x02;
            }
            if Self::is_pressed(bus, player_index, Button::Up) {
                low &= !0x04;
            }
            if Self::is_pressed(bus, player_index, Button::Down) {
                low &= !0x08;
            }
        }

        // P15 low selects action buttons.
        if (p1_select_bits & P1_BUTTON_SELECT) == 0 {
            if Self::is_pressed(bus, player_index, Button::A) {
                low &= !0x01;
            }
            if Self::is_pressed(bus, player_index, Button::B) {
                low &= !0x02;
            }
            if Self::is_pressed(bus, player_index, Button::Select) {
                low &= !0x04;
            }
            if Self::is_pressed(bus, player_index, Button::Start) {
                low &= !0x08;
            }
        }

        low
    }

    fn request_interrupt_on_new_press(bus: &mut Bus, old_low: u8, new_low: u8) {
        let falling_edges = old_low & !new_low;
        if (falling_edges & 0x0F) != 0 {
            let iflags = bus.interrupt_flags() | JOYPAD_INTERRUPT_BIT;
            bus.set_interrupt_flags(iflags);
        }
    }

    fn maybe_cycle_sgb_current_player(bus: &mut Bus, value: u8, old_select: u8) {
        if !bus.hardware_model.supports_sgb_features() {
            return;
        }

        if (value & P1_BUTTON_SELECT) != 0
            && (old_select & P1_BUTTON_SELECT) == 0
            && (bus.joypad.sgb_player_count & 1) == 0
        {
            bus.joypad.sgb_current_player = bus.joypad.sgb_current_player.wrapping_add(1);
            bus.joypad.sgb_current_player &= bus.joypad.sgb_player_count - 1;
        }
    }

    fn maybe_apply_sgb_multiplayer_command(bus: &mut Bus, value: u8) {
        if !bus.hardware_model.supports_sgb_features() {
            return;
        }

        let Some(command) = bus
            .joypad
            .sgb_link
            .on_p1_write(value & (P1_DIRECTION_SELECT | P1_BUTTON_SELECT))
        else {
            return;
        };
        if command.command_id != CMD_MLT_REQ {
            return;
        }

        let Some(flags) = command.payload_bytes().first().copied() else {
            return;
        };
        let mut player_count = (flags & 0x03) + 1;
        if player_count == 3 {
            player_count = 4;
        }
        bus.joypad.sgb_player_count = player_count;
        bus.joypad.sgb_current_player &= player_count - 1;
    }

    pub(in crate::memory) fn write_p1(bus: &mut Bus, value: u8) {
        let old_select = bus.io[0x00] & (P1_DIRECTION_SELECT | P1_BUTTON_SELECT);
        let old_low = Self::selected_low_nibble(bus, old_select);

        Self::maybe_cycle_sgb_current_player(bus, value, old_select);
        Self::maybe_apply_sgb_multiplayer_command(bus, value);
        let new_select = value & (P1_DIRECTION_SELECT | P1_BUTTON_SELECT);
        bus.io[0x00] = new_select;
        let new_low = Self::selected_low_nibble(bus, new_select);
        Self::request_interrupt_on_new_press(bus, old_low, new_low);
    }

    pub(in crate::memory) fn read_p1(bus: &Bus) -> u8 {
        let select = bus.io[0x00] & (P1_DIRECTION_SELECT | P1_BUTTON_SELECT);
        if bus.hardware_model.supports_sgb_features()
            && select == (P1_DIRECTION_SELECT | P1_BUTTON_SELECT)
            && bus.joypad.sgb_player_count > 1
        {
            return P1_UNUSED_HIGH_BITS | select | (0x0F - bus.joypad.sgb_current_player);
        }
        let low = Self::selected_low_nibble(bus, select);
        P1_UNUSED_HIGH_BITS | select | low
    }

    pub(in crate::memory) fn set_button_pressed(bus: &mut Bus, button: Button, pressed: bool) {
        Self::set_player_button_pressed(bus, 0, button, pressed);
    }

    pub(in crate::memory) fn set_player_button_pressed(
        bus: &mut Bus,
        player_index: usize,
        button: Button,
        pressed: bool,
    ) -> bool {
        if player_index >= MAX_SGB_PLAYERS {
            return false;
        }

        let select = bus.io[0x00] & (P1_DIRECTION_SELECT | P1_BUTTON_SELECT);
        let old_low = Self::selected_low_nibble(bus, select);

        let bit = Self::bit(button);
        if pressed {
            bus.joypad.pressed_masks[player_index] |= bit;
        } else {
            bus.joypad.pressed_masks[player_index] &= !bit;
        }

        let new_low = Self::selected_low_nibble(bus, select);
        Self::request_interrupt_on_new_press(bus, old_low, new_low);
        true
    }
}

impl Bus {
    pub(in crate::memory) fn write_p1(&mut self, value: u8) {
        JoypadState::write_p1(self, value);
    }

    pub(in crate::memory) fn read_p1(&self) -> u8 {
        JoypadState::read_p1(self)
    }

    pub fn set_button_pressed(&mut self, button: Button, pressed: bool) {
        JoypadState::set_button_pressed(self, button, pressed);
    }

    pub fn set_player_button_pressed(
        &mut self,
        player_index: usize,
        button: Button,
        pressed: bool,
    ) -> bool {
        JoypadState::set_player_button_pressed(self, player_index, button, pressed)
    }

    pub fn joypad_player_count(&self) -> u8 {
        if self.hardware_model.supports_sgb_features() {
            self.joypad.sgb_player_count
        } else {
            1
        }
    }

    pub fn current_joypad_player_index(&self) -> u8 {
        if self.hardware_model.supports_sgb_features() {
            self.joypad.sgb_current_player
        } else {
            0
        }
    }
}
