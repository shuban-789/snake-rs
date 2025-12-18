use std::io::{self, Write, Read};
use std::time::Duration;
use std::thread;
use std::env;
use std::collections::HashSet;
use rand::Rng;

fn initialize_terminal() {
    print!("\x1b[?1049h");
    print!("\x1b[?25l");
    flush_stdout();
    
    #[cfg(unix)] {
        use std::os::unix::io::AsRawFd;
        unsafe {
            let mut termios: libc::termios = std::mem::zeroed();
            libc::tcgetattr(io::stdin().as_raw_fd(), &mut termios);
            termios.c_lflag &= !(libc::ICANON | libc::ECHO);
            termios.c_cc[libc::VMIN] = 0;
            termios.c_cc[libc::VTIME] = 0;
            libc::tcsetattr(io::stdin().as_raw_fd(), libc::TCSANOW, &termios);
        }
    }
}

fn restore_terminal() {
    print!("\x1b[?1049l");
    print!("\x1b[?25h");
    flush_stdout();
    
    #[cfg(unix)] {
        use std::os::unix::io::AsRawFd;
        unsafe {
            let mut termios: libc::termios = std::mem::zeroed();
            libc::tcgetattr(io::stdin().as_raw_fd(), &mut termios);
            termios.c_lflag |= libc::ICANON | libc::ECHO;
            libc::tcsetattr(io::stdin().as_raw_fd(), libc::TCSANOW, &termios);
        }
    }
}

fn get_terminal_size() -> (i32, i32) {
    #[cfg(unix)] {
        use std::os::unix::io::AsRawFd;
        unsafe {
            let mut size: libc::winsize = std::mem::zeroed();
            libc::ioctl(io::stdout().as_raw_fd(), libc::TIOCGWINSZ, &mut size);
            (size.ws_col as i32, size.ws_row as i32)
        }
    }
    #[cfg(not(unix))] {
        (80, 24)
    }
}

fn sleep_for_frame_time() {
    thread::sleep(Duration::from_millis(100));
}

fn place_random(blacklist: &[(usize, usize, i32)], width: usize, height: usize) -> Option<(usize, usize)> {
    let mut rng = rand::thread_rng();
    
    let occupied: HashSet<(usize, usize)> = blacklist
        .iter()
        .map(|&(x, y, _)| (x, y))
        .collect();
    
    let total_cells = width * height;
    if occupied.len() < total_cells / 2 {
        for _ in 0..total_cells {
            let x = rng.gen_range(0..width);
            let y = rng.gen_range(0..height);
            
            if !occupied.contains(&(x, y)) {
                return Some((x, y));
            }
        }
    } else {
        let valid: Vec<(usize, usize)> = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .filter(|pos| !occupied.contains(pos))
            .collect();
        
        if !valid.is_empty() {
            let idx = rng.gen_range(0..valid.len());
            return Some(valid[idx]);
        }
    }
    
    None
}

fn read_key() -> Option<String> {
    let mut buffer = [0u8; 3];
    match io::stdin().read(&mut buffer) {
        Ok(n) if n > 0 => {
            if buffer[0] == 27 && n >= 3 && buffer[1] == 91 {
                match buffer[2] {
                    65 => Some("UP".to_string()),
                    66 => Some("DOWN".to_string()),
                    67 => Some("RIGHT".to_string()),
                    68 => Some("LEFT".to_string()),
                    _ => None,
                }
            } else {
                Some((buffer[0] as char).to_string())
            }
        }
        _ => None,
    }
}

fn clear_screen() {
    print!("\x1b[2J");
}

fn draw_char(x: i32, y: i32, c: char, col: i32) {
    print!("\x1b[{};{}H\x1b[{}m{}", y + 1, x + 1, col, c);
}

fn flush_stdout() {
    io::stdout().flush().unwrap();
}

fn get_pipe_char(from_dir: i32, to_dir: i32) -> char {
    match (from_dir, to_dir) {
        (0, 0) | (2, 2) => '┃',
        (1, 1) | (3, 3) => '━',
        (3, 0) | (2, 1) => '┗',
        (0, 1) | (3, 2) => '┏',
        (1, 2) | (0, 3) => '┓',
        (2, 3) | (1, 0) => '┛',
        _ => '━',
    }
}

fn draw_border(width: i32, height: i32, color: i32) {
    for x in 0..width {
        draw_char(x, 0, if x == 0 { '┏' } else if x == width - 1 { '┓' } else { '━' }, color);
        draw_char(x, height - 1, if x == 0 { '┗' } else if x == width - 1 { '┛' } else { '━' }, color);
    }

    for y in 1..height - 1 {
        draw_char(0, y, '┃', color);
        draw_char(width - 1, y, '┃', color);
    }
}

