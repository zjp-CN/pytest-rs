//! libtest-compatible argument parser
//!
//! This does not drive parsing but provides [`TestOptsParseState`] to plug into the parsing,
//! allowing additional parsers to be integrated.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_debug_implementations, elided_lifetimes_in_paths)]

use lexarg::Arg;
use lexarg_error::Error;
use lexarg_error::Result;

/// Parsed command-line options
///
/// To parse, see [`TestOptsParseState`]
#[derive(Debug, Default)]
pub struct TestOpts {
    pub list: bool,
    pub filters: Vec<String>,
    pub filter_exact: bool,
    pub force_run_in_process: bool,
    pub exclude_should_panic: bool,
    pub run_ignored: RunIgnored,
    pub run_tests: bool,
    pub bench_benchmarks: bool,
    pub logfile: Option<std::path::PathBuf>,
    pub nocapture: bool,
    pub color: ColorConfig,
    pub format: OutputFormat,
    pub shuffle: bool,
    pub shuffle_seed: Option<u64>,
    pub test_threads: Option<std::num::NonZeroUsize>,
    pub skip: Vec<String>,
    pub time_options: Option<TestTimeOptions>,
    /// Stop at first failing test.
    /// May run a few more tests due to threading, but will
    /// abort as soon as possible.
    pub fail_fast: bool,
    pub options: Options,
    pub allowed_unstable: Vec<String>,
}

/// Whether ignored test should be run or not
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RunIgnored {
    Yes,
    No,
    /// Run only ignored tests
    Only,
}

impl Default for RunIgnored {
    fn default() -> Self {
        Self::No
    }
}

/// Whether should console output be colored or not
#[derive(Copy, Clone, Debug)]
pub enum ColorConfig {
    AutoColor,
    AlwaysColor,
    NeverColor,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self::AutoColor
    }
}

/// Format of the test results output
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    /// Verbose output
    Pretty,
    /// Quiet output
    Terse,
    /// JSON output
    Json,
    /// JUnit output
    Junit,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Pretty
    }
}

/// Structure with parameters for calculating test execution time.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TestTimeOptions {
    /// Denotes if the test critical execution time limit excess should be considered
    /// a test failure.
    pub error_on_excess: bool,
    pub unit_threshold: TimeThreshold,
    pub integration_threshold: TimeThreshold,
    pub doctest_threshold: TimeThreshold,
}

impl Default for TestTimeOptions {
    fn default() -> Self {
        Self {
            error_on_excess: false,
            unit_threshold: TimeThreshold {
                warn: std::time::Duration::from_millis(50),
                critical: std::time::Duration::from_millis(100),
            },
            integration_threshold: TimeThreshold {
                warn: std::time::Duration::from_millis(50),
                critical: std::time::Duration::from_millis(100),
            },
            doctest_threshold: TimeThreshold {
                warn: std::time::Duration::from_millis(50),
                critical: std::time::Duration::from_millis(100),
            },
        }
    }
}

/// Structure denoting time limits for test execution.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct TimeThreshold {
    pub warn: std::time::Duration,
    pub critical: std::time::Duration,
}

impl TimeThreshold {
    /// Attempts to create a `TimeThreshold` instance with values obtained
    /// from the environment variable, and returns `None` if the variable
    /// is not set.
    /// Environment variable format is expected to match `\d+,\d+`.
    ///
    /// # Panics
    ///
    /// Panics if variable with provided name is set but contains inappropriate
    /// value.
    fn from_env_var(env_var_name: &str) -> Result<Option<Self>> {
        use std::str::FromStr;

        let durations_str = match std::env::var(env_var_name) {
            Ok(value) => value,
            Err(_) => {
                return Ok(None);
            }
        };
        let (warn_str, critical_str) = durations_str.split_once(',').ok_or_else(|| {
            Error::msg(format_args!(
                "Duration variable {env_var_name} expected to have 2 numbers separated by comma, but got {durations_str}"
            ))
        })?;

        let parse_u64 = |v| {
            u64::from_str(v).map_err(|_err| {
                Error::msg(format_args!(
                    "Duration value in variable {env_var_name} is expected to be a number, but got {v}"
                ))
            })
        };

        let warn = parse_u64(warn_str)?;
        let critical = parse_u64(critical_str)?;
        if warn > critical {
            panic!("Test execution warn time should be less or equal to the critical time");
        }

        Ok(Some(Self {
            warn: std::time::Duration::from_millis(warn),
            critical: std::time::Duration::from_millis(critical),
        }))
    }
}

/// Options for the test run defined by the caller (instead of CLI arguments).
/// In case we want to add other options as well, just add them in this struct.
#[derive(Copy, Clone, Debug)]
pub struct Options {
    pub display_output: bool,
    pub panic_abort: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            display_output: false,
            panic_abort: false,
        }
    }
}

