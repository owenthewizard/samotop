mod impatient;

pub use impatient::*;

pub use samotop_core::smtp::*;
#[cfg(feature = "parser-peg")]
pub use samotop_parser::*;
#[cfg(feature = "parser-nom")]
pub use samotop_parser_nom::*;

#[cfg(feature = "parser-nom")]
pub type SmtpParser = samotop_parser_nom::SmtpParserNom;
#[cfg(all(feature = "parser-peg", not(feature = "parser-nom")))]
pub type SmtpParser = samotop_parser::SmtpParserPeg;
