#![recursion_limit = "256"]

mod api;
mod app;
mod auth;
mod browser;
mod browser_bridge;
mod captcha;
mod cli;
mod commands;
mod core;
mod media;
mod net;
mod output;
mod workflow;

#[tokio::main]
async fn main() {
    // Portable package exposes only the Studio downloader, regardless of filename.
    std::process::exit(commands::studio_download::standalone_main().await);
}
