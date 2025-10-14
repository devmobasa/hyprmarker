//! Drawing state machine and input state management.

use super::board_mode::BoardMode;
use super::events::{Key, MouseButton};
use super::modifiers::Modifiers;
use super::tool::Tool;
use crate::draw::{CanvasSet, Color, FontDescriptor, Shape};
use crate::util;

/// Current drawing mode state machine.
///
/// Tracks whether the user is idle, actively drawing a shape, or entering text.
/// State transitions occur based on mouse and keyboard events.
#[derive(Debug)]
pub enum DrawingState {
    /// Not actively drawing - waiting for user input
    Idle,
    /// Actively drawing a shape (mouse button held down)
    Drawing {
        /// Which tool is being used for this shape
        tool: Tool,
        /// Starting X coordinate (where mouse was pressed)
        start_x: i32,
        /// Starting Y coordinate (where mouse was pressed)
        start_y: i32,
        /// Accumulated points for freehand drawing
        points: Vec<(i32, i32)>,
    },
    /// Text input mode - user is typing text to place on screen
    TextInput {
        /// X coordinate where text will be placed
        x: i32,
        /// Y coordinate where text will be placed
        y: i32,
        /// Accumulated text buffer
        buffer: String,
    },
}

/// Main input state containing all drawing session state.
///
/// This struct holds the current frame (all drawn shapes), drawing parameters,
/// modifier keys, drawing mode, and UI flags. It processes all keyboard and
/// mouse events to update the drawing state and determine when redraws are needed.
pub struct InputState {
    /// Multi-frame canvas management (transparent, whiteboard, blackboard)
    pub canvas_set: CanvasSet,
    /// Current drawing color (changed with color keys: R, G, B, etc.)
    pub current_color: Color,
    /// Current pen/line thickness in pixels (changed with +/- keys)
    pub current_thickness: f64,
    /// Current font size for text mode (from config)
    pub current_font_size: f64,
    /// Font descriptor for text rendering (family, weight, style)
    pub font_descriptor: FontDescriptor,
    /// Whether to draw background behind text
    pub text_background_enabled: bool,
    /// Arrowhead length in pixels (from config)
    pub arrow_length: f64,
    /// Arrowhead angle in degrees (from config)
    pub arrow_angle: f64,
    /// Current modifier key state
    pub modifiers: Modifiers,
    /// Current drawing mode state machine
    pub state: DrawingState,
    /// Whether user requested to exit the overlay
    pub should_exit: bool,
    /// Whether the display needs to be redrawn
    pub needs_redraw: bool,
    /// Whether the help overlay is currently visible (toggled with F10)
    pub show_help: bool,
    /// Screen width in pixels (set by backend after configuration)
    pub screen_width: u32,
    /// Screen height in pixels (set by backend after configuration)
    pub screen_height: u32,
    /// Previous color before entering board mode (for restoration)
    pub board_previous_color: Option<Color>,
}

impl InputState {
    /// Creates a new InputState with specified defaults.
    ///
    /// Screen dimensions default to 0 and should be updated by the backend
    /// after surface configuration (see `update_screen_dimensions`).
    ///
    /// # Arguments
    /// * `color` - Initial drawing color
    /// * `thickness` - Initial pen thickness in pixels
    /// * `font_size` - Font size for text mode in points
    /// * `font_descriptor` - Font configuration for text rendering
    /// * `text_background_enabled` - Whether to draw background behind text
    /// * `arrow_length` - Arrowhead length in pixels
    /// * `arrow_angle` - Arrowhead angle in degrees
    pub fn with_defaults(
        color: Color,
        thickness: f64,
        font_size: f64,
        font_descriptor: FontDescriptor,
        text_background_enabled: bool,
        arrow_length: f64,
        arrow_angle: f64,
    ) -> Self {
        Self {
            canvas_set: CanvasSet::new(),
            current_color: color,
            current_thickness: thickness,
            current_font_size: font_size,
            font_descriptor,
            text_background_enabled,
            arrow_length,
            arrow_angle,
            modifiers: Modifiers::new(),
            state: DrawingState::Idle,
            should_exit: false,
            needs_redraw: true,
            show_help: false,
            screen_width: 0,
            screen_height: 0,
            board_previous_color: None,
        }
    }

