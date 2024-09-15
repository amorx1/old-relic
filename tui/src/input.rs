use crate::app::Focus;

pub struct Input {
    pub buffer: String,
    pub cursor_position: usize,
}

pub struct Inputs {
    _inputs: [Input; 9],
}

impl Inputs {
    pub fn new() -> Self {
        Inputs {
            _inputs: [
                Input {
                    buffer: String::new(),
                    cursor_position: 0,
                },
                Input {
                    buffer: String::new(),
                    cursor_position: 0,
                },
                Input {
                    buffer: String::new(),
                    cursor_position: 0,
                },
                Input {
                    buffer: String::new(),
                    cursor_position: 0,
                },
                Input {
                    buffer: String::new(),
                    cursor_position: 0,
                },
                Input {
                    buffer: String::new(),
                    cursor_position: 0,
                },
                Input {
                    buffer: String::new(),
                    cursor_position: 0,
                },
                Input {
                    buffer: String::new(),
                    cursor_position: 0,
                },
                Input {
                    buffer: String::new(),
                    cursor_position: 0,
                },
            ],
        }
    }
    pub fn get(&self, focus: Focus) -> &str {
        &self._inputs[focus as usize].buffer
    }

    pub fn get_cursor_position(&self, focus: Focus) -> usize {
        self._inputs[focus as usize].cursor_position
    }

    pub fn len(&self, focus: Focus) -> usize {
        self._inputs[focus as usize].buffer.len()
    }

    pub fn clamp_cursor(&self, focus: Focus, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.len(focus))
    }

    pub fn move_cursor_left(&mut self, focus: Focus) {
        let cursor_moved_left = self._inputs[focus as usize]
            .cursor_position
            .saturating_sub(1);
        self._inputs[focus as usize].cursor_position = self.clamp_cursor(focus, cursor_moved_left);
    }

    pub fn move_cursor_right(&mut self, focus: Focus) {
        let cursor_moved_right = self._inputs[focus as usize]
            .cursor_position
            .saturating_add(1);
        self._inputs[focus as usize].cursor_position = self.clamp_cursor(focus, cursor_moved_right);
    }

    pub fn enter_char(&mut self, focus: Focus, new_char: char) {
        let cursor_position = self.get_cursor_position(focus);
        self._inputs[focus as usize]
            .buffer
            .insert(cursor_position, new_char);

        self.move_cursor_right(focus);
    }

    pub fn delete_char(&mut self, focus: Focus) {
        let is_not_cursor_leftmost = self.get_cursor_position(focus) != 0;
        if is_not_cursor_leftmost {
            let current_index = self.get_cursor_position(focus);
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self._inputs[focus as usize]
                .buffer
                .chars()
                .take(from_left_to_current_index);
            let after_char_to_delete = self._inputs[focus as usize]
                .buffer
                .chars()
                .skip(current_index);

            self._inputs[focus as usize].buffer =
                before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left(focus);
        }
    }

    pub fn set(&mut self, focus: Focus, value: String) {
        self._inputs[focus as usize].buffer = value;
    }

    pub fn clear(&mut self, focus: Focus) {
        self._inputs[focus as usize].buffer.clear();
    }

    pub fn reset_cursor(&mut self, focus: Focus) {
        self._inputs[focus as usize].cursor_position = 0;
    }
}
