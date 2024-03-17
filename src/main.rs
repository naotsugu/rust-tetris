use winit::event::{ Event, WindowEvent };
use winit::event_loop::{ ControlFlow, EventLoop };
use winit::window::WindowBuilder;
use tiny_skia::{ FillRule, Paint, PathBuilder, Pixmap, Rect, Transform };
use winit::keyboard::{ Key::Named, NamedKey };
use std::time::{ Duration, SystemTime };

fn main() {

    let event_loop = EventLoop::new().unwrap();

    let window = WindowBuilder::new()
        .with_inner_size(winit::dpi::LogicalSize::new(200, 440))
        .with_title("Tetris")
        .build(&event_loop).unwrap();

    let window = std::rc::Rc::new(window);
    let context = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

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
                    Named(NamedKey::ArrowLeft) => game.key_pressed(Key::LEFT),
                    Named(NamedKey::ArrowDown) => game.key_pressed(Key::DOWN),
                    Named(NamedKey::ArrowUp) => game.key_pressed(Key::UP),
                    Named(NamedKey::Space) => game.key_pressed(Key::SP),
                    Named(NamedKey::Escape) => game.rerun(),
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
    fn rand() -> Tetromino {
        match rand::random::<u16>() % 7 {
            0 => Tetromino::S, 1 => Tetromino::Z,
            2 => Tetromino::I, 3 => Tetromino::T,
            4 => Tetromino::O, 5 => Tetromino::J,
            6 => Tetromino::L, _ => Tetromino::X,
        }
    }
    fn is_rotatable(&self) -> bool {
        !(matches!(self, Tetromino::O) || matches!(self, Tetromino::X))
    }
    fn color(&self) -> Vec<u8> {
        match self {
            Tetromino::S => vec!(204, 102, 102),
            Tetromino::Z => vec!(102, 204, 102),
            Tetromino::I => vec!(104, 102, 204),
            Tetromino::T => vec!(204, 204, 102),
            Tetromino::O => vec!(204, 102, 204),
            Tetromino::J => vec!(204, 204, 204),
            Tetromino::L => vec!(218, 170, 0),
            _ => vec!(0, 0, 0)
        }
    }
}

/// A Tetromino block.
#[derive(Copy, Clone, Debug)]
struct Block {
    kind: Tetromino,
    points: [[i16; 2]; 4],
}

impl Block {

    fn rand() -> Block {
        Block::block(Tetromino::rand())
    }

    fn block(t: Tetromino) -> Block {
        match t {
            Tetromino::S => Block { kind: Tetromino::S, points: [[ 0, -1], [0,  0], [-1, 0], [-1, 1]] },
            Tetromino::Z => Block { kind: Tetromino::Z, points: [[ 0, -1], [0,  0], [ 1, 0], [ 1, 1]] },
            Tetromino::I => Block { kind: Tetromino::I, points: [[ 0, -1], [0,  0], [ 0, 1], [ 0, 2]] },
            Tetromino::T => Block { kind: Tetromino::T, points: [[-1,  0], [0,  0], [ 1, 0], [ 0, 1]] },
            Tetromino::O => Block { kind: Tetromino::O, points: [[ 0,  0], [1,  0], [ 0, 1], [ 1, 1]] },
            Tetromino::J => Block { kind: Tetromino::J, points: [[-1, -1], [0, -1], [ 0, 0], [ 0, 1]] },
            Tetromino::L => Block { kind: Tetromino::L, points: [[ 1, -1], [0, -1], [ 0, 0], [ 0, 1]] },
            Tetromino::X => Block { kind: Tetromino::X, points: [[ 0,  0], [0,  0], [ 0, 0], [ 0, 0]] }
        }
    }

    fn rotate_left(&self) -> Block {
        if self.kind.is_rotatable() {
            let mut p: [[i16; 2]; 4] = [[0; 2]; 4];
            for i in 0..4 {
                p[i] = [self.points[i][1], -self.points[i][0]];
            }
            Block { kind: self.kind, points: p }
        } else {
            self.clone()
        }
    }

    fn rotate_right(&self) -> Block {
        if self.kind.is_rotatable() {
            let mut p: [[i16; 2]; 4] = [[0; 2]; 4];
            for i in 0..4 {
                p[i] = [-self.points[i][1], self.points[i][0]];
            }
            Block { kind: self.kind, points: p }
        } else {
            self.clone()
        }
    }

    fn min_y(&self) -> i16 {
        let mut ret = self.points[0][1];
        for i in 0..4 {
            ret = std::cmp::min(ret, self.points[i][1]);
        }
        ret
    }

}

#[derive(Debug)]
struct FallingBlock {
    x: i16, y: i16, obj: Block,
}

impl FallingBlock {

    fn new() -> Self {
        let block = Block::rand();
        FallingBlock {
            x: BOARD_WIDTH / 2 + 1,
            y: BOARD_HEIGHT - 1 + block.min_y(),
            obj: block,
        }
    }

    fn empty() -> Self {
        FallingBlock { x: 0, y: 0, obj: Block::block(Tetromino::X) }
    }

    fn down(&self) -> FallingBlock {
        FallingBlock { x: self.x, y: self.y - 1, obj: self.obj }
    }

    fn left(&self) -> FallingBlock {
        FallingBlock { x: self.x - 1, y: self.y, obj: self.obj }
    }