pub const UNSTABLE_OPTIONS: &str = "unstable-options";

pub const OPTIONS_HELP: &str = r#"
Options:
        --include-ignored 
                        Run ignored and not ignored tests
        --ignored       Run only ignored tests
        --force-run-in-process 
                        Forces tests to run in-process when panic=abort
        --exclude-should-panic 
                        Excludes tests marked as should_panic
        --test          Run tests and not benchmarks
        --bench         Run benchmarks instead of tests
        --list          List all tests and benchmarks
        --logfile PATH  Write logs to the specified file
        --nocapture     don't capture stdout/stderr of each task, allow
                        printing directly
        --test-threads n_threads
                        Number of threads used for running tests in parallel
        --skip FILTER   Skip tests whose names contain FILTER (this flag can
                        be used multiple times)
    -q, --quiet         Display one character per test instead of one line.
                        Alias to --format=terse
        --exact         Exactly match filters rather than by substring
        --color auto|always|never
                        Configure coloring of output:
                        auto = colorize if stdout is a tty and tests are run
                        on serially (default);
                        always = always colorize output;
                        never = never colorize output;
        --format pretty|terse|json|junit
                        Configure formatting of output:
                        pretty = Print verbose output;
                        terse = Display one character per test;
                        json = Output a json document;
                        junit = Output a JUnit document
        --show-output   Show captured stdout of successful tests
    -Z unstable-options Enable nightly-only flags:
                        unstable-options = Allow use of experimental features
        --report-time   Show execution time of each test.
                        Threshold values for colorized output can be
                        configured via
                        `RUST_TEST_TIME_UNIT`, `RUST_TEST_TIME_INTEGRATION`
                        and
                        `RUST_TEST_TIME_DOCTEST` environment variables.
                        Expected format of environment variable is
                        `VARIABLE=WARN_TIME,CRITICAL_TIME`.
                        Durations must be specified in milliseconds, e.g.
                        `500,2000` means that the warn time
                        is 0.5 seconds, and the critical time is 2 seconds.
                        Not available for --format=terse
        --ensure-time   Treat excess of the test execution time limit as
                        error.
                        Threshold values for this option can be configured via
                        `RUST_TEST_TIME_UNIT`, `RUST_TEST_TIME_INTEGRATION`
                        and
                        `RUST_TEST_TIME_DOCTEST` environment variables.
                        Expected format of environment variable is
                        `VARIABLE=WARN_TIME,CRITICAL_TIME`.
                        `CRITICAL_TIME` here means the limit that should not
                        be exceeded by test.
        --shuffle       Run tests in random order
        --shuffle-seed SEED
                        Run tests in random order; seed the random number
                        generator with SEED
"#;

pub const AFTER_HELP: &str = r#"
The FILTER string is tested against the name of all tests, and only those
tests whose names contain the filter are run. Multiple filter strings may
be passed, which will run all tests matching any of the filters.

By default, all tests are run in parallel. This can be altered with the
--test-threads flag or the RUST_TEST_THREADS environment variable when running
tests (set it to 1).

By default, the tests are run in alphabetical order. Use --shuffle or set
RUST_TEST_SHUFFLE to run the tests in random order. Pass the generated
"shuffle seed" to --shuffle-seed (or set RUST_TEST_SHUFFLE_SEED) to run the
tests in the same order again. Note that --shuffle and --shuffle-seed do not
affect whether the tests are run in parallel.

All tests have their standard output and standard error captured by default.
This can be overridden with the --nocapture flag or setting RUST_TEST_NOCAPTURE
environment variable to a value other than "0". Logging is not captured by default.

Test Attributes:

    `#[test]`        - Indicates a function is a test to be run. This function
                       takes no arguments.
    `#[bench]`       - Indicates a function is a benchmark to be run. This
                       function takes one argument (test::Bencher).
    `#[should_panic]` - This function (also labeled with `#[test]`) will only pass if
                        the code causes a panic (an assertion failure or panic!)
                        A message may be provided, which the failure string must
                        contain: #[should_panic(expected = "foo")].
    `#[ignore]`       - When applied to a function which is already attributed as a
                        test, then the test runner will ignore these tests during
                        normal test runs. Running with --ignored or --include-ignored will run
                        these tests.
"#;

/// Intermediate CLI parser state for [`TestOpts`]
///
/// See [`TestOptsParseState::parse_next`]
#[derive(Debug, Default)]
pub struct TestOptsParseState {
    opts: TestOpts,
    quiet: bool,
    format: Option<OutputFormat>,
    include_ignored: bool,
    ignored: bool,
}

