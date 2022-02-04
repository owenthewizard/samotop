use samotop::cli::*;

fn main() {
    if let Err(exit) = async_std::task::block_on(Main::from_args().run()) {
        std::process::exit(exit.into())
    }
}