    fn right(&self) -> FallingBlock {
        FallingBlock { x: self.x + 1, y: self.y, obj: self.obj }
    }

    fn rotate_left(&self)-> FallingBlock {
        FallingBlock { x: self.x, y: self.y, obj: self.obj.rotate_left() }
    }

    fn rotate_right(&self)-> FallingBlock {
        FallingBlock { x: self.x, y: self.y, obj: self.obj.rotate_right() }
    }

    fn is_empty(&self) -> bool {
        self.obj.kind == Tetromino::X
    }

    fn point(&self, i: usize) -> (i16, i16) {
        (self.x + self.obj.points[i][0], self.y - self.obj.points[i][1])
    }
}

/// Type of key.
enum Key { LEFT, RIGHT, UP, DOWN, SP, OTHER, }

const UNIT_SIZE: i16 = 20;
const BOARD_WIDTH: i16 = 10;
const BOARD_HEIGHT: i16 = 22;
const BOARD_LEN: usize = BOARD_WIDTH as usize * BOARD_HEIGHT as usize;

/// Game of tetris.
struct Tetris {
    board: [Tetromino; BOARD_LEN],
    current: FallingBlock,
    stopped: bool,
    time: SystemTime,
    score: u32,
}

impl Tetris {

    fn new() -> Self {
        Tetris {
            board: [Tetromino::X; BOARD_LEN],
            current: FallingBlock::empty(),
            stopped: false,
            time: SystemTime::now(),
            score: 0,
        }
    }

    fn rerun(&mut self) {
        self.board = [Tetromino::X; BOARD_LEN];
        self.current = FallingBlock::empty();
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
            Key::LEFT  => self.try_move(self.current.left()),
            Key::RIGHT => self.try_move(self.current.right()),
            Key::UP    => self.try_move(self.current.rotate_right()),
            Key::DOWN  => self.try_move(self.current.rotate_left()),
            Key::OTHER => { self.down(); true },
            Key::SP    => { self.drop_down(); true },
        };
    }

    fn down(&mut self) {
        if self.try_move(self.current.down()) == false {
            self.block_dropped();
        }
    }

    fn drop_down(&mut self) {
        while self.current.y > 0 {
            if self.try_move(self.current.down()) == false {
                break;
            }
        }
        self.block_dropped();
    }

    fn block_dropped(&mut self) {
        for i in 0..4 {
            let (x, y) = self.current.point(i);
            let index = ((y * BOARD_WIDTH) + x) as usize;
            self.board[index] = self.current.obj.kind;
        }
        self.remove_complete_lines();
        if self.current.is_empty() {
            self.put_block();
        }
    }

    fn put_block(&mut self) {
        if self.try_move(FallingBlock::new()) == false {
            self.stopped = true;
        }
    }

    fn try_move(&mut self, block: FallingBlock) -> bool {
        for i in 0..4 {
            let (x, y) = block.point(i);
            if x < 0 || x >= BOARD_WIDTH || y < 0 || y >= BOARD_HEIGHT {
                return false
            }
            if self.shape_at(x, y) != Tetromino::X {
                return false
            }
        }
        self.current = block;
        true
    }

    fn remove_complete_lines(&mut self) {
        let mut line_count = 0;

        for i in (0..BOARD_HEIGHT).rev() {
            let mut complete = true;
            for j in 0.. BOARD_WIDTH {
                if self.shape_at(j, i) == Tetromino::X {
                    complete = false;
                    break
                }
            }
            if complete {
                line_count += 1;
                for k in i.. BOARD_HEIGHT - 1 {
                    for j in 0.. BOARD_WIDTH {
                        self.board[(k * BOARD_WIDTH + j) as usize] = self.shape_at(j, k + 1);
                    }
                }
            }
        }
        self.score += line_count * line_count;
        self.current = FallingBlock::empty();
    }

    fn shape_at(&self, x: i16, y: i16) -> Tetromino {
        self.board[(y * BOARD_WIDTH + x) as usize]
    }

    fn draw(&self, pixmap: &mut Pixmap) {
        for i in 0.. BOARD_HEIGHT {
            for j in 0.. BOARD_WIDTH {
                let kind = self.shape_at(j, BOARD_HEIGHT - i - 1);
                Tetris::draw_square(pixmap, j * UNIT_SIZE, i * UNIT_SIZE, kind);
            }
        }

        for i in 0.. 4 {
            let (x, y) = self.current.point(i);
            Tetris::draw_square(pixmap, x * UNIT_SIZE, (BOARD_HEIGHT - y - 1) * UNIT_SIZE, self.current.obj.kind);
        }
    }

    fn draw_square(pixmap: &mut Pixmap, x: i16, y: i16, kind: Tetromino) {
        if kind == Tetromino::X {
            return;
        }
        let rect = Rect::from_xywh(
            (x + 1) as f32,(y + 1) as f32,
            (UNIT_SIZE - 2) as f32,(UNIT_SIZE - 2) as f32,
        ).unwrap();
        let path = PathBuilder::from_rect(rect);
        let mut paint = Paint::default();
        let color = kind.color();
        paint.set_color_rgba8(color[0], color[1], color[2], 255);
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::EvenOdd,
            Transform::identity(),
            None,
        );
    }

}

