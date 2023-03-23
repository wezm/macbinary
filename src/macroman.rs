#[cfg(feature = "no_std")]
use heapless::String;

#[cfg(feature = "no_std")]
pub trait FromMacRoman {
    fn try_from_macroman(data: &[u8]) -> Option<Self>
    where
        Self: Sized;
}

#[cfg(not(feature = "no_std"))]
pub trait FromMacRoman {
    fn from_macroman(data: &[u8]) -> Self;
}

/// Converts Mac OS Roman character to a Unicode `char`.
///
/// Returns `None` if the character is not part of the Mac OS Roman character set.
#[rustfmt::skip]
pub fn macroman_to_char(macroman: u8) -> Option<char> {
    match macroman {
        0..=127 => Some(macroman as char),
        128 => Some('Ä'), // A dieresis
        129 => Some('Å'), // A ring
        130 => Some('Ç'), // C cedilla
        131 => Some('É'), // E acute
        132 => Some('Ñ'), // N tilde
        133 => Some('Ö'), // O dieresis
        134 => Some('Ü'), // U dieresis
        135 => Some('á'), // a acute
        136 => Some('à'), // a grave
        137 => Some('â'), // a circumflex
        138 => Some('ä'), // a dieresis
        139 => Some('ã'), // a tilde
        140 => Some('å'), // a ring
        141 => Some('ç'), // c cedilla
        142 => Some('é'), // e acute
        143 => Some('è'), // e grave
        144 => Some('ê'), // e circumflex
        145 => Some('ë'), // e dieresis
        146 => Some('í'), // i acute
        147 => Some('ì'), // i grave
        148 => Some('î'), // i circumflex
        149 => Some('ï'), // i dieresis
        150 => Some('ñ'), // n tilde
        151 => Some('ó'), // o acute
        152 => Some('ò'), // o grave
        153 => Some('ô'), // o circumflex
        154 => Some('ö'), // o dieresis
        155 => Some('õ'), // o tilde
        156 => Some('ú'), // u acute
        157 => Some('ù'), // u grave
        158 => Some('û'), // u circumflex
        159 => Some('ü'), // u dieresis
        160 => Some('†'), // dagger
        161 => Some('°'), // degree
        162 => Some('¢'), // cent
        163 => Some('£'), // sterling
        164 => Some('§'), // section
        165 => Some('•'), // bullet
        166 => Some('¶'), // paragraph
        167 => Some('ß'), // German double s
        168 => Some('®'), // registered
        169 => Some('©'), // copyright
        170 => Some('™'), // trademark
        171 => Some('´'), // acute
        172 => Some('¨'), // diaeresis
        174 => Some('Æ'), // AE
        175 => Some('Ø'), // O slash
        177 => Some('±'), // plusminus
        180 => Some('¥'), // yen
        181 => Some('µ'), // micro
        187 => Some('ª'), // ordfeminine
        188 => Some('º'), // ordmasculine
        190 => Some('æ'), // ae
        191 => Some('ø'), // o slash
        192 => Some('¿'), // question down
        193 => Some('¡'), // exclamation down
        194 => Some('¬'), // not
        196 => Some('ƒ'), // florin
        199 => Some('«'), // left guille
        200 => Some('»'), // right guille
        201 => Some('…'), // ellipsis
        202 => Some(' '), // non-breaking space
        203 => Some('À'), // A grave
        204 => Some('Ã'), // A tilde
        205 => Some('Õ'), // O tilde
        206 => Some('Œ'), // OE
        207 => Some('œ'), // oe
        208 => Some('–'), // endash
        209 => Some('—'), // emdash
        210 => Some('“'), // ldquo
        211 => Some('”'), // rdquo
        212 => Some('‘'), // lsquo
        213 => Some('’'), // rsquo
        214 => Some('÷'), // divide
        216 => Some('ÿ'), // y dieresis
        217 => Some('Ÿ'), // Y dieresis
        218 => Some('⁄'), // fraction
        219 => Some('¤'), // currency
        220 => Some('‹'), // left single guille
        221 => Some('›'), // right single guille
        222 => Some('ﬁ'), // fi
        223 => Some('ﬂ'), // fl
        224 => Some('‡'), // double dagger
        225 => Some('·'), // middle dot
        226 => Some('‚'), // single quote base
        227 => Some('„'), // double quote base
        228 => Some('‰'), // perthousand
        229 => Some('Â'), // A circumflex
        230 => Some('Ê'), // E circumflex
        231 => Some('Á'), // A acute
        232 => Some('Ë'), // E dieresis
        233 => Some('È'), // E grave
        234 => Some('Í'), // I acute
        235 => Some('Î'), // I circumflex
        236 => Some('Ï'), // I dieresis
        237 => Some('Ì'), // I grave
        238 => Some('Ó'), // O acute
        239 => Some('Ô'), // O circumflex
        241 => Some('Ò'), // O grave
        242 => Some('Ú'), // U acute
        243 => Some('Û'), // U circumflex
        244 => Some('Ù'), // U grave
        245 => Some('ı'), // dot-less i
        246 => Some('^'), // circumflex
        247 => Some('˜'), // tilde
        248 => Some('¯'), // macron
        249 => Some('˘'), // breve
        250 => Some('˙'), // dot accent
        251 => Some('˚'), // ring
        252 => Some('¸'), // cedilla
        253 => Some('˝'), // Hungarian umlaut (double acute accent)
        254 => Some('˛'), // ogonek
        255 => Some('ˇ'), // caron
        _ => None,
    }
}

#[cfg(not(feature = "no_std"))]
impl FromMacRoman for String {
    fn from_macroman(data: &[u8]) -> Self {
        data.iter()
            .map(|c| macroman_to_char(*c).unwrap_or('\u{FFFD}'))
            .collect()
    }
}

#[cfg(feature = "no_std")]
impl<const N: usize> FromMacRoman for String<N> {
    fn try_from_macroman(bytes: &[u8]) -> Option<String<N>> {
        let mut name: String<N> = String::new();
        for byte in bytes {
            name.push(macroman_to_char(*byte).unwrap_or('\u{FFFD}'))
                .ok()?;
        }
        Some(name)
    }
}
