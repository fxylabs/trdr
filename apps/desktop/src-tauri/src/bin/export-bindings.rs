//! Writes the TypeScript bindings the React app imports.
//!
//! Run after changing a command signature or the command list:
//!
//! ```text
//! cargo run -p trdr-desktop --bin export-bindings
//! ```
//!
//! `cargo test --workspace` fails until the committed file matches what this
//! produces, so forgetting to run it is caught rather than shipped.

fn main()
{
    match trdr_desktop_lib::bindings::export()
    {
        Ok(path) => println!("wrote {}", path.display()),
        Err(error) =>
        {
            eprintln!("could not write the bindings: {error}");
            std::process::exit(1);
        }
    }
}
