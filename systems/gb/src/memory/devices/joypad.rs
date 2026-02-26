use super::super::Bus;
use crate::input::Button;

const P1_DIRECTION_SELECT: u8 = 1 << 4;
const P1_BUTTON_SELECT: u8 = 1 << 5;
const JOYPAD_INTERRUPT_BIT: u8 = 1 << 4;

#[derive(Default)]
pub(in crate::memory) struct JoypadState {
    pressed_mask: u8,
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

    fn is_pressed(bus: &Bus, button: Button) -> bool {
        (bus.joypad.pressed_mask & Self::bit(button)) != 0
    }

    fn selected_low_nibble(bus: &Bus, p1_select_bits: u8) -> u8 {
        let mut low = 0x0F;

        // P14 low selects directional buttons.
        if (p1_select_bits & P1_DIRECTION_SELECT) == 0 {
            if Self::is_pressed(bus, Button::Right) {
                low &= !0x01;
            }
            if Self::is_pressed(bus, Button::Left) {
                low &= !0x02;
            }
            if Self::is_pressed(bus, Button::Up) {
                low &= !0x04;
            }
            if Self::is_pressed(bus, Button::Down) {
                low &= !0x08;
            }
        }

        // P15 low selects action buttons.
        if (p1_select_bits & P1_BUTTON_SELECT) == 0 {
            if Self::is_pressed(bus, Button::A) {
                low &= !0x01;
            }
            if Self::is_pressed(bus, Button::B) {
                low &= !0x02;
            }
            if Self::is_pressed(bus, Button::Select) {
                low &= !0x04;
            }
            if Self::is_pressed(bus, Button::Start) {
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

    pub(in crate::memory) fn write_p1(bus: &mut Bus, value: u8) {
        let old_select = bus.io[0x00] & (P1_DIRECTION_SELECT | P1_BUTTON_SELECT);
        let old_low = Self::selected_low_nibble(bus, old_select);

        let new_select = value & (P1_DIRECTION_SELECT | P1_BUTTON_SELECT);
        bus.io[0x00] = new_select;
        let new_low = Self::selected_low_nibble(bus, new_select);
        Self::request_interrupt_on_new_press(bus, old_low, new_low);
    }

    pub(in crate::memory) fn read_p1(bus: &Bus) -> u8 {
        let select = bus.io[0x00] & (P1_DIRECTION_SELECT | P1_BUTTON_SELECT);
        let low = Self::selected_low_nibble(bus, select);
        select | low
    }

    pub(in crate::memory) fn set_button_pressed(bus: &mut Bus, button: Button, pressed: bool) {
        let select = bus.io[0x00] & (P1_DIRECTION_SELECT | P1_BUTTON_SELECT);
        let old_low = Self::selected_low_nibble(bus, select);

        let bit = Self::bit(button);
        if pressed {
            bus.joypad.pressed_mask |= bit;
        } else {
            bus.joypad.pressed_mask &= !bit;
        }

        let new_low = Self::selected_low_nibble(bus, select);
        Self::request_interrupt_on_new_press(bus, old_low, new_low);
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
}
