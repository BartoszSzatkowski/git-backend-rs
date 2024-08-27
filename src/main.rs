use std::fmt::Display;
use std::fs;
use std::path::Component;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

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

#[derive(Debug)]
struct SimpleGitRef {
    hash: String,
    desc: String,
}

#[derive(Debug)]
struct SimpleRefResponse {
    refs: Vec<SimpleGitRef>,
}

fn main() {
    let mut res = Vec::new();
    for entry in WalkDir::new("./.git/refs")
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter(is_file)
    {
        let entry = entry.unwrap();
        let path = entry.path();
        let hash = fs::read_to_string(&path).unwrap();
        let mut new_path = path.components();
        while let Some(v) = new_path.next() {
            if let Component::Normal(v) = v {
                if v.as_encoded_bytes() == b".git" {
                    // Can be just `v == "bloo"`
                    break;
                }
            }
        }
        let desc = new_path.as_path();

        let git_ref = SimpleGitRef {
            desc: desc.to_string_lossy().to_string(),
            hash,
        };
        res.push(git_ref);
    }

    println!("{:?}", res);
}

fn is_file(dir_entry: &Result<walkdir::DirEntry, walkdir::Error>) -> bool {
    let Ok(dir_entry) = dir_entry else {
        return false;
    };
    let Ok(ent) = dir_entry.metadata() else {
        return false;
    };
    ent.is_file()
}
