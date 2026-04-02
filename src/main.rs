use std::{
    io::{IsTerminal, Read},
    os::{
        fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd},
        unix::process::{CommandExt, ExitStatusExt},
    },
    path::Path,
};
mod terminal;

/// Run a process under a terminal emulator and snapshot the terminal when the process exits.
#[derive(Debug, clap::Parser)]
pub struct Args {
    /// Number of columns in the terminal emulator.
    #[arg(short, long, default_value = "80")]
    cols: u16,

    /// Number of rows in the terminal emulator.
    #[arg(short, long, default_value = "24")]
    rows: u16,

    /// Output path. Use `-` for stdout.
    #[arg(short, long, default_value = "-")]
    output: std::path::PathBuf,

    /// Optional timeout to kill the child process after
    #[arg(short, long)]
    timeout: Option<f32>,

    /// The command to run under.
    #[arg(trailing_var_arg = true, required = true)]
    command: Vec<String>,
}

struct Pty {
    master: Option<OwnedFd>,
    slave: Option<OwnedFd>,
}

fn main() {
    let args = <Args as clap::Parser>::parse();

    // Create a PTY
    let mut pty = Pty::open(args.rows, args.cols).expect("failed to open pty");

    // Create the child process.
    let mut command = std::process::Command::new(&args.command[0]);
    if args.command.len() > 1 {
        command.args(&args.command[1..]);
    }

    // Setup stdio.
    let slave = pty.slave.take().unwrap();
    if std::io::stdin().is_terminal() {
        let io = unsafe { std::process::Stdio::from_raw_fd(libc::dup(slave.as_raw_fd())) };
        command.stdin(io);
    }
    if std::io::stdout().is_terminal() {
        let io = unsafe { std::process::Stdio::from_raw_fd(libc::dup(slave.as_raw_fd())) };
        command.stdout(io);
    }
    if std::io::stderr().is_terminal() {
        let io = unsafe { std::process::Stdio::from_raw_fd(libc::dup(slave.as_raw_fd())) };
        command.stderr(io);
    }
    let slave = slave.into_raw_fd();
    unsafe {
        command.pre_exec(move || {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::ioctl(slave, libc::TIOCSCTTY, 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            libc::close(slave);
            Ok(())
        });
    }

    // Spawn the child process
    let mut child = command.spawn().expect("failed to spawn child process");
    unsafe { libc::close(slave) };
    drop(command);

    // Spawn the timeout thread if necessary
    if let Some(timeout) = args.timeout {
        let pid = child.id();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs_f32(timeout));
            unsafe { libc::kill(pid.try_into().unwrap(), libc::SIGKILL) };
        });
    }

    // Spawn the io/terminal thread.
    let io_thread = std::thread::spawn({
        let output = args.output;
        let options = terminal::Options {
            cols: args.cols,
            rows: args.rows,
            max_scrollback: 10_000,
        };
        move || {
            // Create a terminal emulator.
            let mut terminal = terminal::Terminal::new(options).expect("failed to create terminal");
            let mut buf = Vec::with_capacity(4096);
            buf.resize_with(buf.capacity(), || 0u8);
            loop {
                match pty.read(&mut buf) {
                    Ok(0) => break,
                    Ok(len) => terminal.write(&buf[0..len]),
                    Err(e) if e.raw_os_error() == Some(libc::EIO) => break,
                    Err(e) => panic!("failed to read pty: {e}"),
                }
            }

            // Create output writer.
            let mut output: Box<dyn std::io::Write> = if output == Path::new("-") {
                Box::new(std::io::stdout())
            } else {
                let file = std::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&output)
                    .expect("failed to open output file");
                Box::new(file)
            };

            // Render the snapshot.
            let snapshot = terminal.snapshot().expect("failed to create snapshot");
            for row in snapshot.rows().expect("failed to get rows") {
                let cells = row.expect("failed to get cells");
                for cell in cells {
                    let cell = cell.expect("failed to get cell");
                    let text = cell.text();
                    if text.is_empty() {
                        output.write_all(b" ").expect("failed to write output");
                    } else {
                        output
                            .write_all(text.as_bytes())
                            .expect("failed to write output");
                    }
                }
                output.write_all(b"\n").expect("failed to write output");
            }
        }
    });

    let exit = child.wait().expect("failed to wait for child");
    io_thread.join().expect("the i/o thread failed");

    // Exit with the same status as the child process.
    if let Some(code) = exit.code() {
        std::process::exit(code);
    }
    if let Some(signal) = exit.signal() {
        std::process::exit(signal + 128);
    }
    std::process::exit(111);
}

impl Pty {
    fn open(rows: u16, cols: u16) -> std::io::Result<Self> {
        unsafe {
            let mut master = -1;
            let mut slave = -1;
            let winsize = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            let ret = libc::openpty(
                &raw mut master,
                &raw mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &raw const winsize,
            );
            if ret < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(Self {
                master: Some(OwnedFd::from_raw_fd(master)),
                slave: Some(OwnedFd::from_raw_fd(slave)),
            })
        }
    }
}

impl std::io::Read for Pty {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        unsafe {
            let fd = self.master.as_ref().unwrap();
            let amt = libc::read(fd.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len());
            if amt < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(amt.try_into().unwrap())
        }
    }
}
