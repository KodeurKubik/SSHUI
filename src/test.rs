#[cfg(test)]
mod tests {
    use crate::App;
    use termwiz::input::{InputEvent, KeyCode, KeyEvent, Modifiers};

    struct TestApp {
        counter: i32,
        exit: bool,
        exit_message: Option<String>,
    }

    impl Default for TestApp {
        fn default() -> Self {
            Self {
                counter: 0,
                exit: false,
                exit_message: None,
            }
        }
    }

    impl App for TestApp {
        fn render(
            &mut self,
            terminal: &mut crate::SSHUITerminal,
        ) -> anyhow::Result<Option<String>> {
            terminal.draw(|frame| {
                let area = frame.area();
                assert!(area.width > 0);
                assert!(area.height > 0);
            })?;

            Ok(if self.exit {
                self.exit_message.clone()
            } else {
                None
            })
        }

        fn input(&mut self, event: InputEvent) {
            if let InputEvent::Key(KeyEvent { key, modifiers }) = event {
                match key {
                    KeyCode::Char('q') => self.exit = true,
                    KeyCode::Char('c') if modifiers.contains(Modifiers::CTRL) => {
                        self.exit = true;
                        self.exit_message = Some("Ctrl+C pressed".to_string());
                    }
                    KeyCode::UpArrow => self.counter += 1,
                    KeyCode::DownArrow => self.counter -= 1,
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn test_app_creation() {
        let app = TestApp::default();
        assert_eq!(app.counter, 0);
        assert!(!app.exit);
    }

    #[test]
    fn test_app_input_quit() {
        let mut app = TestApp::default();
        let event = InputEvent::Key(KeyEvent {
            key: KeyCode::Char('q'),
            modifiers: Modifiers::default(),
        });
        app.input(event);
        assert!(app.exit);
    }

    #[test]
    fn test_app_input_ctrl_c() {
        let mut app = TestApp::default();
        let event = InputEvent::Key(KeyEvent {
            key: KeyCode::Char('c'),
            modifiers: Modifiers::CTRL,
        });
        app.input(event);
        assert!(app.exit);
        assert_eq!(app.exit_message, Some("Ctrl+C pressed".to_string()));
    }

    #[test]
    fn test_app_input_counter_up() {
        let mut app = TestApp::default();
        let event = InputEvent::Key(KeyEvent {
            key: KeyCode::UpArrow,
            modifiers: Modifiers::default(),
        });
        app.input(event);
        assert_eq!(app.counter, 1);
    }

    #[test]
    fn test_app_input_counter_down() {
        let mut app = TestApp::default();
        let event = InputEvent::Key(KeyEvent {
            key: KeyCode::DownArrow,
            modifiers: Modifiers::default(),
        });
        app.input(event);
        assert_eq!(app.counter, -1);
    }

    #[test]
    fn test_app_multiple_inputs() {
        let mut app = TestApp::default();

        for _ in 0..3 {
            let event = InputEvent::Key(KeyEvent {
                key: KeyCode::UpArrow,
                modifiers: Modifiers::default(),
            });
            app.input(event);
        }
        assert_eq!(app.counter, 3);

        for _ in 0..2 {
            let event = InputEvent::Key(KeyEvent {
                key: KeyCode::DownArrow,
                modifiers: Modifiers::default(),
            });
            app.input(event);
        }
        assert_eq!(app.counter, 1);
    }

    #[test]
    fn test_app_render_exit_message() {
        use crate::backend::SSHUIBackend;
        use ratatui::Terminal;
        use std::sync::{Arc, Mutex};

        let mut app = TestApp::default();
        app.exit = true;
        app.exit_message = Some("Custom exit".to_string());

        let output = Arc::new(Mutex::new(Vec::new()));
        let output_clone = output.clone();
        let write = move |bytes: &[u8]| {
            if let Ok(mut buf) = output_clone.lock() {
                buf.extend_from_slice(bytes);
            }
        };

        let backend = SSHUIBackend {
            write: Box::new(write),
            size: ratatui::layout::Rect::new(0, 0, 80, 24),
        };

        let mut terminal = Terminal::new(backend).unwrap();
        let result = app.render(&mut terminal).unwrap();

        assert_eq!(result, Some("Custom exit".to_string()));
    }

    #[test]
    fn test_backend_terminal_sizes() {
        use crate::backend::SSHUIBackend;
        use ratatui::backend::Backend;

        let test_cases = vec![(80, 24), (120, 40), (40, 12), (1, 1), (256, 128)];

        for (width, height) in test_cases {
            let write = |_: &[u8]| {};
            let backend = SSHUIBackend {
                write: Box::new(write),
                size: ratatui::layout::Rect::new(0, 0, width, height),
            };

            let size = backend.size().unwrap();
            assert_eq!(size.width, width);
            assert_eq!(size.height, height);
        }
    }

    #[test]
    fn test_backend_cursor_operations() {
        use crate::backend::SSHUIBackend;
        use ratatui::backend::Backend;
        use std::sync::{Arc, Mutex};

        let output = Arc::new(Mutex::new(Vec::new()));
        let output_clone = output.clone();
        let write = move |bytes: &[u8]| {
            if let Ok(mut buf) = output_clone.lock() {
                buf.extend_from_slice(bytes);
            }
        };

        let mut backend = SSHUIBackend {
            write: Box::new(write),
            size: ratatui::layout::Rect::new(0, 0, 80, 24),
        };

        backend.hide_cursor().unwrap();
        backend.show_cursor().unwrap();
        backend.set_cursor_position((10, 5)).unwrap();

        let output = output.lock().unwrap();
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("\x1b[?25l")); // hide cursor
        assert!(output_str.contains("\x1b[?25h")); // show cursor
    }

    #[test]
    fn test_backend_clear() {
        use crate::backend::SSHUIBackend;
        use ratatui::backend::Backend;
        use std::sync::{Arc, Mutex};

        let output = Arc::new(Mutex::new(Vec::new()));
        let output_clone = output.clone();
        let write = move |bytes: &[u8]| {
            if let Ok(mut buf) = output_clone.lock() {
                buf.extend_from_slice(bytes);
            }
        };

        let mut backend = SSHUIBackend {
            write: Box::new(write),
            size: ratatui::layout::Rect::new(0, 0, 80, 24),
        };

        backend.clear().unwrap();

        let output = output.lock().unwrap();
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("\x1b[2J")); // clear screen
    }

    #[test]
    fn test_input_modifiers() {
        let mut app = TestApp::default();

        // Test CTRL modifier
        let event = InputEvent::Key(KeyEvent {
            key: KeyCode::Char('c'),
            modifiers: Modifiers::CTRL,
        });
        app.input(event);
        assert!(app.exit);

        // Reset app
        let mut app = TestApp::default();

        // Test ALT modifier (should be ignored)
        let event = InputEvent::Key(KeyEvent {
            key: KeyCode::Char('q'),
            modifiers: Modifiers::ALT,
        });
        app.input(event);
        assert!(app.exit);
    }

    #[test]
    fn test_backend_window_size() {
        use crate::backend::SSHUIBackend;
        use ratatui::backend::Backend;

        let write = |_: &[u8]| {};
        let mut backend = SSHUIBackend {
            write: Box::new(write),
            size: ratatui::layout::Rect::new(0, 0, 80, 24),
        };

        let window_size = backend.window_size().unwrap();
        assert_eq!(window_size.columns_rows.width, 80);
        assert_eq!(window_size.columns_rows.height, 24);
    }
}
