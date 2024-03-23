use winit::event::{ Event, WindowEvent };
use winit::event_loop::{ ControlFlow, EventLoop };
use winit::window::WindowBuilder;
use winit::keyboard::{ Key::Named, NamedKey };
use tiny_skia::{ FillRule, Paint, PathBuilder, Pixmap, Rect, Transform };
use std::time::{ Duration, SystemTime };

const UNIT_SIZE: i32 = 20;
const BOARD_WIDTH: i32 = 10;
const BOARD_HEIGHT: i32 = 22;

/// Type of the key.
enum Key { LEFT, RIGHT, UP, DOWN, SP, OTHER, }

fn main() {

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = WindowBuilder::new()
        .with_inner_size(winit::dpi::LogicalSize::new(BOARD_WIDTH * UNIT_SIZE, BOARD_HEIGHT * UNIT_SIZE))
        .with_title("Tetris")
        .build(&event_loop).unwrap();

    let window = std::rc::Rc::new(window);
    let context = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

    let mut game: Tetris = Tetris::new();

    let _ = event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => elwt.exit(),
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput {event, .. },
                ..
            } if event.state.is_pressed() => {
                match event.logical_key {
                    Named(NamedKey::ArrowRight) => game.key_pressed(Key::RIGHT),
                    Named(NamedKey::ArrowLeft)  => game.key_pressed(Key::LEFT),
                    Named(NamedKey::ArrowDown)  => game.key_pressed(Key::DOWN),
                    Named(NamedKey::ArrowUp)    => game.key_pressed(Key::UP),
                    Named(NamedKey::Space)      => game.key_pressed(Key::SP),
                    Named(NamedKey::Escape)     => game.rerun(),
                    _ => game.key_pressed(Key::OTHER),
                };
                window.request_redraw();
            },
            Event::AboutToWait => {
                if !game.stopped {
                    game.tick();
                    window.set_title(format!("Tetris:{}", game.score).as_str());
                    window.request_redraw();
                }
            },
            Event::WindowEvent {
                window_id, event: WindowEvent::RedrawRequested
            } if window_id == window.id() => {
                let (width, height) = {
                    let size = window.inner_size();
                    (size.width, size.height)
                };
                surface.resize(
                    core::num::NonZeroU32::new(width).unwrap(),
                    core::num::NonZeroU32::new(height).unwrap(),
                ).unwrap();

                let mut pixmap = Pixmap::new(width, height).unwrap();
                game.draw(&mut pixmap);
                let mut buffer = surface.buffer_mut().unwrap();
                for index in 0..(width * height) as usize {
                    buffer[index] =
                        pixmap.data()[index * 4 + 2] as u32
                     | (pixmap.data()[index * 4 + 1] as u32) << 8
                     | (pixmap.data()[index * 4 + 0] as u32) << 16;
                }
                buffer.present().unwrap();
            },
            _ => ()
        }
    });
}

/// Tetromino is a geometric shape composed of four squares, connected orthogonally.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Tetromino { S, Z, I, T, O, J, L, X, }

impl Tetromino {
    fn rand() -> Self {
        match rand::random::<u32>() % 7 {
            0 => Tetromino::S, 1 => Tetromino::Z,
            2 => Tetromino::I, 3 => Tetromino::T,
            4 => Tetromino::O, 5 => Tetromino::J,
            6 => Tetromino::L, _ => Tetromino::X,
        }
    }

    fn shape(&self) -> [[i32; 2]; 4] {
        match self {
            Tetromino::S => [[ 0, -1], [0,  0], [-1, 0], [-1,  1]],
            Tetromino::Z => [[ 0, -1], [0,  0], [ 1, 0], [ 1,  1]],
            Tetromino::I => [[ 0, -1], [0,  0], [ 0, 1], [ 0,  2]],
            Tetromino::T => [[-1,  0], [0,  0], [ 1, 0], [ 0, -1]],
            Tetromino::O => [[ 0,  0], [1,  0], [ 0, 1], [ 1,  1]],
            Tetromino::J => [[-1, -1], [0, -1], [ 0, 0], [ 0,  1]],
            Tetromino::L => [[ 1, -1], [0, -1], [ 0, 0], [ 0,  1]],
            Tetromino::X => [[0; 2]; 4],
        }
    }

    fn color(&self) -> (u8, u8, u8) {
        match self {
            Tetromino::S => (204, 102, 102),
            Tetromino::Z => (102, 204, 102),
            Tetromino::I => (104, 102, 204),
            Tetromino::T => (204, 204, 102),
            Tetromino::O => (204, 102, 204),
            Tetromino::J => (204, 204, 204),
            Tetromino::L => (218, 170,   0),
            _            => (  0,   0,   0)
        }
    }
}

/// A Tetromino block.
#[derive(Copy, Clone, Debug)]
struct Block {
    kind: Tetromino,
    points: [[i32; 2]; 4],
    x: i32, y: i32,
}

impl Block {

    fn new(x: i32, y: i32) -> Self {
        let kind = Tetromino::rand();
        Block {
            kind,
            points: kind.shape(),
            x,
            y: y  - kind.shape().iter().max_by_key(|p| p[1]).unwrap()[1],
        }
    }

    fn empty() -> Self {
        let kind = Tetromino::X;
        Block { kind, points: kind.shape(), x: 0, y: 0 }
    }

    fn is_empty(&self) -> bool {
        self.kind == Tetromino::X
    }

    fn point(&self, i: usize) -> (i32, i32) {
        (self.x + self.points[i][0], self.y + self.points[i][1])
    }

