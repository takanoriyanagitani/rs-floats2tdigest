use std::io;

use io::BufRead;

use io::Write;

use tdigest::Centroid;
use tdigest::TDigest;

pub struct Digest(pub TDigest);

impl Digest {
    pub fn mean(&self) -> Option<f64> {
        self.0.mean()
    }
    pub fn max(&self) -> Option<f64> {
        self.0.max()
    }
    pub fn min(&self) -> Option<f64> {
        self.0.min()
    }

    pub fn sum(&self) -> f64 {
        self.0.sum()
    }
    pub fn count(&self) -> f64 {
        self.0.count()
    }

    pub fn max_size(&self) -> usize {
        self.0.max_size()
    }

    pub fn centroids(&self) -> &[Centroid] {
        self.0.centroids()
    }
}

impl core::fmt::Display for Digest {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let td = &self.0;
        write!(
            f,
            "Digest {{ mean: {:?}, max: {:?}, min: {:?}, sum: {}, count: {}, centroids: {} centroids }}",
            td.mean(),
            td.max(),
            td.min(),
            td.sum(),
            td.count(),
            td.centroids().len()
        )
    }
}

pub struct Cntrd(pub Centroid);

impl Cntrd {
    pub fn mean(&self) -> f64 {
        self.0.mean()
    }
    pub fn weight(&self) -> f64 {
        self.0.weight()
    }
}

impl core::fmt::Display for Cntrd {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        let c = &self.0;
        write!(
            f,
            "Centroid {{ mean: {}, weight: {} }}",
            c.mean(),
            c.weight()
        )
    }
}

impl Digest {
    pub fn from_doubles<I>(doubles: I, max_size: usize) -> Result<Self, io::Error>
    where
        I: Iterator<Item = Result<f64, io::Error>>,
    {
        let mut td: TDigest = TDigest::new_with_size(max_size);
        for rd in doubles {
            let d: f64 = rd?;
            td.push(d);
        }
        td.flush();
        Ok(Self(td))
    }
}

pub fn brdr2doubles_raw_le<R>(mut brdr: R) -> impl Iterator<Item = Result<f64, io::Error>>
where
    R: BufRead,
{
    let mut buf: [u8; 8] = [0; 8];
    std::iter::from_fn(move || {
        let rslt = brdr.read_exact(&mut buf);
        match rslt {
            Ok(_) => {
                let d: f64 = f64::from_le_bytes(buf);
                Some(Ok(d))
            }
            Err(e) => match e.kind() {
                io::ErrorKind::UnexpectedEof => None,
                _ => Some(Err(e)),
            },
        }
    })
}

pub fn brdr2doubles_strings<R>(brdr: R) -> impl Iterator<Item = Result<f64, io::Error>>
where
    R: BufRead,
{
    brdr.lines()
        .map(|r| r.and_then(|s| str::parse(&s).map_err(io::Error::other)))
}

pub trait DigestSink {
    fn consume(&mut self, d: TDigest) -> Result<(), io::Error>;

    fn consume_source<S>(&mut self, src: S) -> Result<(), io::Error>
    where
        S: Fn() -> Result<TDigest, io::Error>,
    {
        let td: TDigest = src()?;
        self.consume(td)
    }

    fn consume_doubles<I>(&mut self, doubles: I, max_size: usize) -> Result<(), io::Error>
    where
        I: Iterator<Item = Result<f64, io::Error>>,
    {
        let td: TDigest = Digest::from_doubles(doubles, max_size)?.0;
        self.consume(td)
    }
}

impl<F> DigestSink for F
where
    F: FnMut(TDigest) -> Result<(), io::Error>,
{
    fn consume(&mut self, d: TDigest) -> Result<(), io::Error> {
        self(d)
    }
}

pub fn digest2console(td: TDigest) -> Result<(), io::Error> {
    let d: Digest = Digest(td);
    println!("{d}");
    Ok(())
}

