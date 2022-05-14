use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::path::PathBuf;
use std::str::FromStr;

use clap::{ArgEnum, Args, Parser};
use harfbuzz_sys as ffi;
use once_cell::sync::OnceCell;

use crate::helper_cairo::HbFont;

const FONT_SIZE_UPEM: usize = 0x7FFFFFFF;
const FONT_SIZE_NONE: usize = 0;

fn version() -> &'static str {
    unsafe { CStr::from_ptr(ffi::hb_version_string()) }
        .to_str()
        .unwrap()
}

#[derive(Clone, Debug, ArgEnum)]
pub enum Direction {
    LTR,
    RTL,
    TTB,
    BTT,
}

impl FromStr for Direction {
    type Err = String;
    fn from_str(s: &str) -> Result<Direction, String> {
        match s {
            "ltr" => Ok(Direction::LTR),
            "rtl" => Ok(Direction::RTL),
            "ttb" => Ok(Direction::TTB),
            "btt" => Ok(Direction::BTT),
            _ => Err("ltr/rtl/ttb/btt".to_string()),
        }
    }
}

impl Direction {
    fn to_hb(&self) -> ffi::hb_direction_t {
        match self {
            Direction::LTR => ffi::HB_DIRECTION_LTR,
            Direction::RTL => ffi::HB_DIRECTION_RTL,
            Direction::TTB => ffi::HB_DIRECTION_TTB,
            Direction::BTT => ffi::HB_DIRECTION_BTT,
        }
    }
}

#[derive(Clone, Debug, ArgEnum)]
pub enum ClusterLevel {
    MonotoneGraphemes = 0, // ffi::HB_BUFFER_CLUSTER_LEVEL_MONOTONE_GRAPHEMES,
    MonotoneCharacters,    // ffi::HB_BUFFER_CLUSTER_LEVEL_MONOTONE_CHARACTERS,
    Characters,            // ffi::HB_BUFFER_CLUSTER_LEVEL_CHARACTERS,
}

impl FromStr for ClusterLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(ClusterLevel::MonotoneGraphemes),
            "1" => Ok(ClusterLevel::MonotoneCharacters),
            "2" => Ok(ClusterLevel::Characters),
            _ => Err("0/1/2".to_string()),
        }
    }
}

impl ClusterLevel {
    fn to_hb(&self) -> ffi::hb_buffer_cluster_level_t {
        match self {
            ClusterLevel::MonotoneGraphemes => ffi::HB_BUFFER_CLUSTER_LEVEL_MONOTONE_GRAPHEMES,
            ClusterLevel::MonotoneCharacters => ffi::HB_BUFFER_CLUSTER_LEVEL_MONOTONE_CHARACTERS,
            ClusterLevel::Characters => ffi::HB_BUFFER_CLUSTER_LEVEL_CHARACTERS,
        }
    }
}

#[derive(Debug, Parser)]
#[clap(author, version=version(), about, long_about = None)]
pub struct Options<const B: usize = 0> {
    #[clap(flatten, next_help_heading = "Font options")]
    pub font_opts: FontOptions,

    #[clap(flatten, next_help_heading = "Shape options")]
    pub text: TextOptions,

    #[clap(flatten, next_help_heading = "Shape options")]
    pub shape: ShapeOptions,

    #[clap(flatten, next_help_heading = "Features options")]
    pub features: FeatureOptions,

    #[clap(flatten, next_help_heading = "Output destination & format options")]
    pub output: OutputAndFormatOptions,

    #[clap(flatten, next_help_heading = "View options")]
    pub view: ViewOptions,
}

#[derive(Clone, Copy, Debug)]
pub struct FontSize {
    pub x: f32,
    pub y: f32,
}