    fn left(&self)  -> Block { Block { x: self.x - 1, ..*self } }
    fn right(&self) -> Block { Block { x: self.x + 1, ..*self } }
    fn down(&self)  -> Block { Block { y: self.y - 1, ..*self } }

    fn rotate_left(&self)  -> Block { self.rotate(false) }
    fn rotate_right(&self) -> Block { self.rotate(true) }

    fn rotate(&self, clockwise: bool) -> Block {
        let mut points: [[i32; 2]; 4] = [[0; 2]; 4];
        for i in 0..4 {
            points[i] = if clockwise {
                [-self.points[i][1], self.points[i][0]]
            } else {
                [self.points[i][1], -self.points[i][0]]
            };
        }
        Block { points, ..*self }
    }

}


fn index_at(x: i32, y: i32) -> usize {
    (y * BOARD_WIDTH + x) as usize
}

/// Game of tetris.
struct Tetris {
    board: [Tetromino; (BOARD_WIDTH  * BOARD_HEIGHT) as usize],
    current: Block,
    stopped: bool,
    time: SystemTime,
    score: u32,
}

impl Tetris {

    fn new() -> Self {
        Tetris {
            board: [Tetromino::X; (BOARD_WIDTH  * BOARD_HEIGHT) as usize],
            current: Block::empty(),
            stopped: false,
            time: SystemTime::now(),
            score: 0,
        }
    }

    fn rerun(&mut self) {
        self.board = [Tetromino::X; (BOARD_WIDTH  * BOARD_HEIGHT) as usize];
        self.current = Block::empty();
        self.stopped = false;
        self.time = SystemTime::now();
        self.score = 0;
    }

    fn tick(&mut self) {
        if self.current.is_empty() {
            self.put_block();
        } else if self.time.elapsed().unwrap() > Duration::from_millis((1000 - self.score) as u64) {
            self.down();
            self.time = SystemTime::now();
        }
    }

    fn key_pressed(&mut self, key: Key) {
        if self.stopped || self.current.is_empty() {
            return;
        }
        match key {
            Key::LEFT  => { self.try_move(self.current.left()); },
            Key::RIGHT => { self.try_move(self.current.right()); },
            Key::UP    => { self.try_move(self.current.rotate_right()); },
            Key::DOWN  => { self.try_move(self.current.rotate_left()); },
            Key::OTHER => { self.down(); },
            Key::SP    => { self.drop_down(); },
        };
    }

    fn down(&mut self) {
        if !self.try_move(self.current.down()) {
            self.block_dropped();
        }
    }

    fn drop_down(&mut self) {
        while self.current.y > 0 {
            if !self.try_move(self.current.down()) {
                break;
            }
        }
        self.block_dropped();
    }

    fn block_dropped(&mut self) {
        for i in 0..4 {
            let (x, y) = self.current.point(i);
            self.board[index_at(x, y)] = self.current.kind;
        }
        self.remove_complete_lines();
        if self.current.is_empty() {
            self.put_block();
        }
    }

    fn put_block(&mut self) {
        self.stopped = !self.try_move(Block::new(BOARD_WIDTH / 2, BOARD_HEIGHT - 1));
    }

    fn try_move(&mut self, block: Block) -> bool {
        for i in 0..4 {
            let (x, y) = block.point(i);
            if x < 0 || x >= BOARD_WIDTH || y < 0 || y >= BOARD_HEIGHT {
                return false
            }
            if self.board[index_at(x, y)] != Tetromino::X {
                return false
            }
        }
        self.current = block;
        true
    }

    fn remove_complete_lines(&mut self) {
        let mut line_count = 0;

        for y in (0..BOARD_HEIGHT).rev() {
            let mut complete = true;
            for x in 0.. BOARD_WIDTH {
                if self.board[index_at(x, y)] == Tetromino::X {
                    // traverse the rows and if there is a blank, it cannot be completed
                    complete = false;
                    break
                }
            }
            if complete {
                line_count += 1;
                // drop the line above the completed line
                for dy in y..BOARD_HEIGHT - 1 {
                    for x in 0..BOARD_WIDTH {
                        // copy from the above line
                        self.board[index_at(x, dy)] = self.board[index_at(x, dy + 1)];
                    }
                }
            }
        }
        self.score += line_count * line_count;
        self.current = Block::empty();
    }

    fn draw(&self, pixmap: &mut Pixmap) {
        for y in 0..BOARD_HEIGHT {
            for x in 0..BOARD_WIDTH {
                Tetris::draw_square(pixmap, x, y,self.board[index_at(x, y)]);
            }
        }
        for i in 0..4 {
            let (x, y) = self.current.point(i);
            Tetris::draw_square(pixmap, x, y, self.current.kind);
        }
    }

    fn draw_square(pixmap: &mut Pixmap, x: i32, y: i32, kind: Tetromino) {
        if kind == Tetromino::X {
            return;
        }

        // left-bottom to top-left
        let x = x * UNIT_SIZE;
        let y = (BOARD_HEIGHT - 1 - y) * UNIT_SIZE;

        let rect = Rect::from_xywh(
            (x + 1) as f32,
            (y + 1) as f32,
            (UNIT_SIZE - 2) as f32,
            (UNIT_SIZE - 2) as f32,
        ).unwrap();
        let path = PathBuilder::from_rect(rect);
        let mut paint = Paint::default();
        let (r ,g, b) = kind.color();
        paint.set_color_rgba8(r, g, b, 255);
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::EvenOdd,
            Transform::identity(),
            None,
        );
    }
}

