pub use samotop_core::smtp::*;
#[cfg(feature = "parser-peg")]
pub use samotop_parser::*;
#[cfg(feature = "parser-nom")]
pub use samotop_parser_nom::*;

#[cfg(feature = "parser-peg")]
pub use samotop_parser::SmtpParserPeg as SmtpParser;
#[cfg(all(feature = "parser-nom", not(feature = "parser-peg")))]
pub use samotop_parser_nom::SmtpParserNom as SmtpParser;