impl Default for FontSize {
    fn default() -> Self {
        Self {
            x: unsafe { DEFAULT_FONT_SIZE } as f32,
            y: unsafe { DEFAULT_FONT_SIZE } as f32,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FontPpem {
    pub x: u32,
    pub y: u32,
}

impl Default for FontPpem {
    fn default() -> Self {
        FontPpem { x: 0, y: 0 }
    }
}

extern "C" {
    #[no_mangle]
    static SUBPIXEL_BITS: i32;
    #[no_mangle]
    static DEFAULT_FONT_SIZE: usize;
}

type FnSetFontFuncs = unsafe extern "C" fn(*mut ffi::hb_font_t);
struct SetFontFuncs {
    name: &'static str,
    fnptr: FnSetFontFuncs,
}

static SUPPORTED_FONT_FUNCS: &'static [SetFontFuncs] = &[
    SetFontFuncs {
        name: "ot",
        fnptr: ffi::hb_ot_font_set_funcs,
    },
    #[cfg(features = "freetype")]
    SetFontFuncs {
        name: "ft",
        fnptr: ffi::hb_ft_font_set_funcs,
    },
];

#[derive(Debug, Args)]
pub struct FontOptions {
    /// Set font file-name
    #[clap(long)]
    pub font_file: String,

    /// Set face index (default: 0)
    #[clap(long, default_value_t = 0)]
    pub face_index: usize,

    /// Font size, 1/2 integers or 'upem' (default: 256)
    #[clap(long, parse(try_from_str = parse_font_size))]
    pub font_size: Option<FontSize>,

    /// Set x,y pixels per EM, 1/2 integers (default: 0; disabled)
    #[clap(long, parse(try_from_str = parse_font_ppem))]
    pub ppem: Option<FontPpem>,

    /// Set font point-size (default: 0; disabled)
    #[clap(long, default_value_t = 0.)]
    pub ptem: f32,

    /// Set synthetic slant (default: 0)
    /// slant ratio; eg. 0.2
    #[clap(long, default_value_t = 0.)]
    pub slant: f32,

    /// Set font functions implementation to use (default: ft)
    ///
    /// Supported font function implementations are: ft/ot
    #[clap(long)]
    pub font_funcs: Option<String>,

    /// Set FreeType load-flags (default: 2)
    #[clap(long, default_value_t = 2)]
    pub ft_load_flags: usize,

    /// Font variations
    ///
    /// Variations are set globally. The format for specifying variation settings
    /// follows.  All valid CSS font-variation-settings values other than 'normal'
    /// and 'inherited' are also accepted, though, not documented below.
    ///
    /// The format is a tag, optionally followed by an equals sign, followed by a
    /// number. For example:
    ///
    ///   "wght=500"
    ///   "slnt=-7.5"
    #[clap(long, verbatim_doc_comment)]
    pub variations: Vec<String>,

    #[clap(skip)]
    font: RefCell<Option<FontCache>>,
}

#[derive(Clone, Debug)]
struct FontCache {
    blob: *mut ffi::hb_blob_t,
    face: *mut ffi::hb_face_t,
    font: *mut ffi::hb_font_t,
}

impl Default for FontCache {
    fn default() -> Self {
        FontCache {
            blob: std::ptr::null_mut(),
            face: std::ptr::null_mut(),
            font: std::ptr::null_mut(),
        }
    }
}

impl Drop for FontCache {
    fn drop(&mut self) {
        unsafe {
            ffi::hb_font_destroy(self.font);
            ffi::hb_face_destroy(self.face);
            ffi::hb_blob_destroy(self.blob);
        }
    }
}

impl FontOptions {
    pub fn font(&self) -> *mut ffi::hb_font_t {
        self.font.borrow().as_ref().unwrap().font
    }

    fn load_font(&mut self) {
        assert!(
            PathBuf::from(&self.font_file).exists(),
            "{}: Failed reading file",
            self.font_file
        );
        let cache = unsafe {
            let cstr = CString::new(self.font_file.clone()).unwrap();
            let blob = ffi::hb_blob_create_from_file_or_fail(cstr.as_ptr());
            let face = ffi::hb_face_create(blob, self.face_index as _);
            let font = ffi::hb_font_create(face);

            let font_size = self.font_size.unwrap_or_default();
            let font_size_x = if font_size.x == FONT_SIZE_UPEM as f32 {
                ffi::hb_face_get_upem(face) as f32
            } else {
                font_size.x
            };
            let font_size_y = if font_size.y == FONT_SIZE_UPEM as f32 {
                ffi::hb_face_get_upem(face) as f32
            } else {
                font_size.y
            };

            let _ = self.font_size.replace(FontSize {
                x: font_size_x,
                y: font_size_y,
            });

            let ppem = self.ppem.unwrap_or_default();
            ffi::hb_font_set_ppem(font, ppem.x, ppem.y);
            ffi::hb_font_set_ptem(font, self.ptem);

            ffi::hb_font_set_synthetic_slant(font, self.slant);

            let subpixel_bits = SUBPIXEL_BITS;
            let scale_x: f32 = libm::scalbnf(font_size_x, subpixel_bits);
            let scale_y: f32 = libm::scalbnf(font_size_y, subpixel_bits);
            ffi::hb_font_set_scale(font, scale_x as i32, scale_y as i32);

            let variations: Vec<_> = self
                .variations
                .iter()
                .map(|var| {
                    let mut variation: MaybeUninit<ffi::hb_variation_t> = MaybeUninit::zeroed();
                    let is_ok = ffi::hb_variation_from_string(
                        var.as_ptr() as _,
                        var.len() as _,
                        variation.as_mut_ptr(),
                    );
                    assert_eq!(is_ok, 1);
                    variation.assume_init()
                })
                .collect();
            ffi::hb_font_set_variations(font, variations.as_ptr(), self.variations.len() as _);

            let set_font_funcs = if let Some(ref font_funcs_name) = self.font_funcs {
                let mut set_font_funcs: Option<FnSetFontFuncs> = None;
                for font_funcs in SUPPORTED_FONT_FUNCS.iter() {
                    if font_funcs.name == font_funcs_name {
                        set_font_funcs.replace(font_funcs.fnptr);
                        break;
                    }
                }
                if set_font_funcs.is_none() {
                    panic!("")
                }
                set_font_funcs.unwrap()
            } else {
                ffi::hb_ot_font_set_funcs
            };
            set_font_funcs(font);

            #[cfg(features = "freetype")]
            ffi::hb_ft_font_set_load_flags(font, self.ft_load_flags);

            // TODO: sub_font

            FontCache { blob, face, font }
        };

        self.font.replace(Some(cache));
    }
}

pub fn parse_font_size(arg: &str) -> anyhow::Result<FontSize> {
    if arg == "upem" {
        return Ok(FontSize {
            x: FONT_SIZE_UPEM as _,
            y: FONT_SIZE_UPEM as _,
        });
    }
    let arg: Vec<_> = arg
        .split(|c| c == ' ' || c == ',')
        .map(|v| v.trim_matches(|c| c == ' ' || c == ','))
        .filter(|v| !v.is_empty())
        .collect();
    if arg.len() == 1 {
        let size: f32 = arg[0].parse()?;
        Ok(FontSize { x: size, y: size })
    } else if arg.len() == 2 {
        let size_x: f32 = arg[0].parse()?;
        let size_y: f32 = arg[1].parse()?;
        Ok(FontSize {
            x: size_x,
            y: size_y,
        })
    } else {
        anyhow::bail!("font-size argument should be one or two space-separated numbers")
    }
}

pub fn parse_font_ppem(arg: &str) -> anyhow::Result<FontPpem> {
    let arg: Vec<_> = arg
        .split(|c| c == ' ' || c == ',')
        .map(|v| v.trim_matches(|c| c == ' ' || c == ','))
        .filter(|v| !v.is_empty())
        .collect();
    if arg.len() == 1 {
        let size: u32 = arg[0].parse()?;
        Ok(FontPpem { x: size, y: size })
    } else if arg.len() == 2 {
        let size_x: u32 = arg[0].parse()?;
        let size_y: u32 = arg[1].parse()?;
        Ok(FontPpem {
            x: size_x,
            y: size_y,
        })
    } else {
        anyhow::bail!("font-ppem argument should be one or two space-separated numbers")
    }
}

pub trait FontOpts {
    fn font(&self) -> HbFont;
    fn load_font(&mut self);
}

impl FontOpts for Options {
    fn font(&self) -> HbFont {
        unsafe { HbFont::from_raw(ffi::hb_font_reference(self.font_opts.font())) }
    }

    fn load_font(&mut self) {
        self.font_opts.load_font();
    }
}

#[derive(Debug, Args)]
pub struct TextOptions {
    /// Set input text
    #[clap(long)]
    pub text: Option<String>,

    /// Set input text file-name
    ///
    /// If no text is provided, standard input is used for input.
    #[clap(long)]
    pub text_file: Option<PathBuf>,

    /// Set input Unicode codepoints, hex numbers
    #[clap(short = 'u', long)]
    pub unicodes: Vec<u32>,

    /// Set text context before each line
    #[clap(long)]
    pub text_before: Option<String>,

    /// Set text context after each line
    #[clap(long)]
    pub text_after: Option<String>,

    #[clap(skip = RefCell::new(None))]
    lines: RefCell<Option<std::str::Lines<'static>>>,
}

impl TextOptions {
    pub fn read(&mut self) {
        static TEXT: OnceCell<String> = OnceCell::new();
        let text = TEXT
            .get_or_try_init(|| {
                if let Some(ref path) = self.text_file {
                    Ok(std::fs::read_to_string(path)
                        .map_err(|err| anyhow::anyhow!("Can not open '{}'", path.display()))?)
                } else if !self.unicodes.is_empty() {
                    let s = self
                        .unicodes
                        .iter()
                        .map(|u| char::try_from(*u).unwrap())
                        .collect();
                    Ok(s)
                } else if let Some(ref text) = self.text {
                    Ok(text.to_string())
                } else {
                    anyhow::bail!("None of text or unicodes or text-file provided.");
                }
            })
            .unwrap();
        self.lines.replace(Some(text.lines()));
    }

    pub fn readline(&self) -> Option<&'static str> {
        assert!(self.lines.borrow().is_some());
        self.lines
            .borrow_mut()
            .as_mut()
            .map(|lines| lines.next())
            .unwrap()
    }
}

pub trait TextOpts {
    fn text_before(&self) -> Option<&str>;
    fn text_after(&self) -> Option<&str>;

