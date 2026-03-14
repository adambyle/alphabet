# Alphabet text encoding

Alphabet's text encoding conventions are not enforced at the machine level
by any means, but these encoding conventions are followed by all of Alphabet's
dedicated programming languages.

Alphabet recognizes the ASCII character set. Non-graphic, non single space
characters must be escaped. Alphabet accepts the following escapes:
`\0`, `\t`, `\n`, and `\r`. Other characters must be escaped by their
hex code, with the syntax `\x00` where `00` must be 2 hexadecimal digits,
uppercase or lowercase.
