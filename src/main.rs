use std::fmt::Display;

struct PktLine<'a>(&'a str);

impl Display for PktLine<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            return write!(f, "0000");
        }

        if self.0.len() > 65516 {
            panic!("Maximum payload for a PktLine exceeded");
        }

        write!(f, "{:04x}{}", self.0.len() + 4, self.0)
    }
}

fn main() {
    let p = PktLine("kiwi");
    println!("{}", p);
}