    fn read(&mut self);
    fn readline(&self) -> Option<&'static str>;
}

impl TextOpts for Options {
    fn text_before(&self) -> Option<&str> {
        self.text.text_before.as_ref().map(|v| v.as_str())
    }
    fn text_after(&self) -> Option<&str> {
        self.text.text_after.as_ref().map(|v| v.as_str())
    }

    fn read(&mut self) {
        self.text.read();
    }
    fn readline(&self) -> Option<&'static str> {
        self.text.readline()
    }
}

#[derive(Debug, Args)]
pub struct ShapeOptions {
    /// List available shapers and quit
    #[clap(long)]
    pub list_shapers: bool,

    /// Set shapers to try
    #[clap(long, parse(try_from_str = parse_shapers))]
    pub shapers: Vec<std::ffi::CString>,

    /// Set text direction, one of ltr/rtl/ttb/btt (default: auto)
    #[clap(long)]
    pub direction: Option<Direction>,

    /// Set text language (default: $LANG)
    #[clap(long)]
    pub language: Option<String>,

    /// Set text script, ISO-15924 tag (default: auto)
    #[clap(long)]
    pub script: Option<String>,

    #[clap(long)]
    /// Treat text as beginning-of-paragraph
    pub bot: bool,

    #[clap(long)]
    /// Treat text as end-of-paragraph
    pub eot: bool,

