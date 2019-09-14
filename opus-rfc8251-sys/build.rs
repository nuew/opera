use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

/// Compiler argument style to use for bindgen arguments
const BINDINGS_COMPILER_ARG_STYLE: &str = "clang";

/// Output filename for the generated bindings
const BINDINGS_FILENAME: &str = "bindings.rs";

/// Extension for C sourcefile makefile precursors
const OPUS_C_SOURCE_EXT: &str = "mk";

/// Definitions for libopus compilation
const OPUS_DEFINES: &[(&str, Option<&str>)] = &[
    ("OPUS_BUILD", None),
    ("OPUS_VERSION", Some("\"1.0.0-rfc8251\"")),
];

/// Enable warnings during compilation.
///
/// Disabled by default as the unedited source generates one.
const OPUS_ENABLE_WARNINGS: bool = false;

/// Tail of the string for C header sourcefile makefile precursors
const OPUS_H_SOURCE_TAIL: &str = "_headers.txt";

/// Include directories, relative to `OPUS_RFC8251`, to declare in libopus compilation
const OPUS_INCLUDES: &[&str] = &["include", "silk", "silk/float", "silk/fixed", "celt", "src"];

/// Directory where the libopus sources are; relative to `Cargo.toml`
const OPUS_RFC8251: &str = "opus-rfc8251";

/// Regular expression matching all functions to bind to
const OPUS_FUNCS_REGEXP: &str =
    "^((([dr]e)?normalise|(un)?quant|_?ce?lt|alg|compute|ec|kiss|opus|pitch|silk|stereo)_.*|\
     ((amp|log)2(Amp|Log2)|(bits|pulses)2(bits|pulses)|(de|en)code_pulses|anti_collapse|\
     encode_size|get_(pulses|required_bits)|haar1|remove_doubling|spreading_decision)$)";

/// Regular expression matching all items (constants & statics) to bind to
const OPUS_ITEMS_REGEXP: &str = "^((((V|MAX)_)?PITCH|(LOW|HIGH)_RATE|BG_SNR|BWE|CELT|CNG|CODE|EC|\
    (MAX_)?FIND_(PITCH|LPC|LTP)|FLAG|HARM|HP|LA|LAMBDA|LBRR|LOG2_INV_LPC_GAIN|LTP|MODE|MU_LTP|NLSF|\
    NSQ|OPUS|PE|PITCH|RAND_BUF|RESAMPLER|SILK|SP(EECH|READ)|SPARSE(NESS)?|STEREO|TRANSITION|VAD|\
    VARIABLE_HP|cache|fft|mdct|silk)_.*|(MAX_((API_)?FS_KHZ|(SUB_)?FRAME_.*|\
    BANDWIDTH_SWITCH_DELAY_MS|CONSECUTIVE_DTX|DEL_DEC_STATES|FRAMES_PER_PACKET|MATRIX_SIZE|\
    NB_SHELL_BLOCKS|NB_SUBFR|PREDICTION_.*|PULSES|SHAPE_.*)|(EN|DE)CODER_NUM_CHANNELS|\
    (LOG2_)?SHELL_CODEC_FRAME_LENGTH|(MAX|MIN)(_TARGET_RATE_BPS|LPC_.*)|BANDWIDTH_EXPANSION|\
    DECISION_DELAY(_MASK)?|HARMONIC_SHAPING|HARM_SHAPE_FIR_TAPS|HIGH_PASS_INPUT|INPUT_TILT|\
    L(PC|TP)_ORDER|LOG2_SHELL_CODEC_FRAME_LENGTH|LOW_(FREQ_SHAPING|QUALITY_LOW_FREQ_SHAPING_DECR)|\
    LSF_COS_TAB_SZ_FIX|NB_LTP_CBKS|N_RATE_LEVELS|OFFSET_U?V[LH]_Q10|PITCH_DRIFT_FAC_Q16|\
    QUANT_LEVEL_ADJUST_Q10|REDUCE_BITRATE_10_MS_BPS|SHAPE_LPC_WIN_MAX|SHELL_CODEC_FRAME_LENGTH|
    SUBFR_SMTH_COEF|TARGET_RATE_TAB_SZ|SE_HARM_SHAPING|WARPING_MULTIPLIER)$)";

