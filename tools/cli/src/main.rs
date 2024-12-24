#[macro_use]
mod macros;
mod options;
mod commands {
    pub mod compress;
    pub mod decompress;
    pub mod test;
}

use crate::commands::{
    compress::compress_files,
    decompress::decompress_files,
    test::{test_compression, test_compression_mt},
};
use options::{Commands, TopLevel};

fn main() {
    let toplevel: TopLevel = argh::from_env();

    match toplevel.nested {
        Commands::Compress(cmd) => {
            compress_files(&cmd.source);
        }
        Commands::Decompress(cmd) => {
            decompress_files(&cmd.source);
        }
        Commands::Test(cmd) => {
            test_compression(&cmd.source);
        }
        Commands::TestMt(cmd) => {
            test_compression_mt(&cmd.source);
        }
    }

    println!("Done.");
}