pub fn wtr2digestsink4json<W>(mut wtr: W) -> impl DigestSink
where
    W: Write,
{
    move |td: TDigest| {
        serde_json::to_writer(&mut wtr, &td)?;
        wtr.flush()
    }
}

pub fn digest_sink_default() -> impl DigestSink {
    digest2console
}

#[repr(usize)]
#[derive(Default, Debug, Clone, Copy)]
pub enum MaxSizePreset {
    Low = 50,
    #[default]
    Standard = 100,
    High = 200,
    Ultra = 500,
}

#[derive(Debug, Clone, Copy)]
pub enum MaxSize {
    Preset(MaxSizePreset),
    Custom(usize),
}

impl TryFrom<usize> for MaxSize {
    type Error = io::Error;
    fn try_from(u: usize) -> Result<Self, Self::Error> {
        if 0 == u {
            return Err(io::Error::other("0 size got"));
        }

        const LOW: usize = MaxSizePreset::Low as usize;
        const STANDARD: usize = MaxSizePreset::Standard as usize;
        const HIGH: usize = MaxSizePreset::High as usize;
        const ULTRA: usize = MaxSizePreset::Ultra as usize;

        match u {
            LOW => Ok(Self::Preset(MaxSizePreset::Low)),
            STANDARD => Ok(Self::Preset(MaxSizePreset::Standard)),
            HIGH => Ok(Self::Preset(MaxSizePreset::High)),
            ULTRA => Ok(Self::Preset(MaxSizePreset::Ultra)),
            _ => Ok(Self::Custom(u)),
        }
    }
}

impl Default for MaxSize {
    fn default() -> Self {
        Self::Preset(MaxSizePreset::default())
    }
}

impl MaxSize {
    pub fn as_usize(self) -> usize {
        match self {
            Self::Preset(p) => p as usize,
            Self::Custom(u) => u,
        }
    }
}

impl MaxSize {
    pub fn stdin2digest_raw_le(self) -> impl Fn() -> Result<TDigest, io::Error> {
        move || {
            Digest::from_doubles(brdr2doubles_raw_le(io::stdin().lock()), self.as_usize())
                .map(|d| d.0)
        }
    }
}

impl MaxSize {
    pub fn stdin2digest_strings(self) -> impl Fn() -> Result<TDigest, io::Error> {
        move || {
            Digest::from_doubles(brdr2doubles_strings(io::stdin().lock()), self.as_usize())
                .map(|d| d.0)
        }
    }
}

impl MaxSize {
    pub fn stdin2raw_le2doubles2digest2sink<S>(self, sink: &mut S) -> Result<(), io::Error>
    where
        S: DigestSink,
    {
        sink.consume_source(self.stdin2digest_raw_le())
    }
}

impl MaxSize {
    pub fn stdin2strings2doubles2digest2sink<S>(self, sink: &mut S) -> Result<(), io::Error>
    where
        S: DigestSink,
    {
        sink.consume_source(self.stdin2digest_strings())
    }
}

impl MaxSize {
    pub fn stdin2strings2doubles2digest2sink_default(self) -> Result<(), io::Error> {
        self.stdin2strings2doubles2digest2sink(&mut digest2console)
    }
}

impl MaxSize {
    pub fn stdin2raw_le2doubles2digest2sink_default(self) -> Result<(), io::Error> {
        self.stdin2raw_le2doubles2digest2sink(&mut digest2console)
    }
}

impl MaxSize {
    pub fn stdin2raw_le2doubles2digest2sink_json(self) -> Result<(), io::Error> {
        let mut digest2json = wtr2digestsink4json(io::stdout().lock());
        self.stdin2raw_le2doubles2digest2sink(&mut digest2json)
    }
}

impl MaxSize {
    pub fn stdin2strings2doubles2digest2sink_json(self) -> Result<(), io::Error> {
        let mut digest2json = wtr2digestsink4json(io::stdout().lock());
        self.stdin2strings2doubles2digest2sink(&mut digest2json)
    }
}
