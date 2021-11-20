#[cfg(test)]
mod tests;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::ScopedMmap;

#[cfg(windows)]
pub use windows::ScopedMmap;