    #[clap(long)]
    /// Preserve Default-Ignorable characters
    pub preserve_default_ignorables: bool,

    #[clap(long)]
    /// Remove Default-Ignorable characters
    pub remove_default_ignorables: bool,

    #[clap(long)]
    /// Glyph value to replace Default-Ignorables with
    pub invisible_glyph: Option<u32>,

    #[clap(long)]
    /// Use UTF8 byte indices, not char indices
    pub utf8_clusters: bool,

    #[clap(arg_enum, long, default_value_t = ClusterLevel::MonotoneGraphemes)]
    /// Cluster merging level, 0/1/2 (default: 0)
    pub cluster_level: ClusterLevel,

    #[clap(long)]
    /// Rearrange glyph clusters in nominal order
    pub normalize_glyphs: bool,

    #[clap(long)]
    /// Perform sanity checks on shaping results
    pub verify: bool,

    #[clap(short = 'n', long, default_value = "1")]
    /// Run shaper N times (default: 1)
    pub num_iterations: usize,
}

fn parse_shapers(arg: &str) -> anyhow::Result<std::ffi::CString> {
    unsafe {
        let hb_shapers = ffi::hb_shape_list_shapers();
        while !(*hb_shapers).is_null() {
            let cstr = CStr::from_ptr(*hb_shapers);
            if arg == cstr.to_str().unwrap() {
                return Ok(cstr.to_owned());
            }
        }
    }
    anyhow::bail!("Unknown or unsupported shaper: {}", arg)
}

