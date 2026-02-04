# Alphabet text encoding

Alphabet's text encoding conventions are not enforced at the machine level
by any means, but these encoding conventions are followed by all of Alphabet's
dedicated programming languages.

Alphabet recognizes the ASCII character set. Control characters must be escaped.
Alphabet accepts all ISO C character escapes (`\0`, `\a`, `\b`, `\t`, `\n`,
`\v`, `\f`, `\r`). Other control characters must be escaped by their hex code,
with the syntax `\x00` where `00` must be 2 hexadecimal digits, uppercase or lowercase.