impl TestOptsParseState {
    pub fn new() -> Self {
        Default::default()
    }

    /// Check if `arg` is relevant to [`TestOpts`]
    pub fn parse_next<'a>(
        &mut self,
        parser: &mut lexarg::Parser<'a>,
        arg: Arg<'a>,
    ) -> Result<Option<Arg<'a>>> {
        match arg {
            Arg::Long("include-ignored") => {
                self.include_ignored = true;
            }
            Arg::Long("ignored") => self.ignored = true,
            Arg::Long("force-run-in-process") => {
                self.opts.force_run_in_process = true;
            }
            Arg::Long("exclude-should-panic") => {
                self.opts.exclude_should_panic = true;
            }
            Arg::Long("test") => {
                self.opts.run_tests = true;
            }
            Arg::Long("bench") => {
                self.opts.bench_benchmarks = true;
            }
            Arg::Long("list") => {
                self.opts.list = true;
            }
            Arg::Long("logfile") => {
                let path = parser
                    .flag_value()
                    .ok_or_else(|| Error::msg("`--logfile` requires a path"))?;
                self.opts.logfile = Some(std::path::PathBuf::from(path));
            }
            Arg::Long("nocapture") => {
                self.opts.nocapture;
            }
            Arg::Long("test-threads") => {
                let test_threads = parser
                    .flag_value()
                    .ok_or_else(|| Error::msg("`--test-threads` requires a positive integer"))?
                    .to_str()
                    .ok_or_else(|| Error::msg("unsupported value"))?;
                self.opts.test_threads = match test_threads.parse::<std::num::NonZeroUsize>() {
                    Ok(n) => Some(n),
                    Err(_) => {
                        return Err(Error::msg("`--test-threads` must be a positive integer"));
                    }
                };
            }
            Arg::Long("skip") => {
                let filter = parser
                    .flag_value()
                    .ok_or_else(|| Error::msg("`--skip` requires a value"))?
                    .to_str()
                    .ok_or_else(|| Error::msg("unsupported value"))?;
                self.opts.skip.push(filter.to_owned());
            }
            Arg::Long("exact") => {
                self.opts.filter_exact = true;
            }
            Arg::Long("color") => {
                let color = parser
                    .flag_value()
                    .ok_or_else(|| {
                        Error::msg("`--color` requires one of `auto`, `always`, or `never`")
                    })?
                    .to_str()
                    .ok_or_else(|| Error::msg("unsupported value"))?;
                self.opts.color = match color {
                    "auto" => ColorConfig::AutoColor,
                    "always" => ColorConfig::AlwaysColor,
                    "never" => ColorConfig::NeverColor,
                    _ => {
                        return Err(Error::msg("`--color` accepts `auto`, `always`, or `never`"));
                    }
                };
            }
            Arg::Short('q') | Arg::Long("quiet") => {
                self.format = None;
                self.quiet = true;
            }
            Arg::Long("format") => {
                self.quiet = false;
                let format = parser
                    .flag_value()
                    .ok_or_else(|| {
                        Error::msg(
                            "`--format` requires one of `pretty`, `terse`, `json`, or `junit`",
                        )
                    })?
                    .to_str()
                    .ok_or_else(|| Error::msg("unsupported value"))?;
                self.format = Some(match format {
                    "pretty" => OutputFormat::Pretty,
                    "terse" => OutputFormat::Terse,
                    "json" => OutputFormat::Json,
                    "junit" => OutputFormat::Junit,
                    _ => {
                        return Err(Error::msg(
                            "`--format` accepts `pretty`, `terse`, `json`, or `junit`",
                        ));
                    }
                });
            }
            Arg::Long("show-output") => {
                self.opts.options.display_output = true;
            }
            Arg::Short('Z') => {
                let feature = parser
                    .flag_value()
                    .ok_or_else(|| Error::msg("`-Z` requires a feature name"))?
                    .to_str()
                    .ok_or_else(|| Error::msg("unsupported value"))?;
                if !is_nightly() {
                    return Err(Error::msg("`-Z` is only accepted on the nightly compiler"));
                }
                // Don't validate `feature` as other parsers might provide values
                self.opts.allowed_unstable.push(feature.to_owned());
            }
            Arg::Long("report-time") => {
                self.opts
                    .time_options
                    .get_or_insert_with(|| Default::default());
            }
            Arg::Long("ensure-time") => {
                let time = self
                    .opts
                    .time_options
                    .get_or_insert_with(|| Default::default());
                time.error_on_excess = true;
                if let Some(threshold) = TimeThreshold::from_env_var("RUST_TEST_TIME_UNIT")? {
                    time.unit_threshold = threshold;
                }
                if let Some(threshold) = TimeThreshold::from_env_var("RUST_TEST_TIME_INTEGRATION")?
                {
                    time.integration_threshold = threshold;
                }
                if let Some(threshold) = TimeThreshold::from_env_var("RUST_TEST_TIME_DOCTEST")? {
                    time.doctest_threshold = threshold;
                }
            }
            Arg::Long("shuffle") => {
                self.opts.shuffle = true;
            }
            Arg::Long("shuffle-seed") => {
                let seed = parser
                    .flag_value()
                    .ok_or_else(|| Error::msg("`--shuffle-seed` requires a value"))?
                    .to_str()
                    .ok_or_else(|| Error::msg("unsupported value"))?
                    .parse::<u64>()
                    .map_err(Error::msg)?;
                self.opts.shuffle_seed = Some(seed);
            }
            // All values are the same, whether escaped or not, so its a no-op
            Arg::Escape => {}
            Arg::Value(filter) => {
                let filter = filter
                    .to_str()
                    .ok_or_else(|| Error::msg("unsupported value"))?;
                self.opts.filters.push(filter.to_owned());
            }
            _ => {
                return Ok(Some(arg));
            }
        }
        Ok(None)
    }

    /// Finish parsing, resolving to [`TestOpts`]
    pub fn finish(mut self) -> Result<TestOpts> {
        let allow_unstable_options = self
            .opts
            .allowed_unstable
            .iter()
            .any(|f| f == UNSTABLE_OPTIONS);

        if self.opts.force_run_in_process && !allow_unstable_options {
            return Err(Error::msg(
                "`--force-run-in-process` requires `-Zunstable-options`",
            ));
        }

        if self.opts.exclude_should_panic && !allow_unstable_options {
            return Err(Error::msg(
                "`--exclude-should-panic` requires `-Zunstable-options`",
            ));
        }

        if self.opts.shuffle && !allow_unstable_options {
            return Err(Error::msg("`--shuffle` requires `-Zunstable-options`"));
        }
        if !self.opts.shuffle && allow_unstable_options {
            self.opts.shuffle = match std::env::var("RUST_TEST_SHUFFLE") {
                Ok(val) => &val != "0",
                Err(_) => false,
            };
        }

        if self.opts.shuffle_seed.is_some() && !allow_unstable_options {
            return Err(Error::msg("`--shuffle-seed` requires `-Zunstable-options`"));
        }
        if self.opts.shuffle_seed.is_none() && allow_unstable_options {
            self.opts.shuffle_seed = match std::env::var("RUST_TEST_SHUFFLE_SEED") {
                Ok(val) => match val.parse::<u64>() {
                    Ok(n) => Some(n),
                    Err(_) => {
                        return Err(Error::msg(
                            "RUST_TEST_SHUFFLE_SEED is `{val}`, should be a number.",
                        ));
                    }
                },
                Err(_) => None,
            };
        }

        if !self.opts.nocapture {
            self.opts.nocapture = match std::env::var("RUST_TEST_NOCAPTURE") {
                Ok(val) => &val != "0",
                Err(_) => false,
            };
        }

        if self.format.is_some() && !allow_unstable_options {
            return Err(Error::msg("`--format` requires `-Zunstable-options`"));
        }
        if let Some(format) = self.format {
            self.opts.format = format;
        }
        if self.quiet {
            self.opts.format = OutputFormat::Terse;
        }

        self.opts.run_tests |= !self.opts.bench_benchmarks;

        self.opts.run_ignored = match (self.include_ignored, self.ignored) {
            (true, true) => {
                return Err(Error::msg(
                    "`--include-ignored` and `--ignored` are mutually exclusive",
                ))
            }
            (true, false) => RunIgnored::Yes,
            (false, true) => RunIgnored::Only,
            (false, false) => RunIgnored::No,
        };

        if self.opts.test_threads.is_none() {
            if let Ok(value) = std::env::var("RUST_TEST_THREADS") {
                self.opts.test_threads =
                    Some(value.parse::<std::num::NonZeroUsize>().map_err(|_e| {
                        Error::msg(format!(
                            "RUST_TEST_THREADS is `{value}`, should be a positive integer."
                        ))
                    })?);
            }
        }

        let opts = self.opts;
        Ok(opts)
    }
}

// FIXME: Copied from librustc_ast until linkage errors are resolved. Issue #47566
fn is_nightly() -> bool {
    // Whether this is a feature-staged build, i.e., on the beta or stable channel
    let disable_unstable_features = option_env!("CFG_DISABLE_UNSTABLE_FEATURES")
        .map(|s| s != "0")
        .unwrap_or(false);
    // Whether we should enable unstable features for bootstrapping
    let bootstrap = std::env::var("RUSTC_BOOTSTRAP").is_ok();

    bootstrap || !disable_unstable_features
}
