//! Enigma machine (Wehrmacht M3): 3 rotors + reflector + plugboard.
//! Symmetric — the same settings encrypt and decrypt.
use super::prelude::*;

// (name, wiring A→…, notch letter)
const ROTORS: &[(&str, &[u8; 26], u8)] = &[
    ("I", b"EKMFLGDQVZNTOWYHXUSPAIBRCJ", b'Q'),
    ("II", b"AJDKSIRUXBLHWTMCQGZNPYFVOE", b'E'),
    ("III", b"BDFHJLCPRTXVZNYEIWGAKMUSQO", b'V'),
    ("IV", b"ESOVPZJAYQUIRHXLNFTGKDCMWB", b'J'),
    ("V", b"VZBRGITYUPSDNHLXAWMJQOFECK", b'Z'),
];
const REFLECTORS: &[(&str, &[u8; 26])] = &[
    ("B", b"YRUHQSLDPXNGOKMIEBFZCWVJAT"),
    ("C", b"FVPJIAOYEDRZXWGCTKUQSBNMHL"),
];

struct Rotor {
    fwd: [usize; 26],
    inv: [usize; 26],
    pos: usize,
    ring: usize,
    notch: usize,
}

impl Rotor {
    fn thru(&self, c: usize, forward: bool) -> usize {
        let shift = (c + 26 + self.pos - self.ring) % 26;
        let mapped = if forward {
            self.fwd[shift]
        } else {
            self.inv[shift]
        };
        (mapped + 26 - self.pos + self.ring) % 26
    }
    fn at_notch(&self) -> bool {
        self.pos == self.notch
    }
}

fn build_rotor(name: &str, ring: u8, pos: u8) -> Result<Rotor, CoreError> {
    let (_, wiring, notch) = ROTORS
        .iter()
        .find(|(n, _, _)| *n == name)
        .ok_or_else(|| CoreError::Parse(format!("未知转子: {name}（可用 I II III IV V）")))?;
    let mut fwd = [0usize; 26];
    let mut inv = [0usize; 26];
    for (i, &b) in wiring.iter().enumerate() {
        let o = (b - b'A') as usize;
        fwd[i] = o;
        inv[o] = i;
    }
    Ok(Rotor {
        fwd,
        inv,
        pos: (pos.to_ascii_uppercase() - b'A') as usize,
        ring: (ring.to_ascii_uppercase() - b'A') as usize,
        notch: (notch - b'A') as usize,
    })
}

fn build_plugboard(s: &str) -> [usize; 26] {
    let mut pb: [usize; 26] = std::array::from_fn(|i| i);
    let letters: Vec<u8> = s
        .bytes()
        .filter(|b| b.is_ascii_alphabetic())
        .map(|b| b.to_ascii_uppercase())
        .collect();
    for pair in letters.chunks(2) {
        if pair.len() == 2 {
            let (a, b) = ((pair[0] - b'A') as usize, (pair[1] - b'A') as usize);
            pb[a] = b;
            pb[b] = a;
        }
    }
    pb
}

fn three(s: &str) -> [u8; 3] {
    let v: Vec<u8> = s
        .bytes()
        .filter(|b| b.is_ascii_alphabetic())
        .map(|b| b.to_ascii_uppercase())
        .collect();
    [
        *v.first().unwrap_or(&b'A'),
        *v.get(1).unwrap_or(&b'A'),
        *v.get(2).unwrap_or(&b'A'),
    ]
}

fn step(rotors: &mut [Rotor; 3]) {
    let right_notch = rotors[2].at_notch();
    let mid_notch = rotors[1].at_notch();
    if mid_notch {
        rotors[0].pos = (rotors[0].pos + 1) % 26;
        rotors[1].pos = (rotors[1].pos + 1) % 26;
    } else if right_notch {
        rotors[1].pos = (rotors[1].pos + 1) % 26;
    }
    rotors[2].pos = (rotors[2].pos + 1) % 26;
}

struct N;
impl Node for N {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(inputs, "text")?;
        let names: Vec<&str> = pstr(p, "rotors", "I II III").split_whitespace().collect();
        if names.len() != 3 {
            return Err(CoreError::Parse("转子需要 3 个，例如 I II III".into()));
        }
        let rings = three(pstr(p, "ring", "AAA"));
        let poss = three(pstr(p, "position", "AAA"));
        let mut rotors: [Rotor; 3] = [
            build_rotor(names[0], rings[0], poss[0])?,
            build_rotor(names[1], rings[1], poss[1])?,
            build_rotor(names[2], rings[2], poss[2])?,
        ];
        let refl_name = pstr(p, "reflector", "B");
        let (_, refl) = REFLECTORS
            .iter()
            .find(|(n, _)| *n == refl_name)
            .ok_or_else(|| CoreError::Parse(format!("未知反射器: {refl_name}（B 或 C）")))?;
        let pb = build_plugboard(pstr(p, "plugboard", ""));

        let mut out = String::with_capacity(text.len());
        for ch in text.chars() {
            if !ch.is_ascii_alphabetic() {
                out.push(ch);
                continue;
            }
            step(&mut rotors);
            let mut c = (ch.to_ascii_uppercase() as u8 - b'A') as usize;
            c = pb[c];
            c = rotors[2].thru(c, true);
            c = rotors[1].thru(c, true);
            c = rotors[0].thru(c, true);
            c = (refl[c] - b'A') as usize;
            c = rotors[0].thru(c, false);
            c = rotors[1].thru(c, false);
            c = rotors[2].thru(c, false);
            c = pb[c];
            out.push((b'A' + c as u8) as char);
        }
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "enigma",
            CRYPTO,
            "Enigma 机",
            ROSE,
            vec![t_in()],
            vec![t_out()],
            vec![
                ParamSpec::text("rotors", "转子(左→右)", "I II III", false),
                ParamSpec::select("reflector", "反射器", &["B", "C"], "B"),
                ParamSpec::text("ring", "环设置(3字母)", "AAA", false),
                ParamSpec::text("position", "初始位置(3字母)", "AAA", false),
                ParamSpec::text("plugboard", "插线板(如 AB CD)", "", false),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}