/// Parse a {shell, makefile}-formatted variable definition script
fn parse_shell_variables<T>(path: T) -> Vec<(String, String)>
where
    T: AsRef<Path>,
{
    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    let mut file = BufReader::new(File::open(path).expect("couldn't open sources file"));
    let (mut ret, mut buf) = (Vec::new(), String::new());

    // this unwrap should be an expect but that makes the line too long
    while file.read_line(&mut buf).unwrap() != 0 {
        // ignore empty lines & comments
        if buf.len() > 1 && !buf.starts_with('#') {
            // deal with extended lines
            while buf.ends_with("\\\n") {
                buf.truncate(buf.len() - 2); // delete "\\\n"
                assert!(
                    file.read_line(&mut buf)
                        .expect("couldn't read extended line from sources file")
                        != 0,
                    "sources file ended early"
                );
            }

            // parse and return variable
            let (key, value) = buf.split_at(buf.find('=').expect("couldn't parse sources file"));
            ret.push((
                String::from(key.trim_end()),
                String::from(value[1..].trim()),
            ));
        }
        buf.clear();
    }

    ret
}

/// Returns requested libopus's source files from makefile precursors
fn wanted<'a, T: 'a>(opus_rfc8251: &'a Path, path_test: T) -> impl Iterator<Item = PathBuf> + 'a
where
    T: Copy + FnOnce(&PathBuf) -> bool,
{
    opus_rfc8251
        .read_dir()
        .expect("couldn't list C source directory")
        .map(|res| res.expect("couldn't stat file in C source directory"))
        .filter_map(move |dir_entry| Some(dir_entry.path()).filter(path_test))
        .flat_map(parse_shell_variables)
        .flat_map(|(_, value)| value.split(' ').map(String::from).collect::<Vec<_>>())
        .map(move |file| opus_rfc8251.join(file))
}

/// Builds libopus
fn build<T>(opus_rfc8251: T, features: Vec<String>) -> Vec<OsString>
where
    T: AsRef<Path>,
{
    use std::ffi::OsStr;

    let opus_rfc8251 = opus_rfc8251.as_ref();
    let mut build = cc::Build::new();

    // add configured defines.
    // entries in OPUS_DEFINES are always defined, regardless of features
    let defines = OPUS_DEFINES
        .iter()
        .copied()
        .chain(features.iter().map(|define| (&define[..], None)));
    for (define, value) in defines {
        build.define(define, value);
    }

    // add configured include paths
    for include_dir in OPUS_INCLUDES {
        build.include(opus_rfc8251.join(include_dir));
    }

    // add source files
    build.files(wanted(opus_rfc8251, |path| {
        path.extension()
            .map(|ext| ext == OsStr::new(OPUS_C_SOURCE_EXT))
            .unwrap_or_default()
    }));

    build.warnings(OPUS_ENABLE_WARNINGS).compile(OPUS_RFC8251); // build libopus

    // return compiler arguments in the appropriate style
    build
        .compiler(BINDINGS_COMPILER_ARG_STYLE)
        .get_compiler()
        .args()
        .into()
}

/// Generates bindings for libopus
fn generate_bindings<T, U>(opus_rfc8251: T, cc_args: Vec<OsString>, output_file: U)
where
    T: AsRef<Path>,
    U: AsRef<Path>,
{
    use std::ffi::OsStr;

    let opus_rfc8251 = opus_rfc8251.as_ref();
    let mut bindgen = bindgen::builder();

    // source all header files
    for header in wanted(opus_rfc8251, |path| {
        path.file_name()
            .and_then(OsStr::to_str)
            .map(|name| name.ends_with(OPUS_H_SOURCE_TAIL))
            .unwrap_or_default()
    }) {
        let header = header.to_string_lossy();
        if !header.contains("float") && !header.contains("fixed") {
            bindgen = bindgen.header(header);
        }
    }

    // generate bindings
    bindgen
        .clang_args(
            cc_args
                .into_iter()
                .map(OsString::into_string)
                .map(Result::unwrap),
        )
        .whitelist_function(OPUS_FUNCS_REGEXP)
        .whitelist_var(OPUS_ITEMS_REGEXP)
        .generate()
        .expect("unable to generate bindings")
        .write_to_file(output_file)
        .expect("couldn't write out generated bindings")
}

fn main() {
    use std::env::{var_os, vars};

    const CARGO_FEATURE_PREFIX: &str = "CARGO_FEATURE_";

    // get manifest/output dirs & enabled features
    let manifest_dir = PathBuf::from(var_os("CARGO_MANIFEST_DIR").unwrap());
    let output_dir = PathBuf::from(var_os("OUT_DIR").unwrap());
    let features = vars()
        .filter(|(var, _)| var.starts_with(CARGO_FEATURE_PREFIX))
        .map(|(var, _)| String::from(&var[CARGO_FEATURE_PREFIX.len()..]))
        .collect();

    // find libopus source directory
    let opus_rfc8251 = manifest_dir.join(OPUS_RFC8251);

    // build libopus & generate bindings
    let cc_args = build(&opus_rfc8251, features);
    generate_bindings(opus_rfc8251, cc_args, output_dir.join(BINDINGS_FILENAME));
}
