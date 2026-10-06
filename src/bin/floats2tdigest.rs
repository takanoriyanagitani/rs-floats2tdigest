use std::io;
use std::process::ExitCode;

use rs_floats2tdigest::MaxSize;

#[repr(u8)]
#[derive(Default, Debug, Clone, Copy)]
enum InputMode {
    #[default]
    String = 1,
    RawLittleEndian = 2,
}

struct Config {
    pub imode: InputMode,
    pub msize: MaxSize,
}

impl Config {
    pub fn stdin2doubles2digest2sink_json(self) -> Result<(), io::Error> {
        match self.imode {
            InputMode::String => self.msize.stdin2strings2doubles2digest2sink_json(),
            InputMode::RawLittleEndian => self.msize.stdin2raw_le2doubles2digest2sink_json(),
        }
    }
}

fn io_envkey2val(key: &'static str) -> impl Fn() -> String {
    move || std::env::var(key).unwrap_or_default()
}

fn str2usize(s: String) -> Result<usize, io::Error> {
    str::parse(&s).map_err(io::Error::other)
}

fn str2max(s: String) -> Result<MaxSize, io::Error> {
    if s.is_empty() {
        return Ok(MaxSize::default());
    }

    let u: usize = str2usize(s).map_err(|_| io::Error::other("invalid max size ENV_MAX_SIZE"))?;
    u.try_into()
}

fn str2mode(s: String) -> Result<InputMode, io::Error> {
    match s.as_str() {
        "" => Ok(InputMode::default()),
        "strings" | "string" | "str" | "s" => Ok(InputMode::String),
        "le" | "lit" | "little" | "little_endian" => Ok(InputMode::RawLittleEndian),
        _ => Err(io::Error::other("invalid input mode ENV_INPUT_MODE")),
    }
}

fn io_bind<F, G, T, U>(f: F, g: G) -> impl Fn() -> Result<U, io::Error>
where
    F: Fn() -> T,
    G: Fn(T) -> Result<U, io::Error>,
{
    move || {
        let t: T = f();
        g(t)
    }
}

fn io_envkey2mode(key: &'static str) -> impl Fn() -> Result<InputMode, io::Error> {
    io_bind(io_envkey2val(key), str2mode)
}

fn io_envkey2max(key: &'static str) -> impl Fn() -> Result<MaxSize, io::Error> {
    io_bind(io_envkey2val(key), str2max)
}

fn io_max_size() -> impl Fn() -> Result<MaxSize, io::Error> {
    io_envkey2max("ENV_MAX_SIZE")
}

fn io_input_mode() -> impl Fn() -> Result<InputMode, io::Error> {
    io_envkey2mode("ENV_INPUT_MODE")
}

fn io_config() -> impl Fn() -> Result<Config, io::Error> {
    || {
        let imode: InputMode = io_input_mode()()?;
        let msize: MaxSize = io_max_size()()?;
        Ok(Config { imode, msize })
    }
}

fn io_main() -> impl Fn() -> Result<(), io::Error> {
    || {
        let cfg: Config = io_config()()?;
        cfg.stdin2doubles2digest2sink_json()
    }
}

fn sub() -> Result<(), io::Error> {
    io_main()()
}

fn main() -> ExitCode {
    sub().map(|_| ExitCode::SUCCESS).unwrap_or_else(|e| {
        eprintln!("{e}");

        ExitCode::FAILURE
    })
}
