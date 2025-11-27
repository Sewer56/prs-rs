use argh::FromArgs;

#[derive(FromArgs, PartialEq, Debug)]
/// Top-level command.
pub(crate) struct TopLevel {
    #[argh(subcommand)]
    pub(crate) nested: Commands,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub(crate) enum Commands {
    Compress(CompressCommand),
    Decompress(DecompressCommand),
    Test(TestCommand),
    TestMt(TestMtCommand),
}

/// Compresses all PRS files in the given directory.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "compress")]
pub(crate) struct CompressCommand {
    /// path to the file to compress, or directory of files to compress
    #[argh(option)]
    pub(crate) source: String,
}

/// Decompresses all PRS files in the given directory.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "decompress")]
pub(crate) struct DecompressCommand {
    /// path to the file to decompress, or directory of files to decompress
    #[argh(option)]
    pub(crate) source: String,
}

/// Tests that the compressor round trips, by compressing, calculating size and decompressing.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "test")]
pub(crate) struct TestCommand {
    /// path to the file or directory to test
    #[argh(option)]
    pub(crate) source: String,
}

/// Multithreaded test that the compressor round trips, by compressing, calculating size and decompressing.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "test_mt")]
pub(crate) struct TestMtCommand {
    /// path to the file or directory to test
    #[argh(option)]
    pub(crate) source: String,
}