pub trait ShapeOpts {
    fn utf8_clusters(&self) -> bool;
    fn verify(&self) -> bool;
    fn num_iterations(&self) -> usize;

    unsafe fn populate_buffer(
        &self,
        buffer: *mut ffi::hb_buffer_t,
        text: &str,
        text_before: Option<&str>,
        text_after: Option<&str>,
    );
    unsafe fn shape(
        &self,
        font: *mut ffi::hb_font_t,
        buffer: *mut ffi::hb_buffer_t,
    ) -> anyhow::Result<bool>;
}

impl ShapeOpts for Options {
    fn utf8_clusters(&self) -> bool {
        self.shape.utf8_clusters
    }
    fn verify(&self) -> bool {
        self.shape.verify
    }
    fn num_iterations(&self) -> usize {
        self.shape.num_iterations
    }

    unsafe fn populate_buffer(
        &self,
        buffer: *mut ffi::hb_buffer_t,
        text: &str,
        text_before: Option<&str>,
        text_after: Option<&str>,
    ) {
        self.shape
            .populate_buffer(buffer, text, text_before, text_after)
    }

    unsafe fn shape(
        &self,
        font: *mut ffi::hb_font_t,
        buffer: *mut ffi::hb_buffer_t,
    ) -> anyhow::Result<bool> {
        let mut text_buffer: *mut ffi::hb_buffer_t = std::ptr::null_mut();
        if self.shape.verify {
            text_buffer = ffi::hb_buffer_create();
            ffi::hb_buffer_append(text_buffer, buffer, 0, u32::MAX);
        }

        let features: Vec<_> = self
            .features
            .features
            .iter()
            .map(|feat| {
                let mut feature = MaybeUninit::uninit();
                ffi::hb_feature_from_string(
                    feat.as_ptr() as _,
                    feat.len() as _,
                    feature.as_mut_ptr(),
                );
                feature.assume_init()
            })
            .collect();

        let shapers: Vec<*const std::os::raw::c_char> =
            self.shape.shapers.iter().map(|s| s.as_ptr()).collect();
        if ffi::hb_shape_full(
            font,
            buffer,
            features.as_ptr(),
            features.len() as _,
            shapers.as_ptr(),
        ) == 0
        {
            if !text_buffer.is_null() {
                ffi::hb_buffer_destroy(text_buffer);
            }
            anyhow::bail!("all shapers failed");
        }
        Ok(true)
    }
}

#[derive(Debug, Args)]
pub struct FeatureOptions {
    /// Font features
    ///
    /// Features can be enabled or disabled, either globally or limited to
    /// specific character ranges.  The format for specifying feature settings
    /// follows.  All valid CSS font-feature-settings values other than 'normal'
    /// and the global values are also accepted, though not documented below.
    /// CSS string escapes are not supported.
    /// The range indices refer to the positions between Unicode characters,
    /// unless the --utf8-clusters is provided, in which case range indices
    /// refer to UTF-8 byte indices. The position before the first character
    /// is always 0.
    ///
    /// The format is Python-esque.  Here is how it all works:  
    ///
    ///   Syntax:       Value:    Start:    End:  
    ///
    /// Setting value:  
    ///   "kern"        1         0         ∞         # Turn feature on   
    ///   "+kern"       1         0         ∞         # Turn feature on   
    ///   "-kern"       0         0         ∞         # Turn feature off
    ///   "kern=0"      0         0         ∞         # Turn feature off
    ///   "kern=1"      1         0         ∞         # Turn feature on
    ///   "aalt=2"      2         0         ∞         # Choose 2nd alternate
    ///
    /// Setting index:  
    ///   "kern[]"      1         0         ∞         # Turn feature on
    ///   "kern[:]"     1         0         ∞         # Turn feature on
    ///   "kern[5:]"    1         5         ∞         # Turn feature on, partial
    ///   "kern[:5]"    1         0         5         # Turn feature on, partial
    ///   "kern[3:5]"   1         3         5         # Turn feature on, range
    ///   "kern[3]"     1         3         3+1       # Turn feature on, single char
    ///
    /// Mixing it all:
    ///
    ///   "aalt[3:5]=2" 2         3         5         # Turn 2nd alternate on for range
    #[clap(long, verbatim_doc_comment)]
    pub features: Vec<String>,
}

