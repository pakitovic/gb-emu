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
            if let Some(button) = map_key_to_button(code) {
                gb.set_button_pressed(button, true);
            }
            EventAction::Continue
        }
        Event::KeyUp {
            keycode: Some(code),
            repeat: false,
            ..
        } => {
            if let Some(button) = map_key_to_button(code) {
                gb.set_button_pressed(button, false);
            }
            EventAction::Continue
        }
        _ => EventAction::Continue,
    }
}

fn map_key_to_button(code: Keycode) -> Option<Button> {
    match code {
        Keycode::Right => Some(Button::Right),
        Keycode::Left => Some(Button::Left),
        Keycode::Up => Some(Button::Up),
        Keycode::Down => Some(Button::Down),
        Keycode::Z => Some(Button::B),
        Keycode::X => Some(Button::A),
        Keycode::Backspace => Some(Button::Select),
        Keycode::Return => Some(Button::Start),
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
    fn map_key_to_button_maps_expected_keys() {
        assert_eq!(map_key_to_button(Keycode::Right), Some(Button::Right));
        assert_eq!(map_key_to_button(Keycode::Z), Some(Button::B));
        assert_eq!(map_key_to_button(Keycode::X), Some(Button::A));
        assert_eq!(map_key_to_button(Keycode::Return), Some(Button::Start));
        assert_eq!(map_key_to_button(Keycode::Space), None);
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
