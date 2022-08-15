#![allow(dead_code)]

extern crate wee_alloc;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

mod rates;
mod assets;
mod montecarlo;
mod withdrawal;
mod person;
mod util;
mod income;
mod taxes;