#[derive(Copy, Clone, Debug, ArgEnum)]
pub enum OutputFormat {
    ANSI,
    PNG,
    SVG,
    PDF,
    PS,
    EPS,
}

impl FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ansi" => Ok(OutputFormat::ANSI),
            "png" => Ok(OutputFormat::PNG),
            "svg" => Ok(OutputFormat::SVG),
            "pdf" => Ok(OutputFormat::PDF),
            "ps" => Ok(OutputFormat::PS),
            "eps" => Ok(OutputFormat::EPS),
            _ => Err("ansi/png/svg/pdf/ps/eps".to_string()),
        }
    }
}

#[derive(Args)]
pub struct OutputAndFormatOptions {
    /// Set output file-name (default: stdout)
    #[clap(long)]
    pub output_file: Option<String>,

    #[clap(long, verbatim_doc_comment)]
    /// Set output format
    ///
    /// Supported output formats are: ansi/png/svg/pdf/ps/eps
    pub output_format: Option<OutputFormat>,

    #[clap(skip)]
    pub output_fp: Option<Box<dyn std::io::Write>>,
}

impl std::fmt::Debug for OutputAndFormatOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputAndFormatOptions")
            .field("output_file", &self.output_file)
            .field("output_format", &self.output_format)
            .finish()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct FontExtents {
    pub ascent: f64,
    pub descent: f64,
    pub line_gap: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct ViewMargin {
    pub t: f64,
    pub r: f64,
    pub b: f64,
    pub l: f64,
}

impl Default for ViewMargin {
    fn default() -> Self {
        ViewMargin {
            t: 16.,
            l: 16.,
            r: 16.,
            b: 16.,
        }
    }
}

impl std::fmt::Display for ViewMargin {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str(&format!("{},{},{},{}", self.t, self.r, self.b, self.l))
    }
}

#[derive(Debug, Args)]
pub struct ViewOptions {
    /// Annotate output rendering
    #[clap(long)]
    pub annotate: bool,

    /// Set background color, rrggbb/rrggbbaa (default: #FFFFFF)
    #[clap(long, default_value = "#FFFFFF")]
    pub background: String,

    /// Set foreground color, rrggbb/rrggbbaa (default: #000000)
    #[clap(long, default_value = "#000000")]
    pub foreground: String,

    /// Set space between lines (default: 0)
    #[clap(long, default_value_t = 0.)]
    pub line_space: f64,

    /// Set font ascent/descent/line-gap (default: auto)
    #[clap(long, parse(try_from_str = parse_font_extents))]
    pub font_extents: Option<FontExtents>,

    /// Margin around output (default: 16)
    #[clap(long, parse(try_from_str = parse_margin))]
    pub margin: Option<ViewMargin>,
}

fn parse_font_extents(arg: &str) -> anyhow::Result<FontExtents> {
    let arg: Vec<_> = arg
        .split(|c| c == ' ' || c == ',')
        .map(|v| v.trim_matches(|c| c == ' ' || c == ','))
        .filter(|v| !v.is_empty())
        .collect();
    let mut extents = FontExtents {
        ascent: 0.,
        descent: 0.,
        line_gap: 0.,
    };
    match arg.len() {
        1 => extents.ascent = arg[0].parse()?,
        2 => {
            extents.ascent = arg[0].parse()?;
            extents.descent = arg[1].parse()?;
        }
        3 => {
            extents.ascent = arg[0].parse()?;
            extents.descent = arg[1].parse()?;
            extents.line_gap = arg[2].parse()?;
        }
        _ => anyhow::bail!("font-extents argument should be one to three space-separated numbers"),
    }
    Ok(extents)
}