    /// Updates screen dimensions after backend configuration.
    ///
    /// This should be called by the backend when it receives the actual
    /// screen dimensions from the display server.
    ///
    /// # Arguments
    /// * `width` - Screen width in pixels
    /// * `height` - Screen height in pixels
    pub fn update_screen_dimensions(&mut self, width: u32, height: u32) {
        self.screen_width = width;
        self.screen_height = height;
    }

    /// Returns the current board mode.
    pub fn board_mode(&self) -> BoardMode {
        self.canvas_set.active_mode()
    }

    /// Switches to a different board mode with color auto-adjustment.
    ///
    /// Handles mode transitions with automatic color adjustment for contrast:
    /// - Entering board mode: saves current color, applies mode default
    /// - Exiting board mode: restores previous color
    /// - Switching between boards: applies new mode default
    ///
    /// Also resets drawing state to prevent partial shapes crossing modes.
    pub fn switch_board_mode(&mut self, new_mode: BoardMode) {
        let current_mode = self.canvas_set.active_mode();

        // Toggle behavior: if already in target mode, return to transparent
        let target_mode = if current_mode == new_mode && new_mode != BoardMode::Transparent {
            BoardMode::Transparent
        } else {
            new_mode
        };

        // No-op if we're already in the target mode
        if current_mode == target_mode {
            return;
        }

        // Handle color auto-adjustment based on transition type
        match (current_mode, target_mode) {
            // Entering board mode from transparent
            (BoardMode::Transparent, BoardMode::Whiteboard | BoardMode::Blackboard) => {
                // Save current color and apply board default
                self.board_previous_color = Some(self.current_color);
                if let Some(default_color) = target_mode.default_pen_color() {
                    self.current_color = default_color;
                }
            }
            // Exiting board mode to transparent
            (BoardMode::Whiteboard | BoardMode::Blackboard, BoardMode::Transparent) => {
                // Restore previous color if we saved one
                if let Some(prev_color) = self.board_previous_color {
                    self.current_color = prev_color;
                    self.board_previous_color = None;
                }
            }
            // Switching between board modes
            (BoardMode::Whiteboard, BoardMode::Blackboard)
            | (BoardMode::Blackboard, BoardMode::Whiteboard) => {
                // Apply new board's default color
                if let Some(default_color) = target_mode.default_pen_color() {
                    self.current_color = default_color;
                }
            }
            // All other transitions (shouldn't happen, but handle gracefully)
            _ => {}
        }

        // Switch the active frame
        self.canvas_set.switch_mode(target_mode);

        // Reset drawing state to prevent partial shapes crossing modes
        self.state = DrawingState::Idle;

        // Trigger redraw
        self.needs_redraw = true;

        log::info!("Switched from {:?} to {:?} mode", current_mode, target_mode);
    }