fn draw_area(width: i32, height: i32, color: i32) {
    for x in 1..width - 1 {
        for y in 1..height - 1 {
            draw_char(x, y, '+', color);
        }
    }
}

fn help() {
    println!("snake-rs");
    println!();
    println!("Usage:");
    println!("  snake-rs [options]");
    println!();
    println!("Options:");
    println!("  -h, --help        Show this help and exit");
    println!("  --color <n>       Set snake color (ANSI)");
    println!();
    println!("Controls:");
    println!("  W / UP_ARROW      Up");
    println!("  A / LEFT_ARROW    Left");
    println!("  S / DOWN_ARROW    Down");
    println!("  D / RIGHT_ARROW   Right");
    println!("  Q                 Quit");
}

fn main() {
    const UP: i32 = 0;
    const RIGHT: i32 = 1;
    const DOWN: i32 = 2;
    const LEFT: i32 = 3;
    const RED: i32 = 31;
    const GREEN: i32 = 32;
    const YELLOW: i32 = 33;
    const BLUE: i32 = 34;
    const GRAY: i32 = 90;
    const NEUTRAL: i32 = 0;
    let mut COLOR: i32 = NEUTRAL;
    let mut FRUIT_ON_FIELD: bool = false;
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        help();
        return;
    }

    match args[1].as_str() {
        "--color" => {
            let color = args[2].parse::<i32>().unwrap_or_else(|_| {
                help();
                std::process::exit(1);
            });

            let code = match color {
                31 => RED,
                32 => GREEN,
                33 => YELLOW,
                34 => BLUE,
                _ => {
                    help();
                    return;
                }
            };
            COLOR = code;
        }
        _ => {
            help();
            return;
        },
    }

    let mut player_dir: i32;
    let mut head_x: i32;
    let mut head_y: i32;

    let mut body: Vec<(i32, i32, i32)> = Vec::new();
    let mut snake_length: usize = 20;

    initialize_terminal();
    let (grid_width, grid_height) = get_terminal_size();

    head_x = grid_width / 2;
    head_y = grid_height / 2;
    player_dir = RIGHT;
    let mut current_fruit: (i32, i32) = (0, 0);

    draw_border(grid_width, grid_height, COLOR);

    loop {
        sleep_for_frame_time();

        draw_area(grid_width, grid_height, GRAY);

        if FRUIT_ON_FIELD {
            draw_char(current_fruit.0, current_fruit.1, 'o', COLOR);
        }

        if let Some(key) = read_key() {
            let next_dir = match key.as_str() {
                "w" | "UP" => UP,
                "d" | "RIGHT" => RIGHT,
                "s" | "DOWN" => DOWN,
                "a" | "LEFT" => LEFT,
                "q" => break,
                _ => player_dir,
            };

            if (next_dir + 2) % 4 != player_dir {
                player_dir = next_dir;
            }
        }


        match player_dir {
            UP    => head_y -= 1,
            RIGHT => head_x += 1,
            DOWN  => head_y += 1,
            LEFT  => head_x -= 1,
            _ => {}
        }

        if head_x < 1 {
            head_x = grid_width - 2;
        } else if head_x > grid_width - 2 {
            head_x = 1;
        }
        
        if head_y < 1 {
            head_y = grid_height - 2;
        } else if head_y > grid_height - 2 {
            head_y = 1;
        }

        body.push((head_x, head_y, player_dir));

        if body.len() > snake_length {
            body.remove(0);
        }

        if !(FRUIT_ON_FIELD) {
            let body_usize: Vec<(usize, usize, i32)> = body
                .iter()
                .map(|&(x, y, dir)| (x as usize, y as usize, dir))
                .collect();

            if let Some((fruit_x, fruit_y)) = place_random(&body_usize, (grid_width - 1).try_into().unwrap(), (grid_height - 1).try_into().unwrap()) {
                current_fruit = (fruit_x.try_into().unwrap(), fruit_y.try_into().unwrap());
                FRUIT_ON_FIELD = true;
            }
        }

        for i in 0..body.len().saturating_sub(1) {
            let (x, y, dir) = body[i];
            if x == current_fruit.0 && y == current_fruit.1 {
                FRUIT_ON_FIELD = false;
                snake_length += 1;
            }
            let next_dir = if i + 1 < body.len() {
                body[i + 1].2
            } else {
                dir
            };
            let pipe_char = get_pipe_char(dir, next_dir);
            draw_char(x, y, pipe_char, COLOR);
        }

        flush_stdout();
    }

    restore_terminal();
}