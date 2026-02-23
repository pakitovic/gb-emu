use gb_emu::gameboy::GameBoy;
use gb_emu::input::Button;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

pub(super) enum EventAction {
    Continue,
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
        Keycode::Z => Some(Button::A),
        Keycode::X => Some(Button::B),
        Keycode::Backspace => Some(Button::Select),
        Keycode::Return => Some(Button::Start),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_key_to_button_maps_expected_keys() {
        assert_eq!(map_key_to_button(Keycode::Right), Some(Button::Right));
        assert_eq!(map_key_to_button(Keycode::Z), Some(Button::A));
        assert_eq!(map_key_to_button(Keycode::Return), Some(Button::Start));
        assert_eq!(map_key_to_button(Keycode::Space), None);
    }
}