fn parse_margin(arg: &str) -> anyhow::Result<ViewMargin> {
    let arg: Vec<_> = arg
        .split(|c| c == ' ' || c == ',')
        .map(|v| v.trim_matches(|c| c == ' ' || c == ','))
        .filter(|v| !v.is_empty())
        .collect();
    let mut m = ViewMargin::default();
    match arg.len() {
        1 => {
            m.t = arg[0].parse()?;
            m.r = m.t;
        }
        2 => {
            m.t = arg[0].parse()?;
            m.r = arg[1].parse()?;
            m.b = m.t;
        }
        3 => {
            m.t = arg[0].parse()?;
            m.r = arg[1].parse()?;
            m.b = arg[2].parse()?;
            m.l = m.r;
        }
        4 => {
            m.t = arg[0].parse()?;
            m.r = arg[1].parse()?;
            m.b = arg[2].parse()?;
            m.l = arg[3].parse()?;
        }
        _ => {
            anyhow::bail!("margin argument must be one to four space-separated numbers");
        }
    }
    Ok(m)
}

impl ShapeOptions {
    pub unsafe fn setup_buffer(&self, buffer: *mut ffi::hb_buffer_t) {
        if let Some(ref direction) = self.direction {
            let direction = direction.to_hb();
            ffi::hb_buffer_set_direction(buffer, direction);
        }
        if let Some(ref script) = self.script {
            let script = ffi::hb_script_from_string(script.as_ptr() as _, script.len() as _);
            ffi::hb_buffer_set_script(buffer, script)
        }
        let sys_language = std::env::var("LANG").ok();
        let language = self.language.as_ref().or_else(|| sys_language.as_ref());
        if let Some(language) = language {
            let language =
                ffi::hb_language_from_string(language.as_ptr() as _, language.len() as _);
            ffi::hb_buffer_set_language(buffer, language);
        }
        let flags = ffi::HB_BUFFER_FLAG_DEFAULT
            | if self.bot { ffi::HB_BUFFER_FLAG_BOT } else { 0 }
            | if self.eot { ffi::HB_BUFFER_FLAG_EOT } else { 0 }
            | if self.preserve_default_ignorables {
                ffi::HB_BUFFER_FLAG_PRESERVE_DEFAULT_IGNORABLES
            } else {
                0
            }
            | if self.remove_default_ignorables {
                ffi::HB_BUFFER_FLAG_REMOVE_DEFAULT_IGNORABLES
            } else {
                0
            }
            | 0;

        ffi::hb_buffer_set_flags(buffer, flags);
        if let Some(invisible_glyph) = self.invisible_glyph {
            ffi::hb_buffer_set_invisible_glyph(buffer, invisible_glyph);
        }
        ffi::hb_buffer_set_cluster_level(buffer, self.cluster_level.to_hb());
        ffi::hb_buffer_guess_segment_properties(buffer);
    }

    pub unsafe fn copy_buffer_properties(dst: *mut ffi::hb_buffer_t, src: *mut ffi::hb_buffer_t) {
        let mut props: MaybeUninit<ffi::hb_segment_properties_t> = MaybeUninit::uninit();
        ffi::hb_buffer_get_segment_properties(src, props.as_mut_ptr());
        ffi::hb_buffer_set_segment_properties(dst, props.as_ptr());
        ffi::hb_buffer_set_flags(dst, ffi::hb_buffer_get_flags(src));
        ffi::hb_buffer_set_cluster_level(dst, ffi::hb_buffer_get_cluster_level(src));
    }

    pub unsafe fn populate_buffer(
        &self,
        buffer: *mut ffi::hb_buffer_t,
        text: &str,
        text_before: Option<&str>,
        text_after: Option<&str>,
    ) {
        ffi::hb_buffer_clear_contents(buffer);
        if let Some(text_before) = text_before {
            let len = text_before.len();
            ffi::hb_buffer_add_utf8(buffer, text_before.as_ptr() as _, len as _, len as _, 0);
        }
        let text_len = text.len();
        ffi::hb_buffer_add_utf8(buffer, text.as_ptr() as _, text_len as _, 0, text_len as _);
        if let Some(text_after) = text_after {
            let len = text_after.len();
            ffi::hb_buffer_add_utf8(buffer, text_after.as_ptr() as _, len as _, len as _, 0);
        }

        if !self.utf8_clusters {
            // Reset cluster values to refer to Unicode character index
            // instead of UTF-8 index.
            let num_glyphs = ffi::hb_buffer_get_length(buffer);
            let mut length = 0u32;
            let mut info = ffi::hb_buffer_get_glyph_infos(buffer, &mut length as *mut _);
            for i in 0..num_glyphs {
                (*info).cluster = i;
                info = info.offset(1);
            }
        }

        self.setup_buffer(buffer);
    }
}