    /// Processes a key press event.
    ///
    /// Handles all keyboard input including:
    /// - Drawing color selection (R, G, B, Y, O, P, W, K)
    /// - Tool actions (T for text mode, E for clear, Ctrl+Z for undo)
    /// - Text input (when in TextInput state)
    /// - Exit commands (Escape, Ctrl+Q)
    /// - Thickness adjustment (+/-, mouse scroll handled separately)
    /// - Help toggle (F10)
    /// - Modifier key tracking
    pub fn on_key_press(&mut self, key: Key) {
        match key {
            Key::Escape => {
                // Exit drawing mode or cancel current action
                match &self.state {
                    DrawingState::TextInput { .. } => {
                        // Cancel text input
                        self.state = DrawingState::Idle;
                        self.needs_redraw = true;
                    }
                    DrawingState::Drawing { .. } => {
                        // Cancel current drawing
                        self.state = DrawingState::Idle;
                        self.needs_redraw = true;
                    }
                    DrawingState::Idle => {
                        // Exit application
                        self.should_exit = true;
                    }
                }
            }
            Key::Char(c) => {
                // Handle character input - check if we're in text mode first
                if let DrawingState::TextInput { buffer, .. } = &mut self.state {
                    // In text mode, ALL characters go to the buffer
                    buffer.push(c);
                    self.needs_redraw = true;
                } else {
                    // Not in text mode, handle special keys
                    match c {
                        // Board mode toggles (Ctrl+W, Ctrl+B)
                        'w' | 'W' if self.modifiers.ctrl => {
                            log::info!("Ctrl+W pressed - toggling whiteboard mode");
                            self.switch_board_mode(BoardMode::Whiteboard);
                        }
                        'b' | 'B' if self.modifiers.ctrl => {
                            log::info!("Ctrl+B pressed - toggling blackboard mode");
                            self.switch_board_mode(BoardMode::Blackboard);
                        }
                        't' | 'T' if self.modifiers.ctrl && self.modifiers.shift => {
                            // Ctrl+Shift+T: explicit return to transparent
                            log::info!("Ctrl+Shift+T pressed - returning to transparent mode");
                            self.switch_board_mode(BoardMode::Transparent);
                        }
                        't' | 'T' if !self.modifiers.ctrl => {
                            // Regular T: Enter text mode (only if not holding Ctrl)
                            if matches!(self.state, DrawingState::Idle) {
                                self.state = DrawingState::TextInput {
                                    x: (self.screen_width / 2) as i32,
                                    y: (self.screen_height / 2) as i32,
                                    buffer: String::new(),
                                };
                                self.needs_redraw = true;
                            }
                        }
                        'e' | 'E' => {
                            // Clear all annotations in active frame
                            self.canvas_set.clear_active();
                            self.needs_redraw = true;
                        }
                        'z' | 'Z' if self.modifiers.ctrl => {
                            // Undo last shape in active frame
                            if self.canvas_set.active_frame_mut().undo() {
                                self.needs_redraw = true;
                            }
                        }
                        'q' | 'Q' if self.modifiers.ctrl => {
                            // Exit overlay (return to daemon mode)
                            log::info!("Ctrl+Q pressed - setting should_exit=true");
                            self.should_exit = true;
                            // Trigger redraw to force event loop to check should_exit
                            self.needs_redraw = true;
                        }
                        _ => {
                            // Check if it's a color key (only if not holding Ctrl)
                            if !self.modifiers.ctrl {
                                if let Some(color) = util::key_to_color(c) {
                                    self.current_color = color;
                                    self.needs_redraw = true;
                                }
                            }
                        }
                    }
                }
            }
            Key::Backspace => {
                if let DrawingState::TextInput { buffer, .. } = &mut self.state {
                    buffer.pop();
                    self.needs_redraw = true;
                }
            }
            Key::Return => {
                // Finalize text input or insert newline if Shift is held
                if let DrawingState::TextInput { x, y, buffer } = &mut self.state {
                    if self.modifiers.shift {
                        // Shift+Enter: insert newline
                        buffer.push('\n');
                        self.needs_redraw = true;
                    } else {
                        // Plain Enter: finalize text input
                        if !buffer.is_empty() {
                            self.canvas_set.active_frame_mut().add_shape(Shape::Text {
                                x: *x,
                                y: *y,
                                text: buffer.clone(),
                                color: self.current_color,
                                size: self.current_font_size,
                                font_descriptor: self.font_descriptor.clone(),
                                background_enabled: self.text_background_enabled,
                            });
                            self.needs_redraw = true;
                        }
                        self.state = DrawingState::Idle;
                    }
                }
            }
            Key::Space => {
                if let DrawingState::TextInput { buffer, .. } = &mut self.state {
                    buffer.push(' ');
                    self.needs_redraw = true;
                }
            }
            Key::Shift => self.modifiers.shift = true,
            Key::Ctrl => self.modifiers.ctrl = true,
            Key::Alt => self.modifiers.alt = true,
            Key::Tab => self.modifiers.tab = true,
            Key::Plus | Key::Equals => {
                // Increase thickness
                self.current_thickness = (self.current_thickness + 1.0).min(20.0);
                self.needs_redraw = true;
            }
            Key::Minus | Key::Underscore => {
                // Decrease thickness
                self.current_thickness = (self.current_thickness - 1.0).max(1.0);
                self.needs_redraw = true;
            }
            Key::F10 => {
                // Toggle help overlay
                self.show_help = !self.show_help;
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    /// Processes a key release event.
    ///
    /// Currently only tracks modifier key releases to update the modifier state.
    pub fn on_key_release(&mut self, key: Key) {
        match key {
            Key::Shift => self.modifiers.shift = false,
            Key::Ctrl => self.modifiers.ctrl = false,
            Key::Alt => self.modifiers.alt = false,
            Key::Tab => self.modifiers.tab = false,
            _ => {}
        }
    }

    /// Processes a mouse button press event.
    ///
    /// # Arguments
    /// * `button` - Which mouse button was pressed
    /// * `x` - Mouse X coordinate
    /// * `y` - Mouse Y coordinate
    ///
    /// # Behavior
    /// - Left click while Idle: Starts drawing with the current tool (based on modifiers)
    /// - Left click during TextInput: Updates text position
    /// - Right click: Cancels current action
    pub fn on_mouse_press(&mut self, button: MouseButton, x: i32, y: i32) {
        match button {
            MouseButton::Left => {
                // Start drawing with current tool
                if matches!(self.state, DrawingState::Idle) {
                    let tool = self.modifiers.current_tool();
                    self.state = DrawingState::Drawing {
                        tool,
                        start_x: x,
                        start_y: y,
                        points: vec![(x, y)],
                    };
                    self.needs_redraw = true;
                } else if let DrawingState::TextInput { x: tx, y: ty, .. } = &mut self.state {
                    // Update text position if in text mode
                    *tx = x;
                    *ty = y;
                    self.needs_redraw = true;
                }
            }
            MouseButton::Right => {
                // Right-click could cancel or exit
                if !matches!(self.state, DrawingState::Idle) {
                    self.state = DrawingState::Idle;
                    self.needs_redraw = true;
                }
            }
            _ => {}
        }
    }

    /// Processes mouse motion (dragging) events.
    ///
    /// # Arguments
    /// * `x` - Current mouse X coordinate
    /// * `y` - Current mouse Y coordinate
    ///
    /// # Behavior
    /// - When drawing with Pen tool: Adds points to the freehand stroke
    /// - When drawing with other tools: Triggers redraw for live preview
    pub fn on_mouse_motion(&mut self, x: i32, y: i32) {
        if let DrawingState::Drawing { tool, points, .. } = &mut self.state {
            if *tool == Tool::Pen {
                // Add point to freehand stroke
                points.push((x, y));
            }
            // For other tools, we'll update the end point in release
            self.needs_redraw = true;
        }
    }

    /// Processes mouse button release events.
    ///
    /// # Arguments
    /// * `button` - Which mouse button was released
    /// * `x` - Mouse X coordinate at release
    /// * `y` - Mouse Y coordinate at release
    ///
    /// # Behavior
    /// When left button is released during drawing:
    /// - Finalizes the shape using start position and current position
    /// - Adds the completed shape to the frame
    /// - Returns to Idle state
    pub fn on_mouse_release(&mut self, button: MouseButton, x: i32, y: i32) {
        if button != MouseButton::Left {
            return;
        }

        if let DrawingState::Drawing {
            tool,
            start_x,
            start_y,
            points,
        } = &self.state
        {
            let shape = match tool {
                Tool::Pen => Shape::Freehand {
                    points: points.clone(),
                    color: self.current_color,
                    thick: self.current_thickness,
                },
                Tool::Line => Shape::Line {
                    x1: *start_x,
                    y1: *start_y,
                    x2: x,
                    y2: y,
                    color: self.current_color,
                    thick: self.current_thickness,
                },
                Tool::Rect => {
                    // Normalize rectangle to handle dragging in any direction
                    let (x, w) = if x >= *start_x {
                        (*start_x, x - start_x)
                    } else {
                        (x, start_x - x)
                    };
                    let (y, h) = if y >= *start_y {
                        (*start_y, y - start_y)
                    } else {
                        (y, start_y - y)
                    };
                    Shape::Rect {
                        x,
                        y,
                        w,
                        h,
                        color: self.current_color,
                        thick: self.current_thickness,
                    }
                }
                Tool::Ellipse => {
                    let (cx, cy, rx, ry) = util::ellipse_bounds(*start_x, *start_y, x, y);
                    Shape::Ellipse {
                        cx,
                        cy,
                        rx,
                        ry,
                        color: self.current_color,
                        thick: self.current_thickness,
                    }
                }
                Tool::Arrow => Shape::Arrow {
                    x1: *start_x,
                    y1: *start_y,
                    x2: x,
                    y2: y,
                    color: self.current_color,
                    thick: self.current_thickness,
                    arrow_length: self.arrow_length,
                    arrow_angle: self.arrow_angle,
                },
            };

            self.canvas_set.active_frame_mut().add_shape(shape);
            self.state = DrawingState::Idle;
            self.needs_redraw = true;
        }
    }

    /// Returns the shape currently being drawn for live preview.
    ///
    /// # Arguments
    /// * `current_x` - Current mouse X coordinate
    /// * `current_y` - Current mouse Y coordinate
    ///
    /// # Returns
    /// - `Some(Shape)` if actively drawing (for preview rendering)
    /// - `None` if idle or in text input mode
    ///
    /// # Note
    /// For Pen tool (freehand), this clones the points vector. For better performance
    /// with long strokes, consider using `render_provisional_shape` directly with a
    /// borrow instead of calling this method and rendering separately.
    ///
    /// This allows the backend to render a preview of the shape being drawn
    /// before the mouse button is released.
    pub fn get_provisional_shape(&self, current_x: i32, current_y: i32) -> Option<Shape> {
        if let DrawingState::Drawing {
            tool,
            start_x,
            start_y,
            points,
        } = &self.state
        {
            match tool {
                Tool::Pen => Some(Shape::Freehand {
                    points: points.clone(), // TODO: Consider using Cow or separate borrow API
                    color: self.current_color,
                    thick: self.current_thickness,
                }),
                Tool::Line => Some(Shape::Line {
                    x1: *start_x,
                    y1: *start_y,
                    x2: current_x,
                    y2: current_y,
                    color: self.current_color,
                    thick: self.current_thickness,
                }),
                Tool::Rect => {
                    // Normalize rectangle to handle dragging in any direction
                    let (x, w) = if current_x >= *start_x {
                        (*start_x, current_x - start_x)
                    } else {
                        (current_x, start_x - current_x)
                    };
                    let (y, h) = if current_y >= *start_y {
                        (*start_y, current_y - start_y)
                    } else {
                        (current_y, start_y - current_y)
                    };
                    Some(Shape::Rect {
                        x,
                        y,
                        w,
                        h,
                        color: self.current_color,
                        thick: self.current_thickness,
                    })
                }
                Tool::Ellipse => {
                    let (cx, cy, rx, ry) =
                        util::ellipse_bounds(*start_x, *start_y, current_x, current_y);
                    Some(Shape::Ellipse {
                        cx,
                        cy,
                        rx,
                        ry,
                        color: self.current_color,
                        thick: self.current_thickness,
                    })
                }
                Tool::Arrow => Some(Shape::Arrow {
                    x1: *start_x,
                    y1: *start_y,
                    x2: current_x,
                    y2: current_y,
                    color: self.current_color,
                    thick: self.current_thickness,
                    arrow_length: self.arrow_length,
                    arrow_angle: self.arrow_angle,
                }),
                // No provisional shape for other tools
            }
        } else {
            None
        }
    }

    /// Renders the provisional shape directly to a Cairo context without cloning.
    ///
    /// This is an optimized version for freehand drawing that avoids cloning
    /// the points vector on every render, preventing quadratic performance.
    ///
    /// # Arguments
    /// * `ctx` - Cairo context to render to
    /// * `current_x` - Current mouse X coordinate
    /// * `current_y` - Current mouse Y coordinate
    ///
    /// # Returns
    /// `true` if a provisional shape was rendered, `false` otherwise
    pub fn render_provisional_shape(
        &self,
        ctx: &cairo::Context,
        current_x: i32,
        current_y: i32,
    ) -> bool {
        if let DrawingState::Drawing {
            tool,
            start_x: _,
            start_y: _,
            points,
        } = &self.state
        {
            match tool {
                Tool::Pen => {
                    // Render freehand without cloning - just borrow the points
                    crate::draw::render_freehand_borrowed(
                        ctx,
                        points,
                        self.current_color,
                        self.current_thickness,
                    );
                    true
                }
                _ => {
                    // For other tools, use the normal path (no clone needed)
                    if let Some(shape) = self.get_provisional_shape(current_x, current_y) {
                        crate::draw::render_shape(ctx, &shape);
                        true
                    } else {
                        false
                    }
                }
            }
        } else {
            false
        }
    }
}
