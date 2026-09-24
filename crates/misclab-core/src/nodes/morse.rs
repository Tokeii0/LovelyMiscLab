//! International Morse code — encode / decode.
use super::prelude::*;

const TABLE: &[(char, &str)] = &[
    ('A', ".-"),
    ('B', "-..."),
    ('C', "-.-."),
    ('D', "-.."),
    ('E', "."),
    ('F', "..-."),
    ('G', "--."),
    ('H', "...."),
    ('I', ".."),
    ('J', ".---"),
    ('K', "-.-"),
    ('L', ".-.."),
    ('M', "--"),
    ('N', "-."),
    ('O', "---"),
    ('P', ".--."),
    ('Q', "--.-"),
    ('R', ".-."),
    ('S', "..."),
    ('T', "-"),
    ('U', "..-"),
    ('V', "...-"),
    ('W', ".--"),
    ('X', "-..-"),
    ('Y', "-.--"),
    ('Z', "--.."),
    ('0', "-----"),
    ('1', ".----"),
    ('2', "..---"),
    ('3', "...--"),
    ('4', "....-"),
    ('5', "....."),
    ('6', "-...."),
    ('7', "--..."),
    ('8', "---.."),
    ('9', "----."),
    ('.', ".-.-.-"),
    (',', "--..--"),
    ('?', "..--.."),
    ('\'', ".----."),
    ('!', "-.-.--"),
    ('/', "-..-."),
    ('(', "-.--."),
    (')', "-.--.-"),
    ('&', ".-..."),
    (':', "---..."),
    (';', "-.-.-."),
    ('=', "-...-"),
    ('+', ".-.-."),
    ('-', "-....-"),
    ('_', "..--.-"),
    ('"', ".-..-."),
    ('$', "...-..-"),
    ('@', ".--.-."),
];

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let words: Vec<String> = in_text(inputs, "text")?
            .to_uppercase()
            .split_whitespace()
            .map(|word| {
                word.chars()
                    .filter_map(|ch| TABLE.iter().find(|(c, _)| *c == ch).map(|(_, m)| *m))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        Ok(out_text(words.join(" / ")))
    }
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(inputs, "text")?.replace('_', "-");
        let out: String = text
            .split('/')
            .map(|word| {
                word.split_whitespace()
                    .filter_map(|code| TABLE.iter().find(|(_, m)| *m == code).map(|(c, _)| *c))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(" ");
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "morse_encode",
            ENC,
            "摩尔斯编码",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        desc(
            "morse_decode",
            ENC,
            "摩尔斯解码",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![],
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
