use std::fmt::Display;

pub struct PktLine<'a>(pub &'a str);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_prints_service_line_correctly() {
        let line = "# service=git-upload-pack\n";
        let pkt_line = PktLine(line);
        assert_eq!(pkt_line.to_string(), "001e# service=git-upload-pack\n");
    }
}
