use gb_emu::gameboy::GameBoy;
use gb_emu::input::Button;
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;

pub(super) enum EventAction {
    Continue,
    FlushPersistence,
    Quit,
    ShowCartInfo,
}

pub(super) fn process_event(gb: &mut GameBoy, event: Event) -> EventAction {
    match event {
        Event::Quit { .. }
        | Event::KeyDown {
            keycode: Some(Keycode::Escape),
            ..
        } => EventAction::Quit,
        Event::Window {
            win_event: WindowEvent::FocusLost,
            ..
        } => EventAction::FlushPersistence,
        Event::KeyDown {
            keycode: Some(Keycode::F1),
            repeat: false,
            ..
        } => EventAction::ShowCartInfo,
        Event::KeyDown {
            keycode: Some(code),
            repeat: false,
            ..
        } => {
            if let Some((player_index, button)) = map_key_to_player_button(code) {
                gb.set_player_button_pressed(player_index, button, true);
            }
            EventAction::Continue
        }
        Event::KeyUp {
            keycode: Some(code),
            repeat: false,
            ..
        } => {
            if let Some((player_index, button)) = map_key_to_player_button(code) {
                gb.set_player_button_pressed(player_index, button, false);
            }
            EventAction::Continue
        }
        _ => EventAction::Continue,
    }
}

fn map_key_to_player_button(code: Keycode) -> Option<(usize, Button)> {
    match code {
        Keycode::Right => Some((0, Button::Right)),
        Keycode::Left => Some((0, Button::Left)),
        Keycode::Up => Some((0, Button::Up)),
        Keycode::Down => Some((0, Button::Down)),
        Keycode::Z => Some((0, Button::B)),
        Keycode::X => Some((0, Button::A)),
        Keycode::Backspace => Some((0, Button::Select)),
        Keycode::Return => Some((0, Button::Start)),
        Keycode::D => Some((1, Button::Right)),
        Keycode::A => Some((1, Button::Left)),
        Keycode::W => Some((1, Button::Up)),
        Keycode::S => Some((1, Button::Down)),
        Keycode::F => Some((1, Button::B)),
        Keycode::G => Some((1, Button::A)),
        Keycode::R => Some((1, Button::Select)),
        Keycode::T => Some((1, Button::Start)),
        Keycode::L => Some((2, Button::Right)),
        Keycode::J => Some((2, Button::Left)),
        Keycode::I => Some((2, Button::Up)),
        Keycode::K => Some((2, Button::Down)),
        Keycode::U => Some((2, Button::B)),
        Keycode::O => Some((2, Button::A)),
        Keycode::Y => Some((2, Button::Select)),
        Keycode::P => Some((2, Button::Start)),
        Keycode::Kp6 => Some((3, Button::Right)),
        Keycode::Kp4 => Some((3, Button::Left)),
        Keycode::Kp8 => Some((3, Button::Up)),
        Keycode::Kp5 => Some((3, Button::Down)),
        Keycode::Kp1 => Some((3, Button::B)),
        Keycode::Kp2 => Some((3, Button::A)),
        Keycode::Kp7 => Some((3, Button::Select)),
        Keycode::Kp9 => Some((3, Button::Start)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gb_emu::cartridge::Cartridge;

    fn test_gb() -> GameBoy {
        let mut rom = vec![0; 32 * 1024];
        rom[0x0147] = 0x00;
        rom[0x0148] = 0x00;
        let cartridge = Cartridge::from_bytes(rom).expect("test ROM should load");
        GameBoy::new(cartridge)
    }

    #[test]
    fn map_key_to_player_button_maps_expected_keys_for_all_players() {
        assert_eq!(
            map_key_to_player_button(Keycode::Right),
            Some((0, Button::Right))
        );
        assert_eq!(map_key_to_player_button(Keycode::Z), Some((0, Button::B)));
        assert_eq!(
            map_key_to_player_button(Keycode::D),
            Some((1, Button::Right))
        );
        assert_eq!(map_key_to_player_button(Keycode::G), Some((1, Button::A)));
        assert_eq!(
            map_key_to_player_button(Keycode::L),
            Some((2, Button::Right))
        );
        assert_eq!(map_key_to_player_button(Keycode::O), Some((2, Button::A)));
        assert_eq!(
            map_key_to_player_button(Keycode::Kp6),
            Some((3, Button::Right))
        );
        assert_eq!(map_key_to_player_button(Keycode::Kp2), Some((3, Button::A)));
        assert_eq!(map_key_to_player_button(Keycode::Space), None);
    }

    #[test]
    fn process_event_requests_persistence_flush_on_focus_lost() {
        let mut gb = test_gb();
        let event = Event::Window {
            timestamp: 0,
            window_id: 0,
            win_event: WindowEvent::FocusLost,
        };

        let action = process_event(&mut gb, event);

        assert!(matches!(action, EventAction::FlushPersistence));
    }
}
