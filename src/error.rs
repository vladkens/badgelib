/// An error returned by badgelib.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
  /// A color is neither a recognized name nor a valid hexadecimal value.
  #[error("invalid color '{0}'")]
  InvalidColor(String),

  /// A Simple Icons slug does not exist.
  #[error("unknown Simple Icons slug '{0}'")]
  UnknownLogo(String),
}

/// A result returned by badgelib.
pub type Result<T> = std::result::Result<T, Error>;
