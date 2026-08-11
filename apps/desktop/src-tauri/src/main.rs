// Release builds on Windows would otherwise open a console window behind the
// app. trdr ships on macOS, but the attribute costs nothing and stops a Windows
// build from being surprising if one is ever made.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main()
{
    trdr_desktop_lib::run();
}
